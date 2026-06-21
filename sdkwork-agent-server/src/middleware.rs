use axum::{
    extract::{Request, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
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

/// Logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let ctx = RequestContext::from_headers(request.headers());
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
        return token == expected;
    }
    extract_header(headers, "x-sdkwork-access-token").as_deref() == Some(expected)
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let prefix = "Bearer ";
    value
        .strip_prefix(prefix)
        .map(|token| token.trim().to_string())
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
}
