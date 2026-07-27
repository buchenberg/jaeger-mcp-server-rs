//! jaeger-mcp-server
//!
//! An MCP server for querying Jaeger traces via the v3 HTTP API.
//!
//! The server reads MCP JSON-RPC on stdin and writes responses to stdout.
//! All logs are sent to stderr.

mod config;
mod jaeger;
mod server;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use crate::server::JaegerMcp;

#[tokio::main]
async fn main() -> Result<()> {
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
