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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            health_path: "/health".to_string(),
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        if let Ok(addr) = std::env::var("SDKWORK_BIND_ADDRESS") {
            config.bind_address = addr;
        }
        if let Ok(port) = std::env::var("SDKWORK_PORT") {
            config.port = port.parse()?;
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

        Ok(config)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, "info");
        assert!(config.cors_enabled);
    }

    #[test]
    fn bind_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn base_url() {
        let config = ServerConfig::default();
        assert_eq!(config.base_url(), "http://localhost:8080");
    }
}
