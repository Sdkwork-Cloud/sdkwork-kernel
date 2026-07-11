use crate::config::ServerConfig;
use tracing::info;

/// Preflight validation result
#[derive(Debug)]
pub struct PreflightResult {
    pub checks: Vec<PreflightCheck>,
    pub passed: bool,
}

/// Individual preflight check
#[derive(Debug)]
pub struct PreflightCheck {
    pub name: String,
    pub status: PreflightStatus,
    pub message: String,
}

/// Preflight check status
#[derive(Debug, PartialEq)]
pub enum PreflightStatus {
    Passed,
    Warning,
    Failed,
}

/// Run preflight validation checks
pub fn validate(config: &ServerConfig) -> PreflightResult {
    let mut checks = Vec::new();

    // Check bind address
    checks.push(validate_bind_address(&config.bind_address));

    if !config.is_loopback_bind() && config.ingress_auth_mode.eq_ignore_ascii_case("open") {
        checks.push(PreflightCheck {
            name: "ingress_auth_open".to_string(),
            status: PreflightStatus::Failed,
            message: "Open ingress auth is allowed only on loopback bind addresses".to_string(),
        });
    }

    if config.is_production_kernel_profile() && config.bind_address == "0.0.0.0" {
        checks.push(PreflightCheck {
            name: "production_bind".to_string(),
            status: PreflightStatus::Warning,
            message:
                "Production environment binds to 0.0.0.0; place ingress behind a trusted gateway"
                    .to_string(),
        });
    }

    if config.ingress_auth_mode.eq_ignore_ascii_case("token")
        && config.ingress_token.as_deref().is_none_or(str::is_empty)
    {
        checks.push(PreflightCheck {
            name: "ingress_token".to_string(),
            status: PreflightStatus::Failed,
            message: "SDKWORK_KERNEL_INGRESS_TOKEN is required when ingress auth mode is token"
                .to_string(),
        });
    }

    if config.ingress_auth_mode.eq_ignore_ascii_case("jwt") && !config.has_ingress_jwt_material() {
        checks.push(PreflightCheck {
            name: "ingress_jwt_material".to_string(),
            status: PreflightStatus::Failed,
            message: "JWT ingress requires SDKWORK_KERNEL_INGRESS_JWT_SECRET, SDKWORK_KERNEL_INGRESS_JWT_RSA_PUBLIC_KEY_PEM, SDKWORK_KERNEL_INGRESS_JWT_JWKS_FILE, or SDKWORK_KERNEL_INGRESS_JWT_JWKS_URL"
                .to_string(),
        });
    }

    if config.ingress_auth_mode.eq_ignore_ascii_case("jwt")
        && config.is_production_kernel_profile()
        && config
            .ingress_jwt_jwks_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && !config
            .ingress_jwt_jwks_url
            .as_deref()
            .is_some_and(|value| value.trim().starts_with("https://"))
    {
        checks.push(PreflightCheck {
            name: "ingress_jwt_jwks_url_https".to_string(),
            status: PreflightStatus::Failed,
            message: "Production JWT ingress JWKS URL must use https:// (SDKWORK_KERNEL_INGRESS_JWT_JWKS_URL)"
                .to_string(),
        });
    }

    if config.ingress_identity_mode() == crate::ingress_identity::IngressIdentityMode::Bound
        && !config.has_bound_identity()
    {
        checks.push(PreflightCheck {
            name: "ingress_bound_identity".to_string(),
            status: PreflightStatus::Failed,
            message: "Bound ingress identity requires SDKWORK_KERNEL_INGRESS_BOUND_TENANT_ID and SDKWORK_KERNEL_INGRESS_BOUND_USER_ID"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile() && config.rate_limit_rps == 0 {
        checks.push(PreflightCheck {
            name: "rate_limit".to_string(),
            status: PreflightStatus::Failed,
            message: "Production requires a positive SDKWORK_RATE_LIMIT_RPS (default 100)"
                .to_string(),
        });
    }

    if config.metrics_auth_required() && config.effective_metrics_token().is_none() {
        checks.push(PreflightCheck {
            name: "metrics_token".to_string(),
            status: PreflightStatus::Failed,
            message: "Metrics token auth requires SDKWORK_KERNEL_METRICS_TOKEN".to_string(),
        });
    }

    if config.otel_export_enabled() && !cfg!(feature = "observability-otel") {
        checks.push(PreflightCheck {
            name: "otel_feature".to_string(),
            status: PreflightStatus::Failed,
            message: "SDKWORK_OTEL_EXPORTER_OTLP_ENDPOINT requires building sdkwork-agent-server with feature observability-otel"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile() && config.allow_mock_provider_fallback() {
        checks.push(PreflightCheck {
            name: "mock_providers".to_string(),
            status: PreflightStatus::Failed,
            message: "SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS must be unset in production profiles"
                .to_string(),
        });
    }

    if config.requires_distributed_rate_limit() && config.effective_rate_limit_redis_url().is_none()
    {
        checks.push(PreflightCheck {
            name: "rate_limit_redis".to_string(),
            status: PreflightStatus::Failed,
            message: "Production cloud/server deployments require SDKWORK_RATE_LIMIT_REDIS_URL (or SDKWORK_REDIS_URL) for distributed rate limiting"
                .to_string(),
        });
    }

    if config.requires_distributed_idempotency()
        && config.effective_idempotency_redis_url().is_none()
    {
        checks.push(PreflightCheck {
            name: "idempotency_redis".to_string(),
            status: PreflightStatus::Failed,
            message: "Production cloud/server deployments require SDKWORK_IDEMPOTENCY_REDIS_URL for distributed idempotency"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile() && !config.idempotency_require_key {
        checks.push(PreflightCheck {
            name: "idempotency_key_required".to_string(),
            status: PreflightStatus::Failed,
            message:
                "Production profiles must require Idempotency-Key on retry-sensitive mutations"
                    .to_string(),
        });
    }

    if !(60..=7 * 24 * 60 * 60).contains(&config.idempotency_ttl_secs) {
        checks.push(PreflightCheck {
            name: "idempotency_ttl".to_string(),
            status: PreflightStatus::Failed,
            message: "Idempotency retention must be between 60 seconds and 7 days".to_string(),
        });
    }

    if config.idempotency_max_cached_response_bytes == 0
        || config.idempotency_max_cached_response_bytes > 1024 * 1024
        || config.idempotency_max_cached_response_bytes > config.max_body_size
    {
        checks.push(PreflightCheck {
            name: "idempotency_response_limit".to_string(),
            status: PreflightStatus::Failed,
            message: "Idempotency response cache limit must be positive, at most 1 MiB, and no larger than max body size"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile()
        && config.production_scaleout_profile()
        && !config.tenant_token_quota_overrides.is_empty()
        && config.effective_rate_limit_redis_url().is_none()
    {
        checks.push(PreflightCheck {
            name: "tenant_token_quota_redis".to_string(),
            status: PreflightStatus::Failed,
            message: "Tenant token quota overrides require SDKWORK_RATE_LIMIT_REDIS_URL (or SDKWORK_REDIS_URL) in production scale-out profiles"
                .to_string(),
        });
    }

    if config.requires_postgres_runtime_database() && !postgres_runtime_uri_configured() {
        checks.push(PreflightCheck {
            name: "runtime_postgres".to_string(),
            status: PreflightStatus::Failed,
            message: "Production cloud/server deployments require SDKWORK_AGENT_RUNTIME_DATABASE_URL or SDKWORK_AGENT_RUNTIME_POSTGRES_URI for shared session persistence"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile()
        && config.uses_postgres_runtime_database()
        && !postgres_runtime_uri_configured()
    {
        checks.push(PreflightCheck {
            name: "runtime_postgres_uri".to_string(),
            status: PreflightStatus::Failed,
            message: "PostgreSQL runtime engine requires SDKWORK_AGENT_RUNTIME_DATABASE_URL or SDKWORK_AGENT_RUNTIME_POSTGRES_URI"
                .to_string(),
        });
    }

    if config.is_production_kernel_profile()
        && !config.uses_postgres_runtime_database()
        && config.production_scaleout_profile()
    {
        checks.push(PreflightCheck {
            name: "runtime_sqlite_scaling".to_string(),
            status: PreflightStatus::Failed,
            message: "SQLite runtime persistence cannot scale horizontally; set SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres for multi-replica production deployments"
                .to_string(),
        });
    }

    if !(1..=365).contains(&config.runtime_retention_days) {
        checks.push(PreflightCheck {
            name: "runtime_retention_days".to_string(),
            status: PreflightStatus::Failed,
            message: "Runtime retention must be between 1 and 365 days".to_string(),
        });
    }

    if !(1..=10_000).contains(&config.runtime_cleanup_batch_size) {
        checks.push(PreflightCheck {
            name: "runtime_cleanup_batch_size".to_string(),
            status: PreflightStatus::Failed,
            message: "Runtime cleanup batch size must be between 1 and 10000 rows".to_string(),
        });
    }

    if !(10..=24 * 60 * 60).contains(&config.runtime_cleanup_interval_secs) {
        checks.push(PreflightCheck {
            name: "runtime_cleanup_interval".to_string(),
            status: PreflightStatus::Failed,
            message: "Runtime cleanup interval must be between 10 seconds and 24 hours".to_string(),
        });
    }

    // Check port availability
    checks.push(validate_port(config.port));

    // Check log level
    checks.push(validate_log_level(&config.log_level));

    // Check request timeout
    checks.push(validate_timeout(config.request_timeout_secs));

    // Check body size
    checks.push(validate_body_size(config.max_body_size));

    let passed = checks.iter().all(|c| c.status != PreflightStatus::Failed);

    PreflightResult { checks, passed }
}

fn postgres_runtime_uri_configured() -> bool {
    [
        "SDKWORK_AGENT_RUNTIME_DATABASE_URL",
        "SDKWORK_AGENT_RUNTIME_POSTGRES_URI",
    ]
    .into_iter()
    .any(|key| {
        std::env::var(key)
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn validate_bind_address(address: &str) -> PreflightCheck {
    if address.parse::<std::net::IpAddr>().is_ok() || address == "localhost" {
        PreflightCheck {
            name: "bind_address".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Bind address '{}' is valid", address),
        }
    } else {
        PreflightCheck {
            name: "bind_address".to_string(),
            status: PreflightStatus::Warning,
            message: format!("Bind address '{}' may not be valid", address),
        }
    }
}

fn validate_port(port: u16) -> PreflightCheck {
    if port > 0 {
        PreflightCheck {
            name: "port".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Port {} is valid", port),
        }
    } else {
        PreflightCheck {
            name: "port".to_string(),
            status: PreflightStatus::Failed,
            message: "Port must be greater than 0".to_string(),
        }
    }
}

fn validate_log_level(level: &str) -> PreflightCheck {
    match level.to_lowercase().as_str() {
        "trace" | "debug" | "info" | "warn" | "error" => PreflightCheck {
            name: "log_level".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Log level '{}' is valid", level),
        },
        _ => PreflightCheck {
            name: "log_level".to_string(),
            status: PreflightStatus::Warning,
            message: format!("Unknown log level '{}', defaulting to 'info'", level),
        },
    }
}

fn validate_timeout(timeout: u64) -> PreflightCheck {
    if timeout > 0 && timeout <= 300 {
        PreflightCheck {
            name: "request_timeout".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Request timeout {}s is valid", timeout),
        }
    } else {
        PreflightCheck {
            name: "request_timeout".to_string(),
            status: PreflightStatus::Warning,
            message: format!("Request timeout {}s is unusual", timeout),
        }
    }
}

fn validate_body_size(size: usize) -> PreflightCheck {
    if size > 0 && size <= 4 * 1024 * 1024 {
        PreflightCheck {
            name: "max_body_size".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Max body size {} bytes is valid", size),
        }
    } else {
        PreflightCheck {
            name: "max_body_size".to_string(),
            status: PreflightStatus::Failed,
            message: format!(
                "Max body size {size} bytes is unsafe; configure a value between 1 byte and 4 MiB"
            ),
        }
    }
}

/// Print preflight results
pub fn print_results(result: &PreflightResult) {
    info!("=== Preflight Checks ===");
    for check in &result.checks {
        let icon = match check.status {
            PreflightStatus::Passed => "✓",
            PreflightStatus::Warning => "⚠",
            PreflightStatus::Failed => "✗",
        };
        info!("  {} {}: {}", icon, check.name, check.message);
    }
    info!("========================");

    if result.passed {
        info!("All preflight checks passed");
    } else {
        tracing::error!("Some preflight checks failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_default_config() {
        let _lock = crate::testing::env::lock();
        let _profile = crate::testing::env::VarGuard::set("SDKWORK_KERNEL_PROFILE_ID", None);
        let _environment = crate::testing::env::VarGuard::set("SDKWORK_KERNEL_ENVIRONMENT", None);
        let config = ServerConfig::default();
        let result = validate(&config);
        assert!(result.passed, "{result:?}");
    }

    #[test]
    fn jwt_preflight_requires_material() {
        let config = ServerConfig {
            ingress_auth_mode: "jwt".to_string(),
            ..Default::default()
        };
        let result = validate(&config);
        assert!(!result.passed);
        assert!(result
            .checks
            .iter()
            .any(|check| check.name == "ingress_jwt_material"));
    }

    #[test]
    fn production_jwt_jwks_url_requires_https() {
        let config = ServerConfig {
            environment: "production".to_string(),
            ingress_auth_mode: "jwt".to_string(),
            ingress_jwt_jwks_url: Some("http://idp.example.com/jwks".to_string()),
            ..Default::default()
        };
        let result = validate(&config);
        assert!(result
            .checks
            .iter()
            .any(|check| check.name == "ingress_jwt_jwks_url_https"));
    }

    #[test]
    fn cloud_production_preflight_requires_redis_and_postgres_uri() {
        let config = ServerConfig {
            environment: "production".to_string(),
            deployment_profile: Some("cloud".to_string()),
            kernel_runtime_target: Some("server".to_string()),
            runtime_database_engine: "postgres".to_string(),
            ingress_auth_mode: "token".to_string(),
            ingress_token: Some("secret".to_string()),
            rate_limit_rps: 100,
            metrics_auth_mode: "token".to_string(),
            metrics_token: Some("metrics-secret".to_string()),
            ..Default::default()
        };
        let result = validate(&config);
        assert!(!result.passed);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.name == "rate_limit_redis"),
            "expected rate_limit_redis failure"
        );
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.name == "runtime_postgres"),
            "expected runtime_postgres failure"
        );
    }

    #[test]
    fn production_rejects_mock_provider_override() {
        let _lock = crate::testing::env::lock();
        let _profile = crate::testing::env::VarGuard::set(
            "SDKWORK_KERNEL_PROFILE_ID",
            Some("cloud.production"),
        );
        let _mock =
            crate::testing::env::VarGuard::set("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS", Some("1"));
        let config = ServerConfig {
            environment: "production".to_string(),
            ingress_auth_mode: "token".to_string(),
            ingress_token: Some("secret".to_string()),
            rate_limit_rps: 100,
            ..Default::default()
        };
        let result = validate(&config);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.name == "mock_providers"),
            "production must fail when mock override is enabled"
        );
    }

    #[test]
    fn production_topology_profile_requires_rate_limit_without_environment_literal() {
        let _lock = crate::testing::env::lock();
        let _profile = crate::testing::env::VarGuard::set(
            "SDKWORK_KERNEL_PROFILE_ID",
            Some("cloud.production"),
        );
        let config = ServerConfig {
            environment: "development".to_string(),
            deployment_profile: Some("cloud".to_string()),
            kernel_runtime_target: Some("server".to_string()),
            rate_limit_rps: 0,
            ..Default::default()
        };
        let result = validate(&config);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.name == "rate_limit"),
            "topology production profile must enforce rate limits even when environment is not production"
        );
    }
}
