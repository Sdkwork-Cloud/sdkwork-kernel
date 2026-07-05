//! SDKWork HTTP response helpers (`API_SPEC.md` §15–§16).

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sdkwork_agent_kernel::KernelError;
use sdkwork_utils_rust::{
    offset_window_page_info, OffsetListPageParams, SdkWorkApiResponse, SdkWorkPageData,
    SdkWorkProblemDetail, SdkWorkResourceData, SdkWorkResultCode, SDKWORK_TRACE_ID_HEADER,
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
        if lower.contains("not found") {
            return Self::not_found(error, trace_id);
        }
        if lower.contains("is closed") || lower.contains("already closed") {
            return Self::conflict(error, trace_id);
        }
        Self::internal(error, trace_id)
    }

    pub fn from_status(status: StatusCode, detail: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(
            crate::problem_details::result_code_for_status(status),
            detail,
            trace_id,
        )
    }

    pub fn from_kernel(error: KernelError, trace_id: impl Into<String>) -> Self {
        let trace_id = trace_id.into();
        match error {
            KernelError::Validation { message } => Self::invalid_parameter(message, trace_id),
            KernelError::CapabilityMissing { capability_id } => Self::service_unavailable(
                format!("missing capability: {capability_id}"),
                trace_id,
            ),
            KernelError::ProviderUnavailable { provider_id } => Self::service_unavailable(
                format!("provider unavailable: {provider_id}"),
                trace_id,
            ),
            KernelError::PolicyDenied { reason_code } => Self::forbidden(reason_code, trace_id),
            KernelError::Internal { message } => Self::internal(message, trace_id),
            KernelError::Structured { info } => {
                let detail = if info.message.trim().is_empty() {
                    info.kind.as_str().to_string()
                } else {
                    info.message.clone()
                };
                match info.kind {
                    sdkwork_agent_kernel::KernelErrorKind::ValidationError => {
                        Self::invalid_parameter(detail, trace_id)
                    }
                    sdkwork_agent_kernel::KernelErrorKind::PermissionRequired
                    | sdkwork_agent_kernel::KernelErrorKind::PolicyDenied
                    | sdkwork_agent_kernel::KernelErrorKind::SecurityViolation => {
                        Self::forbidden(detail, trace_id)
                    }
                    sdkwork_agent_kernel::KernelErrorKind::Conflict => Self::conflict(detail, trace_id),
                    sdkwork_agent_kernel::KernelErrorKind::CapabilityMissing
                    | sdkwork_agent_kernel::KernelErrorKind::ProviderUnavailable
                    | sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
                    | sdkwork_agent_kernel::KernelErrorKind::RateLimited => {
                        Self::service_unavailable(detail, trace_id)
                    }
                    _ => Self::internal(detail, trace_id),
                }
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let problem = SdkWorkProblemDetail::platform(self.code, self.detail, self.trace_id);
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            Json(problem),
        )
            .into_response()
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
    let count = items.len();
    let page_data = SdkWorkPageData {
        items,
        page_info: offset_window_page_info(Some(count), None, false),
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
        page_info: offset_window_page_info(
            Some(page_size),
            has_more.then(|| (params.offset + page_size as i64).to_string()),
            has_more,
        ),
    };
    api_success(page_data, trace_id)
}
