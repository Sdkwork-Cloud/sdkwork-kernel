//! Prometheus-compatible HTTP metrics (`OBSERVABILITY_SPEC.md`).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::config::ServerConfig;
use crate::health::{self, HealthState};
use crate::persistence::PersistenceState;

const SERVICE_NAME: &str = "sdkwork-agent-server";
const RUNTIME_TARGET: &str = "server";
static SSE_ACTIVE_CONNECTIONS: AtomicU64 = AtomicU64::new(0);

pub fn record_sse_connection_open() {
    SSE_ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_sse_connection_close() {
    let _ = SSE_ACTIVE_CONNECTIONS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(1))
    });
}

const DURATION_BUCKETS_SECS: [f64; 12] = [
    0.005,
    0.01,
    0.025,
    0.05,
    0.1,
    0.25,
    0.5,
    1.0,
    2.5,
    5.0,
    10.0,
    f64::INFINITY,
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RequestKey {
    method: String,
    route: String,
    status: String,
    api_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DurationKey {
    method: String,
    route: String,
    le: String,
    api_surface: String,
}

#[derive(Debug, Clone)]
struct StaticLabels {
    service: String,
    environment: String,
    deployment_profile: String,
    runtime_target: String,
}

/// Bounded operational backend labels for Prometheus gauges.
#[derive(Debug, Clone)]
pub struct OperationalProfile {
    pub persistence_backend: String,
    pub rate_limit_backend: String,
}

impl OperationalProfile {
    pub fn from_runtime(persistence_backend: &str, rate_limit_uses_redis: bool) -> Self {
        Self {
            persistence_backend: persistence_backend.to_string(),
            rate_limit_backend: if rate_limit_uses_redis {
                "redis".to_string()
            } else {
                "memory".to_string()
            },
        }
    }
}

/// In-process HTTP metrics registry for Prometheus scraping.
#[derive(Debug)]
pub struct MetricsRegistry {
    labels: StaticLabels,
    requests: Mutex<HashMap<RequestKey, u64>>,
    duration_buckets: Mutex<HashMap<DurationKey, u64>>,
    model_invocations: Mutex<HashMap<(String, String), u64>>,
    model_token_usage: Mutex<HashMap<(String, String), u64>>,
    auth_failures_total: AtomicU64,
    rate_limited_total: AtomicU64,
    tenant_token_quota_rejected_total: AtomicU64,
}

impl MetricsRegistry {
    pub fn from_config(config: &ServerConfig) -> Arc<Self> {
        Arc::new(Self {
            labels: StaticLabels {
                service: SERVICE_NAME.to_string(),
                environment: operational_environment_label(config),
                deployment_profile: config.effective_deployment_profile().to_string(),
                runtime_target: RUNTIME_TARGET.to_string(),
            },
            requests: Mutex::new(HashMap::new()),
            duration_buckets: Mutex::new(HashMap::new()),
            model_invocations: Mutex::new(HashMap::new()),
            model_token_usage: Mutex::new(HashMap::new()),
            auth_failures_total: AtomicU64::new(0),
            rate_limited_total: AtomicU64::new(0),
            tenant_token_quota_rejected_total: AtomicU64::new(0),
        })
    }

    pub fn record_request(
        &self,
        method: &str,
        route: &str,
        status: u16,
        api_surface: Option<&str>,
        duration_secs: f64,
    ) {
        let status_label = status.to_string();
        let surface = api_surface.unwrap_or("unknown").to_string();
        let key = RequestKey {
            method: method.to_string(),
            route: route.to_string(),
            status: status_label,
            api_surface: surface.clone(),
        };
        if let Ok(mut requests) = self.requests.lock() {
            *requests.entry(key).or_insert(0) += 1;
        }

        if let Ok(mut buckets) = self.duration_buckets.lock() {
            for le in DURATION_BUCKETS_SECS {
                if duration_secs <= le {
                    let le_label = if le.is_infinite() {
                        "+Inf".to_string()
                    } else {
                        le.to_string()
                    };
                    let bucket_key = DurationKey {
                        method: method.to_string(),
                        route: route.to_string(),
                        le: le_label,
                        api_surface: surface.clone(),
                    };
                    *buckets.entry(bucket_key).or_insert(0) += 1;
                }
            }
        }

        if matches!(status, 401 | 403) {
            self.auth_failures_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_rate_limited(&self) {
        self.rate_limited_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_tenant_token_quota_rejection(&self) {
        self.tenant_token_quota_rejected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_model_invocation(&self, provider_id: &str, status: &str) {
        let provider = sanitize_metric_label(provider_id, "unknown");
        let status_label = sanitize_metric_label(status, "unknown");
        if let Ok(mut invocations) = self.model_invocations.lock() {
            *invocations.entry((provider, status_label)).or_insert(0) += 1;
        }
    }

    pub fn record_model_token_usage(&self, provider_id: &str, direction: &str, tokens: u64) {
        if tokens == 0 {
            return;
        }
        let provider = sanitize_metric_label(provider_id, "unknown");
        let direction_label = sanitize_metric_label(direction, "total");
        if let Ok(mut usage) = self.model_token_usage.lock() {
            *usage.entry((provider, direction_label)).or_insert(0) += tokens;
        }
    }

    pub fn render_prometheus(&self, health_serving: bool, profile: &OperationalProfile) -> String {
        let mut output = String::new();
        let base = format!(
            "service=\"{}\",environment=\"{}\",deployment_profile=\"{}\",runtime_target=\"{}\"",
            self.labels.service,
            self.labels.environment,
            self.labels.deployment_profile,
            self.labels.runtime_target,
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_health_status Service health gauge (1=serving, 0=not serving)."
        );
        let _ = writeln!(output, "# TYPE sdkwork_kernel_health_status gauge");
        let _ = writeln!(
            output,
            "sdkwork_kernel_health_status{{{base}}} {}",
            u64::from(health_serving)
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_http_requests_total Total HTTP requests by route template."
        );
        let _ = writeln!(output, "# TYPE sdkwork_kernel_http_requests_total counter");
        if let Ok(requests) = self.requests.lock() {
            let mut entries: Vec<_> = requests.iter().collect();
            entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
            for (key, count) in entries {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_http_requests_total{{{base},method=\"{}\",route=\"{}\",status=\"{}\",api_surface=\"{}\"}} {count}",
                    key.method, key.route, key.status, key.api_surface
                );
            }
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_http_request_duration_seconds HTTP request duration histogram."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_http_request_duration_seconds histogram"
        );
        if let Ok(buckets) = self.duration_buckets.lock() {
            let mut entries: Vec<_> = buckets.iter().collect();
            entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
            for (key, count) in entries {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_http_request_duration_seconds_bucket{{{base},method=\"{}\",route=\"{}\",api_surface=\"{}\",le=\"{}\"}} {count}",
                    key.method, key.route, key.api_surface, key.le
                );
            }
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_http_auth_failures_total Ingress auth or identity failures."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_http_auth_failures_total counter"
        );
        let _ = writeln!(
            output,
            "sdkwork_kernel_http_auth_failures_total{{{base}}} {}",
            self.auth_failures_total.load(Ordering::Relaxed)
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_http_rate_limited_total Requests rejected by rate limiting."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_http_rate_limited_total counter"
        );
        let _ = writeln!(
            output,
            "sdkwork_kernel_http_rate_limited_total{{{base}}} {}",
            self.rate_limited_total.load(Ordering::Relaxed)
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_tenant_token_quota_rejected_total Model invoke requests rejected by tenant daily token quota."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_tenant_token_quota_rejected_total counter"
        );
        let _ = writeln!(
            output,
            "sdkwork_kernel_tenant_token_quota_rejected_total{{{base}}} {}",
            self.tenant_token_quota_rejected_total
                .load(Ordering::Relaxed)
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_sse_active_connections Active SSE connections in this server process."
        );
        let _ = writeln!(output, "# TYPE sdkwork_kernel_sse_active_connections gauge");
        let _ = writeln!(
            output,
            "sdkwork_kernel_sse_active_connections{{{base}}} {}",
            SSE_ACTIVE_CONNECTIONS.load(Ordering::Relaxed)
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_runtime_persistence_backend_info Active runtime persistence backend."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_runtime_persistence_backend_info gauge"
        );
        let _ = writeln!(
            output,
            "sdkwork_kernel_runtime_persistence_backend_info{{{base},backend=\"{}\"}} 1",
            sanitize_metric_label(&profile.persistence_backend, "sqlite")
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_rate_limit_backend_info Active HTTP rate-limit backend."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_rate_limit_backend_info gauge"
        );
        let _ = writeln!(
            output,
            "sdkwork_kernel_rate_limit_backend_info{{{base},backend=\"{}\"}} 1",
            sanitize_metric_label(&profile.rate_limit_backend, "memory")
        );

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_model_invocations_total Model invoke operations by provider and status."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_model_invocations_total counter"
        );
        if let Ok(invocations) = self.model_invocations.lock() {
            let mut entries: Vec<_> = invocations.iter().collect();
            entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
            for ((provider_id, status), count) in entries {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_model_invocations_total{{{base},provider_id=\"{provider_id}\",status=\"{status}\"}} {count}"
                );
            }
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_model_tokens_total Aggregate model token usage by provider and direction."
        );
        let _ = writeln!(output, "# TYPE sdkwork_kernel_model_tokens_total counter");
        if let Ok(usage) = self.model_token_usage.lock() {
            let mut entries: Vec<_> = usage.iter().collect();
            entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
            for ((provider_id, direction), count) in entries {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_model_tokens_total{{{base},provider_id=\"{provider_id}\",direction=\"{direction}\"}} {count}"
                );
            }
        }

        output
    }
}

fn sanitize_metric_label(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    let normalized: String = trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

async fn health_serving(_health_state: &HealthState, persistence: &PersistenceState) -> bool {
    let persistence_health = persistence.run(|state| state.health()).await;
    let components = vec![health::persistence_component_for_metrics(
        persistence_health,
    )];
    health::aggregate_component_status(&components) != "unhealthy"
}

/// Prometheus text exposition for operators (`GET /metrics`).
pub async fn prometheus_metrics(
    State((metrics, health_state, persistence, profile)): State<(
        Arc<MetricsRegistry>,
        Arc<HealthState>,
        Arc<PersistenceState>,
        OperationalProfile,
    )>,
) -> impl IntoResponse {
    let serving = health_serving(health_state.as_ref(), persistence.as_ref()).await;
    let body = metrics.render_prometheus(serving, &profile);
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

fn operational_environment_label(config: &ServerConfig) -> String {
    if config.is_production_kernel_profile() {
        return "production".to_string();
    }
    normalize_environment(&config.environment)
}

fn normalize_environment(environment: &str) -> String {
    match environment.to_ascii_lowercase().as_str() {
        "production" => "production".to_string(),
        "staging" | "stage" => "staging".to_string(),
        "test" => "test".to_string(),
        _ => "development".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_core_metric_families() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let profile = OperationalProfile::from_runtime("sqlite", false);
        registry.record_request("GET", "/healthz", 200, None, 0.01);
        registry.record_model_invocation("rig", "completed");
        let body = registry.render_prometheus(true, &profile);
        assert!(body.contains("sdkwork_kernel_health_status"));
        assert!(body.contains("sdkwork_kernel_http_requests_total"));
        assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_bucket"));
        assert!(body.contains("sdkwork_kernel_http_auth_failures_total"));
        assert!(body.contains("sdkwork_kernel_http_rate_limited_total"));
        assert!(body.contains("sdkwork_kernel_runtime_persistence_backend_info"));
        assert!(body.contains("sdkwork_kernel_rate_limit_backend_info"));
        assert!(body.contains("sdkwork_kernel_model_invocations_total"));
    }

    #[test]
    fn records_model_token_usage_counters() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let profile = OperationalProfile::from_runtime("sqlite", false);
        registry.record_model_token_usage("rig", "input", 12);
        registry.record_model_token_usage("rig", "output", 8);
        let body = registry.render_prometheus(true, &profile);
        assert!(body.contains("sdkwork_kernel_model_tokens_total"));
        assert!(body.contains("direction=\"input\""));
        assert!(body.contains("direction=\"output\""));
    }

    #[test]
    fn production_topology_profile_labels_metrics_as_production() {
        let _lock = crate::testing::env::lock();
        let _profile = crate::testing::env::VarGuard::set(
            "SDKWORK_KERNEL_PROFILE_ID",
            Some("cloud.production"),
        );
        let config = ServerConfig {
            environment: "development".to_string(),
            ..Default::default()
        };
        let registry = MetricsRegistry::from_config(&config);
        let body =
            registry.render_prometheus(true, &OperationalProfile::from_runtime("postgres", true));
        assert!(body.contains("environment=\"production\""));
    }

    #[test]
    fn records_auth_failure_counter_for_401() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let profile = OperationalProfile::from_runtime("postgres", true);
        registry.record_request(
            "GET",
            "/internal/v3/api/intelligence/runtime/snapshot",
            401,
            Some("internal-api"),
            0.001,
        );
        let body = registry.render_prometheus(true, &profile);
        assert!(body.contains("sdkwork_kernel_http_auth_failures_total"));
        assert!(body.contains("backend=\"postgres\""));
        assert!(body.contains("backend=\"redis\""));
        assert!(body.contains("} 1"));
    }
}
