//! RFC 9457 Problem Details for HTTP APIs (`API_SPEC.md` §15).
//!
//! Middleware and legacy handler paths use [`ProblemDetail`] builders that
//! serialize through [`SdkWorkProblemDetail`] with numeric platform `code` and
//! required `traceId`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sdkwork_utils_rust::{SdkWorkProblemDetail, SdkWorkResultCode};

/// Builder for middleware-friendly problem responses.
#[derive(Debug)]
pub struct ProblemDetail {
    status: StatusCode,
    detail: Option<String>,
    trace_id: Option<String>,
    instance: Option<String>,
}

impl ProblemDetail {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            detail: None,
            trace_id: None,
            instance: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    fn into_sdk(self) -> SdkWorkProblemDetail {
        let code = result_code_for_status(self.status);
        let trace_id = self
            .trace_id
            .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());
        let detail = self.detail.unwrap_or_else(|| code.title().to_string());
        let mut problem = SdkWorkProblemDetail::platform(code, detail, trace_id);
        problem.instance = self.instance;
        problem
    }
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let problem = self.into_sdk();
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            Json(problem),
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

pub fn result_code_for_status(status: StatusCode) -> SdkWorkResultCode {
    match status {
        StatusCode::BAD_REQUEST => SdkWorkResultCode::InvalidParameter,
        StatusCode::UNAUTHORIZED => SdkWorkResultCode::AuthenticationRequired,
        StatusCode::FORBIDDEN => SdkWorkResultCode::PermissionRequired,
        StatusCode::NOT_FOUND => SdkWorkResultCode::NotFound,
        StatusCode::METHOD_NOT_ALLOWED => SdkWorkResultCode::MethodNotAllowed,
        StatusCode::REQUEST_TIMEOUT => SdkWorkResultCode::RequestTimeout,
        StatusCode::CONFLICT => SdkWorkResultCode::Conflict,
        StatusCode::TOO_MANY_REQUESTS => SdkWorkResultCode::RateLimitExceeded,
        StatusCode::SERVICE_UNAVAILABLE => SdkWorkResultCode::ServiceUnavailable,
        StatusCode::GATEWAY_TIMEOUT => SdkWorkResultCode::GatewayTimeout,
        _ if status.is_server_error() => SdkWorkResultCode::InternalError,
        _ => SdkWorkResultCode::InvalidParameter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_detail_serializes_rfc_9457_fields() {
        let problem = ProblemDetail::new(StatusCode::TOO_MANY_REQUESTS)
            .with_detail("Rate limit exceeded")
            .with_trace_id("00000000-0000-0000-0000-000000000001")
            .into_sdk();
        let json = serde_json::to_value(&problem).expect("serialize");
        assert_eq!(json["status"], 429);
        assert_eq!(json["code"], 42901);
        assert_eq!(json["detail"], "Rate limit exceeded");
        assert_eq!(json["traceId"], "00000000-0000-0000-0000-000000000001");
        assert!(json["type"].as_str().unwrap().contains("429"));
    }

    #[test]
    fn problem_detail_uses_platform_title_when_detail_missing() {
        let problem = ProblemDetail::new(StatusCode::NOT_FOUND)
            .with_trace_id("00000000-0000-0000-0000-000000000002")
            .into_sdk();
        let json = serde_json::to_value(&problem).expect("serialize");
        assert_eq!(json["status"], 404);
        assert_eq!(json["code"], 40401);
        assert_eq!(json["detail"], "Not found");
    }
}
