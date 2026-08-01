//! Contract tests: sandbox lifecycle events in the stream protocol.
//!
//! `AgentStreamEvent::Sandbox` correlates the execution stream with the
//! bound sandbox session lifecycle (pending -> active -> completed /
//! failed), carries the session identity, and participates in the common
//! event envelope (event_type / session_id / stream_id / JSON payload).

use sdkwork_agent_kernel::{AgentStreamEvent, AgentStreamSink, SandboxEvent, SandboxEventPhase};

#[test]
fn sandbox_event_phase_round_trips_through_strings() {
    for phase in [
        SandboxEventPhase::Pending,
        SandboxEventPhase::Active,
        SandboxEventPhase::Completed,
        SandboxEventPhase::Failed,
    ] {
        assert_eq!(SandboxEventPhase::from_str(phase.as_str()), Some(phase));
    }
    assert_eq!(SandboxEventPhase::from_str("unknown"), None);
    assert_eq!(SandboxEventPhase::Active.as_str(), "active");
}

#[test]
fn phase_constructors_build_events() {
    let pending = SandboxEvent::pending("evt.1", "sandbox-session-1");
    assert_eq!(pending.phase, SandboxEventPhase::Pending);
    assert_eq!(pending.sandbox_session_id, "sandbox-session-1");
    assert!(pending.message.is_none());

    let active = SandboxEvent::active("evt.2", "sandbox-session-1");
    assert_eq!(active.phase, SandboxEventPhase::Active);

    let completed = SandboxEvent::completed("evt.3", "sandbox-session-1");
    assert_eq!(completed.phase, SandboxEventPhase::Completed);

    let failed =
        SandboxEvent::failed("evt.4", "sandbox-session-1").with_message("provider unresponsive");
    assert_eq!(failed.phase, SandboxEventPhase::Failed);
    assert_eq!(failed.message.as_deref(), Some("provider unresponsive"));
}

#[test]
fn sandbox_event_has_stable_dot_delimited_type() {
    let event = AgentStreamEvent::Sandbox(SandboxEvent::failed("evt.1", "sandbox-session-1"));
    assert_eq!(event.event_type(), "agent.stream.sandbox");
    assert_eq!(event.event_id(), "evt.1");
    // The event family groups sandbox lifecycle under the stream family.
    assert!(event
        .event_type()
        .starts_with(sdkwork_agent_kernel::AGENT_STREAM_EVENT_FAMILY));
}

#[test]
fn sandbox_event_accepts_correlation_identities() {
    let event = AgentStreamEvent::Sandbox(
        SandboxEvent::active("evt.1", "sandbox-session-1")
            .with_session_id("session-1")
            .with_stream_id("stream-1"),
    );

    assert_eq!(event.session_id(), Some("session-1"));
    assert_eq!(event.stream_id(), Some("stream-1"));

    // with_session_id_optional only attaches when a value is present.
    let untouched = event.with_session_id_optional(&None);
    assert_eq!(untouched.session_id(), Some("session-1"));

    let bare = AgentStreamEvent::Sandbox(SandboxEvent::active("evt.2", "sandbox-session-1"));
    let attached = bare.with_session_id_optional(&Some("session-9".to_string()));
    assert_eq!(attached.session_id(), Some("session-9"));

    let reattached = untouched.with_session_id("session-2");
    assert_eq!(reattached.session_id(), Some("session-2"));
}

#[test]
fn sandbox_event_bridges_to_kernel_event_envelope() {
    let event = AgentStreamEvent::Sandbox(
        SandboxEvent::failed("evt.1", "sandbox-session-1")
            .with_session_id("session-1")
            .with_message("lease lost"),
    );

    let kernel_event = event.to_kernel_event();
    assert_eq!(kernel_event.event_type, "agent.stream.sandbox");
    assert!(kernel_event.payload.contains("sandbox-session-1"));
}

#[test]
fn sandbox_event_stream_correlation_via_builders() {
    let mut sink = sdkwork_agent_kernel::InMemoryAgentStreamSink::default();
    sink.push_event(AgentStreamEvent::Sandbox(SandboxEvent::pending(
        "evt.0",
        "sandbox-session-1",
    )));
    sink.push_event(AgentStreamEvent::Sandbox(
        SandboxEvent::active("evt.1", "sandbox-session-1").with_session_id("session-1"),
    ));
    sink.push_event(AgentStreamEvent::Sandbox(SandboxEvent::completed(
        "evt.2",
        "sandbox-session-1",
    )));

    let events = sink.events();
    assert_eq!(events.len(), 3);
    let phases: Vec<SandboxEventPhase> = events
        .iter()
        .filter_map(|event| match event {
            AgentStreamEvent::Sandbox(sandbox_event) => Some(sandbox_event.phase),
            _ => None,
        })
        .collect();
    assert_eq!(
        phases,
        vec![
            SandboxEventPhase::Pending,
            SandboxEventPhase::Active,
            SandboxEventPhase::Completed,
        ]
    );
}
