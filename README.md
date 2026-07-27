# jaeger-mcp-server

An [MCP](https://modelcontextprotocol.io) server for [Jaeger](https://www.jaegertracing.io/) written in Rust, using the [`rmcp`](https://crates.io/crates/rmcp) SDK and the Jaeger v3 HTTP API.

Feature-parity port of the Node.js `jaeger-mcp-server`, with the following fixes baked in:

- `find-traces` sends `start_time_min` / `start_time_max` as RFC 3339 strings (was raw millis → HTTP 400).
- `duration_min` / `duration_max` are serialized as `google.protobuf.Duration` (`1.500s`), not `100ms`.
- `attributes` are forwarded to the API (`query.attributes[key]=value` for each entry).
- `find-traces` defaults to `search_depth = 20` when omitted.
- Auth is set once on the HTTP client rather than per request.

Only the **HTTP** protocol is implemented. gRPC (`tonic` + `prost`) is planned as a follow-up.

## Build

```bash
cargo build --release
```

Output: `target/release/jaeger-mcp-server` (`.exe` on Windows).

## Run

```bash
JAEGER_URL=http://localhost JAEGER_PORT=16686 ./target/release/jaeger-mcp-server
```

The process reads MCP JSON-RPC on stdin and writes it to stdout. All logs go to stderr.

## Environment variables

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `JAEGER_URL` | yes | — | Base URL of Jaeger (`http://host` or `https://host`). Scheme optional. |
| `JAEGER_PORT` | no | `16686` (or `443` for `https`) | Jaeger v3 HTTP API port. |
| `JAEGER_AUTHORIZATION_HEADER` | no | — | Value for the `Authorization` header (e.g. `Basic <base64>` or `Bearer <token>`). |
| `RUST_LOG` | no | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`). |

## VS Code / Copilot MCP config

```jsonc
{
  "servers": {
    "jaeger-mcp-server": {
      "type": "stdio",
      "command": "jaeger-mcp-server",
      "env": {
        "JAEGER_URL": "http://localhost",
        "JAEGER_PORT": "16686"
      }
    }
  }
}
```

## Tools exposed

- `get_services` — list all known service names.
- `get_operations` — list operations for a service, optionally filtered by span kind.
- `get_trace` — fetch a trace by 32-char hex id, with optional start/end time bounds.
- `find_traces` — search traces by service, operation, attributes, time window, and duration.

## Testing

```bash
cargo test
```

## License

MIT
