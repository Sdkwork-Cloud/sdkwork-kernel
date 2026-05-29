use sdkwork_agent_kernel::{
    EventStreamFilter, KernelEventRedaction, KernelEventSource, KernelResult,
};
use sdkwork_code_kernel::{
    CodeEventKind, CodeKernelCapability, CodeKernelEvent, CodeProviderBinding, CodeSession,
    CodeSessionState, CodeTask, CodeTaskIntent, Workspace,
};

#[test]
fn code_session_preserves_workspace_tasks_provider_bindings_and_state() {
    let workspace = Workspace::new("workspace.1", "workspace");
    let task = CodeTask::new(
        "code.task.1",
        workspace.clone(),
        CodeTaskIntent::new("Fix verification failure"),
    );

    let session = CodeSession::new("code.session.1", workspace.clone())
        .with_provider_binding(
            CodeProviderBinding::new("code_workspace", "provider.code.workspace")
                .with_capability(CodeKernelCapability::WorkspaceRead.as_str()),
        )
        .add_task(task)
        .transition(CodeSessionState::Active)
        .expect("session opens");

    assert_eq!(session.session_id, "code.session.1");
    assert_eq!(session.workspace.workspace_id, workspace.workspace_id);
    assert_eq!(session.state, CodeSessionState::Active);
    assert_eq!(session.tasks[0].task_id, "code.task.1");
    assert_eq!(
        session.provider_bindings[0].provider_family,
        "code_workspace"
    );
    assert_eq!(
        session.provider_bindings[0].capabilities,
        ["code.workspace.read"]
    );
}

#[test]
fn code_kernel_event_maps_to_kernel_event_with_stable_code_family() -> KernelResult<()> {
    let event = CodeKernelEvent::new(
        "event.code.patch.1",
        CodeEventKind::PatchApplied,
        "workspace.1",
    )
    .for_session("code.session.1")
    .for_task("code.task.1")
    .with_artifact_id("artifact.patch.1")
    .with_payload("patch_id=patch.1;status=applied")
    .with_redaction(KernelEventRedaction::Internal)
    .to_kernel_event();

    assert_eq!(event.event_type, "code.patch.applied");
    assert_eq!(event.source, KernelEventSource::CodeKernel);
    assert_eq!(event.session_id.as_deref(), Some("code.session.1"));
    assert_eq!(event.task_id.as_deref(), Some("code.task.1"));
    assert_eq!(
        event.payload,
        "workspace_id=workspace.1;artifact_id=artifact.patch.1;patch_id=patch.1;status=applied"
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.code.event.v1")
    );

    let filter = EventStreamFilter::new().with_event_family("code.patch.");
    assert!(filter.matches(&event));

    Ok(())
}
