use sdkwork_agent_kernel::{
    EventRecorder, EventStream, EventStreamCursor, EventStreamFilter, EventStreamStatus,
    KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    TraceContext,
};

#[test]
fn event_stream_filter_matches_session_task_run_source_severity_and_family() {
    let filter = EventStreamFilter::new()
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .from_source(KernelEventSource::Tool)
        .with_min_severity(KernelEventSeverity::Warn)
        .with_event_family("agent.tool");

    let matching = KernelEvent::new(
        "event.1",
        "agent.tool.call.failed",
        KernelEventSeverity::Error,
        "tool failed",
    )
    .from_source(KernelEventSource::Tool)
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1");
    let wrong_family = KernelEvent::new(
        "event.2",
        "agent.model.failed",
        KernelEventSeverity::Error,
        "model failed",
    )
    .from_source(KernelEventSource::Model)
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1");
    let too_low_severity = KernelEvent::new(
        "event.3",
        "agent.tool.call.streamed",
        KernelEventSeverity::Info,
        "line",
    )
    .from_source(KernelEventSource::Tool)
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1");

    assert!(filter.matches(&matching));
    assert!(!filter.matches(&wrong_family));
    assert!(!filter.matches(&too_low_severity));
}

#[test]
fn event_stream_assigns_ordered_sequences_and_supports_cursor_resume() {
    let mut stream = EventStream::new("stream.1");
    stream.publish(event("event.1", "agent.task.created"));
    stream.publish(event("event.2", "agent.task.started"));
    stream.publish(event("event.3", "agent.task.completed"));

    let first = stream
        .subscribe(
            "subscription.1",
            EventStreamFilter::new(),
            EventStreamCursor::from_start(),
            2,
        )
        .expect("first batch");
    let next = stream
        .subscribe(
            "subscription.1",
            EventStreamFilter::new(),
            first.next_cursor.clone(),
            10,
        )
        .expect("next batch");

    assert_eq!(first.stream_id, "stream.1");
    assert_eq!(first.subscription_id, "subscription.1");
    assert_eq!(first.events.len(), 2);
    assert_eq!(first.events[0].sequence, 1);
    assert_eq!(first.events[0].event.event_id, "event.1");
    assert_eq!(first.events[1].sequence, 2);
    assert!(first.has_more);
    assert_eq!(first.next_cursor.last_sequence, Some(2));

    assert_eq!(next.events.len(), 1);
    assert_eq!(next.events[0].sequence, 3);
    assert_eq!(next.events[0].event.event_id, "event.3");
    assert!(!next.has_more);
    assert_eq!(next.status, EventStreamStatus::Open);
}

#[test]
fn event_stream_can_replay_events_from_recorder_with_replay_markers() {
    let mut recorder = EventRecorder::new();
    recorder.record(
        event("event.1", "agent.task.created")
            .for_session("session.1")
            .with_trace_context(TraceContext::new("trace.1", "span.1")),
    );
    recorder.record(
        event("event.2", "agent.task.completed")
            .for_session("session.1")
            .with_redaction(KernelEventRedaction::Internal),
    );

    let stream = EventStream::from_recorder("stream.replay", &recorder).mark_replay();
    let batch = stream
        .subscribe(
            "subscription.replay",
            EventStreamFilter::new().for_session("session.1"),
            EventStreamCursor::from_start(),
            10,
        )
        .expect("replay batch");

    assert_eq!(batch.events.len(), 2);
    assert!(batch.events.iter().all(|item| item.event.replay));
    assert_eq!(
        batch.events[0]
            .event
            .trace_context
            .as_ref()
            .unwrap()
            .trace_id,
        "trace.1"
    );
    assert_eq!(
        batch.events[1].event.redaction_classification,
        KernelEventRedaction::Internal
    );
}

#[test]
fn event_stream_filters_by_event_family_and_source_for_ui_protocol_subscribers() {
    let mut stream = EventStream::new("stream.1");
    stream
        .publish(event("event.1", "agent.tool.call.started").from_source(KernelEventSource::Tool));
    stream.publish(
        event("event.2", "agent.tool.call.completed").from_source(KernelEventSource::Tool),
    );
    stream.publish(event("event.3", "agent.model.completed").from_source(KernelEventSource::Model));

    let batch = stream
        .subscribe(
            "subscription.tool",
            EventStreamFilter::new()
                .from_source(KernelEventSource::Tool)
                .with_event_family("agent.tool"),
            EventStreamCursor::from_start(),
            10,
        )
        .expect("filtered batch");

    assert_eq!(batch.events.len(), 2);
    assert_eq!(batch.events[0].event.event_id, "event.1");
    assert_eq!(batch.events[1].event.event_id, "event.2");
    assert!(batch
        .events
        .iter()
        .all(|item| item.event.source == KernelEventSource::Tool));
}

#[test]
fn event_stream_completion_and_error_are_observable_to_subscribers() {
    let mut completed = EventStream::new("stream.completed");
    completed.publish(event("event.1", "agent.task.completed"));
    completed.complete();

    let completed_batch = completed
        .subscribe(
            "subscription.completed",
            EventStreamFilter::new(),
            EventStreamCursor::from_start(),
            10,
        )
        .expect("completed batch");

    assert_eq!(completed_batch.status, EventStreamStatus::Completed);
    assert_eq!(completed_batch.events.len(), 1);
    assert!(completed_batch.completion_event_id.is_some());

    let mut failed = EventStream::new("stream.failed");
    failed.fail(KernelError::timeout("stream backend timed out"));

    let error = failed
        .subscribe(
            "subscription.failed",
            EventStreamFilter::new(),
            EventStreamCursor::from_start(),
            10,
        )
        .expect_err("failed stream reports error");

    assert_eq!(error.kind(), sdkwork_agent_kernel::KernelErrorKind::Timeout);
    assert_eq!(failed.status(), EventStreamStatus::Failed);
}

fn event(event_id: &str, event_type: &str) -> KernelEvent {
    KernelEvent::new(event_id, event_type, KernelEventSeverity::Info, "payload")
}
