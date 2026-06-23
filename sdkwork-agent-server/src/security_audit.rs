use tracing::warn;

/// Structured security audit events for authn/authz failures (SECURITY_SPEC §5).
pub fn log_auth_failure(
    event: &'static str,
    request_id: Option<&str>,
    path: &str,
    tenant_id: Option<&str>,
    user_id: Option<&str>,
    detail: &str,
) {
    warn!(
        target: "security_audit",
        security_event = event,
        request_id = request_id.unwrap_or("unknown"),
        path = path,
        tenant_id = tenant_id.unwrap_or(""),
        user_id = user_id.unwrap_or(""),
        detail = detail,
        "security audit event"
    );
}

pub fn log_access_denied(
    event: &'static str,
    request_id: Option<&str>,
    resource: &str,
    tenant_id: Option<&str>,
    user_id: Option<&str>,
    detail: &str,
) {
    warn!(
        target: "security_audit",
        security_event = event,
        request_id = request_id.unwrap_or("unknown"),
        resource = resource,
        tenant_id = tenant_id.unwrap_or(""),
        user_id = user_id.unwrap_or(""),
        detail = detail,
        "security audit event"
    );
}
