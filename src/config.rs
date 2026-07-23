use anyhow::{anyhow, Result};

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Normalized base URL, e.g. `http://localhost` or `https://jaeger.example.com`.
    pub url: String,
    /// Port for the Jaeger v3 HTTP API.
    pub port: u16,
    /// Optional value for the `Authorization` header on every request.
    pub authorization_header: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let raw_url = std::env::var("JAEGER_URL")
            .map_err(|_| anyhow!("JAEGER_URL environment variable is required"))?;

        let url = normalize_url(&raw_url);
        let port = match std::env::var("JAEGER_PORT") {
            Ok(v) => v
                .parse::<u16>()
                .map_err(|e| anyhow!("invalid JAEGER_PORT `{v}`: {e}"))?,
            Err(_) => default_port(&url),
        };

        let authorization_header = std::env::var("JAEGER_AUTHORIZATION_HEADER").ok();

        Ok(Self {
            url,
            port,
            authorization_header,
        })
    }

    /// Returns `<scheme>://<host>:<port>`.
    pub fn base_url(&self) -> String {
        format!("{}:{}", self.url, self.port)
    }
}

fn normalize_url(url: &str) -> String {
    if url.contains("://") {
        url.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", url.trim_end_matches('/'))
    }
}

fn default_port(url: &str) -> u16 {
    if url.starts_with("https://") {
        443
    } else {
        16686
    }
}
