//! Prometheus-compatible HTTP metrics (`OBSERVABILITY_SPEC.md`).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::config::ServerConfig;
use crate::health::{self, HealthState};
use crate::persistence::PersistenceState;

const SERVICE_NAME: &str = "sdkwork-agent-server";
const RUNTIME_TARGET: &str = "server";
const OVERFLOW_LABEL: &str = "overflow";
const MAX_METRIC_LABEL_LENGTH: usize = 256;
const MAX_HTTP_REQUEST_SERIES: usize = 2_048;
const MAX_HTTP_DURATION_SERIES: usize = 4_096;
const MAX_MODEL_INVOCATION_SERIES: usize = 512;
const MAX_MODEL_TOKEN_USAGE_SERIES: usize = 256;
static SSE_ACTIVE_CONNECTIONS: AtomicU64 = AtomicU64::new(0);

pub fn record_sse_connection_open() {
    SSE_ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_sse_connection_close() {
    let _ = SSE_ACTIVE_CONNECTIONS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(1))
    });
}

const DURATION_BUCKETS_SECS: [(f64, &str); 12] = [
    (0.005, "0.005"),
    (0.01, "0.01"),
    (0.025, "0.025"),
    (0.05, "0.05"),
    (0.1, "0.1"),
    (0.25, "0.25"),
    (0.5, "0.5"),
    (1.0, "1"),
    (2.5, "2.5"),
    (5.0, "5"),
    (10.0, "10"),
    (f64::INFINITY, "+Inf"),
];
const DURATION_SERIES_PER_FAMILY: usize = DURATION_BUCKETS_SECS.len() + 2;
const MAX_HTTP_DURATION_FAMILIES: usize = MAX_HTTP_DURATION_SERIES / DURATION_SERIES_PER_FAMILY;
const MAX_HTTP_DURATION_REGULAR_FAMILIES: usize = MAX_HTTP_DURATION_FAMILIES.saturating_sub(1);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RequestKey {
    method: String,
    route: String,
    status: String,
    api_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DurationFamilyKey {
    method: String,
    route: String,
    api_surface: String,
}

#[derive(Debug, Clone)]
struct DurationHistogram {
    buckets: [u64; DURATION_BUCKETS_SECS.len()],
    count: u64,
    sum: f64,
}

impl Default for DurationHistogram {
    fn default() -> Self {
        Self {
            buckets: [0; DURATION_BUCKETS_SECS.len()],
            count: 0,
            sum: 0.0,
        }
    }
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

pub type PrometheusMetricsState = (
    Arc<MetricsRegistry>,
    Arc<HealthState>,
    Arc<PersistenceState>,
    OperationalProfile,
);

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
    duration_histograms: Mutex<HashMap<DurationFamilyKey, DurationHistogram>>,
    model_invocations: Mutex<HashMap<(String, String), u64>>,
    model_token_usage: Mutex<HashMap<(String, String), u64>>,
    auth_failures_total: AtomicU64,
    rate_limited_total: AtomicU64,
    tenant_token_quota_rejected_total: AtomicU64,
    request_series_overflow_total: AtomicU64,
    duration_series_overflow_total: AtomicU64,
    model_invocation_series_overflow_total: AtomicU64,
    model_token_usage_series_overflow_total: AtomicU64,
    provider_admission_capacity: AtomicU64,
    provider_admission_wait_capacity: AtomicU64,
    provider_admission_active: AtomicU64,
    provider_admission_waiting: AtomicU64,
    provider_admission_queue_full_total: AtomicU64,
    provider_admission_timeout_total: AtomicU64,
    provider_admission_closed_total: AtomicU64,
    provider_admission_acquire_duration: Mutex<DurationHistogram>,
    render_lock: Mutex<()>,
}

/// RAII observation for one request waiting on provider admission.
pub struct ProviderAdmissionWaitGuard {
    metrics: Arc<MetricsRegistry>,
    started_at: Instant,
    finished: bool,
}

/// RAII observation for one admitted provider invocation.
pub struct ProviderAdmissionActiveGuard {
    metrics: Arc<MetricsRegistry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderAdmissionRejection {
    QueueFull,
    Timeout,
    Closed,
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
            duration_histograms: Mutex::new(HashMap::new()),
            model_invocations: Mutex::new(HashMap::new()),
            model_token_usage: Mutex::new(HashMap::new()),
            auth_failures_total: AtomicU64::new(0),
            rate_limited_total: AtomicU64::new(0),
            tenant_token_quota_rejected_total: AtomicU64::new(0),
            request_series_overflow_total: AtomicU64::new(0),
            duration_series_overflow_total: AtomicU64::new(0),
            model_invocation_series_overflow_total: AtomicU64::new(0),
            model_token_usage_series_overflow_total: AtomicU64::new(0),
            provider_admission_capacity: AtomicU64::new(config.provider_max_concurrency as u64),
            provider_admission_wait_capacity: AtomicU64::new(config.provider_max_waiters as u64),
            provider_admission_active: AtomicU64::new(0),
            provider_admission_waiting: AtomicU64::new(0),
            provider_admission_queue_full_total: AtomicU64::new(0),
            provider_admission_timeout_total: AtomicU64::new(0),
            provider_admission_closed_total: AtomicU64::new(0),
            provider_admission_acquire_duration: Mutex::new(DurationHistogram::default()),
            render_lock: Mutex::new(()),
        })
    }

    pub fn begin_provider_admission_wait(self: &Arc<Self>) -> ProviderAdmissionWaitGuard {
        saturating_atomic_add(&self.provider_admission_waiting, 1);
        ProviderAdmissionWaitGuard {
            metrics: self.clone(),
            started_at: Instant::now(),
            finished: false,
        }
    }

    pub fn record_provider_admission_rejection(&self, reason: ProviderAdmissionRejection) {
        let counter = match reason {
            ProviderAdmissionRejection::QueueFull => &self.provider_admission_queue_full_total,
            ProviderAdmissionRejection::Timeout => &self.provider_admission_timeout_total,
            ProviderAdmissionRejection::Closed => &self.provider_admission_closed_total,
        };
        saturating_atomic_add(counter, 1);
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
        let method_label = sanitize_metric_label(method, "unknown");
        let route_label = sanitize_route_label(route, "unknown");
        let surface = sanitize_metric_label(api_surface.unwrap_or("unknown"), "unknown");
        let key = RequestKey {
            method: method_label.clone(),
            route: route_label.clone(),
            status: status_label,
            api_surface: surface.clone(),
        };
        {
            let mut requests = lock_recover(&self.requests);
            let overflowed = record_bounded_series(
                &mut requests,
                key,
                overflow_request_key(),
                MAX_HTTP_REQUEST_SERIES,
                1,
                1,
            );
            if overflowed {
                saturating_atomic_add(&self.request_series_overflow_total, 1);
            }
        }

        {
            let mut histograms = lock_recover(&self.duration_histograms);
            let overflowed = record_duration_histogram(
                &mut histograms,
                DurationFamilyKey {
                    method: method_label,
                    route: route_label,
                    api_surface: surface,
                },
                duration_secs,
            );
            if overflowed {
                saturating_atomic_add(&self.duration_series_overflow_total, 1);
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
        {
            let mut invocations = lock_recover(&self.model_invocations);
            let overflowed = record_bounded_series(
                &mut invocations,
                (provider, status_label),
                overflow_pair(),
                MAX_MODEL_INVOCATION_SERIES,
                1,
                1,
            );
            if overflowed {
                saturating_atomic_add(&self.model_invocation_series_overflow_total, 1);
            }
        }
    }

    pub fn record_model_token_usage(&self, provider_id: &str, direction: &str, tokens: u64) {
        if tokens == 0 {
            return;
        }
        let provider = sanitize_metric_label(provider_id, "unknown");
        let direction_label = sanitize_metric_label(direction, "total");
        {
            let mut usage = lock_recover(&self.model_token_usage);
            let overflowed = record_bounded_series(
                &mut usage,
                (provider, direction_label),
                overflow_pair(),
                MAX_MODEL_TOKEN_USAGE_SERIES,
                1,
                tokens,
            );
            if overflowed {
                saturating_atomic_add(&self.model_token_usage_series_overflow_total, 1);
            }
        }
    }

    pub fn render_prometheus(&self, health_serving: bool, profile: &OperationalProfile) -> String {
        let _render_guard = lock_recover(&self.render_lock);
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
        {
            let requests = lock_recover(&self.requests);
            let mut entries: Vec<_> = requests.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
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
        {
            let histograms = lock_recover(&self.duration_histograms);
            let mut entries: Vec<_> = histograms.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for (key, histogram) in entries {
                for (index, (_, le)) in DURATION_BUCKETS_SECS.iter().enumerate() {
                    let count = histogram.buckets[index];
                    let _ = writeln!(
                        output,
                        "sdkwork_kernel_http_request_duration_seconds_bucket{{{base},method=\"{}\",route=\"{}\",api_surface=\"{}\",le=\"{le}\"}} {count}",
                        key.method, key.route, key.api_surface
                    );
                }
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_http_request_duration_seconds_count{{{base},method=\"{}\",route=\"{}\",api_surface=\"{}\"}} {}",
                    key.method, key.route, key.api_surface, histogram.count
                );
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_http_request_duration_seconds_sum{{{base},method=\"{}\",route=\"{}\",api_surface=\"{}\"}} {}",
                    key.method, key.route, key.api_surface, histogram.sum
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

        for (name, help, value) in [
            (
                "sdkwork_kernel_provider_admission_capacity",
                "Configured provider admission capacity in this server process.",
                self.provider_admission_capacity.load(Ordering::Relaxed),
            ),
            (
                "sdkwork_kernel_provider_admission_active",
                "Provider invocations currently holding an admission permit.",
                self.provider_admission_active.load(Ordering::Relaxed),
            ),
            (
                "sdkwork_kernel_provider_admission_wait_capacity",
                "Configured provider admission waiting capacity in this server process.",
                self.provider_admission_wait_capacity
                    .load(Ordering::Relaxed),
            ),
            (
                "sdkwork_kernel_provider_admission_waiting",
                "Provider invocations currently waiting for an admission permit.",
                self.provider_admission_waiting.load(Ordering::Relaxed),
            ),
        ] {
            let _ = writeln!(output, "# HELP {name} {help}");
            let _ = writeln!(output, "# TYPE {name} gauge");
            let _ = writeln!(output, "{name}{{{base}}} {value}");
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_provider_admission_rejected_total Provider invocations rejected before provider execution."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_provider_admission_rejected_total counter"
        );
        for (reason, counter) in [
            ("queue_full", &self.provider_admission_queue_full_total),
            ("timeout", &self.provider_admission_timeout_total),
            ("closed", &self.provider_admission_closed_total),
        ] {
            let _ = writeln!(
                output,
                "sdkwork_kernel_provider_admission_rejected_total{{{base},reason=\"{reason}\"}} {}",
                counter.load(Ordering::Relaxed)
            );
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_provider_admission_acquire_duration_seconds Provider admission permit acquisition duration histogram."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_provider_admission_acquire_duration_seconds histogram"
        );
        {
            let histogram = lock_recover(&self.provider_admission_acquire_duration);
            for (index, (_, le)) in DURATION_BUCKETS_SECS.iter().enumerate() {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_provider_admission_acquire_duration_seconds_bucket{{{base},le=\"{le}\"}} {}",
                    histogram.buckets[index]
                );
            }
            let _ = writeln!(
                output,
                "sdkwork_kernel_provider_admission_acquire_duration_seconds_count{{{base}}} {}",
                histogram.count
            );
            let _ = writeln!(
                output,
                "sdkwork_kernel_provider_admission_acquire_duration_seconds_sum{{{base}}} {}",
                histogram.sum
            );
        }

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
        {
            let invocations = lock_recover(&self.model_invocations);
            let mut entries: Vec<_> = invocations.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
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
        {
            let usage = lock_recover(&self.model_token_usage);
            let mut entries: Vec<_> = usage.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for ((provider_id, direction), count) in entries {
                let _ = writeln!(
                    output,
                    "sdkwork_kernel_model_tokens_total{{{base},provider_id=\"{provider_id}\",direction=\"{direction}\"}} {count}"
                );
            }
        }

        let _ = writeln!(
            output,
            "# HELP sdkwork_kernel_metrics_series_overflow_total Observations aggregated into a bounded overflow series after a metric-family series cap was reached."
        );
        let _ = writeln!(
            output,
            "# TYPE sdkwork_kernel_metrics_series_overflow_total counter"
        );
        for (metric_family, counter) in [
            (
                "sdkwork_kernel_http_requests_total",
                &self.request_series_overflow_total,
            ),
            (
                "sdkwork_kernel_http_request_duration_seconds",
                &self.duration_series_overflow_total,
            ),
            (
                "sdkwork_kernel_model_invocations_total",
                &self.model_invocation_series_overflow_total,
            ),
            (
                "sdkwork_kernel_model_tokens_total",
                &self.model_token_usage_series_overflow_total,
            ),
        ] {
            let _ = writeln!(
                output,
                "sdkwork_kernel_metrics_series_overflow_total{{{base},metric_family=\"{metric_family}\"}} {}",
                counter.load(Ordering::Relaxed)
            );
        }

        output
    }
}

impl ProviderAdmissionWaitGuard {
    pub fn acquired(mut self) -> ProviderAdmissionActiveGuard {
        saturating_atomic_sub(&self.metrics.provider_admission_waiting, 1);
        let duration_secs = self.started_at.elapsed().as_secs_f64();
        record_histogram_observation(
            &mut lock_recover(&self.metrics.provider_admission_acquire_duration),
            duration_secs,
        );
        saturating_atomic_add(&self.metrics.provider_admission_active, 1);
        self.finished = true;
        ProviderAdmissionActiveGuard {
            metrics: self.metrics.clone(),
        }
    }
}

impl Drop for ProviderAdmissionWaitGuard {
    fn drop(&mut self) {
        if !self.finished {
            saturating_atomic_sub(&self.metrics.provider_admission_waiting, 1);
        }
    }
}

impl Drop for ProviderAdmissionActiveGuard {
    fn drop(&mut self) {
        saturating_atomic_sub(&self.metrics.provider_admission_active, 1);
    }
}

fn record_bounded_series<K>(
    series: &mut HashMap<K, u64>,
    key: K,
    overflow_key: K,
    max_series: usize,
    reserved_overflow_series: usize,
    amount: u64,
) -> bool
where
    K: Eq + Hash,
{
    if let Some(count) = series.get_mut(&key) {
        *count = count.saturating_add(amount);
        return false;
    }

    let regular_capacity = max_series.saturating_sub(reserved_overflow_series);
    if series.len() < regular_capacity {
        series.insert(key, amount);
        return false;
    }

    let count = series.entry(overflow_key).or_insert(0);
    *count = count.saturating_add(amount);
    true
}

fn record_duration_histogram(
    histograms: &mut HashMap<DurationFamilyKey, DurationHistogram>,
    key: DurationFamilyKey,
    duration_secs: f64,
) -> bool {
    let overflow_key = overflow_duration_family_key();
    let use_overflow = if histograms.contains_key(&key) {
        false
    } else if histograms.len() < MAX_HTTP_DURATION_REGULAR_FAMILIES {
        histograms.insert(key.clone(), DurationHistogram::default());
        false
    } else {
        true
    };
    let target = if use_overflow { overflow_key } else { key };
    let histogram = histograms.entry(target).or_default();
    record_histogram_observation(histogram, duration_secs);
    use_overflow
}

fn record_histogram_observation(histogram: &mut DurationHistogram, duration_secs: f64) {
    let observed = if duration_secs.is_nan() || duration_secs.is_sign_negative() {
        0.0
    } else if duration_secs.is_infinite() {
        f64::MAX
    } else {
        duration_secs
    };
    histogram.count = histogram.count.saturating_add(1);
    let next_sum = histogram.sum + observed;
    histogram.sum = if next_sum.is_finite() {
        next_sum
    } else {
        f64::MAX
    };
    for (index, (upper_bound, _)) in DURATION_BUCKETS_SECS.iter().enumerate() {
        if observed <= *upper_bound {
            histogram.buckets[index] = histogram.buckets[index].saturating_add(1);
        }
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn overflow_request_key() -> RequestKey {
    RequestKey {
        method: OVERFLOW_LABEL.to_string(),
        route: OVERFLOW_LABEL.to_string(),
        status: OVERFLOW_LABEL.to_string(),
        api_surface: OVERFLOW_LABEL.to_string(),
    }
}

fn overflow_duration_family_key() -> DurationFamilyKey {
    DurationFamilyKey {
        method: OVERFLOW_LABEL.to_string(),
        route: OVERFLOW_LABEL.to_string(),
        api_surface: OVERFLOW_LABEL.to_string(),
    }
}

fn overflow_pair() -> (String, String) {
    (OVERFLOW_LABEL.to_string(), OVERFLOW_LABEL.to_string())
}

fn saturating_atomic_add(counter: &AtomicU64, amount: u64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_add(amount))
    });
}

fn saturating_atomic_sub(counter: &AtomicU64, amount: u64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(amount))
    });
}

fn sanitize_metric_label(value: &str, fallback: &str) -> String {
    sanitize_label(value, fallback, false)
}

fn sanitize_route_label(value: &str, fallback: &str) -> String {
    sanitize_label(value, fallback, true)
}

fn sanitize_label(value: &str, fallback: &str, preserve_route_syntax: bool) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }

    const TRUNCATED_SUFFIX: &str = "_truncated";
    let mut normalized = String::with_capacity(trimmed.len().min(MAX_METRIC_LABEL_LENGTH));
    let mut chars = trimmed.chars();
    for _ in 0..MAX_METRIC_LABEL_LENGTH {
        let Some(ch) = chars.next() else {
            break;
        };
        let safe = ch.is_ascii_alphanumeric()
            || matches!(ch, '.' | '_' | '-')
            || (preserve_route_syntax && matches!(ch, '/' | '{' | '}' | ':' | '*'));
        normalized.push(if safe { ch } else { '_' });
    }
    if chars.next().is_some() {
        normalized.truncate(MAX_METRIC_LABEL_LENGTH - TRUNCATED_SUFFIX.len());
        normalized.push_str(TRUNCATED_SUFFIX);
    }

    if normalized.is_empty() {
        return fallback.to_string();
    }
    if normalized == OVERFLOW_LABEL {
        normalized.push_str("_value");
    }
    normalized
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
    State((metrics, health_state, persistence, profile)): State<PrometheusMetricsState>,
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
        assert!(body.contains("sdkwork_kernel_provider_admission_capacity"));
        assert!(body.contains("sdkwork_kernel_provider_admission_wait_capacity"));
        assert!(body.contains("sdkwork_kernel_provider_admission_active"));
        assert!(body.contains("sdkwork_kernel_provider_admission_waiting"));
        assert!(body.contains("sdkwork_kernel_provider_admission_rejected_total"));
        assert!(body.contains("sdkwork_kernel_provider_admission_acquire_duration_seconds_bucket"));
        assert!(body.contains("sdkwork_kernel_metrics_series_overflow_total"));
    }

    #[test]
    fn provider_admission_guards_track_wait_active_and_cancelled_lifecycles() {
        let config = ServerConfig {
            provider_max_concurrency: 3,
            ..Default::default()
        };
        let registry = MetricsRegistry::from_config(&config);

        let cancelled_wait = registry.begin_provider_admission_wait();
        assert_eq!(
            registry.provider_admission_waiting.load(Ordering::Relaxed),
            1
        );
        drop(cancelled_wait);
        assert_eq!(
            registry.provider_admission_waiting.load(Ordering::Relaxed),
            0
        );

        let active = registry.begin_provider_admission_wait().acquired();
        assert_eq!(
            registry.provider_admission_waiting.load(Ordering::Relaxed),
            0
        );
        assert_eq!(
            registry.provider_admission_active.load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            lock_recover(&registry.provider_admission_acquire_duration).count,
            1
        );
        drop(active);
        assert_eq!(
            registry.provider_admission_active.load(Ordering::Relaxed),
            0
        );

        registry.record_provider_admission_rejection(ProviderAdmissionRejection::QueueFull);
        registry.record_provider_admission_rejection(ProviderAdmissionRejection::Timeout);
        registry.record_provider_admission_rejection(ProviderAdmissionRejection::Closed);
        let body =
            registry.render_prometheus(true, &OperationalProfile::from_runtime("sqlite", false));
        assert!(body.contains("sdkwork_kernel_provider_admission_capacity{"));
        assert!(body.contains("runtime_target=\"server\"} 3"));
        assert!(body.contains("sdkwork_kernel_provider_admission_rejected_total{"));
        assert!(body.contains("reason=\"queue_full\"} 1"));
        assert!(body.contains("reason=\"timeout\"} 1"));
        assert!(body.contains("reason=\"closed\"} 1"));
        assert!(body.contains("sdkwork_kernel_provider_admission_acquire_duration_seconds_count{"));
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

    #[test]
    fn http_metric_series_are_bounded_and_preserve_overflow_totals() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let observations = MAX_HTTP_REQUEST_SERIES + 64;
        for index in 0..observations {
            registry.record_request(
                "GET",
                &format!("/synthetic/routes/{index}"),
                200,
                Some("internal-api"),
                0.0,
            );
        }

        {
            let requests = registry.requests.lock().expect("request metrics lock");
            assert_eq!(requests.len(), MAX_HTTP_REQUEST_SERIES);
            assert_eq!(requests.values().copied().sum::<u64>(), observations as u64);
            let overflow_count = requests
                .get(&overflow_request_key())
                .copied()
                .expect("request overflow series");
            assert_eq!(
                registry
                    .request_series_overflow_total
                    .load(Ordering::Relaxed),
                overflow_count
            );
        }

        {
            let histograms = registry
                .duration_histograms
                .lock()
                .expect("duration metrics lock");
            assert!(histograms.len() <= MAX_HTTP_DURATION_FAMILIES);
            assert_eq!(
                histograms
                    .values()
                    .map(|histogram| histogram.count)
                    .sum::<u64>(),
                observations as u64
            );
            let overflow_count = histograms
                .get(&overflow_duration_family_key())
                .map(|histogram| histogram.count)
                .expect("overflow histogram");
            assert!(overflow_count > 0);
            assert!(histograms.values().all(|histogram| {
                histogram.buckets[DURATION_BUCKETS_SECS.len() - 1] == histogram.count
            }));
            assert_eq!(
                registry
                    .duration_series_overflow_total
                    .load(Ordering::Relaxed),
                overflow_count
            );
        }

        let body =
            registry.render_prometheus(true, &OperationalProfile::from_runtime("sqlite", false));
        assert!(body.contains("method=\"overflow\",route=\"overflow\""));
        assert!(body.contains("metric_family=\"sdkwork_kernel_http_requests_total\""));
        assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_count"));
        assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_sum"));
    }

    #[test]
    fn http_duration_histogram_is_cumulative_and_preserves_count_and_sum() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        registry.record_request("GET", "/healthz", 200, Some("internal-api"), 0.01);
        registry.record_request("GET", "/healthz", 200, Some("internal-api"), 0.26);
        registry.record_request("GET", "/healthz", 200, Some("internal-api"), 12.0);

        let key = DurationFamilyKey {
            method: "GET".to_string(),
            route: "/healthz".to_string(),
            api_surface: "internal-api".to_string(),
        };
        let histograms = lock_recover(&registry.duration_histograms);
        let histogram = histograms.get(&key).expect("duration histogram");
        assert_eq!(histogram.count, 3);
        assert!((histogram.sum - 12.27).abs() < f64::EPSILON);
        assert_eq!(histogram.buckets[0], 0);
        assert_eq!(histogram.buckets[1], 1);
        assert_eq!(histogram.buckets[5], 1);
        assert_eq!(histogram.buckets[6], 2);
        assert_eq!(histogram.buckets[10], 2);
        assert_eq!(histogram.buckets[11], histogram.count);
    }

    #[test]
    fn concurrent_prometheus_renders_produce_complete_histogram_families() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let profile = OperationalProfile::from_runtime("sqlite", false);
        registry.record_request("GET", "/healthz", 200, Some("internal-api"), 0.01);

        let mut workers = Vec::new();
        for _ in 0..8 {
            let registry = Arc::clone(&registry);
            let profile = profile.clone();
            workers.push(std::thread::spawn(move || {
                let body = registry.render_prometheus(true, &profile);
                assert_eq!(
                    body.matches("# TYPE sdkwork_kernel_http_request_duration_seconds histogram")
                        .count(),
                    1
                );
                assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_bucket"));
                assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_count"));
                assert!(body.contains("sdkwork_kernel_http_request_duration_seconds_sum"));
            }));
        }
        for worker in workers {
            worker.join().expect("metrics render worker");
        }
    }

    #[test]
    fn model_metric_series_are_bounded_and_preserve_overflow_totals() {
        let registry = MetricsRegistry::from_config(&ServerConfig::default());
        let invocation_observations = MAX_MODEL_INVOCATION_SERIES + 37;
        for index in 0..invocation_observations {
            registry.record_model_invocation(&format!("provider.{index}"), "completed");
        }
        let token_observations = MAX_MODEL_TOKEN_USAGE_SERIES + 29;
        for index in 0..token_observations {
            registry.record_model_token_usage(&format!("provider.{index}"), "input", 3);
        }

        {
            let invocations = registry
                .model_invocations
                .lock()
                .expect("model invocation metrics lock");
            assert_eq!(invocations.len(), MAX_MODEL_INVOCATION_SERIES);
            assert_eq!(
                invocations.values().copied().sum::<u64>(),
                invocation_observations as u64
            );
            let overflow_count = invocations
                .get(&overflow_pair())
                .copied()
                .expect("model invocation overflow series");
            assert_eq!(
                registry
                    .model_invocation_series_overflow_total
                    .load(Ordering::Relaxed),
                overflow_count
            );
        }

        {
            let usage = registry
                .model_token_usage
                .lock()
                .expect("model token metrics lock");
            assert_eq!(usage.len(), MAX_MODEL_TOKEN_USAGE_SERIES);
            assert_eq!(
                usage.values().copied().sum::<u64>(),
                (token_observations * 3) as u64
            );
            let overflow_tokens = usage
                .get(&overflow_pair())
                .copied()
                .expect("model token overflow series");
            let overflow_observations = registry
                .model_token_usage_series_overflow_total
                .load(Ordering::Relaxed);
            assert_eq!(overflow_tokens, overflow_observations * 3);
        }
    }

    #[test]
    fn dynamic_metric_labels_are_length_bounded_and_cannot_spoof_overflow() {
        let long_label = "x".repeat(MAX_METRIC_LABEL_LENGTH + 128);
        let normalized = sanitize_metric_label(&long_label, "unknown");
        assert_eq!(normalized.len(), MAX_METRIC_LABEL_LENGTH);
        assert!(normalized.ends_with("_truncated"));

        let route = sanitize_route_label("/internal/v3/api/{session_id}\n\"injected\"", "unknown");
        assert!(route.starts_with("/internal/v3/api/{session_id}"));
        assert!(!route.contains(['\n', '\r', '"', '\\']));
        assert_eq!(
            sanitize_metric_label(OVERFLOW_LABEL, "unknown"),
            "overflow_value"
        );
    }
}
