use sdkwork_agent_kernel::{
    EventRecorder, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    TraceContext,
};

#[test]
fn event_envelope_exposes_required_standard_fields() {
    let event = KernelEvent::new(
        "event.1",
        "agent.task.started",
        KernelEventSeverity::Info,
        "task_id=task.1",
    )
    .occurred_at("2026-05-27T12:00:00Z")
    .from_source(KernelEventSource::Runtime)
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .for_step("step.1")
    .with_correlation("correlation.1")
    .caused_by("event.0")
    .with_redaction(KernelEventRedaction::Internal)
    .with_payload_schema("sdkwork.agent.task.started.v1")
    .with_trace_context(TraceContext::new("trace.1", "span.1").with_parent_span("span.0"));

    assert_eq!(event.event_id, "event.1");
    assert_eq!(event.occurred_at.as_deref(), Some("2026-05-27T12:00:00Z"));
    assert_eq!(event.source, KernelEventSource::Runtime);
    assert_eq!(event.session_id.as_deref(), Some("session.1"));
    assert_eq!(event.task_id.as_deref(), Some("task.1"));
    assert_eq!(event.run_id.as_deref(), Some("run.1"));
    assert_eq!(event.step_id.as_deref(), Some("step.1"));
    assert_eq!(event.correlation_id.as_deref(), Some("correlation.1"));
    assert_eq!(event.causation_id.as_deref(), Some("event.0"));
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.task.started.v1")
    );
    assert_eq!(
        event
            .trace_context
            .as_ref()
            .unwrap()
            .parent_span_id
            .as_deref(),
        Some("span.0")
    );
    assert!(!event.replay);
}

#[test]
fn replayed_event_preserves_identity_and_marks_replay() {
    let original = KernelEvent::new(
        "event.1",
        "agent.tool.completed",
        KernelEventSeverity::Info,
        "tool_call_id=tool-call.1",
    )
    .occurred_at("2026-05-27T12:00:00Z")
    .for_task("task.1");

    let replayed = original.clone().mark_replay();

    assert_eq!(replayed.event_id, original.event_id);
    assert_eq!(replayed.occurred_at, original.occurred_at);
    assert_eq!(replayed.task_id, original.task_id);
    assert!(replayed.replay);
}

#[test]
fn event_recorder_filters_by_session_task_and_severity() {
    let mut recorder = EventRecorder::new();
    recorder.record(
        KernelEvent::new(
            "event.1",
            "agent.task.started",
            KernelEventSeverity::Info,
            "",
        )
        .for_session("session.1")
        .for_task("task.1"),
    );
    recorder.record(
        KernelEvent::new(
            "event.2",
            "agent.policy.denied",
            KernelEventSeverity::Warn,
            "",
        )
        .for_session("session.1")
        .for_task("task.2"),
    );
    recorder.record(
        KernelEvent::new(
            "event.3",
            "agent.runtime.failed",
            KernelEventSeverity::Error,
            "",
        )
        .for_session("session.2"),
    );

    assert_eq!(recorder.by_session("session.1").len(), 2);
    assert_eq!(recorder.by_task("task.2").len(), 1);
    assert_eq!(recorder.by_min_severity(KernelEventSeverity::Warn).len(), 2);
}

#[test]
fn secret_or_unknown_redaction_is_treated_as_sensitive() {
    assert!(!KernelEventRedaction::Public.is_sensitive());
    assert!(KernelEventRedaction::Internal.is_sensitive());
    assert!(KernelEventRedaction::TenantSensitive.is_sensitive());
    assert!(KernelEventRedaction::PersonalData.is_sensitive());
    assert!(KernelEventRedaction::Secret.is_sensitive());
    assert!(KernelEventRedaction::Regulated.is_sensitive());
    assert!(KernelEventRedaction::Unknown.is_sensitive());
}
