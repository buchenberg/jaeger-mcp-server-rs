mod config;
mod jaeger;
mod server;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use crate::server::JaegerMcp;

#[tokio::main]
async fn main() -> Result<()> {
    // MCP uses stdout for JSON-RPC — send all logs to stderr.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("starting jaeger-mcp-server");

    let handler = JaegerMcp::from_env()?;
    let service = handler.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
