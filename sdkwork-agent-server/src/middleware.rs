use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::{info, Span};

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
            request_id: extract_header(headers, "x-request-id").unwrap_or_else(|| uuid_simple()),
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

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

/// Logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    info!("{} {}", method, uri);

    let response = next.run(request).await;

    let duration = start.elapsed();
    info!(
        "{} {} -> {} ({:.2}ms)",
        method,
        uri,
        response.status(),
        duration.as_secs_f64() * 1000.0
    );

    response
}

/// CORS middleware
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::permissive()
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
}
