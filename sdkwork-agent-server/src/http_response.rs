//! SDKWork HTTP response helpers (`API_SPEC.md` §15–§16).

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sdkwork_agent_kernel::KernelError;
use sdkwork_utils_rust::{
    cursor_list_page_data, offset_paged_list_page_info, OffsetListPageParams, SdkWorkApiResponse,
    SdkWorkPageData, SdkWorkProblemDetail, SdkWorkResourceData, SdkWorkResultCode,
    DEFAULT_LIST_PAGE_SIZE, SDKWORK_TRACE_ID_HEADER,
};

/// Handler error mapped to `application/problem+json` with numeric platform `code`.
#[derive(Debug)]
pub struct ApiError {
    pub code: SdkWorkResultCode,
    pub detail: String,
    pub trace_id: String,
}

impl ApiError {
    pub fn new(
        code: SdkWorkResultCode,
        detail: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            code,
            detail: detail.into(),
            trace_id: trace_id.into(),
        }
    }

    pub fn not_found(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::NotFound, detail, trace_id)
    }

    pub fn forbidden(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::PermissionRequired, detail, trace_id)
    }

    pub fn conflict(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::Conflict, detail, trace_id)
    }

    pub fn internal(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::InternalError, detail, trace_id)
    }

    pub fn invalid_parameter(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::InvalidParameter, detail, trace_id)
    }

    pub fn service_unavailable(detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(SdkWorkResultCode::ServiceUnavailable, detail, trace_id)
    }

    pub fn from_persistence(error: String, trace_id: impl Into<String>) -> Self {
        let trace_id = trace_id.into();
        let lower = error.to_lowercase();
        tracing::error!(trace_id = %trace_id, error = %error, "persistence operation failed");
        if lower.contains("not found") {
            return Self::not_found("resource not found", trace_id);
        }
        if lower.contains("persistence admission") {
            return Self::service_unavailable("persistence capacity unavailable", trace_id);
        }
        if lower.contains("constraint violation")
            || lower.contains("unique constraint")
            || lower.contains("is not active")
            || lower.contains("is closed")
            || lower.contains("already closed")
        {
            return Self::conflict("resource state conflict", trace_id);
        }
        Self::internal("persistence operation failed", trace_id)
    }

    pub fn from_status(
        status: StatusCode,
        detail: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            crate::problem_details::result_code_for_status(status),
            detail,
            trace_id,
        )
    }

    pub fn from_kernel(error: KernelError, trace_id: impl Into<String>) -> Self {
        let trace_id = trace_id.into();
        tracing::error!(trace_id = %trace_id, error = %error, "kernel operation failed");
        match error {
            KernelError::Validation { message } => Self::invalid_parameter(message, trace_id),
            KernelError::CapabilityMissing { .. } => {
                Self::service_unavailable("required capability unavailable", trace_id)
            }
            KernelError::ProviderUnavailable { .. } => {
                Self::service_unavailable("provider unavailable", trace_id)
            }
            KernelError::PolicyDenied { reason_code } => Self::forbidden(reason_code, trace_id),
            KernelError::Internal { .. } => Self::internal("kernel operation failed", trace_id),
            KernelError::Structured { info } => match info.kind {
                sdkwork_agent_kernel::KernelErrorKind::ValidationError => {
                    Self::invalid_parameter("request validation failed", trace_id)
                }
                sdkwork_agent_kernel::KernelErrorKind::PermissionRequired
                | sdkwork_agent_kernel::KernelErrorKind::PolicyDenied
                | sdkwork_agent_kernel::KernelErrorKind::SecurityViolation => {
                    Self::forbidden("operation is not permitted", trace_id)
                }
                sdkwork_agent_kernel::KernelErrorKind::Conflict => {
                    Self::conflict("resource state conflict", trace_id)
                }
                sdkwork_agent_kernel::KernelErrorKind::CapabilityMissing
                | sdkwork_agent_kernel::KernelErrorKind::ProviderUnavailable
                | sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
                | sdkwork_agent_kernel::KernelErrorKind::RateLimited => {
                    Self::service_unavailable("runtime dependency unavailable", trace_id)
                }
                _ => Self::internal("kernel operation failed", trace_id),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let trace_id = self.trace_id.clone();
        let problem = SdkWorkProblemDetail::platform(self.code, self.detail, self.trace_id);
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        with_trace_header(
            (
                status,
                [(header::CONTENT_TYPE, "application/problem+json")],
                Json(problem),
            )
                .into_response(),
            &trace_id,
        )
    }
}

fn with_trace_header(mut response: Response, trace_id: &str) -> Response {
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        response
            .headers_mut()
            .insert(SDKWORK_TRACE_ID_HEADER, value);
    }
    response
}

/// Serialize a success envelope with optional `X-SdkWork-Trace-Id`.
pub fn api_success<T: serde::Serialize>(data: T, trace_id: &str) -> Response {
    let body = SdkWorkApiResponse::success(data, trace_id);
    with_trace_header((StatusCode::OK, Json(body)).into_response(), trace_id)
}

/// Serialize a created resource envelope (`201`).
pub fn api_created<T: serde::Serialize>(item: T, trace_id: &str) -> Response {
    let body = SdkWorkApiResponse::success(SdkWorkResourceData { item }, trace_id);
    with_trace_header((StatusCode::CREATED, Json(body)).into_response(), trace_id)
}

/// Serialize an accepted asynchronous operation envelope (`202`).
pub fn api_accepted<T: serde::Serialize>(data: T, trace_id: &str) -> Response {
    let body = SdkWorkApiResponse::success(data, trace_id);
    with_trace_header((StatusCode::ACCEPTED, Json(body)).into_response(), trace_id)
}

/// Serialize a single-resource success envelope (`200`, `data.item`).
pub fn api_item<T: serde::Serialize>(item: T, trace_id: &str) -> Response {
    api_success(SdkWorkResourceData { item }, trace_id)
}

/// Success response with no body (`204`) and trace header.
pub fn api_no_content(trace_id: &str) -> Response {
    with_trace_header(StatusCode::NO_CONTENT.into_response(), trace_id)
}

/// Bounded catalog list (models/tools) using the standard page envelope.
pub fn catalog_list_response<T: serde::Serialize>(items: Vec<T>, trace_id: &str) -> Response {
    let page_size = if items.is_empty() {
        i64::from(DEFAULT_LIST_PAGE_SIZE)
    } else {
        items.len() as i64
    };
    let page_data = SdkWorkPageData {
        items,
        page_info: offset_paged_list_page_info(
            OffsetListPageParams::parse(Some(1), Some(page_size)),
            false,
        ),
    };
    api_success(page_data, trace_id)
}

/// Build offset-mode list payload using `limit + 1` fetch semantics.
pub fn offset_list_response<T: serde::Serialize>(
    mut items: Vec<T>,
    page_size: i64,
    params: OffsetListPageParams,
    trace_id: &str,
) -> Response {
    let page_size = page_size.max(1) as usize;
    let has_more = items.len() > page_size;
    if has_more {
        items.truncate(page_size);
    }
    let page_data = SdkWorkPageData {
        items,
        page_info: offset_paged_list_page_info(params, has_more),
    };
    api_success(page_data, trace_id)
}

/// Build cursor-mode list payload using `limit + 1` fetch semantics.
pub fn cursor_list_response<T: serde::Serialize>(
    mut items: Vec<T>,
    page_size: i64,
    id_for_cursor: impl Fn(&T) -> String,
    trace_id: &str,
) -> Response {
    let page_size = page_size.max(1) as usize;
    let has_more = items.len() > page_size;
    if has_more {
        items.truncate(page_size);
    }
    let next_cursor = if has_more {
        items.last().map(id_for_cursor)
    } else {
        None
    };
    let page_data = cursor_list_page_data(items, page_size, next_cursor, has_more);
    api_success(page_data, trace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_state_and_identity_conflicts_map_to_conflict() {
        for message in [
            "failed to append completed message turn: constraint violation: session session.1 is not active",
            "query error: UNIQUE constraint failed: messages.message_id",
            "session is closed",
        ] {
            let error = ApiError::from_persistence(message.to_string(), "trace.conflict");
            assert_eq!(error.code, SdkWorkResultCode::Conflict, "{message}");
            assert_eq!(error.detail, "resource state conflict");
        }
    }

    #[test]
    fn persistence_internal_errors_remain_sanitized() {
        let error = ApiError::from_persistence(
            "transaction error: password=secret host=internal".to_string(),
            "trace.internal",
        );
        assert_eq!(error.code, SdkWorkResultCode::InternalError);
        assert_eq!(error.detail, "persistence operation failed");
    }

    #[test]
    fn persistence_admission_errors_map_to_service_unavailable() {
        for message in [
            "persistence admission queue full",
            "persistence admission timeout",
            "persistence admission closed",
        ] {
            let error = ApiError::from_persistence(message.to_string(), "trace.capacity");
            assert_eq!(error.code, SdkWorkResultCode::ServiceUnavailable);
            assert_eq!(error.detail, "persistence capacity unavailable");
        }
    }
}
