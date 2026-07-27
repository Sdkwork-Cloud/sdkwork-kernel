use std::{collections::HashMap, sync::RwLock, time::Duration};

use sdkwork_agent_kernel::{
    AgentSession, KernelError, KernelResult, ProviderSessionActivityProvider,
    ProviderSessionActivitySink, SessionActivityEvidenceKind, SessionActivityFreshness,
    SessionActivityInteractionHint, SessionActivitySnapshot, SessionActivityState,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::SessionAdapter;

pub const DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL: Duration = Duration::from_secs(30);
pub const MAX_PROVIDER_SESSION_ACTIVITY_CLOCK_SKEW: Duration = Duration::from_secs(30);

/// Adapter capability for projecting provider Session runtime observations onto
/// the shared `AgentSession` activity contract.
pub trait ProviderSessionActivityAdapter: SessionAdapter {
    type ExternalActivity;

    fn to_session_activity(
        &self,
        external: &Self::ExternalActivity,
    ) -> KernelResult<SessionActivitySnapshot>;

    fn to_agent_session_with_activity(
        &self,
        external_session: &Self::ExternalSession,
        external_activity: &Self::ExternalActivity,
    ) -> KernelResult<AgentSession> {
        let mut session = self.to_agent_session(external_session)?;
        session.apply_activity(self.to_session_activity(external_activity)?)?;
        Ok(session)
    }
}

/// Thread-safe, process-local activity source for runtime facade queries.
///
/// Provider event/status bridges record only observations they can prove. A
/// read recalculates freshness, so an observation cannot remain working after
/// its TTL merely because no later provider event arrived.
#[derive(Debug, Default)]
pub struct InMemoryProviderSessionActivityProvider {
    snapshots: RwLock<HashMap<String, SessionActivitySnapshot>>,
}

impl InMemoryProviderSessionActivityProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &self,
        snapshot: SessionActivitySnapshot,
    ) -> KernelResult<SessionActivitySnapshot> {
        let snapshot = refresh_provider_session_activity(&snapshot)?;
        if snapshot.observed_at.is_none() || snapshot.evidence_kind.is_none() {
            return Err(KernelError::validation(
                "ingested provider session activity must include observed_at and evidence_kind",
            ));
        }
        let mut snapshots = self.snapshots.write().map_err(|_| KernelError::Internal {
            message: "provider session activity store lock poisoned".to_string(),
        })?;
        if let Some(existing) = snapshots.get(&snapshot.provider_session_id) {
            let ordering = compare_activity_observation_time(&snapshot, existing)?;
            if ordering.is_lt() {
                return refresh_provider_session_activity(existing);
            }
            if ordering.is_eq() {
                if existing == &snapshot {
                    return Ok(snapshot);
                }
                return Err(KernelError::validation(
                    "conflicting provider session activity observations share observed_at",
                ));
            }
        }
        snapshots.insert(snapshot.provider_session_id.clone(), snapshot.clone());
        Ok(snapshot)
    }

    pub fn remove(&self, provider_session_id: &str) -> KernelResult<()> {
        validate_provider_session_id(provider_session_id)?;
        let mut snapshots = self.snapshots.write().map_err(|_| KernelError::Internal {
            message: "provider session activity store lock poisoned".to_string(),
        })?;
        snapshots.remove(provider_session_id);
        Ok(())
    }
}

impl ProviderSessionActivityProvider for InMemoryProviderSessionActivityProvider {
    fn get_provider_session_activity(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<SessionActivitySnapshot> {
        validate_provider_session_id(provider_session_id)?;
        let snapshot = self
            .snapshots
            .read()
            .map_err(|_| KernelError::Internal {
                message: "provider session activity store lock poisoned".to_string(),
            })?
            .get(provider_session_id)
            .cloned()
            .unwrap_or_else(|| SessionActivitySnapshot::unsupported(provider_session_id));
        refresh_provider_session_activity(&snapshot)
    }
}

impl ProviderSessionActivitySink for InMemoryProviderSessionActivityProvider {
    fn ingest_provider_session_activity(
        &self,
        snapshot: SessionActivitySnapshot,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.record(snapshot)
    }
}

pub fn session_activity_from_provider_observation(
    provider_session_id: &str,
    state: SessionActivityState,
    evidence_kind: SessionActivityEvidenceKind,
    interaction_hint: Option<SessionActivityInteractionHint>,
    observed_at: &str,
    stale_after: Duration,
) -> KernelResult<SessionActivitySnapshot> {
    session_activity_from_provider_observation_at(
        provider_session_id,
        state,
        evidence_kind,
        interaction_hint,
        observed_at,
        stale_after,
        &crate::now_iso(),
    )
}

pub fn session_activity_from_provider_observation_at(
    provider_session_id: &str,
    state: SessionActivityState,
    evidence_kind: SessionActivityEvidenceKind,
    interaction_hint: Option<SessionActivityInteractionHint>,
    observed_at: &str,
    stale_after: Duration,
    now: &str,
) -> KernelResult<SessionActivitySnapshot> {
    validate_provider_session_id(provider_session_id)?;
    if stale_after.is_zero() {
        return Err(KernelError::validation(
            "provider session activity stale_after must be greater than zero",
        ));
    }
    if interaction_hint.is_some() && state != SessionActivityState::Waiting {
        return Err(KernelError::validation(
            "provider session interaction hint requires waiting activity",
        ));
    }
    let observed_at = parse_activity_timestamp("observed_at", observed_at)?;
    let now = parse_activity_timestamp("now", now)?;
    let stale_after = time::Duration::try_from(stale_after).map_err(|_| {
        KernelError::validation("provider session activity stale_after is out of range")
    })?;
    let max_clock_skew = time::Duration::try_from(MAX_PROVIDER_SESSION_ACTIVITY_CLOCK_SKEW)
        .expect("provider session activity clock skew constant must fit time::Duration");
    if observed_at > now + max_clock_skew {
        return Err(KernelError::validation(
            "provider session activity observed_at exceeds the allowed clock skew",
        ));
    }
    let fresh_until = observed_at.checked_add(stale_after).ok_or_else(|| {
        KernelError::validation("provider session activity fresh_until is out of range")
    })?;
    let freshness = if now <= fresh_until {
        SessionActivityFreshness::Fresh
    } else {
        SessionActivityFreshness::Stale
    };

    Ok(SessionActivitySnapshot::observed(
        provider_session_id,
        state,
        freshness,
        evidence_kind,
        interaction_hint,
        format_activity_timestamp(observed_at)?,
        format_activity_timestamp(fresh_until)?,
    ))
}

pub fn refresh_provider_session_activity(
    snapshot: &SessionActivitySnapshot,
) -> KernelResult<SessionActivitySnapshot> {
    refresh_provider_session_activity_at(snapshot, &crate::now_iso())
}

pub fn refresh_provider_session_activity_at(
    snapshot: &SessionActivitySnapshot,
    now: &str,
) -> KernelResult<SessionActivitySnapshot> {
    validate_provider_session_id(&snapshot.provider_session_id)?;
    if snapshot.freshness == SessionActivityFreshness::Unsupported {
        if snapshot.state.is_some()
            || snapshot.interaction_hint.is_some()
            || snapshot.fresh_until.is_some()
        {
            return Err(KernelError::validation(
                "unsupported provider session activity cannot include state, interaction_hint, or fresh_until",
            ));
        }
        match (&snapshot.evidence_kind, &snapshot.observed_at) {
            (Some(_), Some(observed_at)) => {
                let observed_at = parse_activity_timestamp("observed_at", observed_at)?;
                let now = parse_activity_timestamp("now", now)?;
                validate_observation_clock_skew(observed_at, now)?;
            }
            (None, None) => {}
            _ => {
                return Err(KernelError::validation(
                    "unsupported provider session activity evidence_kind and observed_at must appear together",
                ));
            }
        }
        return Ok(snapshot.clone());
    }
    let state = snapshot.state.ok_or_else(|| {
        KernelError::validation("supported provider session activity must include state")
    })?;
    let evidence_kind = snapshot.evidence_kind.ok_or_else(|| {
        KernelError::validation("supported provider session activity must include evidence_kind")
    })?;
    if snapshot.interaction_hint.is_some() && state != SessionActivityState::Waiting {
        return Err(KernelError::validation(
            "provider session interaction hint requires waiting activity",
        ));
    }
    let observed_at = snapshot.observed_at.as_deref().ok_or_else(|| {
        KernelError::validation("supported provider session activity must include observed_at")
    })?;
    let fresh_until = snapshot.fresh_until.as_deref().ok_or_else(|| {
        KernelError::validation("supported provider session activity must include fresh_until")
    })?;
    let observed_at = parse_activity_timestamp("observed_at", observed_at)?;
    let fresh_until = parse_activity_timestamp("fresh_until", fresh_until)?;
    let now = parse_activity_timestamp("now", now)?;
    validate_observation_clock_skew(observed_at, now)?;
    if fresh_until <= observed_at {
        return Err(KernelError::validation(
            "provider session activity fresh_until must follow observed_at",
        ));
    }
    let freshness = if snapshot.freshness == SessionActivityFreshness::Fresh && now <= fresh_until {
        SessionActivityFreshness::Fresh
    } else {
        SessionActivityFreshness::Stale
    };
    Ok(SessionActivitySnapshot::observed(
        &snapshot.provider_session_id,
        state,
        freshness,
        evidence_kind,
        snapshot.interaction_hint,
        format_activity_timestamp(observed_at)?,
        format_activity_timestamp(fresh_until)?,
    ))
}

fn compare_activity_observation_time(
    left: &SessionActivitySnapshot,
    right: &SessionActivitySnapshot,
) -> KernelResult<std::cmp::Ordering> {
    let left = left.observed_at.as_deref().ok_or_else(|| {
        KernelError::validation("ingested provider session activity must include observed_at")
    })?;
    let right = right.observed_at.as_deref().ok_or_else(|| {
        KernelError::validation("stored provider session activity must include observed_at")
    })?;
    Ok(parse_activity_timestamp("observed_at", left)?
        .cmp(&parse_activity_timestamp("observed_at", right)?))
}

fn validate_observation_clock_skew(
    observed_at: OffsetDateTime,
    now: OffsetDateTime,
) -> KernelResult<()> {
    let max_clock_skew = time::Duration::try_from(MAX_PROVIDER_SESSION_ACTIVITY_CLOCK_SKEW)
        .expect("provider session activity clock skew constant must fit time::Duration");
    if observed_at > now + max_clock_skew {
        return Err(KernelError::validation(
            "provider session activity observed_at exceeds the allowed clock skew",
        ));
    }
    Ok(())
}

fn validate_provider_session_id(provider_session_id: &str) -> KernelResult<()> {
    if provider_session_id.trim().is_empty() {
        return Err(KernelError::validation(
            "provider session activity provider_session_id must not be empty",
        ));
    }
    Ok(())
}

fn parse_activity_timestamp(field: &str, value: &str) -> KernelResult<OffsetDateTime> {
    OffsetDateTime::parse(value.trim(), &Rfc3339).map_err(|_| {
        KernelError::validation(format!(
            "provider session activity {field} must be an RFC3339 timestamp"
        ))
    })
}

fn format_activity_timestamp(value: OffsetDateTime) -> KernelResult<String> {
    value
        .format(&Rfc3339)
        .map_err(|error| KernelError::Internal {
            message: format!("failed to format provider session activity timestamp: {error}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_is_fresh_only_inside_its_ttl() {
        let fresh = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Working,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2026-07-27T12:00:00Z",
            Duration::from_secs(30),
            "2026-07-27T12:00:30Z",
        )
        .expect("fresh observation");
        let stale = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Working,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2026-07-27T12:00:00Z",
            Duration::from_secs(30),
            "2026-07-27T12:00:31Z",
        )
        .expect("stale observation");

        assert_eq!(fresh.freshness, SessionActivityFreshness::Fresh);
        assert_eq!(stale.freshness, SessionActivityFreshness::Stale);
        assert_eq!(fresh.fresh_until.as_deref(), Some("2026-07-27T12:00:30Z"));
    }

    #[test]
    fn invalid_or_untrusted_observation_time_fails_closed() {
        assert!(session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Working,
            SessionActivityEvidenceKind::ProviderStatus,
            None,
            "not-a-time",
            Duration::from_secs(30),
            "2026-07-27T12:00:00Z",
        )
        .is_err());
        assert!(session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Working,
            SessionActivityEvidenceKind::ProviderStatus,
            None,
            "2026-07-27T12:01:00Z",
            Duration::from_secs(30),
            "2026-07-27T12:00:00Z",
        )
        .is_err());
    }

    #[test]
    fn supported_snapshot_requires_a_positive_freshness_window() {
        let snapshot = SessionActivitySnapshot::observed(
            "provider.session.zero-window",
            SessionActivityState::Working,
            SessionActivityFreshness::Fresh,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2026-07-27T12:00:00Z",
            "2026-07-27T12:00:00Z",
        );

        assert!(refresh_provider_session_activity_at(&snapshot, "2026-07-27T12:00:00Z").is_err());
    }

    #[test]
    fn runtime_query_expires_cached_activity_and_fails_closed_for_unknown_session() {
        let store = InMemoryProviderSessionActivityProvider::new();
        let snapshot = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Waiting,
            SessionActivityEvidenceKind::ProviderEvent,
            Some(SessionActivityInteractionHint::ApprovalRequired),
            "2000-01-01T12:00:00Z",
            Duration::from_secs(30),
            "2000-01-01T12:00:00Z",
        )
        .expect("fresh activity");
        store.record(snapshot).expect("record activity");

        let cached = store
            .snapshots
            .read()
            .expect("activity store")
            .get("provider.session.1")
            .cloned()
            .expect("cached activity");
        let expired = refresh_provider_session_activity_at(&cached, "2000-01-01T12:00:31Z")
            .expect("expired activity");
        let unknown = store
            .get_provider_session_activity("provider.session.unknown")
            .expect("unsupported activity");

        assert_eq!(expired.freshness, SessionActivityFreshness::Stale);
        assert_eq!(
            expired.interaction_hint,
            Some(SessionActivityInteractionHint::ApprovalRequired)
        );
        assert_eq!(unknown.freshness, SessionActivityFreshness::Unsupported);
        assert_eq!(unknown.provider_session_id, "provider.session.unknown");
    }

    #[test]
    fn ingestion_is_monotonic_and_rejects_same_time_conflicts() {
        let store = InMemoryProviderSessionActivityProvider::new();
        let newer = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Waiting,
            SessionActivityEvidenceKind::ProviderEvent,
            Some(SessionActivityInteractionHint::UserInputRequired),
            "2000-01-01T00:00:10Z",
            Duration::from_secs(30),
            "2000-01-01T00:00:10Z",
        )
        .expect("newer activity");
        let older = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Working,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2000-01-01T00:00:00Z",
            Duration::from_secs(30),
            "2000-01-01T00:00:00Z",
        )
        .expect("older activity");
        let conflict = session_activity_from_provider_observation_at(
            "provider.session.1",
            SessionActivityState::Idle,
            SessionActivityEvidenceKind::ProviderEvent,
            None,
            "2000-01-01T00:00:10Z",
            Duration::from_secs(30),
            "2000-01-01T00:00:10Z",
        )
        .expect("conflicting activity");

        store.record(newer).expect("record newer activity");
        let retained = store.record(older).expect("ignore older activity");

        assert_eq!(retained.state, Some(SessionActivityState::Waiting));
        assert!(store.record(conflict).is_err());
    }
}
