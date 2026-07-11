use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::{config::ServerConfig, persistence::PersistenceState, runtime::RuntimeState};

#[derive(Clone)]
struct RedisDependency {
    name: &'static str,
    client: redis::Client,
}

/// Framework-owned readiness adapter for all required runtime dependencies.
#[derive(Clone)]
pub struct RuntimeReadiness {
    persistence: Arc<PersistenceState>,
    config: Arc<ServerConfig>,
    runtime: RuntimeState,
    redis_dependencies: Vec<RedisDependency>,
}

impl RuntimeReadiness {
    pub fn new(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
        runtime: RuntimeState,
    ) -> Result<Self, String> {
        let mut redis_dependencies = Vec::new();
        if config.is_production_kernel_profile() && config.effective_deployment_profile() == "cloud"
        {
            let rate_limit_url = config.effective_rate_limit_redis_url().ok_or_else(|| {
                "cloud production readiness requires rate-limit Redis configuration".to_string()
            })?;
            redis_dependencies.push(RedisDependency {
                name: "rate_limit_redis",
                client: redis::Client::open(rate_limit_url)
                    .map_err(|error| format!("invalid rate-limit Redis configuration: {error}"))?,
            });
            let idempotency_url = config.effective_idempotency_redis_url().ok_or_else(|| {
                "cloud production readiness requires idempotency Redis configuration".to_string()
            })?;
            redis_dependencies.push(RedisDependency {
                name: "idempotency_redis",
                client: redis::Client::open(idempotency_url)
                    .map_err(|error| format!("invalid idempotency Redis configuration: {error}"))?,
            });
        }
        Ok(Self {
            persistence,
            config,
            runtime,
            redis_dependencies,
        })
    }
}

impl sdkwork_web_bootstrap::ReadinessCheck for RuntimeReadiness {
    fn check(&self) -> sdkwork_web_bootstrap::ReadinessFuture<'_> {
        let persistence = self.persistence.clone();
        let config = self.config.clone();
        let runtime = self.runtime.clone();
        let redis_dependencies = self.redis_dependencies.clone();
        Box::pin(async move {
            persistence.run(|state| state.readiness()).await?;
            if config.is_production_kernel_profile() {
                validate_required_runtime(&runtime)?;
            }
            if config.is_production_kernel_profile()
                && config.effective_deployment_profile() == "cloud"
            {
                for dependency in redis_dependencies {
                    let mut connection = time_limited_redis_connection(&dependency).await?;
                    let response = tokio::time::timeout(
                        Duration::from_secs(2),
                        redis::cmd("PING").query_async::<String>(&mut connection),
                    )
                    .await
                    .map_err(|_| format!("{} readiness timed out", dependency.name))?
                    .map_err(|error| format!("{} readiness failed: {error}", dependency.name))?;
                    if response != "PONG" {
                        return Err(format!(
                            "{} readiness returned an unexpected response",
                            dependency.name
                        ));
                    }
                }
            }
            Ok(())
        })
    }
}

async fn time_limited_redis_connection(
    dependency: &RedisDependency,
) -> Result<redis::aio::MultiplexedConnection, String> {
    tokio::time::timeout(
        Duration::from_secs(2),
        dependency.client.get_multiplexed_async_connection(),
    )
    .await
    .map_err(|_| format!("{} connection timed out", dependency.name))?
    .map_err(|error| format!("{} connection failed: {error}", dependency.name))
}

fn validate_required_runtime(runtime: &RuntimeState) -> Result<(), String> {
    let diagnostics = runtime.agent_runtime().diagnostics();
    if diagnostics.state != "ready" || !diagnostics.missing_required_capabilities.is_empty() {
        return Err("required runtime capabilities are unavailable".to_string());
    }
    let required_provider_ids: HashSet<&str> = runtime
        .agent_runtime()
        .capability_manifest()
        .capabilities
        .iter()
        .filter(|capability| capability.required)
        .map(|capability| capability.provider_id.as_str())
        .collect();
    for provider_id in required_provider_ids {
        let provider = diagnostics.provider(provider_id).ok_or_else(|| {
            format!("required provider is missing from diagnostics: {provider_id}")
        })?;
        if !provider.typed_registered {
            return Err(format!("required provider is not typed: {provider_id}"));
        }
        let healthy = provider.health.as_ref().is_some_and(|health| {
            matches!(
                health.status.to_ascii_lowercase().as_str(),
                "available" | "ready" | "healthy"
            )
        });
        if !healthy {
            return Err(format!("required provider is unavailable: {provider_id}"));
        }
    }
    Ok(())
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub components: Vec<ComponentHealth>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

/// Health check state
#[derive(Debug, Clone)]
pub struct HealthState {
    pub start_time: std::time::Instant,
    pub version: String,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn persistence_component_for_metrics(health: Result<bool, String>) -> ComponentHealth {
    match health {
        Ok(true) => ComponentHealth {
            name: "persistence".to_string(),
            status: "available".to_string(),
            message: None,
        },
        Ok(false) => ComponentHealth {
            name: "persistence".to_string(),
            status: "degraded".to_string(),
            message: Some("database health probe returned false".to_string()),
        },
        Err(error) => ComponentHealth {
            name: "persistence".to_string(),
            status: "unavailable".to_string(),
            message: Some(error),
        },
    }
}

pub(crate) fn aggregate_component_status(components: &[ComponentHealth]) -> String {
    if components
        .iter()
        .any(|component| component.status == "unavailable")
    {
        "unhealthy".to_string()
    } else if components
        .iter()
        .any(|component| component.status == "degraded")
    {
        "degraded".to_string()
    } else {
        "healthy".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_state_uptime() {
        let state = HealthState::new();
        let _ = state.uptime_secs();
    }

    #[tokio::test]
    async fn standalone_readiness_checks_database_and_schema() {
        let config = Arc::new(ServerConfig::default());
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let runtime = RuntimeState::try_for_config(config.as_ref()).expect("runtime");
        let readiness = RuntimeReadiness::new(persistence, config, runtime).expect("readiness");
        sdkwork_web_bootstrap::ReadinessCheck::check(&readiness)
            .await
            .expect("ready");
    }
}
