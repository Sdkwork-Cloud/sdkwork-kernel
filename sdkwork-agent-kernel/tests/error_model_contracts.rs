use sdkwork_agent_kernel::{
    KernelError, KernelErrorKind, KernelErrorSource, KernelEventRedaction, KernelEventSeverity,
    ProtocolError, TraceContext,
};

#[test]
fn legacy_error_variants_expose_stable_typed_error_metadata() {
    let validation = KernelError::validation("bad payload");
    let missing = KernelError::CapabilityMissing {
        capability_id: "model.streaming".to_string(),
    };
    let unavailable = KernelError::ProviderUnavailable {
        provider_id: "provider.model.fake".to_string(),
    };
    let denied = KernelError::PolicyDenied {
        reason_code: "command_denied".to_string(),
    };
    let internal = KernelError::Internal {
        message: "database connection string leaked internally".to_string(),
    };

    assert_eq!(validation.kind(), KernelErrorKind::ValidationError);
    assert_eq!(validation.code(), "validation_error");
    assert_eq!(validation.safe_message(), "bad payload");
    assert!(validation.safe_for_user());
    assert!(!validation.retryable());

    assert_eq!(missing.kind(), KernelErrorKind::CapabilityMissing);
    assert_eq!(missing.code(), "capability_missing");
    assert_eq!(missing.safe_message(), "required capability is unavailable");
    assert!(!missing.retryable());

    assert_eq!(unavailable.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(unavailable.code(), "provider_unavailable");
    assert_eq!(unavailable.provider_id(), Some("provider.model.fake"));
    assert!(unavailable.retryable());

    assert_eq!(denied.kind(), KernelErrorKind::PolicyDenied);
    assert_eq!(denied.code(), "policy_denied");
    assert_eq!(denied.safe_message(), "request denied by policy");
    assert!(!denied.retryable());

    assert_eq!(internal.kind(), KernelErrorKind::InternalError);
    assert_eq!(internal.code(), "internal_error");
    assert_eq!(internal.safe_message(), "internal kernel error");
    assert!(!internal.safe_for_user());
}

#[test]
fn structured_error_preserves_provider_source_trace_details_and_redaction() {
    let error = KernelError::provider_error(
        "provider.timeout",
        "model provider timed out after retry budget",
    )
    .with_provider("provider.model.fake")
    .from_source(KernelErrorSource::Model)
    .with_trace_context(TraceContext::new("trace.1", "span.error"))
    .with_detail("model_request_id", "model-request.1")
    .with_redaction(KernelEventRedaction::Internal)
    .with_retryable(true)
    .with_safe_for_user(false)
    .with_safe_message("model provider is temporarily unavailable");

    assert_eq!(error.kind(), KernelErrorKind::ProviderError);
    assert_eq!(error.code(), "provider.timeout");
    assert_eq!(
        error.message(),
        "model provider timed out after retry budget"
    );
    assert_eq!(
        error.safe_message(),
        "model provider is temporarily unavailable"
    );
    assert_eq!(error.provider_id(), Some("provider.model.fake"));
    assert_eq!(error.source(), KernelErrorSource::Model);
    assert_eq!(error.trace_context().unwrap().trace_id, "trace.1");
    assert_eq!(
        error.detail_value("model_request_id"),
        Some("model-request.1")
    );
    assert_eq!(
        error.redaction_classification(),
        KernelEventRedaction::Internal
    );
    assert!(error.retryable());
    assert!(!error.safe_for_user());
}

#[test]
fn standard_error_constructors_cover_runtime_and_security_kinds() {
    let permission = KernelError::permission_required("approval required")
        .with_detail("permission_scope", "host.process.execute");
    let timeout = KernelError::timeout("tool call timed out").with_retryable(true);
    let cancelled = KernelError::cancelled("run cancelled by user");
    let conflict = KernelError::conflict("run already active");
    let rate_limited = KernelError::rate_limited("model quota exceeded").with_retryable(true);
    let exhausted = KernelError::resource_exhausted("context window exceeded");
    let unsafe_content = KernelError::unsafe_content("prompt injection indicator found");
    let security = KernelError::security_violation("path traversal denied");

    assert_eq!(permission.kind(), KernelErrorKind::PermissionRequired);
    assert_eq!(permission.code(), "permission_required");
    assert_eq!(
        permission.detail_value("permission_scope"),
        Some("host.process.execute")
    );
    assert_eq!(timeout.kind(), KernelErrorKind::Timeout);
    assert!(timeout.retryable());
    assert_eq!(cancelled.kind(), KernelErrorKind::Cancelled);
    assert_eq!(conflict.kind(), KernelErrorKind::Conflict);
    assert_eq!(rate_limited.kind(), KernelErrorKind::RateLimited);
    assert!(rate_limited.retryable());
    assert_eq!(exhausted.kind(), KernelErrorKind::ResourceExhausted);
    assert_eq!(unsafe_content.kind(), KernelErrorKind::UnsafeContent);
    assert_eq!(security.kind(), KernelErrorKind::SecurityViolation);
}

#[test]
fn kernel_error_maps_to_protocol_safe_error_without_leaking_internal_details() {
    let internal = KernelError::Internal {
        message: "database password was rejected".to_string(),
    };
    let provider = KernelError::provider_error(
        "provider.anthropic.raw",
        "raw provider stack trace with tenant detail",
    )
    .with_provider("provider.model.fake")
    .with_safe_for_user(false)
    .with_safe_message("provider failed");

    let protocol_internal = ProtocolError::from_kernel_error(internal);
    let protocol_provider = ProtocolError::from_kernel_error(provider);

    assert_eq!(protocol_internal.code, "internal_error");
    assert_eq!(protocol_internal.safe_message, "internal kernel error");
    assert_eq!(protocol_provider.code, "provider.anthropic.raw");
    assert_eq!(protocol_provider.safe_message, "provider failed");
}

#[test]
fn kernel_error_maps_to_event_for_telemetry_and_ui_diagnostics() {
    let error = KernelError::security_violation("path traversal denied")
        .with_trace_context(TraceContext::new("trace.1", "span.error"))
        .with_detail("path_policy", "workspace_roots")
        .with_redaction(KernelEventRedaction::Internal);

    let event = error.to_event("event.error.1");

    assert_eq!(event.event_type, "agent.error.occurred");
    assert_eq!(event.severity, KernelEventSeverity::Error);
    assert_eq!(event.trace_context.as_ref().unwrap().span_id, "span.error");
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.error.v1")
    );
    assert!(event.payload.contains("kind=security_violation"));
    assert!(event.payload.contains("code=security_violation"));
    assert!(event.payload.contains("safe_for_user=true"));
    assert!(!event.payload.contains("path_policy=workspace_roots"));
}
