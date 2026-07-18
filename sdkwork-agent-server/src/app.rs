use axum::extract::FromRef;
use axum::{middleware as axum_middleware, routing::get, Router};
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;

use crate::api::internal_runtime;
use crate::config::ServerConfig;
use crate::health;
use crate::metrics;
use crate::middleware;
use crate::persistence::PersistenceState;
use crate::runtime_routes::{build_internal_runtime_routes, INTERNAL_RUNTIME_MOUNT_PREFIX};

impl FromRef<OperationalRoutesState>
    for (
        Arc<metrics::MetricsRegistry>,
        Arc<health::HealthState>,
        Arc<PersistenceState>,
        metrics::OperationalProfile,
    )
{
    fn from_ref(input: &OperationalRoutesState) -> Self {
        (
            input.metrics.clone(),
            input.health.clone(),
            input.persistence.clone(),
            input.operational_profile.clone(),
        )
    }
}

/// Shared router state for operational endpoints (health + metrics).
#[derive(Clone)]
pub struct OperationalRoutesState {
    pub health: Arc<health::HealthState>,
    pub persistence: Arc<PersistenceState>,
    pub metrics: Arc<metrics::MetricsRegistry>,
    pub operational_profile: metrics::OperationalProfile,
}

impl FromRef<OperationalRoutesState> for (Arc<health::HealthState>, Arc<PersistenceState>) {
    fn from_ref(input: &OperationalRoutesState) -> Self {
        (input.health.clone(), input.persistence.clone())
    }
}

impl FromRef<OperationalRoutesState>
    for (
        Arc<metrics::MetricsRegistry>,
        Arc<health::HealthState>,
        Arc<PersistenceState>,
    )
{
    fn from_ref(input: &OperationalRoutesState) -> Self {
        (
            input.metrics.clone(),
            input.health.clone(),
            input.persistence.clone(),
        )
    }
}

/// Build the agent-server Axum router with ingress auth, logging, and CORS layers applied.
pub fn build_app(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    runtime_state: Arc<internal_runtime::InternalRuntimeApiState>,
) -> Router {
    try_build_app(config, health_state, persistence, runtime_state)
        .expect("agent-server app should initialize")
}

pub fn try_build_app(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    runtime_state: Arc<internal_runtime::InternalRuntimeApiState>,
) -> anyhow::Result<Router> {
    let rate_limit = Arc::new(
        crate::rate_limit::RateLimitState::try_from_config(config.as_ref())
            .map_err(|message| anyhow::anyhow!(message))?,
    );
    try_build_app_with_rate_limit(config, health_state, persistence, runtime_state, rate_limit)
}

pub async fn build_app_async(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    runtime_state: Arc<internal_runtime::InternalRuntimeApiState>,
) -> anyhow::Result<Router> {
    let rate_limit = Arc::new(
        crate::rate_limit::RateLimitState::try_from_config_async(config.as_ref())
            .await
            .map_err(|message| anyhow::anyhow!(message))?,
    );
    try_build_app_with_rate_limit(config, health_state, persistence, runtime_state, rate_limit)
}

fn try_build_app_with_rate_limit(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    runtime_state: Arc<internal_runtime::InternalRuntimeApiState>,
    rate_limit: Arc<crate::rate_limit::RateLimitState>,
) -> anyhow::Result<Router> {
    let idempotency = Arc::new(crate::idempotency::IdempotencyState::from_config(
        config.as_ref(),
    )?);
    let metrics_registry = metrics::MetricsRegistry::from_config(config.as_ref());
    runtime_state.runtime.attach_metrics(&metrics_registry);
    persistence.attach_metrics(&metrics_registry);
    let operational_profile = metrics::OperationalProfile::from_runtime(
        persistence.persistence_backend_label(),
        rate_limit.uses_redis(),
    );
    let operational_state = OperationalRoutesState {
        health: health_state,
        persistence: persistence.clone(),
        metrics: metrics_registry.clone(),
        operational_profile,
    };

    let ingress_state = Arc::new(
        crate::ingress_state::IngressMiddlewareState::from_config(config.clone())
            .map_err(|message| anyhow::anyhow!(message))?,
    );

    let health_routes = Router::new()
        .route("/metrics", get(metrics::prometheus_metrics))
        .with_state(operational_state);

    // Per-route timeout layers are applied inside `build_internal_runtime_routes`
    // so that SSE streaming routes receive a longer timeout than standard
    // JSON routes. Do NOT add an outer TimeoutLayer here — it would fire
    // the shorter timeout on SSE connections.
    let internal_runtime = Router::new().nest(
        INTERNAL_RUNTIME_MOUNT_PREFIX,
        build_internal_runtime_routes(runtime_state.clone()),
    );

    let standard_routes = Router::new().merge(health_routes).merge(internal_runtime);
    let standard_routes = service_router(
        standard_routes,
        ServiceRouterConfig::default()
            .skip_metrics()
            .with_readiness_check(Arc::new(
                health::RuntimeReadiness::new(
                    persistence.clone(),
                    config.clone(),
                    runtime_state.runtime.clone(),
                )
                .map_err(anyhow::Error::msg)?,
            )),
    );

    Ok(Router::new()
        .merge(standard_routes)
        .layer(axum::Extension(metrics_registry))
        .layer(axum_middleware::from_fn_with_state(
            idempotency,
            crate::idempotency::middleware,
        ))
        .layer(RequestBodyLimitLayer::new(config.max_body_size))
        .layer(axum_middleware::from_fn_with_state(
            rate_limit,
            middleware::rate_limit_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            ingress_state.clone(),
            middleware::ingress_identity_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            ingress_state,
            middleware::ingress_auth_middleware,
        ))
        .layer(axum_middleware::from_fn(
            middleware::security_headers_middleware,
        ))
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        .layer(axum_middleware::from_fn(
            middleware::request_context_middleware,
        ))
        .layer(middleware::cors_layer(&config)))
}

/// Build a test router with in-memory persistence and open ingress auth.
pub fn build_test_app(config: Arc<ServerConfig>) -> Router {
    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(
        PersistenceState::memory().expect("in-memory persistence should initialize for tests"),
    );
    let runtime_state = Arc::new(
        internal_runtime::InternalRuntimeApiState::new(persistence.clone(), config.clone())
            .expect("runtime state should initialize for tests"),
    );
    try_build_app(config, health_state, persistence, runtime_state)
        .expect("test app should initialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_does_not_panic() {
        let _app = build_test_app(Arc::new(ServerConfig::default()));
    }

    #[tokio::test]
    async fn build_app_async_returns_error_for_invalid_ingress_jwt_config() {
        let config = Arc::new(ServerConfig {
            ingress_auth_mode: "jwt".to_string(),
            ..Default::default()
        });
        let health_state = Arc::new(health::HealthState::new());
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let runtime_state = Arc::new(
            internal_runtime::InternalRuntimeApiState::new(persistence.clone(), config.clone())
                .expect("runtime state"),
        );

        let result = build_app_async(config, health_state, persistence, runtime_state).await;

        assert!(
            result.is_err(),
            "invalid JWT ingress config must fail startup"
        );
        let message = result.expect_err("startup error").to_string();
        assert!(
            message.contains("SDKWORK_KERNEL_INGRESS_JWT_SECRET"),
            "startup error should name the missing JWT secret: {message}"
        );
    }
}
