use axum::{
    http::StatusCode,
    middleware as axum_middleware,
    routing::get,
    Router,
};
use axum::extract::FromRef;
use std::sync::Arc;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::api::internal_runtime;
use crate::config::ServerConfig;
use crate::health;
use crate::metrics;
use crate::middleware;
use crate::persistence::PersistenceState;
use crate::runtime_routes::{build_internal_runtime_routes, INTERNAL_RUNTIME_MOUNT_PREFIX};

impl FromRef<OperationalRoutesState> for (
    Arc<metrics::MetricsRegistry>,
    Arc<health::HealthState>,
    Arc<PersistenceState>,
    metrics::OperationalProfile,
) {
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

impl FromRef<OperationalRoutesState> for (
    Arc<metrics::MetricsRegistry>,
    Arc<health::HealthState>,
    Arc<PersistenceState>,
) {
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
    let metrics_registry = metrics::MetricsRegistry::from_config(config.as_ref());
    let rate_limit = Arc::new(crate::rate_limit::RateLimitState::from_config(config.as_ref()));
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
            .expect("ingress middleware state should initialize"),
    );

    let health_routes = Router::new()
        .route(&config.health_path, get(health::health_check))
        .route("/ready", get(health::readiness_check))
        .route("/live", get(health::liveness_check))
        .route("/metrics", get(metrics::prometheus_metrics))
        .with_state(operational_state);

    let internal_runtime = Router::new()
        .nest(
            INTERNAL_RUNTIME_MOUNT_PREFIX,
            build_internal_runtime_routes(runtime_state),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.request_timeout_secs),
        ));

    let standard_routes = Router::new().merge(health_routes).merge(internal_runtime);

    Router::new()
        .merge(standard_routes)
        .layer(axum::Extension(metrics_registry))
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
        .layer(axum_middleware::from_fn(middleware::security_headers_middleware))
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        .layer(axum_middleware::from_fn(
            middleware::request_context_middleware,
        ))
        .layer(middleware::cors_layer(&config))
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
    build_app(config, health_state, persistence, runtime_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_does_not_panic() {
        let _app = build_test_app(Arc::new(ServerConfig::default()));
    }
}
