#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const { version } = require('./package.json');

const REPO = 'buchenberg/jaeger-mcp-server-rs';

const TARGETS = {
  'win32-x64':    'x86_64-pc-windows-msvc',
  'linux-x64':    'x86_64-unknown-linux-musl',
  'darwin-x64':   'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
};

const key = `${process.platform}-${process.arch}`;
const target = TARGETS[key];

if (!target) {
  console.error(
    `[jaeger-mcp-server] Unsupported platform: ${key}.\n` +
    `Build from source: https://github.com/${REPO}`
  );
  process.exit(1);
}

const BIN_EXT = process.platform === 'win32' ? '.exe' : '';
const BIN_NAME = `jaeger-mcp-server${BIN_EXT}`;
const ARCHIVE_NAME = `jaeger-mcp-server-${target}.tar.gz`;
const DOWNLOAD_URL = `https://github.com/${REPO}/releases/download/v${version}/${ARCHIVE_NAME}`;
const BIN_DIR = path.join(__dirname, 'bin');
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);
const ARCHIVE_PATH = path.join(BIN_DIR, ARCHIVE_NAME);

if (fs.existsSync(BIN_PATH)) {
  process.exit(0);
}

fs.mkdirSync(BIN_DIR, { recursive: true });

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (u) => {
      https.get(u, { headers: { 'User-Agent': 'jaeger-mcp-server-installer' } }, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return follow(res.headers.location);
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} downloading ${u}`));
          return;
        }
        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
        file.on('error', reject);
        res.on('error', reject);
      }).on('error', reject);
    };
    follow(url);
  });
}

async function main() {
  console.log(`[jaeger-mcp-server] Downloading binary for ${key} ...`);
  console.log(`[jaeger-mcp-server] ${DOWNLOAD_URL}`);
  await download(DOWNLOAD_URL, ARCHIVE_PATH);

  console.log(`[jaeger-mcp-server] Extracting ...`);
  const result = spawnSync('tar', ['xzf', ARCHIVE_PATH, '-C', BIN_DIR], { stdio: 'inherit' });
  if (result.status !== 0) {
    console.error('[jaeger-mcp-server] Extraction failed.');
    process.exit(1);
  }

  fs.unlinkSync(ARCHIVE_PATH);

  if (process.platform !== 'win32') {
    fs.chmodSync(BIN_PATH, 0o755);
  }

  console.log(`[jaeger-mcp-server] Installed to ${BIN_PATH}`);
}

main().catch((err) => {
  console.error(`[jaeger-mcp-server] Install failed: ${err.message}`);
  process.exit(1);
});
