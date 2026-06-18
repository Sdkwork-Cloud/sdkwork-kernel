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
            database_path: "./data/agent-server.sqlite".to_string(),
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
