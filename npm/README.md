# jaeger-mcp-server-rs

An [MCP](https://modelcontextprotocol.io) server for [Jaeger](https://www.jaegertracing.io/) written in Rust. Distributed as a prebuilt native binary — no Node.js runtime required at execution time.

## Installation

```bash
npx -y jaeger-mcp-server-rs
```

Or install globally:

```bash
npm install -g jaeger-mcp-server-rs
jaeger-mcp-server
```

On install, the postinstall script downloads the prebuilt binary for your platform from GitHub Releases. Supported platforms:

| Platform | Architecture |
|---|---|
| Windows | x64 |
| Linux | x64 (musl) |
| macOS | x64, arm64 |

## VS Code / Copilot MCP config

```jsonc
{
  "servers": {
    "jaeger-mcp-server": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "jaeger-mcp-server-rs"],
      "env": {
        "JAEGER_URL": "http://localhost",
        "JAEGER_PORT": "16686"
      }
    }
  }
}
```

## Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `JAEGER_URL` | yes | — | Base URL of Jaeger (`http://host` or `https://host`). |
| `JAEGER_PORT` | no | `16686` | Jaeger v3 HTTP API port. |
| `JAEGER_AUTHORIZATION_HEADER` | no | — | Value for the `Authorization` header (e.g. `Bearer <token>`). |
| `RUST_LOG` | no | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`). |

## Tools

- `get-services` — list all known service names.
- `get-operations` — list operations for a service, optionally filtered by span kind.
- `get-trace` — fetch a trace by 32-char hex ID, with optional time bounds.
- `find-traces` — search traces by service, operation, attributes, time window, and duration.

## Source

[github.com/buchenberg/jaeger-mcp-server-rs](https://github.com/buchenberg/jaeger-mcp-server-rs)

## License

MIT
