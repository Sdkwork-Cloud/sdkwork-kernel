use axum::{
    extract::{Request, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use subtle::ConstantTimeEq;
use tracing::{info, warn};

use crate::config::ServerConfig;

/// Request context extracted from headers
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub subject_id: Option<String>,
}

impl RequestContext {
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Self {
        Self {
            request_id: extract_header(headers, "x-request-id").unwrap_or_else(generate_request_id),
            tenant_id: extract_header(headers, "x-sdkwork-tenant-id"),
            user_id: extract_header(headers, "x-sdkwork-user-id"),
            subject_id: extract_header(headers, "x-subject-id"),
        }
    }
}

fn extract_header(headers: &axum::http::HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

/// Attach request context for downstream handlers and logging.
pub async fn request_context_middleware(mut request: Request, next: Next) -> Response {
    let ctx = RequestContext::from_headers(request.headers());
    request.extensions_mut().insert(ctx);
    next.run(request).await
}

/// Logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::from_headers(request.headers()));
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    info!(
        request_id = %ctx.request_id,
        tenant_id = ?ctx.tenant_id,
        user_id = ?ctx.user_id,
        "{} {}",
        method,
        uri
    );

    let mut response = next.run(request).await;

    if let Ok(value) = axum::http::HeaderValue::from_str(&ctx.request_id) {
        response.headers_mut().insert("x-request-id", value);
    }

    let duration = start.elapsed();
    info!(
        request_id = %ctx.request_id,
        tenant_id = ?ctx.tenant_id,
        user_id = ?ctx.user_id,
        status = %response.status(),
        duration_ms = duration.as_secs_f64() * 1000.0,
        "{} {} -> {}",
        method,
        uri,
        response.status()
    );

    response
}

/// Optional ingress token auth for non-health API routes.
pub async fn ingress_auth_middleware(
    State(config): State<Arc<ServerConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !config.ingress_auth_mode.eq_ignore_ascii_case("token") {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if path == "/health" || path == "/ready" || path == "/live" {
        return Ok(next.run(request).await);
    }

    let expected = config
        .ingress_token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            warn!("ingress auth mode token is enabled but SDKWORK_KERNEL_INGRESS_TOKEN is missing");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    if authorize_request(request.headers(), expected) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn authorize_request(headers: &HeaderMap, expected: &str) -> bool {
    if let Some(token) = bearer_token(headers) {
        return constant_time_eq(&token, expected);
    }
    if let Some(token) = extract_header(headers, "x-api-key") {
        return constant_time_eq(&token, expected);
    }
    extract_header(headers, "x-sdkwork-access-token")
        .map(|token| constant_time_eq(&token, expected))
        .unwrap_or(false)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let prefix = "Bearer ";
    value
        .strip_prefix(prefix)
        .map(|token| token.trim().to_string())
}

#[derive(Debug)]
struct RateBucket {
    tokens: f64,
    last_refill: Instant,
}

/// Shared token-bucket rate limiter keyed by tenant/user or client address.
#[derive(Debug, Clone)]
pub struct RateLimitState {
    rps: u32,
    burst: u32,
    buckets: Arc<Mutex<HashMap<String, RateBucket>>>,
}

impl RateLimitState {
    pub fn from_config(config: &ServerConfig) -> Self {
        Self {
            rps: config.rate_limit_rps,
            burst: config.rate_limit_burst.max(1),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.rps > 0
    }

    pub fn try_acquire(&self, key: &str) -> bool {
        if !self.is_enabled() {
            return true;
        }

        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let now = Instant::now();
        let bucket = buckets.entry(key.to_string()).or_insert(RateBucket {
            tokens: f64::from(self.burst),
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * f64::from(self.rps)).min(f64::from(self.burst));
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

fn rate_limit_key(headers: &HeaderMap, ctx: Option<&RequestContext>) -> String {
    if let Some(ctx) = ctx {
        if let (Some(tenant), Some(user)) = (&ctx.tenant_id, &ctx.user_id) {
            return format!("{tenant}:{user}");
        }
        if let Some(tenant) = &ctx.tenant_id {
            return format!("tenant:{tenant}");
        }
    }

    extract_header(headers, "x-forwarded-for")
        .or_else(|| extract_header(headers, "x-real-ip"))
        .unwrap_or_else(|| "global".to_string())
}

/// Reject excess ingress traffic before handlers run.
pub async fn rate_limit_middleware(
    State(rate_limit): State<Arc<RateLimitState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !rate_limit.is_enabled() {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if path == "/health" || path == "/ready" || path == "/live" {
        return Ok(next.run(request).await);
    }

    let ctx = request.extensions().get::<RequestContext>().cloned();
    let key = rate_limit_key(request.headers(), ctx.as_ref());
    if rate_limit.try_acquire(&key) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
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
            HeaderName::from_static("x-sdkwork-tenant-id"),
            HeaderName::from_static("x-sdkwork-user-id"),
            HeaderName::from_static("x-sdkwork-access-token"),
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
        headers.insert("x-request-id", HeaderValue::from_static("req.1"));
        headers.insert("x-sdkwork-tenant-id", HeaderValue::from_static("tenant.1"));

        let ctx = RequestContext::from_headers(&headers);
        assert_eq!(ctx.request_id, "req.1");
        assert_eq!(ctx.tenant_id, Some("tenant.1".to_string()));
        assert_eq!(ctx.user_id, None);
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
        let state = RateLimitState {
            rps: 1,
            burst: 1,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        };
        assert!(state.try_acquire("client.1"));
        assert!(!state.try_acquire("client.1"));
        assert!(state.try_acquire("client.2"));
    }
}
