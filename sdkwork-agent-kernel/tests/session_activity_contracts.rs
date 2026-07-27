use sdkwork_agent_kernel::{
    AgentManifest, AgentSession, KernelResult, ProviderSessionActivityProvider, RuntimeBuilder,
    SessionActivityEvidenceKind, SessionActivityFreshness, SessionActivityInteractionHint,
    SessionActivitySnapshot, SessionActivityState, SessionState,
};
use std::sync::Arc;

const ACTIVITY_RUNTIME_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.activity-contract",
  "name": "sdkwork-activity-contract-agent",
  "display_name": "SDKWork Activity Contract Agent",
  "description": "Agent used to prove provider-scoped provider session activity.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#;

#[test]
fn fresh_provider_activity_projects_lifecycle_state() {
    let activity = SessionActivitySnapshot::observed(
        "session.activity.fresh",
        SessionActivityState::Working,
        SessionActivityFreshness::Fresh,
        SessionActivityEvidenceKind::ProviderStatus,
        None,
        "2026-07-27T12:00:00Z",
        "2026-07-27T12:00:30Z",
    );
    let session = AgentSession::new("session.activity.fresh")
        .with_activity(activity)
        .expect("matching activity");

    assert_eq!(session.state, SessionState::Working);
    assert!(session.activity.is_authoritative());
}

#[test]
fn stale_or_unsupported_activity_never_projects_ready() {
    let stale = SessionActivitySnapshot::observed(
        "session.activity.stale",
        SessionActivityState::Idle,
        SessionActivityFreshness::Stale,
        SessionActivityEvidenceKind::ProviderEvent,
        None,
        "2026-07-27T11:59:00Z",
        "2026-07-27T11:59:30Z",
    );
    let stale_session = AgentSession::new("session.activity.stale")
        .with_activity(stale)
        .expect("matching activity");
    let unsupported_session = AgentSession::new("session.activity.unsupported")
        .with_activity(SessionActivitySnapshot::unsupported(
            "session.activity.unsupported",
        ))
        .expect("matching activity");

    assert_eq!(stale_session.state, SessionState::Created);
    assert_eq!(unsupported_session.state, SessionState::Created);
    assert!(!stale_session.activity.is_authoritative());
    assert!(!unsupported_session.activity.is_authoritative());
}

#[test]
fn terminal_lifecycle_state_is_not_reopened_by_activity() {
    let mut session = AgentSession::new("session.activity.archived");
    session.state = SessionState::Archived;
    session
        .apply_activity(SessionActivitySnapshot::observed(
            "session.activity.archived",
            SessionActivityState::Working,
            SessionActivityFreshness::Fresh,
            SessionActivityEvidenceKind::ProviderProcess,
            None,
            "2026-07-27T12:00:00Z",
            "2026-07-27T12:00:30Z",
        ))
        .expect("matching activity");

    assert_eq!(session.state, SessionState::Archived);
}

#[test]
fn activity_identity_and_interaction_contracts_fail_closed() {
    let mismatch = AgentSession::new("session.activity.expected").with_activity(
        SessionActivitySnapshot::observed(
            "session.activity.other",
            SessionActivityState::Waiting,
            SessionActivityFreshness::Fresh,
            SessionActivityEvidenceKind::ProviderEvent,
            Some(SessionActivityInteractionHint::UserInputRequired),
            "2026-07-27T12:00:00Z",
            "2026-07-27T12:00:30Z",
        ),
    );

    assert!(mismatch.is_err());
}

#[test]
fn runtime_keeps_provider_activity_isolated_by_provider_id() {
    let manifest = AgentManifest::from_json(ACTIVITY_RUNTIME_MANIFEST_JSON).expect("manifest");
    let runtime = RuntimeBuilder::new("runtime.activity-contract", manifest)
        .register_provider_session_activity_provider(
            "provider.model.codex",
            Arc::new(StaticActivityProvider(SessionActivityState::Working)),
        )
        .register_provider_session_activity_provider(
            "provider.model.claude-code",
            Arc::new(StaticActivityProvider(SessionActivityState::Waiting)),
        )
        .bootstrap()
        .expect("runtime")
        .runtime;

    assert_eq!(
        runtime.provider_session_activity_provider_ids(),
        ["provider.model.codex", "provider.model.claude-code"]
    );

    let codex = runtime
        .provider_session_activity_provider_by_id("provider.model.codex")
        .expect("codex activity")
        .get_provider_session_activity("shared-provider-session")
        .expect("codex snapshot");
    let claude = runtime
        .provider_session_activity_provider_by_id("provider.model.claude-code")
        .expect("claude activity")
        .get_provider_session_activity("shared-provider-session")
        .expect("claude snapshot");

    assert_eq!(codex.state, Some(SessionActivityState::Working));
    assert_eq!(claude.state, Some(SessionActivityState::Waiting));
}

struct StaticActivityProvider(SessionActivityState);

impl ProviderSessionActivityProvider for StaticActivityProvider {
    fn get_provider_session_activity(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<SessionActivitySnapshot> {
        Ok(SessionActivitySnapshot::observed(
            provider_session_id,
            self.0,
            SessionActivityFreshness::Fresh,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2026-07-27T12:00:00Z",
            "2026-07-27T12:00:30Z",
        ))
    }
}
