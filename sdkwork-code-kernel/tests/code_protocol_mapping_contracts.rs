use sdkwork_agent_kernel::{KernelEventRedaction, ProtocolFamily, ProtocolObjectKind};
use sdkwork_code_kernel::{
    CodeArtifact, CodeArtifactKind, CodeKernelEvent, CodePlan, CodePlanStep,
    CodeProtocolObjectMapper, CodeProviderBinding, CodeReviewStatus, CodeSession, CodeTask,
    CodeTaskIntent, PatchOperation, PatchSet, StandardCodeProtocolObjectMapper, Workspace,
};

#[test]
fn standard_code_mapper_maps_code_objects_to_protocol_envelopes_without_payload_leaks() {
    let mapper = StandardCodeProtocolObjectMapper::new(ProtocolFamily::KernelUiClient);
    let workspace = Workspace::new("workspace.1", "workspace");
    let task = CodeTask::new(
        "code.task.1",
        workspace.clone(),
        CodeTaskIntent::new("Fix secret token handling").with_context_path("src/lib.rs"),
    )
    .with_plan(
        CodePlan::new("plan.1")
            .add_step(CodePlanStep::new("step.1", "code.patch.apply").requires_policy()),
    )
    .with_review_status(CodeReviewStatus::Required);

    let task_envelope = mapper.map_task(&task).expect("task maps");
    assert_eq!(task_envelope.protocol, ProtocolFamily::KernelUiClient);
    assert_eq!(
        task_envelope.object_kind,
        ProtocolObjectKind::ExtensionObject
    );
    assert_eq!(task_envelope.object_id, "code.task.1");
    assert_eq!(
        task_envelope.payload_schema.as_deref(),
        Some("sdkwork.code.task.v1")
    );
    assert_eq!(
        task_envelope.metadata_value("sdkwork.code.object_kind"),
        Some("code_task")
    );
    assert_eq!(
        task_envelope.metadata_value("sdkwork.code.workspace_id"),
        Some("workspace.1")
    );
    assert_eq!(
        task_envelope.metadata_value("sdkwork.code.review_status"),
        Some("required")
    );
    assert!(task_envelope.payload.contains("plan_steps=1"));
    assert!(!task_envelope.payload.contains("secret token"));

    let session = CodeSession::new("code.session.1", workspace.clone()).with_provider_binding(
        CodeProviderBinding::new("code_workspace", "provider.code.workspace.typed")
            .with_capability("code.workspace.read"),
    );
    let session_envelope = mapper.map_session(&session).expect("session maps");
    assert_eq!(
        session_envelope.object_kind,
        ProtocolObjectKind::ExtensionObject
    );
    assert_eq!(
        session_envelope.payload_schema.as_deref(),
        Some("sdkwork.code.session.v1")
    );
    assert_eq!(
        session_envelope.metadata_value("sdkwork.code.object_kind"),
        Some("code_session")
    );
    assert_eq!(
        session_envelope.metadata_value("sdkwork.code.workspace_id"),
        Some("workspace.1")
    );
    assert_eq!(
        session_envelope.metadata_value("sdkwork.code.provider_bindings"),
        Some("1")
    );

    let patch = PatchSet::new("patch.1", "workspace.1", "Delete generated client")
        .add_operation(PatchOperation::delete_file("generated/client.ts"));
    let patch_envelope = mapper.map_patch(&patch).expect("patch maps");
    assert_eq!(
        patch_envelope.object_kind,
        ProtocolObjectKind::ExtensionObject
    );
    assert_eq!(
        patch_envelope.payload_schema.as_deref(),
        Some("sdkwork.code.patch.v1")
    );
    assert_eq!(
        patch_envelope.metadata_value("sdkwork.code.object_kind"),
        Some("patch_set")
    );
    assert_eq!(
        patch_envelope.metadata_value("sdkwork.code.side_effect_level"),
        Some("destructive")
    );
    assert_eq!(
        patch_envelope.metadata_value("sdkwork.code.affected_files"),
        Some("generated/client.ts")
    );

    let artifact = CodeArtifact::new(
        "artifact.review.1",
        "workspace.1",
        CodeArtifactKind::ReviewReport,
        "Review report",
        "sensitive review content",
    )
    .with_redaction(KernelEventRedaction::Internal);
    let artifact_envelope = mapper.map_artifact(&artifact).expect("artifact maps");
    assert_eq!(
        artifact_envelope.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        artifact_envelope.metadata_value("sdkwork.code.artifact_kind"),
        Some("review_report")
    );
    assert!(!artifact_envelope
        .payload
        .contains("sensitive review content"));

    let event = CodeKernelEvent::new(
        "event.patch.1",
        sdkwork_code_kernel::CodeEventKind::PatchApplied,
        "workspace.1",
    )
    .for_task("code.task.1")
    .with_redaction(KernelEventRedaction::Internal);
    let event_envelope = mapper.map_event(&event).expect("event maps");
    assert_eq!(event_envelope.object_kind, ProtocolObjectKind::KernelEvent);
    assert_eq!(
        event_envelope.payload_schema.as_deref(),
        Some("sdkwork.code.event.v1")
    );
    assert_eq!(
        event_envelope.metadata_value("sdkwork.event.type"),
        Some("code.patch.applied")
    );
    assert_eq!(
        event_envelope.metadata_value("sdkwork.code.workspace_id"),
        Some("workspace.1")
    );
}
