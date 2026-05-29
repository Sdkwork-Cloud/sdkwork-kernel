use sdkwork_agent_kernel::{
    AgentArtifact, AgentMessage, AgentMessageRole, AgentPart, ArtifactKind, KernelError,
    KernelErrorSource, KernelEvent, KernelEventRedaction, KernelEventSeverity, ProtocolFamily,
    ProtocolObjectEnvelope, ProtocolObjectKind, ProtocolObjectMapper, StandardProtocolObjectMapper,
    TraceContext,
};

#[test]
fn protocol_object_envelope_preserves_protocol_kind_schema_trace_redaction_and_metadata() {
    let envelope = ProtocolObjectEnvelope::new(
        ProtocolFamily::A2a,
        ProtocolObjectKind::AgentMessage,
        "message.1",
        "message_id=message.1;role=user",
    )
    .with_external_id("a2a-message.1")
    .with_schema("sdkwork.agent.message.v1")
    .with_metadata("a2a.message.id", "external-message.1")
    .with_trace_context(TraceContext::new("trace.1", "span.protocol"))
    .with_redaction(KernelEventRedaction::Internal)
    .with_loss_note("a2a.parts.inline_binary_omitted");

    envelope.validate().expect("namespaced metadata is valid");

    assert_eq!(envelope.protocol, ProtocolFamily::A2a);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::AgentMessage);
    assert_eq!(envelope.object_id, "message.1");
    assert_eq!(envelope.external_id.as_deref(), Some("a2a-message.1"));
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.message.v1")
    );
    assert_eq!(
        envelope.metadata_value("a2a.message.id"),
        Some("external-message.1")
    );
    assert_eq!(envelope.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(
        envelope.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        envelope.loss_notes,
        ["a2a.parts.inline_binary_omitted".to_string()]
    );
}

#[test]
fn protocol_object_envelope_rejects_unnamespaced_metadata_keys() {
    let error = ProtocolObjectEnvelope::new(
        ProtocolFamily::Http,
        ProtocolObjectKind::KernelEvent,
        "event.1",
        "event",
    )
    .with_metadata("request_id", "unsafe-unscoped")
    .validate()
    .expect_err("metadata keys must be namespaced");

    assert!(error
        .to_string()
        .contains("metadata key must be namespaced"));
}

#[test]
fn standard_mapper_maps_agent_message_without_leaking_sensitive_part_payload() {
    let mapper = StandardProtocolObjectMapper::new(ProtocolFamily::A2a);
    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.1", "secret-token").with_redaction(KernelEventRedaction::Secret),
            AgentPart::artifact_ref("part.2", "artifact.1"),
        ],
    )
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .with_trace_context(TraceContext::new("trace.1", "span.message"))
    .mark_untrusted();

    let envelope = mapper.map_message(&message).expect("message maps");

    assert_eq!(envelope.protocol, ProtocolFamily::A2a);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::AgentMessage);
    assert_eq!(envelope.object_id, "message.1");
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.message.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.session_id"),
        Some("session.1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.task_id"),
        Some("task.1")
    );
    assert_eq!(
        envelope.trace_context.as_ref().unwrap().span_id,
        "span.message"
    );
    assert_eq!(
        envelope.redaction_classification,
        KernelEventRedaction::Secret
    );
    assert!(envelope.payload.contains("parts=2"));
    assert!(envelope.payload.contains("untrusted=true"));
    assert!(!envelope.payload.contains("secret-token"));
}

#[test]
fn standard_mapper_maps_agent_artifact_as_authorized_reference_metadata() {
    let mapper = StandardProtocolObjectMapper::new(ProtocolFamily::KernelUiClient);
    let artifact = AgentArtifact::new(
        "artifact.1",
        "task.1",
        ArtifactKind::Patch,
        "host://artifacts/change.patch",
    )
    .produced_by_step("step.1")
    .with_mime_type("text/x-patch")
    .with_name("change.patch")
    .with_redaction(KernelEventRedaction::Internal);

    let envelope = mapper.map_artifact(&artifact).expect("artifact maps");

    assert_eq!(envelope.protocol, ProtocolFamily::KernelUiClient);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::AgentArtifact);
    assert_eq!(envelope.object_id, "artifact.1");
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.artifact.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.task_id"),
        Some("task.1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.step_id"),
        Some("step.1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.artifact.mime_type"),
        Some("text/x-patch")
    );
    assert!(envelope.payload.contains("kind=patch"));
    assert!(envelope
        .payload
        .contains("content_ref=host://artifacts/change.patch"));
}

#[test]
fn standard_mapper_maps_kernel_event_for_protocol_streams() {
    let mapper = StandardProtocolObjectMapper::new(ProtocolFamily::WebSocket);
    let event = KernelEvent::new(
        "event.1",
        "agent.task.started",
        KernelEventSeverity::Info,
        "task_id=task.1",
    )
    .for_session("session.1")
    .for_task("task.1")
    .with_trace_context(TraceContext::new("trace.1", "span.event"))
    .with_redaction(KernelEventRedaction::Internal)
    .with_payload_schema("sdkwork.agent.task.started.v1");

    let envelope = mapper.map_event(&event).expect("event maps");

    assert_eq!(envelope.protocol, ProtocolFamily::WebSocket);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::KernelEvent);
    assert_eq!(envelope.object_id, "event.1");
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.task.started.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.event.type"),
        Some("agent.task.started")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.session_id"),
        Some("session.1")
    );
    assert_eq!(
        envelope.trace_context.as_ref().unwrap().span_id,
        "span.event"
    );
}

#[test]
fn standard_mapper_maps_kernel_error_using_safe_protocol_payload() {
    let mapper = StandardProtocolObjectMapper::new(ProtocolFamily::Http);
    let error =
        KernelError::provider_error("provider.raw.failure", "raw stack trace with tenant secret")
            .from_source(KernelErrorSource::Model)
            .with_trace_context(TraceContext::new("trace.1", "span.error"))
            .with_detail("raw_debug", "tenant-secret")
            .with_redaction(KernelEventRedaction::Internal)
            .with_safe_message("provider failed");

    let envelope = mapper.map_error(&error).expect("error maps");

    assert_eq!(envelope.protocol, ProtocolFamily::Http);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::KernelError);
    assert_eq!(envelope.object_id, "error.provider.raw.failure");
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.error.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.error.kind"),
        Some("provider_error")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.error.source"),
        Some("model")
    );
    assert!(envelope.payload.contains("safe_message=provider failed"));
    assert!(!envelope.payload.contains("tenant-secret"));
    assert!(!envelope.payload.contains("raw stack trace"));
}
