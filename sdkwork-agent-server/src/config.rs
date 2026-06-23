use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-tenant rate limit override for commercial quota profiles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantRateLimitOverride {
    pub rps: u32,
    pub burst: u32,
}

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
    /// SQLite database path for session persistence when runtime engine is sqlite.
    pub database_path: String,
    /// Runtime session database engine: sqlite | postgres
    pub runtime_database_engine: String,
    /// Deployment hosting profile (for example cloud-hosted, self-hosted).
    pub kernel_hosting: Option<String>,
    /// Runtime target (for example server, desktop).
    pub kernel_runtime_target: Option<String>,
    /// Runtime environment: development | production
    pub environment: String,
    /// Ingress auth mode: open | token | jwt
    pub ingress_auth_mode: String,
    /// Required bearer/static token when ingress_auth_mode is token
    pub ingress_token: Option<String>,
    /// HS256 secret for ingress_auth_mode jwt
    pub ingress_jwt_secret: Option<String>,
    /// Optional issuer (`iss`) for ingress JWT validation
    pub ingress_jwt_issuer: Option<String>,
    /// Optional audience (`aud`) for ingress JWT validation
    pub ingress_jwt_audience: Option<String>,
    /// Ingress JWT signing algorithm profile: hs256 | rs256 (when not using JWKS file)
    pub ingress_jwt_algorithm: String,
    /// RS256 public key PEM for ingress JWT validation
    pub ingress_jwt_rsa_public_key_pem: Option<String>,
    /// Local JWKS JSON file for RS256 ingress JWT validation (kid lookup)
    pub ingress_jwt_jwks_file: Option<String>,
    /// Remote JWKS URL fetched once at startup for RS256 ingress JWT validation (kid lookup)
    pub ingress_jwt_jwks_url: Option<String>,
    /// Fixed tenant identity for bound ingress identity mode (non-loopback token deployments).
    pub ingress_bound_tenant_id: Option<String>,
    /// Fixed user identity for bound ingress identity mode (non-loopback token deployments).
    pub ingress_bound_user_id: Option<String>,
    /// Per-client request rate limit (requests/sec). Zero disables limiting.
    pub rate_limit_rps: u32,
    /// Burst capacity for the token-bucket rate limiter.
    pub rate_limit_burst: u32,
    /// Redis URL for distributed rate limiting (`redis_cache` profile).
    pub rate_limit_redis_url: Option<String>,
    /// Optional per-tenant rate limit overrides keyed by tenant id.
    pub tenant_rate_limit_overrides: HashMap<String, TenantRateLimitOverride>,
    /// Metrics scrape auth mode: open | token
    pub metrics_auth_mode: String,
    /// Bearer token for `/metrics` when metrics_auth_mode is token.
    pub metrics_token: Option<String>,
    /// Optional OTLP HTTP endpoint for distributed tracing export.
    pub otel_exporter_otlp_endpoint: Option<String>,
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
            runtime_database_engine: "sqlite".to_string(),
            kernel_hosting: None,
            kernel_runtime_target: None,
            environment: "development".to_string(),
            ingress_auth_mode: "open".to_string(),
            ingress_token: None,
            ingress_jwt_secret: None,
            ingress_jwt_issuer: None,
            ingress_jwt_audience: None,
            ingress_jwt_algorithm: "hs256".to_string(),
            ingress_jwt_rsa_public_key_pem: None,
            ingress_jwt_jwks_file: None,
            ingress_jwt_jwks_url: None,
            ingress_bound_tenant_id: None,
            ingress_bound_user_id: None,
            rate_limit_rps: 0,
            rate_limit_burst: 200,
            rate_limit_redis_url: None,
            tenant_rate_limit_overrides: HashMap::new(),
            metrics_auth_mode: "open".to_string(),
            metrics_token: None,
            otel_exporter_otlp_endpoint: None,
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
        if let Ok(engine) = std::env::var("SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE") {
            config.runtime_database_engine = engine;
        }
        if let Ok(hosting) = std::env::var("SDKWORK_KERNEL_HOSTING") {
            let trimmed = hosting.trim().to_string();
            if !trimmed.is_empty() {
                config.kernel_hosting = Some(trimmed);
            }
        }
        if let Ok(target) = std::env::var("SDKWORK_KERNEL_RUNTIME_TARGET") {
            let trimmed = target.trim().to_string();
            if !trimmed.is_empty() {
                config.kernel_runtime_target = Some(trimmed);
            }
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
        if let Ok(tenant_id) = std::env::var("SDKWORK_KERNEL_INGRESS_BOUND_TENANT_ID") {
            let trimmed = tenant_id.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_bound_tenant_id = Some(trimmed);
            }
        }
        if let Ok(user_id) = std::env::var("SDKWORK_KERNEL_INGRESS_BOUND_USER_ID") {
            let trimmed = user_id.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_bound_user_id = Some(trimmed);
            }
        }
        if let Ok(secret) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_SECRET") {
            let trimmed = secret.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_secret = Some(trimmed);
            }
        }
        if let Ok(issuer) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_ISSUER") {
            let trimmed = issuer.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_issuer = Some(trimmed);
            }
        }
        if let Ok(audience) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_AUDIENCE") {
            let trimmed = audience.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_audience = Some(trimmed);
            }
        }
        if let Ok(algorithm) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_ALGORITHM") {
            let trimmed = algorithm.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_algorithm = trimmed;
            }
        }
        if let Ok(pem) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_RSA_PUBLIC_KEY_PEM") {
            let trimmed = pem.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_rsa_public_key_pem = Some(trimmed);
            }
        }
        if let Ok(jwks_file) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_JWKS_FILE") {
            let trimmed = jwks_file.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_jwks_file = Some(trimmed);
            }
        }
        if let Ok(jwks_url) = std::env::var("SDKWORK_KERNEL_INGRESS_JWT_JWKS_URL") {
            let trimmed = jwks_url.trim().to_string();
            if !trimmed.is_empty() {
                config.ingress_jwt_jwks_url = Some(trimmed);
            }
        }
        if let Ok(overrides) = std::env::var("SDKWORK_TENANT_RATE_LIMIT_OVERRIDES") {
            let trimmed = overrides.trim();
            if !trimmed.is_empty() {
                config.tenant_rate_limit_overrides =
                    serde_json::from_str(trimmed).map_err(|error| {
                        anyhow::anyhow!(
                            "SDKWORK_TENANT_RATE_LIMIT_OVERRIDES must be JSON object: {error}"
                        )
                    })?;
            }
        }
        if let Ok(rps) = std::env::var("SDKWORK_RATE_LIMIT_RPS") {
            config.rate_limit_rps = rps.parse().unwrap_or(0);
        }
        if let Ok(burst) = std::env::var("SDKWORK_RATE_LIMIT_BURST") {
            config.rate_limit_burst = burst.parse().unwrap_or(200);
        }
        if let Ok(redis_url) = std::env::var("SDKWORK_RATE_LIMIT_REDIS_URL")
            .or_else(|_| std::env::var("SDKWORK_REDIS_URL"))
        {
            let trimmed = redis_url.trim().to_string();
            if !trimmed.is_empty() {
                config.rate_limit_redis_url = Some(trimmed);
            }
        }
        if let Ok(sse_timeout) = std::env::var("SDKWORK_SSE_REQUEST_TIMEOUT") {
            config.sse_request_timeout_secs = sse_timeout.parse()?;
        }
        if let Ok(mode) = std::env::var("SDKWORK_KERNEL_METRICS_AUTH_MODE") {
            config.metrics_auth_mode = mode;
        }
        if let Ok(token) = std::env::var("SDKWORK_KERNEL_METRICS_TOKEN") {
            let trimmed = token.trim().to_string();
            if !trimmed.is_empty() {
                config.metrics_token = Some(trimmed);
            }
        }
        if let Ok(endpoint) = std::env::var("SDKWORK_OTEL_EXPORTER_OTLP_ENDPOINT") {
            let trimmed = endpoint.trim().to_string();
            if !trimmed.is_empty() {
                config.otel_exporter_otlp_endpoint = Some(trimmed);
            }
        }
        if config.environment.eq_ignore_ascii_case("production")
            && config.ingress_auth_mode.eq_ignore_ascii_case("open")
        {
            config.ingress_auth_mode = "token".to_string();
        }
        if config.environment.eq_ignore_ascii_case("production")
            && config.kernel_runtime_target.as_deref() == Some("server")
            && config.runtime_database_engine.eq_ignore_ascii_case("sqlite")
        {
            if std::env::var("SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE").is_err() {
                config.runtime_database_engine = "postgres".to_string();
            }
        }
        config.normalize_security();

        Ok(config)
    }

    fn normalize_security(&mut self) {
        if !self.is_loopback_bind() && self.ingress_auth_mode.eq_ignore_ascii_case("open") {
            self.ingress_auth_mode = "token".to_string();
        }
        if self.environment.eq_ignore_ascii_case("production")
            && self.cors_origins.iter().any(|origin| origin == "*")
        {
            self.cors_origins = vec![
                "https://kernel.sdkwork.com".to_string(),
                "https://app.sdkwork.com".to_string(),
            ];
        }
        if self.environment.eq_ignore_ascii_case("production")
            && self.cors_origins.iter().all(|origin| {
                origin.contains("127.0.0.1") || origin.contains("localhost")
            })
        {
            self.cors_origins = vec![
                "https://kernel.sdkwork.com".to_string(),
                "https://app.sdkwork.com".to_string(),
            ];
        }
        if self.rate_limit_rps == 0
            && (self.environment.eq_ignore_ascii_case("production") || !self.is_loopback_bind())
        {
            self.rate_limit_rps = if self.environment.eq_ignore_ascii_case("production") {
                100
            } else {
                50
            };
        }
        if self.rate_limit_rps > 0 && self.rate_limit_burst == 0 {
            self.rate_limit_burst = self.rate_limit_rps.saturating_mul(2);
        }
        if (self.environment.eq_ignore_ascii_case("production") || !self.is_loopback_bind())
            && self.metrics_auth_mode.eq_ignore_ascii_case("open")
        {
            self.metrics_auth_mode = "token".to_string();
        }
        if self.metrics_auth_mode.eq_ignore_ascii_case("token")
            && self
                .metrics_token
                .as_deref()
                .is_none_or(str::is_empty)
        {
            self.metrics_token = self.ingress_token.clone();
        }
    }

    pub fn metrics_auth_required(&self) -> bool {
        self.metrics_auth_mode.eq_ignore_ascii_case("token")
    }

    pub fn effective_metrics_token(&self) -> Option<&str> {
        self.metrics_token
            .as_deref()
            .or(self.ingress_token.as_deref())
            .filter(|token| !token.is_empty())
    }

    pub fn otel_export_enabled(&self) -> bool {
        self.otel_exporter_otlp_endpoint
            .as_deref()
            .is_some_and(|endpoint| !endpoint.is_empty())
    }

    pub fn uses_postgres_runtime_database(&self) -> bool {
        matches!(
            self.runtime_database_engine.to_ascii_lowercase().as_str(),
            "postgres" | "postgresql"
        )
    }

    pub fn effective_rate_limit_redis_url(&self) -> Option<&str> {
        self.rate_limit_redis_url
            .as_deref()
            .filter(|url| !url.is_empty())
    }

    pub fn requires_distributed_rate_limit(&self) -> bool {
        if self.rate_limit_rps == 0 {
            return false;
        }
        let cloud_or_server = self.kernel_hosting.as_deref() == Some("cloud-hosted")
            || self.kernel_runtime_target.as_deref() == Some("server");
        self.environment.eq_ignore_ascii_case("production") && cloud_or_server
    }

    pub fn ingress_auth_secured(&self) -> bool {
        self.ingress_auth_mode.eq_ignore_ascii_case("token")
            || self.ingress_auth_mode.eq_ignore_ascii_case("jwt")
    }

    pub fn has_ingress_jwt_material(&self) -> bool {
        self.ingress_jwt_secret
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            || self
                .ingress_jwt_rsa_public_key_pem
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            || self
                .ingress_jwt_jwks_file
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            || self
                .ingress_jwt_jwks_url
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    pub fn requires_postgres_runtime_database(&self) -> bool {
        let cloud_or_server = self.kernel_hosting.as_deref() == Some("cloud-hosted")
            || self.kernel_runtime_target.as_deref() == Some("server");
        self.environment.eq_ignore_ascii_case("production")
            && cloud_or_server
            && self.uses_postgres_runtime_database()
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
    fn cloud_production_requires_postgres_and_redis_profiles() {
        let config = ServerConfig {
            environment: "production".to_string(),
            kernel_hosting: Some("cloud-hosted".to_string()),
            kernel_runtime_target: Some("server".to_string()),
            runtime_database_engine: "postgres".to_string(),
            rate_limit_rps: 100,
            ..Default::default()
        };
        assert!(config.uses_postgres_runtime_database());
        assert!(config.requires_postgres_runtime_database());
        assert!(config.requires_distributed_rate_limit());
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
    fn production_normalizes_metrics_auth_to_token() {
        let mut config = ServerConfig {
            environment: "production".to_string(),
            bind_address: "0.0.0.0".to_string(),
            ingress_auth_mode: "token".to_string(),
            ingress_token: Some("secret".to_string()),
            ..Default::default()
        };
        config.normalize_security();
        assert_eq!(config.metrics_auth_mode, "token");
        assert_eq!(config.effective_metrics_token(), Some("secret"));
    }
}
