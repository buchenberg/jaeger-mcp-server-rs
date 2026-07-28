<#
.SYNOPSIS
    Windows installer for the jaeger-mcp-server (Rust binary).
.DESCRIPTION
    Downloads the latest (or a specific) pre-built release from GitHub,
    extracts the binary, installs it to a local directory, and optionally
    adds it to the user's PATH.
.PARAMETER Version
    Specific version tag to install (e.g. "v0.4.1").  Omit to fetch the
    latest release.
.PARAMETER InstallDir
    Directory where the binary will be placed.
    Default: $env:LOCALAPPDATA\jaeger-mcp-server
.PARAMETER NoPath
    Skip adding the install directory to the user's PATH.
.PARAMETER SkipVerify
    Skip the post-install validation (PE header check).
.EXAMPLE
    .\install.ps1
    Installs the latest release to %LOCALAPPDATA%\jaeger-mcp-server and
    adds it to PATH.
.EXAMPLE
    .\install.ps1 -Version v0.4.0 -NoPath
    Installs version 0.4.0 without modifying PATH.
.NOTES
    The jaeger-mcp-server requires at least the JAEGER_URL environment
    variable before it can be launched.  See the post-install prompts or
    README for details.
#>

param(
    [string]$Version,
    [string]$InstallDir = "$env:LOCALAPPDATA\jaeger-mcp-server",
    [switch]$NoPath,
    [switch]$SkipVerify
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# ── constants ────────────────────────────────────────────────────────────────
$Repo   = 'buchenberg/jaeger-mcp-server-rs'
$Target = 'x86_64-pc-windows-msvc'
$Bin    = 'jaeger-mcp-server.exe'

# ── helpers ──────────────────────────────────────────────────────────────────

function Write-Step($msg) {
    Write-Host "==> " -NoNewline -ForegroundColor Cyan
    Write-Host $msg
}

function Write-OK($msg) {
    Write-Host "  v " -NoNewline -ForegroundColor Green
    Write-Host $msg
}

function Write-Warn($msg) {
    Write-Host "  ! " -NoNewline -ForegroundColor Yellow
    Write-Host $msg
}

# ── pre-flight checks ────────────────────────────────────────────────────────

Write-Step 'Checking system …'

if ($env:PROCESSOR_ARCHITECTURE -notmatch 'AMD64|x86_64') {
    throw 'This installer only supports x86_64 (AMD64) Windows.  PRs welcome for ARM64!'
}
Write-OK "Architecture: x86_64"
Write-OK "PowerShell $($PSVersionTable.PSVersion)"

# ── version resolution ───────────────────────────────────────────────────────

if (-not $Version) {
    Write-Step 'Resolving latest release …'

    try {
        $req = [System.Net.WebRequest]::Create("https://github.com/$Repo/releases/latest")
        $req.AllowAutoRedirect = $false
        $req.UserAgent = 'jaeger-mcp-server-installer/1.0'
        $resp = $req.GetResponse()
        $location = $resp.Headers['Location']
        $resp.Close()

        if ($location -match '/tag/(?<tag>.+)$') {
            $Version = $Matches['tag']
        } else {
            throw 'Could not parse latest version from redirect URL.'
        }
    } catch {
        throw "Failed to resolve latest version: $_"
    }

    Write-OK "Latest version: $Version"
} else {
    Write-OK "Requested version: $Version"
}

$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/jaeger-mcp-server-$Target.tar.gz"
$TempDir     = Join-Path $env:TEMP "jaeger-mcp-server-$PID"
$Archive     = Join-Path $TempDir "jaeger-mcp-server-$Target.tar.gz"

# ── prepare directories ──────────────────────────────────────────────────────

Write-Step 'Preparing directories …'

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}
if (-not (Test-Path $TempDir)) {
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
}

Write-OK "Install dir: $InstallDir"

# ── download ─────────────────────────────────────────────────────────────────

Write-Step "Downloading $DownloadUrl …"

try {
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
    $wc = New-Object System.Net.WebClient
    $wc.Headers.Add('User-Agent', 'jaeger-mcp-server-installer/1.0')
    $wc.DownloadFile($DownloadUrl, $Archive)
} catch {
    throw "Download failed: $_"
}

$archiveSize = (Get-Item $Archive).Length
Write-OK "Downloaded $($archiveSize.ToString('N0')) bytes"

# ── extract ──────────────────────────────────────────────────────────────────

Write-Step 'Extracting …'

try {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    # Gunzip to a temp tar
    $TarFile = $Archive -replace '\.gz$', ''
    $srcStream  = [System.IO.File]::OpenRead($Archive)
    $destStream = [System.IO.File]::Create($TarFile)
    $gzStream   = New-Object System.IO.Compression.GZipStream($srcStream, [System.IO.Compression.CompressionMode]::Decompress)
    try {
        $gzStream.CopyTo($destStream)
    } finally {
        if ($gzStream)   { $gzStream.Dispose() }
        if ($destStream) { $destStream.Dispose() }
        if ($srcStream)  { $srcStream.Dispose() }
    }

    # Extract the exe from tar
    $tarStream = [System.IO.File]::OpenRead($TarFile)
    $tarReader = New-Object System.IO.BinaryReader($tarStream)
    try {
        while ($tarStream.Position -lt $tarStream.Length) {
            $header = $tarReader.ReadBytes(512)
            if ($header[0] -eq 0) { break }

            $name    = [System.Text.Encoding]::ASCII.GetString($header[0..99]).Trim([char]0)
            $sizeStr = [System.Text.Encoding]::ASCII.GetString($header[124..135]).Trim([char]0)
            $size    = [Convert]::ToInt64($sizeStr, 8)

            if ($name -eq $Bin) {
                $data     = $tarReader.ReadBytes([int]$size)
                $destPath = Join-Path $InstallDir $Bin
                [System.IO.File]::WriteAllBytes($destPath, $data)
                Write-OK "Extracted $Bin"
            }

            $padding = (512 - ($size % 512)) % 512
            $tarStream.Seek($padding, [System.IO.SeekOrigin]::Current) | Out-Null
        }
    } finally {
        if ($tarReader) { $tarReader.Dispose() }
        if ($tarStream) { $tarStream.Dispose() }
    }
} catch {
    throw "Extraction failed: $_"
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

$exePath = Join-Path $InstallDir $Bin
Write-OK "Binary: $exePath"

# ── PATH ─────────────────────────────────────────────────────────────────────

if (-not $NoPath) {
    Write-Step 'Checking PATH …'

    $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $currentPath) { $currentPath = '' }

    if ($currentPath -split ';' -contains $InstallDir) {
        Write-OK 'Already in user PATH'
    } else {
        $newPath = if ($currentPath) { "$currentPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$env:Path;$InstallDir"
        Write-OK 'Added to user PATH'
    }
}

# ── smoke test ───────────────────────────────────────────────────────────────

if (-not $SkipVerify) {
    Write-Step 'Verifying installation …'

    if (-not (Test-Path $exePath)) {
        throw "Binary not found at $exePath after extraction."
    }

    $bytes = [System.IO.File]::ReadAllBytes($exePath)
    if ($bytes.Length -lt 2) {
        throw "Binary at $exePath appears empty or truncated."
    }

    if ($bytes[0] -eq 0x4D -and $bytes[1] -eq 0x5A) {
        Write-OK "Valid PE binary — $($bytes.Length.ToString('N0')) bytes"
    } else {
        Write-Warn "Missing MZ header; file may not be a valid executable."
    }
}

# ── final message ────────────────────────────────────────────────────────────

# Determine the command string to show: short name if on PATH, full path otherwise
if ($NoPath) {
    $cmdRef = "`"$exePath`""
} else {
    $cmdRef = 'jaeger-mcp-server'
}

Write-Host ''
Write-Host '===========================================================' -ForegroundColor Cyan
Write-Host '  jaeger-mcp-server installed successfully!'               -ForegroundColor White
Write-Host '===========================================================' -ForegroundColor Cyan
Write-Host ''
Write-Host 'Required environment variable:' -ForegroundColor Yellow
Write-Host '  JAEGER_URL     - base URL of your Jaeger query service'
Write-Host ''
Write-Host 'Optional environment variables:' -ForegroundColor Yellow
Write-Host '  JAEGER_PORT                  - override default port (16686 / 443)'
Write-Host '  JAEGER_AUTHORIZATION_HEADER  - value for Authorization header'
Write-Host '  RUST_LOG                     - log level (default: info)'
Write-Host ''
Write-Host 'Quick start:' -ForegroundColor Yellow
Write-Host '  1. Set JAEGER_URL permanently for your user account:'
Write-Host '     [Environment]::SetEnvironmentVariable("JAEGER_URL","http://your-jaeger-host","User")'
Write-Host ''
Write-Host '  2. Add it to your MCP client config (Claude Desktop, etc.):'
Write-Host '     {'
Write-Host "       `"command`": $cmdRef,"
Write-Host '       "env": {'
Write-Host '         "JAEGER_URL": "http://your-jaeger-host",'
Write-Host '         "JAEGER_PORT": "16686"'
Write-Host '       }'
Write-Host '     }'
Write-Host ''
Write-Host "Binary installed to: $exePath" -ForegroundColor Green
Write-Host ''

if (-not $NoPath) {
    Write-Host 'NOTE: PATH was updated. Restart your terminal for it to take'
    Write-Host 'effect in new shells.'
    Write-Host ''
}
