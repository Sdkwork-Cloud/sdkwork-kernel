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
    if size > 0 && size <= 100 * 1024 * 1024 {
        PreflightCheck {
            name: "max_body_size".to_string(),
            status: PreflightStatus::Passed,
            message: format!("Max body size {} bytes is valid", size),
        }
    } else {
        PreflightCheck {
            name: "max_body_size".to_string(),
            status: PreflightStatus::Warning,
            message: format!("Max body size {} bytes is unusual", size),
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
        let config = ServerConfig::default();
        let result = validate(&config);
        assert!(result.passed);
    }

    #[test]
    fn validate_invalid_port() {
        let config = ServerConfig {
            port: 0,
            ..Default::default()
        };
        let result = validate(&config);
        assert!(!result.passed);
    }
}
