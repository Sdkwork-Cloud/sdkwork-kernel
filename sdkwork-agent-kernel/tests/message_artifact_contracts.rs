use sdkwork_agent_kernel::{
    AgentArtifact, AgentMessage, AgentMessageRole, AgentPart, ArtifactKind, KernelEventRedaction,
    KernelEventSeverity, PolicyCategory, TraceContext,
};

#[test]
fn agent_message_preserves_role_context_parts_trace_redaction_and_metadata() {
    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![AgentPart::text("part.1", "hello").with_redaction(KernelEventRedaction::Public)],
    )
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .for_step("step.1")
    .created_at("2026-05-27T12:00:00Z")
    .with_trace_context(TraceContext::new("trace.1", "span.1"))
    .with_metadata("a2a.message.id", "external-message.1")
    .mark_untrusted();

    assert_eq!(message.message_id, "message.1");
    assert_eq!(message.role, AgentMessageRole::User);
    assert_eq!(message.session_id.as_deref(), Some("session.1"));
    assert_eq!(message.task_id.as_deref(), Some("task.1"));
    assert_eq!(message.run_id.as_deref(), Some("run.1"));
    assert_eq!(message.step_id.as_deref(), Some("step.1"));
    assert_eq!(message.created_at.as_deref(), Some("2026-05-27T12:00:00Z"));
    assert_eq!(message.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(
        message.metadata_value("a2a.message.id"),
        Some("external-message.1")
    );
    assert!(message.untrusted);
    assert_eq!(message.highest_redaction(), KernelEventRedaction::Public);
}

#[test]
fn agent_message_rejects_empty_parts() {
    let error = AgentMessage::new("message.1", AgentMessageRole::Agent, vec![])
        .validate()
        .expect_err("empty messages are invalid");

    assert!(error.to_string().contains("at least one part"));
}

#[test]
fn agent_parts_support_standard_content_references_schema_and_provenance() {
    let json = AgentPart::json("part.json", "{\"answer\":42}")
        .with_schema("sdkwork.answer.v1")
        .from_provider("provider.fake")
        .with_redaction(KernelEventRedaction::Internal);
    let file = AgentPart::file_ref("part.file", "host://workspace/README.md", "text/markdown")
        .with_name("README.md");
    let artifact = AgentPart::artifact_ref("part.artifact", "artifact.1");
    let tool_call = AgentPart::tool_call_ref("part.tool", "tool-call.1");
    let error = AgentPart::error("part.error", "policy_denied", "permission denied");

    assert_eq!(json.schema.as_deref(), Some("sdkwork.answer.v1"));
    assert_eq!(json.provenance.as_deref(), Some("provider.fake"));
    assert_eq!(
        json.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        file.content_ref.as_deref(),
        Some("host://workspace/README.md")
    );
    assert_eq!(file.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(file.name.as_deref(), Some("README.md"));
    assert_eq!(artifact.artifact_id.as_deref(), Some("artifact.1"));
    assert_eq!(tool_call.tool_call_id.as_deref(), Some("tool-call.1"));
    assert_eq!(error.error_code.as_deref(), Some("policy_denied"));
}

#[test]
fn agent_message_maps_to_kernel_event_for_timeline_and_protocol_streams() {
    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::Agent,
        vec![AgentPart::text("part.1", "done").with_redaction(KernelEventRedaction::Internal)],
    )
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .with_trace_context(TraceContext::new("trace.1", "span.1"));

    let event = message.to_event("event.message.1");

    assert_eq!(event.event_type, "agent.message.created");
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert_eq!(event.session_id.as_deref(), Some("session.1"));
    assert_eq!(event.task_id.as_deref(), Some("task.1"));
    assert_eq!(event.run_id.as_deref(), Some("run.1"));
    assert_eq!(event.trace_context.as_ref().unwrap().span_id, "span.1");
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.message.event.v1")
    );
    assert!(event.payload.contains("message_id=message.1"));
    assert!(event.payload.contains("role=agent"));
    assert!(event.payload.contains("parts=1"));
}

#[test]
fn agent_artifact_preserves_reference_provenance_redaction_and_retention_metadata() {
    let artifact = AgentArtifact::new(
        "artifact.1",
        "task.1",
        ArtifactKind::File,
        "host://artifacts/report.md",
    )
    .produced_by_step("step.1")
    .with_mime_type("text/markdown")
    .created_at("2026-05-27T12:00:00Z")
    .with_name("report.md")
    .with_provenance("provider.tool.report")
    .with_redaction(KernelEventRedaction::TenantSensitive)
    .with_retention_policy("tenant.default.30d")
    .with_metadata("sdkwork.code.workspace", "workspace.1");

    assert_eq!(artifact.artifact_id, "artifact.1");
    assert_eq!(artifact.task_id, "task.1");
    assert_eq!(artifact.producer_step_id.as_deref(), Some("step.1"));
    assert_eq!(artifact.kind, ArtifactKind::File);
    assert_eq!(artifact.content_ref, "host://artifacts/report.md");
    assert_eq!(artifact.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(artifact.name.as_deref(), Some("report.md"));
    assert_eq!(artifact.provenance.as_deref(), Some("provider.tool.report"));
    assert_eq!(
        artifact.redaction_classification,
        KernelEventRedaction::TenantSensitive
    );
    assert_eq!(
        artifact.retention_policy.as_deref(),
        Some("tenant.default.30d")
    );
    assert_eq!(
        artifact.metadata_value("sdkwork.code.workspace"),
        Some("workspace.1")
    );
}

#[test]
fn agent_artifact_builds_policy_request_for_authorized_read_and_write() {
    let artifact = AgentArtifact::new(
        "artifact.1",
        "task.1",
        ArtifactKind::File,
        "host://artifacts/report.md",
    )
    .with_redaction(KernelEventRedaction::TenantSensitive);

    let read = artifact.read_policy_request("policy.artifact.read.1");
    let write = artifact.write_policy_request("policy.artifact.write.1");

    assert_eq!(read.typed_category, Some(PolicyCategory::ArtifactRead));
    assert_eq!(read.resource, "artifact.1");
    assert_eq!(read.task_id.as_deref(), Some("task.1"));
    assert_eq!(
        read.redaction_classification,
        KernelEventRedaction::TenantSensitive
    );

    assert_eq!(write.typed_category, Some(PolicyCategory::ArtifactWrite));
    assert_eq!(write.resource, "artifact.1");
    assert_eq!(write.task_id.as_deref(), Some("task.1"));
}

#[test]
fn agent_artifact_maps_to_kernel_event_for_ui_and_audit_observers() {
    let artifact = AgentArtifact::new(
        "artifact.1",
        "task.1",
        ArtifactKind::Patch,
        "host://artifacts/change.patch",
    )
    .produced_by_step("step.1")
    .with_redaction(KernelEventRedaction::Internal);

    let event = artifact.to_event("event.artifact.1");

    assert_eq!(event.event_type, "agent.artifact.created");
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert_eq!(event.task_id.as_deref(), Some("task.1"));
    assert_eq!(event.step_id.as_deref(), Some("step.1"));
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.artifact.event.v1")
    );
    assert!(event.payload.contains("artifact_id=artifact.1"));
    assert!(event.payload.contains("kind=patch"));
}
