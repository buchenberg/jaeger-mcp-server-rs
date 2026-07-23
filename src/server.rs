use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    ErrorData, ServerHandler,
};
use serde::Deserialize;

use crate::{
    config::Config,
    jaeger::{FindTracesQuery, JaegerClient},
};

#[derive(Clone)]
pub struct JaegerMcp {
    client: JaegerClient,
    tool_router: ToolRouter<Self>,
}

impl JaegerMcp {
    pub fn from_env() -> Result<Self> {
        let client = JaegerClient::new(Config::from_env()?)?;
        Ok(Self {
            client,
            tool_router: Self::tool_router(),
        })
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetOperationsArgs {
    /// Filters operations by service name.
    pub service: String,
    /// Filters operations by OpenTelemetry span kind (`server`, `client`, `producer`, `consumer`, `internal`).
    #[serde(default)]
    pub span_kind: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetTraceArgs {
    /// OpenTelemetry-compatible trace id, 32-character hex string.
    pub trace_id: String,
    /// Start time bound in RFC 3339 format (optional).
    #[serde(default)]
    pub start_time: Option<DateTime<Utc>>,
    /// End time bound in RFC 3339 format (optional).
    #[serde(default)]
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FindTracesArgs {
    /// Filters spans by service name.
    pub service_name: String,
    /// Filters spans by operation / span name (optional).
    #[serde(default)]
    pub operation_name: Option<String>,
    /// Attribute equality filters. Scalar values are stringified.
    #[serde(default)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
    /// Start of the interval (inclusive), RFC 3339.
    pub start_time_min: DateTime<Utc>,
    /// End of the interval (exclusive), RFC 3339.
    pub start_time_max: DateTime<Utc>,
    /// Minimum span duration in milliseconds (optional).
    #[serde(default)]
    pub duration_min: Option<f64>,
    /// Maximum span duration in milliseconds (optional).
    #[serde(default)]
    pub duration_max: Option<f64>,
    /// Maximum number of traces to return. Defaults to 20 when omitted.
    #[serde(default)]
    pub search_depth: Option<u32>,
}

#[tool_router]
impl JaegerMcp {
    #[tool(description = "Gets the service names as JSON array of string")]
    async fn get_services(&self) -> Result<CallToolResult, ErrorData> {
        let value = self.client.get_services().await.map_err(err)?;
        Ok(CallToolResult::success(vec![Content::text(value.to_string())]))
    }

    #[tool(
        description = "Gets the operations as JSON array of object with `name` and `spanKind` properties"
    )]
    async fn get_operations(
        &self,
        Parameters(args): Parameters<GetOperationsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .get_operations(&args.service, args.span_kind.as_deref())
            .await
            .map_err(err)?;
        Ok(CallToolResult::success(vec![Content::text(value.to_string())]))
    }

    #[tool(
        description = "Gets the spans by the given trace by ID as JSON in the OpenTelemetry resource spans format"
    )]
    async fn get_trace(
        &self,
        Parameters(args): Parameters<GetTraceArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .get_trace(&args.trace_id, args.start_time, args.end_time)
            .await
            .map_err(err)?;
        Ok(CallToolResult::success(vec![Content::text(value.to_string())]))
    }

    #[tool(
        description = "Searches spans and returns them in the OpenTelemetry resource spans format"
    )]
    async fn find_traces(
        &self,
        Parameters(args): Parameters<FindTracesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let attributes = args.attributes.as_ref().map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), json_scalar_to_string(v)))
                .collect::<HashMap<String, String>>()
        });

        let query = FindTracesQuery {
            service_name: &args.service_name,
            operation_name: args.operation_name.as_deref(),
            attributes: attributes.as_ref(),
            start_time_min: args.start_time_min,
            start_time_max: args.start_time_max,
            duration_min_ms: args.duration_min,
            duration_max_ms: args.duration_max,
            search_depth: args.search_depth.or(Some(20)),
        };

        let value = self.client.find_traces(&query).await.map_err(err)?;
        Ok(CallToolResult::success(vec![Content::text(value.to_string())]))
    }
}

#[tool_handler]
impl ServerHandler for JaegerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "MCP server for querying Jaeger traces via the v3 HTTP API".into(),
            ),
            ..Default::default()
        }
    }
}

fn err(e: anyhow::Error) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

fn json_scalar_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
