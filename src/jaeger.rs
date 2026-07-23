use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{header, Client, StatusCode};
use serde_json::{json, Value};

use crate::config::Config;

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

impl JaegerClient {
    pub fn new(config: Config) -> Result<Self> {
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

    pub async fn get_services(&self) -> Result<Value> {
        let url = format!("{}/api/v3/services", self.base_url);
        let resp = self.http.get(url).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn get_operations(
        &self,
        service: &str,
        span_kind: Option<&str>,
    ) -> Result<Value> {
        let url = format!("{}/api/v3/operations", self.base_url);
        let mut params: Vec<(&str, String)> = vec![("service", service.to_string())];
        if let Some(kind) = span_kind {
            params.push(("span_kind", kind.to_lowercase()));
        }
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn get_trace(
        &self,
        trace_id: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Value> {
        let url = format!("{}/api/v3/traces/{}", self.base_url, trace_id);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(t) = start_time {
            params.push(("startTime", t.to_rfc3339()));
        }
        if let Some(t) = end_time {
            params.push(("endTime", t.to_rfc3339()));
        }
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    pub async fn find_traces(&self, q: &FindTracesQuery<'_>) -> Result<Value> {
        let url = format!("{}/api/v3/traces", self.base_url);
        let mut params: Vec<(String, String)> = vec![
            ("query.service_name".into(), q.service_name.to_string()),
            ("query.start_time_min".into(), q.start_time_min.to_rfc3339()),
            ("query.start_time_max".into(), q.start_time_max.to_rfc3339()),
        ];
        if let Some(op) = q.operation_name {
            params.push(("query.operation_name".into(), op.to_string()));
        }
        if let Some(d) = q.duration_min_ms {
            params.push(("query.duration_min".into(), format_duration_ms(d)));
        }
        if let Some(d) = q.duration_max_ms {
            params.push(("query.duration_max".into(), format_duration_ms(d)));
        }
        if let Some(depth) = q.search_depth {
            params.push(("query.search_depth".into(), depth.to_string()));
        }
        if let Some(attrs) = q.attributes {
            for (k, v) in attrs {
                params.push((format!("query.attributes[{}]", k), v.clone()));
            }
        }
        let resp = self.http.get(url).query(&params).send().await?;
        Self::json_or_empty(resp).await
    }

    async fn json_or_empty(resp: reqwest::Response) -> Result<Value> {
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(json!({}));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("jaeger request failed: {status} — {body}"));
        }
        let text = resp.text().await?;
        if text.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).context("failed to parse jaeger response as JSON")
    }
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
}
