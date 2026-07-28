use anyhow::Result;
use thiserror::Error;

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

/// Errors produced when loading configuration from environment variables.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// The `JAEGER_URL` environment variable is not set.
    #[error("JAEGER_URL environment variable is required")]
    MissingUrl,
    /// The `JAEGER_PORT` value cannot be parsed as a u16.
    #[error("invalid JAEGER_PORT `{0}`: {1}")]
    InvalidPort(String, anyhow::Error),
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let raw_url = std::env::var("JAEGER_URL").map_err(|_| ConfigError::MissingUrl)?;

        let url = normalize_url(&raw_url);
        let port = match std::env::var("JAEGER_PORT") {
            Ok(v) => v
                .parse::<u16>()
                .map_err(|e| ConfigError::InvalidPort(v.clone(), anyhow::Error::msg(e.to_string())))?,
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

/// Strips trailing slashes and ensures a scheme prefix.
fn normalize_url(url: &str) -> String {
    if url.contains("://") {
        url.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", url.trim_end_matches('/'))
    }
}

/// Returns the default Jaeger port for a given URL scheme.
fn default_port(url: &str) -> u16 {
    if url.starts_with("https://") {
        443
    } else {
        16686
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_adds_http_when_no_scheme() {
        assert_eq!(normalize_url("localhost"), "http://localhost");
    }

    #[test]
    fn normalize_url_preserves_existing_scheme() {
        assert_eq!(normalize_url("https://jaeger.example.com"), "https://jaeger.example.com");
    }

    #[test]
    fn normalize_url_strips_trailing_slash() {
        assert_eq!(normalize_url("http://localhost/"), "http://localhost");
    }

    #[test]
    fn normalize_url_strips_trailing_slash_with_scheme() {
        assert_eq!(normalize_url("https://jaeger.example.com/"), "https://jaeger.example.com");
    }

    #[test]
    fn default_port_returns_16686_for_http() {
        assert_eq!(default_port("http://localhost"), 16686);
    }

    #[test]
    fn default_port_returns_443_for_https() {
        assert_eq!(default_port("https://jaeger.example.com"), 443);
    }

    #[test]
    fn default_port_returns_16686_for_unknown_scheme() {
        assert_eq!(default_port("ftp://localhost"), 16686);
    }
}
