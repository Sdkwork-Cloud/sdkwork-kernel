use std::sync::Arc;

use sdkwork_agent_kernel::{
    ProviderSessionActivitySink, SessionActivityInteractionHint, SessionActivitySnapshot,
    SessionActivityState,
};
use sdkwork_agent_provider_core::{
    session_activity_from_provider_observation, DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
};
use serde::{Deserialize, Serialize};

use crate::SdkRuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkRuntimeActivityPhase {
    Started,
    Working,
    Waiting,
    Failed,
    Idle,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkRuntimeActivityInteractionHint {
    ApprovalRequired,
    UserInputRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkRuntimeTerminalState {
    Idle,
    Failed,
}

/// Provider-owned live activity carried on the same correlated worker request.
///
/// These events describe only work started by the current SDKWork runtime or
/// events emitted by that owned provider instance. They are not an attach or
/// discovery mechanism for independently running provider processes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkRuntimeActivityEvent {
    pub provider_session_id: String,
    pub phase: SdkRuntimeActivityPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_hint: Option<SdkRuntimeActivityInteractionHint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_state: Option<SdkRuntimeTerminalState>,
    pub observed_at: String,
}

impl SdkRuntimeActivityEvent {
    pub fn to_session_activity(&self) -> Result<SessionActivitySnapshot, SdkRuntimeError> {
        let state = match self.phase {
            SdkRuntimeActivityPhase::Started | SdkRuntimeActivityPhase::Working => {
                ensure_terminal_state_absent(self)?;
                SessionActivityState::Working
            }
            SdkRuntimeActivityPhase::Waiting => {
                ensure_terminal_state_absent(self)?;
                SessionActivityState::Waiting
            }
            SdkRuntimeActivityPhase::Failed => {
                ensure_terminal_state_absent(self)?;
                SessionActivityState::Failed
            }
            SdkRuntimeActivityPhase::Idle => {
                ensure_terminal_state_absent(self)?;
                SessionActivityState::Idle
            }
            SdkRuntimeActivityPhase::Terminal => match self.terminal_state.ok_or_else(|| {
                SdkRuntimeError::new(
                    "invalid_activity_event",
                    "terminal activity event requires terminal_state",
                )
            })? {
                SdkRuntimeTerminalState::Idle => SessionActivityState::Idle,
                SdkRuntimeTerminalState::Failed => SessionActivityState::Failed,
            },
        };
        let interaction_hint = match self.interaction_hint {
            Some(SdkRuntimeActivityInteractionHint::ApprovalRequired) => {
                Some(SessionActivityInteractionHint::ApprovalRequired)
            }
            Some(SdkRuntimeActivityInteractionHint::UserInputRequired) => {
                Some(SessionActivityInteractionHint::UserInputRequired)
            }
            None => None,
        };

        session_activity_from_provider_observation(
            &self.provider_session_id,
            state,
            sdkwork_agent_kernel::SessionActivityEvidenceKind::ProviderEvent,
            interaction_hint,
            &self.observed_at,
            DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
        )
        .map_err(|error| SdkRuntimeError::new("invalid_activity_event", error.to_string()))
    }
}

fn ensure_terminal_state_absent(event: &SdkRuntimeActivityEvent) -> Result<(), SdkRuntimeError> {
    if event.terminal_state.is_some() {
        return Err(SdkRuntimeError::new(
            "invalid_activity_event",
            "terminal_state is valid only for terminal activity events",
        ));
    }
    Ok(())
}

pub trait SdkRuntimeActivityEventSink: Send + Sync {
    fn ingest_runtime_activity(
        &self,
        event: SdkRuntimeActivityEvent,
    ) -> Result<(), SdkRuntimeError>;
}

/// Converts typed worker activity into the kernel activity sink contract.
pub struct ProviderSessionActivityRuntimeSink {
    sink: Arc<dyn ProviderSessionActivitySink>,
}

impl ProviderSessionActivityRuntimeSink {
    pub fn new(sink: Arc<dyn ProviderSessionActivitySink>) -> Self {
        Self { sink }
    }
}

impl SdkRuntimeActivityEventSink for ProviderSessionActivityRuntimeSink {
    fn ingest_runtime_activity(
        &self,
        event: SdkRuntimeActivityEvent,
    ) -> Result<(), SdkRuntimeError> {
        let snapshot = event.to_session_activity()?;
        self.sink
            .ingest_provider_session_activity(snapshot)
            .map(|_| ())
            .map_err(|error| SdkRuntimeError::new("activity_sink_failed", error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{
        ProviderSessionActivityProvider, SessionActivityFreshness, SessionActivityState,
    };
    use sdkwork_agent_provider_core::InMemoryProviderSessionActivityProvider;

    #[test]
    fn all_owned_runtime_phases_map_without_inventing_activity() {
        let cases = [
            (
                SdkRuntimeActivityPhase::Started,
                SessionActivityState::Working,
            ),
            (
                SdkRuntimeActivityPhase::Working,
                SessionActivityState::Working,
            ),
            (
                SdkRuntimeActivityPhase::Waiting,
                SessionActivityState::Waiting,
            ),
            (
                SdkRuntimeActivityPhase::Failed,
                SessionActivityState::Failed,
            ),
            (SdkRuntimeActivityPhase::Idle, SessionActivityState::Idle),
        ];

        for (index, (phase, expected)) in cases.into_iter().enumerate() {
            let snapshot = SdkRuntimeActivityEvent {
                provider_session_id: format!("provider.{index}"),
                phase,
                interaction_hint: None,
                terminal_state: None,
                observed_at: sdkwork_agent_provider_core::now_iso(),
            }
            .to_session_activity()
            .expect("phase should map");
            assert_eq!(snapshot.state, Some(expected));
            assert_eq!(snapshot.freshness, SessionActivityFreshness::Fresh);
        }
    }

    #[test]
    fn terminal_outcome_controls_terminal_projection() {
        for (terminal_state, expected) in [
            (SdkRuntimeTerminalState::Idle, SessionActivityState::Idle),
            (
                SdkRuntimeTerminalState::Failed,
                SessionActivityState::Failed,
            ),
        ] {
            let snapshot = SdkRuntimeActivityEvent {
                provider_session_id: format!("provider.terminal.{expected:?}"),
                phase: SdkRuntimeActivityPhase::Terminal,
                interaction_hint: None,
                terminal_state: Some(terminal_state),
                observed_at: sdkwork_agent_provider_core::now_iso(),
            }
            .to_session_activity()
            .expect("terminal event should map");
            assert_eq!(snapshot.state, Some(expected));
        }
    }

    #[test]
    fn runtime_sink_records_and_queries_the_shared_activity_store() {
        let store = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let sink = ProviderSessionActivityRuntimeSink::new(store.clone());
        sink.ingest_runtime_activity(SdkRuntimeActivityEvent {
            provider_session_id: "provider.live".to_string(),
            phase: SdkRuntimeActivityPhase::Working,
            interaction_hint: None,
            terminal_state: None,
            observed_at: sdkwork_agent_provider_core::now_iso(),
        })
        .expect("activity should ingest");

        let snapshot = store
            .get_provider_session_activity("provider.live")
            .expect("activity should query");
        assert_eq!(snapshot.state, Some(SessionActivityState::Working));
    }

    #[test]
    fn malformed_terminal_event_fails_closed() {
        let error = SdkRuntimeActivityEvent {
            provider_session_id: "provider.invalid".to_string(),
            phase: SdkRuntimeActivityPhase::Terminal,
            interaction_hint: None,
            terminal_state: None,
            observed_at: sdkwork_agent_provider_core::now_iso(),
        }
        .to_session_activity()
        .expect_err("terminal state is required");
        assert_eq!(error.code, "invalid_activity_event");
    }
}
