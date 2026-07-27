use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::{header, Client, StatusCode};
use serde_json::{json, Value};
use thiserror::Error;

use crate::config::Config;

const API_SERVICES: &str = "/api/v3/services";
const API_OPERATIONS: &str = "/api/v3/operations";
const API_TRACES: &str = "/api/v3/traces";

/// Thin wrapper around the Jaeger v3 HTTP API.
///
/// Notes on the wire format:
/// * Timestamps map to `google.protobuf.Timestamp` and must be RFC 3339 strings
///   (`2026-07-22T00:00:00Z`). Passing raw milliseconds returns HTTP 400.
/// * Durations map to `google.protobuf.Duration` whose JSON form is seconds with
///   an `s` suffix (`1.500s`). `100ms` is *not* accepted by the gateway.
/// * Map fields (like `attributes`) are encoded as repeated
///   `query.attributes[key]=value` query parameters.
#[derive(Debug, Clone)]
pub struct JaegerClient {
    http: Client,
    base_url: String,
}

/// Errors produced by Jaeger API operations.
#[derive(Error, Debug)]
pub enum JaegerError {
    /// An HTTP request error occurred.
    #[error("jaeger request failed: {0}")]
    Request(#[from] reqwest::Error),
    /// The HTTP response indicates a failure.
    #[error("jaeger request failed: {status} — {body}")]
    RequestFailed { status: reqwest::StatusCode, body: String },
    /// The response could not be parsed as JSON.
    #[error("failed to parse jaeger response as JSON")]
    ParseError(#[from] serde_json::Error),
    /// The trace ID is not a valid 32-character hex string.
    #[error("invalid trace_id: must be a 32-character hex string")]
    InvalidTraceId,
}

impl JaegerClient {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let mut headers = header::HeaderMap::new();
        if let Some(auth) = config.authorization_header.as_deref() {
            let value = header::HeaderValue::from_str(auth)
                .context("invalid JAEGER_AUTHORIZATION_HEADER value")?;
            headers.insert(header::AUTHORIZATION, value);
        }
        let http = Client::builder()
            .default_headers(headers)
            .user_agent(concat!("jaeger-mcp-server/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            base_url: config.base_url(),
        })
    }

    pub async fn get_services(&self) -> Result<Value, JaegerError> {
        let url = format!("{}{API_SERVICES}", self.base_url);
        let resp = self.http.get(url).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn get_operations(
        &self,
        service: &str,
        span_kind: Option<&str>,
    ) -> Result<Value, JaegerError> {
        let url = format!("{}{API_OPERATIONS}", self.base_url);
        let params = self.build_query_params(|p| {
            p.push(("service".to_string(), service.to_string()));
            if let Some(kind) = span_kind {
                p.push(("span_kind".to_string(), kind.to_lowercase()));
            }
        });
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn get_trace(
        &self,
        trace_id: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Value, JaegerError> {
        if !is_valid_trace_id(trace_id) {
            return Err(JaegerError::InvalidTraceId);
        }
        let url = format!("{}{API_TRACES}/{trace_id}", self.base_url);
        let params = self.build_query_params(|p| {
            if let Some(t) = start_time {
                p.push(("startTime".to_string(), t.to_rfc3339()));
            }
            if let Some(t) = end_time {
                p.push(("endTime".to_string(), t.to_rfc3339()));
            }
        });
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn find_traces(&self, q: &FindTracesQuery<'_>) -> Result<Value, JaegerError> {
        let url = format!("{}{API_TRACES}", self.base_url);
        let params = self.build_query_params(|p| {
            p.push(("query.service_name".to_string(), q.service_name.to_string()));
            p.push(("query.start_time_min".to_string(), q.start_time_min.to_rfc3339()));
            p.push(("query.start_time_max".to_string(), q.start_time_max.to_rfc3339()));
            if let Some(op) = q.operation_name {
                p.push(("query.operation_name".to_string(), op.to_string()));
            }
            if let Some(d) = q.duration_min_ms {
                p.push(("query.duration_min".to_string(), format_duration_ms(d)));
            }
            if let Some(d) = q.duration_max_ms {
                p.push(("query.duration_max".to_string(), format_duration_ms(d)));
            }
            if let Some(depth) = q.search_depth {
                p.push(("query.search_depth".to_string(), depth.to_string()));
            }
            if let Some(attrs) = q.attributes {
                for (k, v) in attrs {
                    p.push((format!("query.attributes[{k}]"), v.clone()));
                }
            }
        });
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    fn build_query_params<F>(&self, f: F) -> Vec<(String, String)>
    where
        F: FnOnce(&mut Vec<(String, String)>),
    {
        let mut params: Vec<(String, String)> = Vec::new();
        f(&mut params);
        params
    }

    async fn json_or_empty(resp: reqwest::Response) -> Result<Value, JaegerError> {
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(json!({}));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(JaegerError::RequestFailed { status, body });
        }
        let text = resp.text().await?;
        if text.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).map_err(JaegerError::from)
    }
}

fn is_valid_trace_id(trace_id: &str) -> bool {
    trace_id.len() == 32 && trace_id.chars().all(|c| c.is_ascii_hexdigit())
}

pub struct FindTracesQuery<'a> {
    pub service_name: &'a str,
    pub operation_name: Option<&'a str>,
    pub attributes: Option<&'a HashMap<String, String>>,
    pub start_time_min: DateTime<Utc>,
    pub start_time_max: DateTime<Utc>,
    pub duration_min_ms: Option<f64>,
    pub duration_max_ms: Option<f64>,
    pub search_depth: Option<u32>,
}

/// Google protobuf `Duration` JSON serialization: seconds with `s` suffix.
fn format_duration_ms(ms: f64) -> String {
    format!("{:.3}s", ms / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_is_seconds_with_suffix() {
        assert_eq!(format_duration_ms(100.0), "0.100s");
        assert_eq!(format_duration_ms(1_500.0), "1.500s");
    }

    #[test]
    fn duration_zero() {
        assert_eq!(format_duration_ms(0.0), "0.000s");
    }

    #[test]
    fn duration_large_value() {
        assert_eq!(format_duration_ms(3_600_000.0), "3600.000s");
    }

    #[test]
    fn valid_trace_id_accepts_32_hex() {
        assert!(is_valid_trace_id("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"));
    }

    #[test]
    fn invalid_trace_id_rejects_short() {
        assert!(!is_valid_trace_id("abc123"));
    }

    #[test]
    fn invalid_trace_id_rejects_non_hex() {
        assert!(!is_valid_trace_id("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
    }

    #[test]
    fn invalid_trace_id_rejects_empty() {
        assert!(!is_valid_trace_id(""));
    }
}
