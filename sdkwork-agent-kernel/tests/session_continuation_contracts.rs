//! Contract tests for session continuation and message lineage.
//!
//! `SessionContinuation` models how a run attaches to existing session
//! history (resume / continue latest / fork / resume-at), aligning with the
//! agent SDK resume primitives. `AgentMessage.parent_message_id` chains
//! messages into a fork-safe lineage graph.

use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, SessionContinuation, SessionContinuationMode,
};

fn text_part(text: &str) -> AgentPart {
    AgentPart::text("part.1", text)
}

#[test]
fn continuation_mode_vocabulary_is_stable() {
    assert_eq!(
        SessionContinuation::resume("session.a").mode().as_str(),
        "resume"
    );
    assert_eq!(
        SessionContinuation::continue_latest().mode().as_str(),
        "continue_latest"
    );
    assert_eq!(
        SessionContinuation::fork("session.a").mode().as_str(),
        "fork"
    );
    assert_eq!(
        SessionContinuation::resume_at("session.a", "2026-08-01T00:00:00Z")
            .mode()
            .as_str(),
        "resume_at"
    );
}

#[test]
fn continuation_target_session_resolution() {
    let resume = SessionContinuation::resume("session.resume");
    assert_eq!(resume.mode().target_session_id(), Some("session.resume"));

    let latest = SessionContinuation::continue_latest();
    assert_eq!(latest.mode().target_session_id(), None);

    let fork = SessionContinuation::fork("session.source");
    assert_eq!(fork.mode().target_session_id(), Some("session.source"));

    let resume_at = SessionContinuation::resume_at("session.at", "2026-08-01T00:00:00Z");
    assert_eq!(resume_at.mode().target_session_id(), Some("session.at"));
}

#[test]
fn fork_continuation_carries_truncation_point() {
    let fork = SessionContinuation::fork_before("session.source", "msg.truncate.42");
    match fork.mode() {
        SessionContinuationMode::Fork {
            source_session_id,
            before_message_id,
        } => {
            assert_eq!(source_session_id, "session.source");
            assert_eq!(before_message_id.as_deref(), Some("msg.truncate.42"));
        }
        other => panic!("expected Fork mode, got {:?}", other.as_str()),
    }

    let plain_fork = SessionContinuation::fork("session.source");
    match plain_fork.mode() {
        SessionContinuationMode::Fork {
            before_message_id, ..
        } => assert!(before_message_id.is_none()),
        other => panic!("expected Fork mode, got {:?}", other.as_str()),
    }
}

#[test]
fn continuation_reason_is_optional_metadata() {
    let continuation = SessionContinuation::resume("session.a").with_reason("user requested");
    assert_eq!(continuation.reason.as_deref(), Some("user requested"));

    let plain = SessionContinuation::resume("session.a");
    assert!(plain.reason.is_none());
}

#[test]
fn message_parent_chain_links_lineage() {
    // Round 1: assistant message without a parent (root of the lineage).
    let assistant_1 = AgentMessage::new(
        "msg.assistant.1",
        AgentMessageRole::Agent,
        vec![text_part("first answer")],
    )
    .for_session("session.lineage");
    assert!(assistant_1.parent_message_id.is_none());

    // Round 2: assistant message produced by a tool call in round 1.
    let assistant_2 = AgentMessage::new(
        "msg.assistant.2",
        AgentMessageRole::Agent,
        vec![text_part("second answer")],
    )
    .for_session("session.lineage")
    .with_parent_message("msg.assistant.1");
    assert_eq!(
        assistant_2.parent_message_id.as_deref(),
        Some("msg.assistant.1")
    );

    // Sub-agent message: parent is the tool call that spawned it.
    let subagent_result = AgentMessage::new(
        "msg.subagent.1",
        AgentMessageRole::Agent,
        vec![text_part("subagent result")],
    )
    .for_session("session.lineage")
    .with_parent_message("tool-call.delegate.7");
    assert_eq!(
        subagent_result.parent_message_id.as_deref(),
        Some("tool-call.delegate.7")
    );
}

#[test]
fn message_lineage_round_trips_through_equality() {
    let message = AgentMessage::new(
        "msg.chain.1",
        AgentMessageRole::Tool,
        vec![text_part("tool output")],
    )
    .with_parent_message("msg.assistant.0");

    let clone = message.clone();
    assert_eq!(message, clone);
    assert_eq!(message.parent_message_id, clone.parent_message_id);
}

#[test]
fn session_fork_identity_fields_are_present_on_session_model() {
    // The session model already carries fork lineage identity; this pins the
    // field vocabulary consumers rely on.
    let session = sdkwork_agent_kernel::AgentSession::new("session.fork.1");
    assert_eq!(session.session_id, "session.fork.1");
    // Fork and parent identity are explicit optional fields.
    let _parent: Option<String> = session.parent_session_id;
    let _forked_from: Option<String> = session.forked_from_id;
}
