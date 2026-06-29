//! RFC 9457 Problem Details for HTTP APIs.
//!
//! Provides a structured JSON error response format that replaces bare
//! `StatusCode` returns with a machine-readable `application/problem+json`
//! body containing `type`, `title`, `status`, `detail`, and `instance`
//! fields.
//!
//! All HTTP error responses from the internal-api surface use this format
//! to give SDK consumers and UI clients consistent, actionable error
//! metadata.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// RFC 9457 Problem Details JSON object.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetail {
    /// A URI reference identifying the problem type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// A short, human-readable summary of the problem type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The HTTP status code generated for this occurrence.
    pub status: u16,
    /// A human-readable explanation specific to this occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A URI reference identifying the specific occurrence of the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Optional trace ID for distributed correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ProblemDetail {
    /// Create a new `ProblemDetail` with the given status code.
    pub fn new(status: StatusCode) -> Self {
        Self {
            r#type: Some(format!(
                "https://docs.sdkwork.com/problems/{}",
                status.as_u16()
            )),
            title: Some(canonical_title(status).to_string()),
            status: status.as_u16(),
            detail: None,
            instance: None,
            trace_id: None,
        }
    }

    /// Attach a human-readable detail message.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attach a request instance URI.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Attach a trace ID for distributed correlation.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            Json(self),
        )
            .into_response()
    }
}

/// Convert a `StatusCode` to a `ProblemDetail` response.
pub fn problem_response(status: StatusCode) -> Response {
    ProblemDetail::new(status).into_response()
}

/// Convert a `StatusCode` with a detail message to a `ProblemDetail` response.
pub fn problem_response_with_detail(status: StatusCode, detail: impl Into<String>) -> Response {
    ProblemDetail::new(status)
        .with_detail(detail)
        .into_response()
}

/// Canonical human-readable title for each HTTP status code.
fn canonical_title(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "Bad Request",
        StatusCode::UNAUTHORIZED => "Unauthorized",
        StatusCode::FORBIDDEN => "Forbidden",
        StatusCode::NOT_FOUND => "Not Found",
        StatusCode::CONFLICT => "Conflict",
        StatusCode::REQUEST_TIMEOUT => "Request Timeout",
        StatusCode::TOO_MANY_REQUESTS => "Too Many Requests",
        StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error",
        StatusCode::SERVICE_UNAVAILABLE => "Service Unavailable",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_detail_serializes_rfc_9457_fields() {
        let problem = ProblemDetail::new(StatusCode::TOO_MANY_REQUESTS)
            .with_detail("Rate limit exceeded")
            .with_trace_id("trace.abc123");
        let json = serde_json::to_value(&problem).expect("serialize");
        assert_eq!(json["status"], 429);
        assert_eq!(json["title"], "Too Many Requests");
        assert_eq!(json["detail"], "Rate limit exceeded");
        assert_eq!(json["traceId"], "trace.abc123");
        assert!(json["type"].as_str().unwrap().contains("429"));
    }

    #[test]
    fn problem_detail_skips_none_fields() {
        let problem = ProblemDetail::new(StatusCode::NOT_FOUND);
        let json = serde_json::to_value(&problem).expect("serialize");
        assert_eq!(json["status"], 404);
        assert!(json.get("detail").is_none());
        assert!(json.get("instance").is_none());
        assert!(json.get("traceId").is_none());
    }
}
