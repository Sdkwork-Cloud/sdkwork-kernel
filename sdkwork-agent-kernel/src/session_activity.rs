use crate::{KernelResult, SessionState};

/// Provider-neutral activity state observed from a native agent runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivityState {
    Idle,
    Working,
    Waiting,
    Failed,
}

impl SessionActivityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Failed => "failed",
        }
    }
}

/// Whether an activity observation is still safe to project into lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivityFreshness {
    Fresh,
    Stale,
    Unsupported,
}

impl SessionActivityFreshness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Provider facts that are strong enough to support an activity observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivityEvidenceKind {
    ProviderStatus,
    ProviderEvent,
    ProviderLock,
    ProviderProcess,
}

/// Provider-neutral reason why a waiting session needs human interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivityInteractionHint {
    ApprovalRequired,
    UserInputRequired,
}

impl SessionActivityInteractionHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalRequired => "approval_required",
            Self::UserInputRequired => "user_input_required",
        }
    }
}

impl SessionActivityEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderStatus => "provider_status",
            Self::ProviderEvent => "provider_event",
            Self::ProviderLock => "provider_lock",
            Self::ProviderProcess => "provider_process",
        }
    }
}

/// One bounded native-runtime activity observation.
///
/// Persisted history metadata and file modification times are not activity
/// evidence. Only fresh provider status, event, lock, or process observations
/// may project a session into an executable lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionActivitySnapshot {
    pub provider_session_id: String,
    pub state: Option<SessionActivityState>,
    pub freshness: SessionActivityFreshness,
    pub evidence_kind: Option<SessionActivityEvidenceKind>,
    pub interaction_hint: Option<SessionActivityInteractionHint>,
    pub observed_at: Option<String>,
    pub fresh_until: Option<String>,
}

impl SessionActivitySnapshot {
    pub fn unsupported(provider_session_id: impl Into<String>) -> Self {
        Self {
            provider_session_id: provider_session_id.into(),
            state: None,
            freshness: SessionActivityFreshness::Unsupported,
            evidence_kind: None,
            interaction_hint: None,
            observed_at: None,
            fresh_until: None,
        }
    }

    pub fn unsupported_with_evidence(
        provider_session_id: impl Into<String>,
        evidence_kind: SessionActivityEvidenceKind,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            provider_session_id: provider_session_id.into(),
            state: None,
            freshness: SessionActivityFreshness::Unsupported,
            evidence_kind: Some(evidence_kind),
            interaction_hint: None,
            observed_at: Some(observed_at.into()),
            fresh_until: None,
        }
    }

    pub fn observed(
        provider_session_id: impl Into<String>,
        state: SessionActivityState,
        freshness: SessionActivityFreshness,
        evidence_kind: SessionActivityEvidenceKind,
        interaction_hint: Option<SessionActivityInteractionHint>,
        observed_at: impl Into<String>,
        fresh_until: impl Into<String>,
    ) -> Self {
        Self {
            provider_session_id: provider_session_id.into(),
            state: Some(state),
            freshness,
            evidence_kind: Some(evidence_kind),
            interaction_hint,
            observed_at: Some(observed_at.into()),
            fresh_until: Some(fresh_until.into()),
        }
    }

    pub fn is_authoritative(&self) -> bool {
        self.freshness == SessionActivityFreshness::Fresh
            && self.state.is_some()
            && self.evidence_kind.is_some()
            && self.observed_at.is_some()
            && self.fresh_until.is_some()
    }

    /// Projects a provider observation without allowing missing or expired facts
    /// to masquerade as an idle/ready session.
    pub fn project_lifecycle_state(&self, current: SessionState) -> SessionState {
        if current.is_terminal() {
            return current;
        }
        if !self.is_authoritative() {
            return SessionState::Created;
        }
        match self.state {
            Some(SessionActivityState::Idle) => SessionState::Active,
            Some(SessionActivityState::Working) => SessionState::Working,
            Some(SessionActivityState::Waiting) => SessionState::Waiting,
            Some(SessionActivityState::Failed) => SessionState::Failed,
            None => SessionState::Created,
        }
    }
}

/// Runtime query boundary consumed by product-facing session facades.
///
/// This provider is scoped to one registered provider identity. A facade must
/// select that provider before querying its provider session id.
///
/// Implementations must return `Unsupported` or `Stale` when the provider
/// cannot prove current activity. Absence of evidence must never become idle.
pub trait ProviderSessionActivityProvider: Send + Sync {
    fn get_provider_session_activity(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<SessionActivitySnapshot>;
}

/// Provider-neutral ingestion boundary for live runtime observations.
///
/// Protocol-specific collectors must map their status or event into a
/// `SessionActivitySnapshot` before calling this sink. Static history readers
/// must not publish observations through this interface.
pub trait ProviderSessionActivitySink: Send + Sync {
    fn ingest_provider_session_activity(
        &self,
        snapshot: SessionActivitySnapshot,
    ) -> KernelResult<SessionActivitySnapshot>;
}
