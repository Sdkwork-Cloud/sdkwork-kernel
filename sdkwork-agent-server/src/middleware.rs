use axum::{
    extract::{Request, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::config::ServerConfig;
use crate::http_surface::{classify_api_surface, route_template};
use crate::ingress_identity;
use crate::ingress_state::IngressMiddlewareState;
use crate::metrics::MetricsRegistry;
use crate::problem_details::ProblemDetail;
use crate::rate_limit::RateLimitState;
use crate::security_audit;

/// Request context extracted from headers
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: Option<String>,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub subject_id: Option<String>,
    pub api_surface: Option<&'static str>,
    pub route_template: String,
}

impl RequestContext {
    pub fn from_headers_and_path(headers: &axum::http::HeaderMap, path: &str) -> Self {
        let trace_id = headers
            .get("traceparent")
            .and_then(|value| value.to_str().ok())
            .and_then(crate::observability::trace_id_from_traceparent);
        Self {
            request_id: generate_request_id(),
            trace_id,
            tenant_id: extract_header(headers, "x-sdkwork-tenant-id"),
            user_id: extract_header(headers, "x-sdkwork-user-id"),
            subject_id: extract_header(headers, "x-subject-id"),
            api_surface: classify_api_surface(path),
            route_template: route_template(path),
        }
    }

    pub fn problem_trace_id(&self) -> String {
        if let Some(trace_id) = self.trace_id.as_deref().filter(|value| !value.is_empty()) {
            return trace_id.to_string();
        }
        self.request_id
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect()
    }
}

fn extract_header(headers: &axum::http::HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn generate_request_id() -> String {
    format!("req.{}", uuid::Uuid::now_v7().simple())
}

/// Attach request context for downstream handlers and logging.
pub async fn request_context_middleware(mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let mut ctx = RequestContext::from_headers_and_path(request.headers(), path);
    ctx.request_id = generate_request_id();
    request.extensions_mut().insert(ctx);
    next.run(request).await
}

fn is_health_probe(path: &str) -> bool {
    matches!(path, "/health" | "/ready" | "/live")
}

fn is_metrics_path(path: &str) -> bool {
    path == "/metrics"
}

fn is_operational_probe(path: &str) -> bool {
    is_health_probe(path) || is_metrics_path(path)
}

/// Logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let metrics = request.extensions().get::<Arc<MetricsRegistry>>().cloned();
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| {
            RequestContext::from_headers_and_path(request.headers(), request.uri().path())
        });
    let method = request.method().clone();
    let start = std::time::Instant::now();

    info!(
        request_id = %ctx.request_id,
        trace_id = ?ctx.trace_id,
        tenant_id = ?ctx.tenant_id,
        user_id = ?ctx.user_id,
        api_surface = ?ctx.api_surface,
        route = %ctx.route_template,
        method = %method,
        "http.request.start"
    );

    let mut response = next.run(request).await;

    if let Ok(value) = axum::http::HeaderValue::from_str(&ctx.request_id) {
        response.headers_mut().insert("x-request-id", value);
    }

    let duration = start.elapsed();
    let status = response.status().as_u16();
    if let Some(metrics) = metrics {
        metrics.record_request(
            method.as_str(),
            &ctx.route_template,
            status,
            ctx.api_surface,
            duration.as_secs_f64(),
        );
    }

    info!(
        request_id = %ctx.request_id,
        trace_id = ?ctx.trace_id,
        tenant_id = ?ctx.tenant_id,
        user_id = ?ctx.user_id,
        api_surface = ?ctx.api_surface,
        route = %ctx.route_template,
        method = %method,
        status = status,
        duration_ms = duration.as_secs_f64() * 1000.0,
        "http.request.complete"
    );

    response
}

/// Resolve and validate caller identity after ingress token auth succeeds.
pub async fn ingress_identity_middleware(
    State(ingress): State<Arc<IngressMiddlewareState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetail> {
    let config = ingress.config.as_ref();
    if config.ingress_auth_mode.eq_ignore_ascii_case("open") {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if is_health_probe(path) || is_metrics_path(path) {
        return Ok(next.run(request).await);
    }

    if config.ingress_auth_mode.eq_ignore_ascii_case("jwt") {
        let ctx = request
            .extensions()
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_else(|| RequestContext::from_headers_and_path(request.headers(), path));
        let has_identity = ctx
            .tenant_id
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && ctx
                .user_id
                .as_deref()
                .is_some_and(|value| !value.is_empty());
        if has_identity {
            request.extensions_mut().insert(ctx);
            return Ok(next.run(request).await);
        }
        security_audit::log_auth_failure(
            "ingress.identity_rejected",
            Some(ctx.request_id.as_str()),
            path,
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            "jwt ingress missing verified tenant/user identity",
        );
        return Err(ProblemDetail::new(StatusCode::FORBIDDEN)
            .with_detail("JWT ingress missing verified tenant/user identity")
            .with_trace_id(ctx.problem_trace_id()));
    }

    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::from_headers_and_path(request.headers(), path));
    let resolved = ingress_identity::resolve_request_identity(config, request.headers(), ctx);
    let resolved = match resolved {
        Ok(ctx) => ctx,
        Err(status) => {
            let path = request.uri().path();
            let ctx = request.extensions().get::<RequestContext>();
            security_audit::log_auth_failure(
                "ingress.identity_rejected",
                ctx.map(|value| value.request_id.as_str()),
                path,
                ctx.and_then(|value| value.tenant_id.as_deref()),
                ctx.and_then(|value| value.user_id.as_deref()),
                &format!("status={status}"),
            );
            return Err(ProblemDetail::new(status)
                .with_detail(format!("Ingress identity resolution failed: {status}"))
                .with_trace_id(ctx.map(|v| v.problem_trace_id()).unwrap_or_else(|| "unknown".to_string())));
        }
    };
    request.extensions_mut().insert(resolved);
    Ok(next.run(request).await)
}

/// Attach baseline security headers required by SECURITY_SPEC.
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'"),
    );
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
    response
}

/// Optional ingress token or JWT auth for non-health API routes.
pub async fn ingress_auth_middleware(
    State(ingress): State<Arc<IngressMiddlewareState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetail> {
    let config = ingress.config.as_ref();
    let auth_mode = config.ingress_auth_mode.to_ascii_lowercase();
    if auth_mode != "token" && auth_mode != "jwt" {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if is_health_probe(path) {
        return Ok(next.run(request).await);
    }

    if is_metrics_path(path) {
        if !config.metrics_auth_required() {
            return Ok(next.run(request).await);
        }
        let expected = config.effective_metrics_token().ok_or_else(|| {
            warn!("metrics auth mode token is enabled but no metrics token is configured");
            ProblemDetail::new(StatusCode::SERVICE_UNAVAILABLE)
                .with_detail("Metrics auth token is not configured")
        })?;
        if authorize_request(request.headers(), expected) {
            return Ok(next.run(request).await);
        }
        let ctx = request.extensions().get::<RequestContext>();
        security_audit::log_auth_failure(
            "metrics.token_rejected",
            ctx.map(|value| value.request_id.as_str()),
            path,
            ctx.and_then(|value| value.tenant_id.as_deref()),
            ctx.and_then(|value| value.user_id.as_deref()),
            "invalid or missing metrics credential",
        );
        return Err(ProblemDetail::new(StatusCode::UNAUTHORIZED)
            .with_detail("Invalid or missing metrics credential")
            .with_trace_id(ctx.map(|v| v.problem_trace_id()).unwrap_or_else(|| "unknown".to_string())));
    }

    if auth_mode == "jwt" {
        let bearer = bearer_token(request.headers()).ok_or_else(|| {
            let ctx = request.extensions().get::<RequestContext>();
            security_audit::log_auth_failure(
                "ingress.jwt_rejected",
                ctx.map(|value| value.request_id.as_str()),
                path,
                ctx.and_then(|value| value.tenant_id.as_deref()),
                ctx.and_then(|value| value.user_id.as_deref()),
                "missing bearer jwt",
            );
            ProblemDetail::new(StatusCode::UNAUTHORIZED)
                .with_detail("Missing Bearer JWT")
                .with_trace_id(ctx.map(|v| v.problem_trace_id()).unwrap_or_else(|| "unknown".to_string()))
        })?;
        let identity = ingress
            .jwt_validator
            .as_ref()
            .ok_or_else(|| {
                ProblemDetail::new(StatusCode::SERVICE_UNAVAILABLE)
                    .with_detail("JWT validator is not initialized")
            })?
            .validate(&bearer)
            .map_err(|status| {
                let ctx = request.extensions().get::<RequestContext>();
                security_audit::log_auth_failure(
                    "ingress.jwt_rejected",
                    ctx.map(|value| value.request_id.as_str()),
                    path,
                    ctx.and_then(|value| value.tenant_id.as_deref()),
                    ctx.and_then(|value| value.user_id.as_deref()),
                    &format!("status={status}"),
                );
                ProblemDetail::new(status)
                    .with_detail("JWT validation failed")
                    .with_trace_id(ctx.map(|v| v.problem_trace_id()).unwrap_or_else(|| "unknown".to_string()))
            })?;
        let mut ctx = request
            .extensions()
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_else(|| RequestContext::from_headers_and_path(request.headers(), path));
        ctx.tenant_id = Some(identity.tenant_id);
        ctx.user_id = Some(identity.user_id);
        ctx.subject_id = None;
        request.extensions_mut().insert(ctx);
        return Ok(next.run(request).await);
    }

    let expected = config
        .ingress_token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            warn!("ingress auth mode token is enabled but SDKWORK_KERNEL_INGRESS_TOKEN is missing");
            ProblemDetail::new(StatusCode::SERVICE_UNAVAILABLE)
                .with_detail("Ingress token is not configured")
        })?;

    if authorize_request(request.headers(), expected) {
        Ok(next.run(request).await)
    } else {
        let path = request.uri().path();
        let ctx = request.extensions().get::<RequestContext>();
        security_audit::log_auth_failure(
            "ingress.token_rejected",
            ctx.map(|value| value.request_id.as_str()),
            path,
            ctx.and_then(|value| value.tenant_id.as_deref()),
            ctx.and_then(|value| value.user_id.as_deref()),
            "invalid or missing ingress credential",
        );
        Err(ProblemDetail::new(StatusCode::UNAUTHORIZED)
            .with_detail("Invalid or missing ingress credential")
            .with_trace_id(ctx.map(|v| v.problem_trace_id()).unwrap_or_else(|| "unknown".to_string())))
    }
}

fn authorize_request(headers: &HeaderMap, expected: &str) -> bool {
    if let Some(token) = bearer_token(headers) {
        return ingress_identity::constant_time_eq(&token, expected);
    }
    if let Some(token) = extract_header(headers, "x-api-key") {
        return ingress_identity::constant_time_eq(&token, expected);
    }
    extract_header(headers, "x-sdkwork-access-token")
        .map(|token| ingress_identity::constant_time_eq(&token, expected))
        .unwrap_or(false)
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    const PREFIX: &str = "Bearer ";
    if value.len() <= PREFIX.len() {
        return None;
    }
    if !value[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        return None;
    }
    Some(value[PREFIX.len()..].trim().to_string())
}

fn rate_limit_key(headers: &HeaderMap, ctx: Option<&RequestContext>) -> String {
    if let Some(ctx) = ctx {
        if let (Some(tenant), Some(user)) = (&ctx.tenant_id, &ctx.user_id) {
            if !tenant.is_empty() && !user.is_empty() {
                return format!("identity:{tenant}:{user}");
            }
        }
        if let Some(tenant) = &ctx.tenant_id {
            if !tenant.is_empty() {
                return format!("tenant:{tenant}");
            }
        }
    }

    if let Some(token) = bearer_token(headers)
        .or_else(|| extract_header(headers, "x-api-key"))
        .or_else(|| extract_header(headers, "x-sdkwork-access-token"))
    {
        // Use SHA-256 for a stable, platform-independent fingerprint.
        // DefaultHasher is not guaranteed stable across Rust versions or
        // platforms, which would cause rate-limit keys to change unpredictably.
        let digest = sdkwork_utils_rust::sha256_hash(token.as_bytes());
        return format!("ingress-token:{}", &digest[..16]);
    }

    "global".to_string()
}

/// Reject excess ingress traffic before handlers run.
pub async fn rate_limit_middleware(
    State(rate_limit): State<Arc<RateLimitState>>,
    request: Request,
    next: Next,
) -> Result<Response, ProblemDetail> {
    if !rate_limit.is_enabled() {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if is_operational_probe(path) {
        return Ok(next.run(request).await);
    }

    let metrics = request.extensions().get::<Arc<MetricsRegistry>>().cloned();
    let ctx = request.extensions().get::<RequestContext>().cloned();
    let tenant_id = ctx.as_ref().and_then(|value| value.tenant_id.as_deref());
    let key = rate_limit_key(request.headers(), ctx.as_ref());
    if rate_limit.try_acquire(&key, tenant_id).await {
        Ok(next.run(request).await)
    } else {
        if let Some(metrics) = metrics {
            metrics.record_rate_limited();
        }
        Err(ProblemDetail::new(StatusCode::TOO_MANY_REQUESTS)
            .with_detail("Rate limit exceeded; try again later")
            .with_trace_id(
                ctx.as_ref()
                    .map(|v| v.problem_trace_id())
                    .unwrap_or_else(|| "unknown".to_string()),
            ))
    }
}

/// Build CORS layer from server configuration.
pub fn cors_layer(config: &ServerConfig) -> tower_http::cors::CorsLayer {
    if !config.cors_enabled {
        return tower_http::cors::CorsLayer::new();
    }

    let mut layer = tower_http::cors::CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-sdkwork-tenant-id"),
            HeaderName::from_static("x-sdkwork-user-id"),
            HeaderName::from_static("x-sdkwork-access-token"),
            HeaderName::from_static("x-sdkwork-identity-mac"),
            HeaderName::from_static("x-subject-id"),
        ]);

    if config.cors_origins.iter().any(|origin| origin == "*") {
        layer = layer.allow_origin(tower_http::cors::Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        layer = layer.allow_origin(origins);
    }

    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_context_from_headers() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req.client"));
        headers.insert("x-sdkwork-tenant-id", HeaderValue::from_static("tenant.1"));

        let ctx = RequestContext::from_headers_and_path(
            &headers,
            "/internal/v3/api/intelligence/runtime/snapshot",
        );
        assert_ne!(ctx.request_id, "req.client");
        assert_eq!(ctx.tenant_id, Some("tenant.1".to_string()));
        assert_eq!(ctx.user_id, None);
        assert_eq!(ctx.api_surface, Some("internal-api"));
    }

    #[test]
    fn bearer_token_parses_authorization_header() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret-token"),
        );
        assert_eq!(bearer_token(&headers), Some("secret-token".to_string()));
    }

    #[test]
    fn rate_limit_rejects_when_burst_exhausted() {
        use crate::rate_limit::RateLimitState;

        let state = RateLimitState::from_config(&ServerConfig {
            rate_limit_rps: 1,
            rate_limit_burst: 1,
            ..Default::default()
        });
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            assert!(state.try_acquire("client.1", None).await);
            assert!(!state.try_acquire("client.1", None).await);
            assert!(state.try_acquire("client.2", None).await);
        });
    }
}
