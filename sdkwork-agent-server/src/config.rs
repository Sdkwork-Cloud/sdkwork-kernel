use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Max request body size in bytes
    pub max_body_size: usize,
    /// Health check endpoint path
    pub health_path: String,
    /// SQLite database path for session persistence
    pub database_path: String,
    /// Runtime environment: development | production
    pub environment: String,
    /// Ingress auth mode: open | token
    pub ingress_auth_mode: String,
    /// Required bearer/static token when ingress_auth_mode is token
    pub ingress_token: Option<String>,
    /// Per-client request rate limit (requests/sec). Zero disables limiting.
    pub rate_limit_rps: u32,
    /// Burst capacity for the token-bucket rate limiter.
    pub rate_limit_burst: u32,
    /// SSE/streaming request timeout in seconds (long-lived connections).
    pub sse_request_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            cors_enabled: true,
            cors_origins: vec![
                "http://127.0.0.1:5173".to_string(),
                "http://localhost:5173".to_string(),
            ],
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            health_path: "/health".to_string(),
            database_path: "./data/agent-server.sqlite".to_string(),
            environment: "development".to_string(),
            ingress_auth_mode: "open".to_string(),
            ingress_token: None,
            rate_limit_rps: 0,
            rate_limit_burst: 200,
            sse_request_timeout_secs: 3600,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        if let Ok(bind) = std::env::var("SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND") {
            let (bind_address, port) =
                parse_ingress_bind("SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND", &bind)?;
            config.bind_address = bind_address;
            config.port = port;
        } else {
            if let Ok(addr) = std::env::var("SDKWORK_BIND_ADDRESS") {
                config.bind_address = addr;
            }
            if let Ok(port) = std::env::var("SDKWORK_PORT") {
                config.port = port.parse()?;
            }
        }
        if let Ok(level) = std::env::var("SDKWORK_LOG_LEVEL") {
            config.log_level = level;
        }
        if let Ok(cors) = std::env::var("SDKWORK_CORS_ENABLED") {
            config.cors_enabled = cors.parse().unwrap_or(true);
        }
        if let Ok(origins) = std::env::var("SDKWORK_CORS_ORIGINS") {
            config.cors_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(timeout) = std::env::var("SDKWORK_REQUEST_TIMEOUT") {
            config.request_timeout_secs = timeout.parse()?;
        }
        if let Ok(database_path) = std::env::var("SDKWORK_DATABASE_PATH") {
            config.database_path = database_path;
        }
        if let Ok(environment) = std::env::var("SDKWORK_KERNEL_ENVIRONMENT") {
            config.environment = environment;
        }
        if let Ok(auth_mode) = std::env::var("SDKWORK_KERNEL_INGRESS_AUTH_MODE") {
            config.ingress_auth_mode = auth_mode;
        }
        if let Ok(token) = std::env::var("SDKWORK_KERNEL_INGRESS_TOKEN") {
            let trimmed = token.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_token = Some(trimmed);
            }
        }
        if let Ok(rps) = std::env::var("SDKWORK_RATE_LIMIT_RPS") {
            config.rate_limit_rps = rps.parse().unwrap_or(0);
        }
        if let Ok(burst) = std::env::var("SDKWORK_RATE_LIMIT_BURST") {
            config.rate_limit_burst = burst.parse().unwrap_or(200);
        }
        if let Ok(sse_timeout) = std::env::var("SDKWORK_SSE_REQUEST_TIMEOUT") {
            config.sse_request_timeout_secs = sse_timeout.parse()?;
        }
        if config.environment.eq_ignore_ascii_case("production")
            && config.ingress_auth_mode.eq_ignore_ascii_case("open")
        {
            config.ingress_auth_mode = "token".to_string();
        }
        config.normalize_security();

        Ok(config)
    }

    fn normalize_security(&mut self) {
        if self.environment.eq_ignore_ascii_case("production")
            && self.cors_origins.iter().any(|origin| origin == "*")
        {
            self.cors_origins = vec![
                "https://kernel.sdkwork.com".to_string(),
                "https://app.sdkwork.com".to_string(),
            ];
        }
        if self.environment.eq_ignore_ascii_case("production") && self.rate_limit_rps == 0 {
            self.rate_limit_rps = 100;
        }
        if self.rate_limit_rps > 0 && self.rate_limit_burst == 0 {
            self.rate_limit_burst = self.rate_limit_rps.saturating_mul(2);
        }
    }

    pub fn is_development(&self) -> bool {
        self.environment.eq_ignore_ascii_case("development")
    }

    /// When true, typed provider failures fall back to the bridge mock path in development.
    pub fn allow_mock_provider_fallback(&self) -> bool {
        if self.environment.eq_ignore_ascii_case("production") {
            return false;
        }
        if let Ok(value) = std::env::var("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS") {
            return matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
        }
        cfg!(debug_assertions)
    }

    /// Get the full bind address (address:port)
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }

    /// Get the base URL for the server
    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
}

fn parse_ingress_bind(env_name: &str, bind: &str) -> anyhow::Result<(String, u16)> {
    let trimmed = bind.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{env_name} cannot be empty");
    }

    if let Ok(socket_addr) = trimmed.parse::<std::net::SocketAddr>() {
        return Ok((socket_addr.ip().to_string(), socket_addr.port()));
    }

    let (host, port) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow::anyhow!("{env_name} must be host:port or a socket address"))?;
    let port = port
        .parse::<u16>()
        .map_err(|error| anyhow::anyhow!("{env_name} port is invalid: {error}"))?;
    Ok((host.to_string(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, "info");
        assert!(config.cors_enabled);
        assert!(!config.cors_origins.iter().any(|origin| origin == "*"));
    }

    #[test]
    fn bind_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn base_url() {
        let config = ServerConfig::default();
        assert_eq!(config.base_url(), "http://localhost:8080");
    }

    #[test]
    fn parse_ingress_bind_host_port() {
        let (host, port) = parse_ingress_bind(
            "SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND",
            "127.0.0.1:18280",
        )
        .expect("host:port bind should parse");
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 18280);
    }
}
