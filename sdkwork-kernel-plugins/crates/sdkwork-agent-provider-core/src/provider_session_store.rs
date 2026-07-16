use crate::{create_session_from_config, now_iso, uuid_simple, SessionConfig};
use sdkwork_agent_kernel::{AgentMessage, AgentSession, KernelError, KernelResult, SessionState};
use sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

/// Maximum provider-local sessions retained in process memory per adapter.
const MAX_PROVIDER_SESSIONS: usize = 10_000;

/// Maximum provider-local session rows returned by one list page.
const MAX_PROVIDER_SESSION_PAGE_SIZE: usize = 200;

/// Maximum conversation messages retained per provider-local session.
const MAX_PROVIDER_CONVERSATION_MESSAGES: usize = 10_000;

/// Maximum number of incremental lifecycle changes retained for subscribers.
const MAX_PROVIDER_SESSION_CHANGES: usize = 10_000;

/// Maximum lifecycle changes copied into one incremental response page.
const MAX_PROVIDER_SESSION_CHANGE_PAGE_SIZE: usize = 200;

/// Canonical provider snapshot instant format. `%.f` keeps meaningful
/// sub-second precision while avoiding synthetic `.000000000` suffixes for
/// provider timestamps that are precise only to whole seconds.
const PROVIDER_SNAPSHOT_TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.fZ";

/// Filters for listing persisted provider sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionListQuery {
    pub agent_id: Option<String>,
    pub active_only: bool,
    pub limit: Option<usize>,
    /// Keyset cursor: the last `(updated_at, session_id)` returned by the
    /// previous page. Results are ordered newest-first.
    pub after_updated_at: Option<String>,
    pub after_session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSessionChangeKind {
    Created,
    Updated,
    Resumed,
    Closed,
    Deleted,
    Synchronized,
    MessageAppended,
}

/// One monotonically ordered provider-local session change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSessionChange {
    pub sequence: u64,
    pub provider_id: String,
    pub session_id: String,
    pub kind: ProviderSessionChangeKind,
    pub state: Option<SessionState>,
    pub occurred_at: String,
}

/// Bounded incremental change page used by polling or streaming adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSessionChangeBatch {
    pub changes: Vec<ProviderSessionChange>,
    pub next_cursor: u64,
    pub has_more: bool,
}

/// In-memory session + conversation store for a single provider adapter.
pub struct InMemoryProviderSessionStore {
    provider_id: String,
    inner: RwLock<ProviderSessionInner>,
}

/// Applies the provider-independent invariants required before a native
/// snapshot enters lifecycle storage or the unified runtime store.
pub fn finalize_provider_session_snapshot(
    provider_id: &str,
    mut session: AgentSession,
) -> KernelResult<AgentSession> {
    if provider_id.trim().is_empty() {
        return Err(KernelError::validation("provider_id must not be empty"));
    }
    if session.session_id.trim().is_empty() {
        return Err(KernelError::validation("session_id must not be empty"));
    }
    normalize_stable_session_reference(
        &mut session.parent_session_id,
        &session.session_id,
        "parent_session_id",
    )?;
    normalize_stable_session_reference(
        &mut session.forked_from_id,
        &session.session_id,
        "forked_from_id",
    )?;
    normalize_child_session_ids(&mut session)?;
    normalize_session_timestamp(&mut session.created_at, "created_at")?;
    normalize_session_timestamp(&mut session.updated_at, "updated_at")?;
    normalize_session_timestamp(&mut session.ended_at, "ended_at")?;
    normalize_session_timestamp(&mut session.archived_at, "archived_at")?;
    validate_session_timestamp_order(&session)?;
    normalize_session_token_total(&mut session)?;
    ensure_provider_metadata(&mut session, provider_id);
    Ok(session)
}

struct ProviderSessionInner {
    sessions: HashMap<String, AgentSession>,
    conversations: HashMap<String, Vec<AgentMessage>>,
    messages_by_id: HashMap<String, (String, AgentMessage)>,
    next_change_sequence: u64,
    changes: VecDeque<ProviderSessionChange>,
}

impl InMemoryProviderSessionStore {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            inner: RwLock::new(ProviderSessionInner {
                sessions: HashMap::new(),
                conversations: HashMap::new(),
                messages_by_id: HashMap::new(),
                next_change_sequence: 1,
                changes: VecDeque::new(),
            }),
        }
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the latest committed provider-local lifecycle sequence.
    pub fn current_change_cursor(&self) -> KernelResult<u64> {
        Ok(self
            .read_inner()?
            .next_change_sequence
            .checked_sub(1)
            .expect("provider change sequence starts at one"))
    }

    pub fn create_session(
        &self,
        agent_id: &str,
        user_ref: Option<&str>,
        config: SessionConfig,
    ) -> KernelResult<AgentSession> {
        let now = now_iso();
        let session_id = format!("{}.{}", self.provider_id, uuid_simple());
        let mut session = create_session_from_config(
            &session_id,
            Some(agent_id.to_string()),
            user_ref.map(String::from),
            None,
            config,
            now.as_str(),
        );
        session.updated_at = Some(now);
        session.state = SessionState::Active;
        ensure_provider_metadata(&mut session, &self.provider_id);

        let mut inner = self.write_inner()?;
        if inner.sessions.len() >= MAX_PROVIDER_SESSIONS {
            return Err(KernelError::validation(format!(
                "provider session store capacity exceeded ({MAX_PROVIDER_SESSIONS})"
            )));
        }
        ensure_change_sequence_available(&inner)?;
        inner.sessions.insert(session_id, session.clone());
        inner
            .conversations
            .entry(session.session_id.clone())
            .or_default();
        self.record_change(&mut inner, &session, ProviderSessionChangeKind::Created);
        Ok(session)
    }

    pub fn get_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        self.find_session(session_id)?
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))
    }

    pub fn find_session(&self, session_id: &str) -> KernelResult<Option<AgentSession>> {
        Ok(self.read_inner()?.sessions.get(session_id).cloned())
    }

    pub fn update_session(&self, mut session: AgentSession) -> KernelResult<AgentSession> {
        let mut inner = self.write_inner()?;
        let existing_created_at = inner
            .sessions
            .get(&session.session_id)
            .map(|existing| existing.created_at.clone())
            .ok_or_else(|| {
                KernelError::validation(format!("session not found: {}", session.session_id))
            })?;
        session.created_at = existing_created_at.or(session.created_at);
        let mut session = finalize_provider_session_snapshot(&self.provider_id, session)?;
        let existing = inner.sessions.get(&session.session_id).ok_or_else(|| {
            KernelError::validation(format!("session not found: {}", session.session_id))
        })?;
        reject_terminal_state_regression(existing.state, session.state)?;
        session.created_at = existing.created_at.clone().or(session.created_at);
        merge_stable_session_reference(
            &mut session.parent_session_id,
            existing.parent_session_id.as_deref(),
            "parent_session_id",
        )?;
        merge_stable_session_reference(
            &mut session.forked_from_id,
            existing.forked_from_id.as_deref(),
            "forked_from_id",
        )?;
        session.ended_at = session.ended_at.or_else(|| existing.ended_at.clone());
        session.archived_at = session.archived_at.or_else(|| existing.archived_at.clone());
        merge_monotonic_session_aggregates(&mut session, existing)?;
        ensure_change_sequence_available(&inner)?;
        touch_session(&mut session);
        ensure_terminal_session_timestamps(&mut session);
        inner
            .sessions
            .insert(session.session_id.clone(), session.clone());
        self.record_change(&mut inner, &session, ProviderSessionChangeKind::Updated);
        Ok(session)
    }

    /// Insert or refresh an externally discovered native provider session.
    pub fn synchronize_session(&self, mut session: AgentSession) -> KernelResult<AgentSession> {
        let mut inner = self.write_inner()?;
        if let Some(existing) = inner.sessions.get(&session.session_id) {
            session.created_at = existing.created_at.clone().or(session.created_at);
        }
        let mut session = finalize_provider_session_snapshot(&self.provider_id, session)?;
        if let Some(existing) = inner.sessions.get(&session.session_id) {
            reject_terminal_state_regression(existing.state, session.state)?;
            if session.updated_at.is_none() {
                session.updated_at = existing.updated_at.clone();
            }
            if snapshot_is_older(&session, existing) {
                return Ok(existing.clone());
            }
            session.created_at = existing.created_at.clone().or(session.created_at);
            merge_stable_session_reference(
                &mut session.parent_session_id,
                existing.parent_session_id.as_deref(),
                "parent_session_id",
            )?;
            merge_stable_session_reference(
                &mut session.forked_from_id,
                existing.forked_from_id.as_deref(),
                "forked_from_id",
            )?;
            session.ended_at = session.ended_at.or_else(|| existing.ended_at.clone());
            session.archived_at = session.archived_at.or_else(|| existing.archived_at.clone());
            merge_monotonic_session_aggregates(&mut session, existing)?;
            ensure_terminal_session_timestamps(&mut session);
            if &session == existing {
                return Ok(existing.clone());
            }
        } else if session.updated_at.is_none() {
            touch_session(&mut session);
        }
        ensure_terminal_session_timestamps(&mut session);
        if !inner.sessions.contains_key(&session.session_id)
            && inner.sessions.len() >= MAX_PROVIDER_SESSIONS
        {
            return Err(KernelError::validation(format!(
                "provider session store capacity exceeded ({MAX_PROVIDER_SESSIONS})"
            )));
        }
        ensure_change_sequence_available(&inner)?;
        inner
            .sessions
            .insert(session.session_id.clone(), session.clone());
        inner
            .conversations
            .entry(session.session_id.clone())
            .or_default();
        self.record_change(
            &mut inner,
            &session,
            ProviderSessionChangeKind::Synchronized,
        );
        Ok(session)
    }

    pub fn delete_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        let mut inner = self.write_inner()?;
        if !inner.sessions.contains_key(session_id) {
            return Err(KernelError::validation(format!(
                "session not found: {session_id}"
            )));
        }
        ensure_change_sequence_available(&inner)?;
        let session = inner
            .sessions
            .remove(session_id)
            .expect("session existence checked before deletion");
        if let Some(messages) = inner.conversations.remove(session_id) {
            for message in messages {
                inner.messages_by_id.remove(&message.message_id);
            }
        }
        self.record_change(&mut inner, &session, ProviderSessionChangeKind::Deleted);
        Ok(session)
    }

    pub fn resume_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        self.transition_session(
            session_id,
            SessionState::Active,
            ProviderSessionChangeKind::Resumed,
        )
    }

    pub fn close_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        self.transition_session(
            session_id,
            SessionState::Closed,
            ProviderSessionChangeKind::Closed,
        )
    }

    pub fn list_active_sessions(&self) -> KernelResult<Vec<AgentSession>> {
        self.list_sessions(&SessionListQuery {
            active_only: true,
            ..SessionListQuery::default()
        })
    }

    pub fn list_sessions(&self, query: &SessionListQuery) -> KernelResult<Vec<AgentSession>> {
        if query.after_updated_at.is_some() != query.after_session_id.is_some() {
            return Err(KernelError::validation(
                "session list cursor requires both after_updated_at and after_session_id",
            ));
        }
        if query
            .after_session_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(KernelError::validation(
                "after_session_id must not be empty",
            ));
        }
        let cursor_at = query
            .after_updated_at
            .as_deref()
            .map(|value| {
                sdkwork_utils_rust::parse_datetime(value, None)
                    .map(|parsed| {
                        sdkwork_utils_rust::format_datetime(parsed, Some("%Y-%m-%dT%H:%M:%S%.9fZ"))
                    })
                    .ok_or_else(|| {
                        KernelError::validation("after_updated_at must be an RFC 3339 timestamp")
                    })
            })
            .transpose()?;
        let inner = self.read_inner()?;
        let mut sessions: Vec<AgentSession> = inner
            .sessions
            .values()
            .filter(|session| {
                if query.active_only && !session.state.is_active() {
                    return false;
                }
                if let Some(agent_id) = query.agent_id.as_deref() {
                    if session.agent_id.as_deref() != Some(agent_id) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        sort_sessions_by_updated_at(&mut sessions);

        if let (Some(after_at), Some(after_id)) =
            (cursor_at.as_deref(), query.after_session_id.as_deref())
        {
            sessions.retain(|session| {
                let sort_at = session_sort_timestamp(session);
                // Descending `(timestamp, id)` keyset continuation.
                sort_at < after_at
                    || (sort_at == after_at && session.session_id.as_str() < after_id)
            });
        }

        let limit = query
            .limit
            .unwrap_or(DEFAULT_LIST_PAGE_SIZE as usize)
            .clamp(1, MAX_PROVIDER_SESSION_PAGE_SIZE);
        if sessions.len() > limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }

    pub fn get_conversation_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        let inner = self.read_inner()?;
        if !inner.sessions.contains_key(session_id) {
            return Err(KernelError::validation(format!(
                "session not found: {session_id}"
            )));
        }
        Ok(inner
            .conversations
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn append_conversation_message(
        &self,
        session_id: &str,
        mut message: AgentMessage,
    ) -> KernelResult<()> {
        let mut inner = self.write_inner()?;
        if !inner.sessions.contains_key(session_id) {
            return Err(KernelError::validation(format!(
                "session not found: {session_id}"
            )));
        }
        if message.message_id.trim().is_empty() {
            return Err(KernelError::validation("message_id must not be empty"));
        }
        if let Some(bound_session_id) = message.session_id.as_deref() {
            if bound_session_id != session_id {
                return Err(KernelError::validation(format!(
                    "message {} belongs to session {bound_session_id}, expected {session_id}",
                    message.message_id
                )));
            }
        } else {
            message.session_id = Some(session_id.to_string());
        }
        normalize_session_timestamp(&mut message.created_at, "message.created_at")?;
        if let Some((owner_session_id, existing)) = inner.messages_by_id.get(&message.message_id) {
            if owner_session_id != session_id {
                return Err(KernelError::validation(format!(
                    "message {} already belongs to session {owner_session_id}",
                    message.message_id
                )));
            }
            if existing != &message {
                return Err(KernelError::validation(format!(
                    "message {} already exists with different content",
                    message.message_id
                )));
            }
            return Ok(());
        }
        if inner
            .sessions
            .get(session_id)
            .is_some_and(|session| session.state.is_terminal())
        {
            return Err(KernelError::validation(format!(
                "cannot append a message to terminal session {session_id}"
            )));
        }
        let next_message_count = inner
            .sessions
            .get(session_id)
            .expect("session existence checked before append")
            .message_count
            .checked_add(1)
            .ok_or_else(|| KernelError::validation("session message count overflow"))?;
        ensure_change_sequence_available(&inner)?;
        inner
            .conversations
            .entry(session_id.to_string())
            .or_default()
            .push(message.clone());
        let conversation = inner
            .conversations
            .get_mut(session_id)
            .expect("conversation exists after push");
        if conversation.len() > MAX_PROVIDER_CONVERSATION_MESSAGES {
            let overflow = conversation.len() - MAX_PROVIDER_CONVERSATION_MESSAGES;
            let evicted_message_ids: Vec<_> = conversation
                .drain(0..overflow)
                .map(|message| message.message_id)
                .collect();
            for message_id in evicted_message_ids {
                inner.messages_by_id.remove(&message_id);
            }
        }
        inner.messages_by_id.insert(
            message.message_id.clone(),
            (session_id.to_string(), message),
        );
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.message_count = next_message_count;
            touch_session(session);
        }
        let session = inner
            .sessions
            .get(session_id)
            .cloned()
            .expect("session exists after conversation append");
        self.record_change(
            &mut inner,
            &session,
            ProviderSessionChangeKind::MessageAppended,
        );
        Ok(())
    }

    pub fn changes_since(
        &self,
        after_sequence: u64,
        limit: Option<usize>,
    ) -> KernelResult<ProviderSessionChangeBatch> {
        let inner = self.read_inner()?;
        if after_sequence >= inner.next_change_sequence && after_sequence != 0 {
            return Err(KernelError::validation(format!(
                "session change cursor {after_sequence} is ahead of the latest available sequence {}",
                inner.next_change_sequence.saturating_sub(1)
            )));
        }
        if let Some(oldest) = inner.changes.front() {
            if after_sequence.saturating_add(1) < oldest.sequence {
                return Err(KernelError::validation(format!(
                    "session change cursor {after_sequence} expired; oldest available sequence is {}",
                    oldest.sequence
                )));
            }
        }
        let limit = limit
            .unwrap_or(DEFAULT_LIST_PAGE_SIZE as usize)
            .clamp(1, MAX_PROVIDER_SESSION_CHANGE_PAGE_SIZE);
        let mut matching = inner
            .changes
            .iter()
            .filter(|change| change.sequence > after_sequence);
        let changes: Vec<_> = matching.by_ref().take(limit).cloned().collect();
        let has_more = matching.next().is_some();
        let next_cursor = changes
            .last()
            .map(|change| change.sequence)
            .unwrap_or(after_sequence);
        Ok(ProviderSessionChangeBatch {
            changes,
            next_cursor,
            has_more,
        })
    }

    fn transition_session(
        &self,
        session_id: &str,
        state: SessionState,
        kind: ProviderSessionChangeKind,
    ) -> KernelResult<AgentSession> {
        let mut inner = self.write_inner()?;
        let current_state = inner
            .sessions
            .get(session_id)
            .map(|session| session.state)
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))?;
        if current_state == state {
            return inner.sessions.get(session_id).cloned().ok_or_else(|| {
                KernelError::validation(format!("session not found: {session_id}"))
            });
        }
        if state == SessionState::Active && current_state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot resume terminal session from state {:?}",
                current_state
            )));
        }
        if state == SessionState::Closed
            && matches!(current_state, SessionState::Failed | SessionState::Archived)
        {
            return Err(KernelError::validation(format!(
                "cannot close terminal session from state {:?}",
                current_state
            )));
        }
        ensure_change_sequence_available(&inner)?;
        let session = inner
            .sessions
            .get_mut(session_id)
            .expect("session existence checked before transition");
        session.state = state;
        touch_session(session);
        if state == SessionState::Closed {
            session.ended_at = session.updated_at.clone();
        }
        let session = session.clone();
        self.record_change(&mut inner, &session, kind);
        Ok(session)
    }

    fn record_change(
        &self,
        inner: &mut ProviderSessionInner,
        session: &AgentSession,
        kind: ProviderSessionChangeKind,
    ) {
        let sequence = inner.next_change_sequence;
        inner.next_change_sequence = inner
            .next_change_sequence
            .checked_add(1)
            .expect("change sequence availability checked before mutation");
        inner.changes.push_back(ProviderSessionChange {
            sequence,
            provider_id: self.provider_id.clone(),
            session_id: session.session_id.clone(),
            kind,
            state: (kind != ProviderSessionChangeKind::Deleted).then_some(session.state),
            occurred_at: now_iso(),
        });
        if inner.changes.len() > MAX_PROVIDER_SESSION_CHANGES {
            inner.changes.pop_front();
        }
    }

    fn read_inner(&self) -> KernelResult<std::sync::RwLockReadGuard<'_, ProviderSessionInner>> {
        self.inner.read().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })
    }

    fn write_inner(&self) -> KernelResult<std::sync::RwLockWriteGuard<'_, ProviderSessionInner>> {
        self.inner.write().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })
    }
}

pub fn sort_sessions_by_updated_at(sessions: &mut [AgentSession]) {
    sessions.sort_by(|left, right| {
        session_sort_timestamp(right)
            .cmp(session_sort_timestamp(left))
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
}

fn session_sort_timestamp(session: &AgentSession) -> &str {
    session
        .updated_at
        .as_deref()
        .or(session.created_at.as_deref())
        .unwrap_or("")
}

fn touch_session(session: &mut AgentSession) {
    session.updated_at = Some(now_iso());
}

fn reject_terminal_state_regression(
    existing: SessionState,
    incoming: SessionState,
) -> KernelResult<()> {
    if existing.is_terminal() && !incoming.is_terminal() {
        return Err(KernelError::validation(format!(
            "cannot transition terminal session from {existing:?} to {incoming:?}"
        )));
    }
    Ok(())
}

fn ensure_change_sequence_available(inner: &ProviderSessionInner) -> KernelResult<()> {
    if inner.next_change_sequence == u64::MAX {
        return Err(KernelError::validation(
            "provider session change sequence exhausted",
        ));
    }
    Ok(())
}

fn merge_monotonic_session_aggregates(
    incoming: &mut AgentSession,
    existing: &AgentSession,
) -> KernelResult<()> {
    incoming.message_count = incoming.message_count.max(existing.message_count);
    incoming.token_usage.input_tokens = incoming
        .token_usage
        .input_tokens
        .max(existing.token_usage.input_tokens);
    incoming.token_usage.output_tokens = incoming
        .token_usage
        .output_tokens
        .max(existing.token_usage.output_tokens);
    incoming.token_usage.cached_tokens = incoming
        .token_usage
        .cached_tokens
        .max(existing.token_usage.cached_tokens);
    incoming.token_usage.reasoning_tokens = incoming
        .token_usage
        .reasoning_tokens
        .max(existing.token_usage.reasoning_tokens);
    let counted_total = incoming
        .token_usage
        .input_tokens
        .checked_add(incoming.token_usage.output_tokens)
        .ok_or_else(|| KernelError::validation("session token usage overflow"))?;
    incoming.token_usage.total_tokens = incoming
        .token_usage
        .total_tokens
        .max(existing.token_usage.total_tokens)
        .max(counted_total);
    incoming.tool_call_count = incoming.tool_call_count.max(existing.tool_call_count);
    incoming.compression_count = incoming.compression_count.max(existing.compression_count);
    incoming.cost_cents = match (incoming.cost_cents, existing.cost_cents) {
        (Some(incoming), Some(existing)) => Some(incoming.max(existing)),
        (incoming, existing) => incoming.or(existing),
    };
    incoming.change_summary.additions = incoming
        .change_summary
        .additions
        .max(existing.change_summary.additions);
    incoming.change_summary.deletions = incoming
        .change_summary
        .deletions
        .max(existing.change_summary.deletions);
    incoming.change_summary.files_changed = incoming
        .change_summary
        .files_changed
        .max(existing.change_summary.files_changed);
    Ok(())
}

fn normalize_session_token_total(session: &mut AgentSession) -> KernelResult<()> {
    let counted_total = session
        .token_usage
        .input_tokens
        .checked_add(session.token_usage.output_tokens)
        .ok_or_else(|| KernelError::validation("session token usage overflow"))?;
    session.token_usage.total_tokens = session.token_usage.total_tokens.max(counted_total);
    Ok(())
}

fn ensure_terminal_session_timestamps(session: &mut AgentSession) {
    let terminal_at = session
        .updated_at
        .clone()
        .or_else(|| session.created_at.clone());
    if session.state.is_terminal() && session.ended_at.is_none() {
        session.ended_at = terminal_at.clone();
    }
    if session.state == SessionState::Archived && session.archived_at.is_none() {
        session.archived_at = terminal_at;
    }
}

fn normalize_stable_session_reference(
    value: &mut Option<String>,
    session_id: &str,
    field: &str,
) -> KernelResult<()> {
    let Some(reference) = value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        *value = None;
        return Ok(());
    };
    if reference == session_id {
        return Err(KernelError::validation(format!(
            "{field} must not reference the session itself"
        )));
    }
    *value = Some(reference.to_string());
    Ok(())
}

fn merge_stable_session_reference(
    incoming: &mut Option<String>,
    existing: Option<&str>,
    field: &str,
) -> KernelResult<()> {
    if let Some(existing) = existing {
        if incoming
            .as_deref()
            .is_some_and(|incoming| incoming != existing)
        {
            return Err(KernelError::validation(format!(
                "{field} cannot change from {existing}"
            )));
        }
        *incoming = Some(existing.to_string());
    }
    Ok(())
}

fn normalize_child_session_ids(session: &mut AgentSession) -> KernelResult<()> {
    let mut seen = HashSet::with_capacity(session.child_session_ids.len());
    let mut normalized = Vec::with_capacity(session.child_session_ids.len());
    for child_session_id in &session.child_session_ids {
        let child_session_id = child_session_id.trim();
        if child_session_id.is_empty() {
            return Err(KernelError::validation(
                "child_session_ids must not contain an empty session id",
            ));
        }
        if child_session_id == session.session_id {
            return Err(KernelError::validation(
                "child_session_ids must not reference the session itself",
            ));
        }
        if seen.insert(child_session_id.to_string()) {
            normalized.push(child_session_id.to_string());
        }
    }
    session.child_session_ids = normalized;
    Ok(())
}

fn ensure_provider_metadata(session: &mut AgentSession, provider_id: &str) {
    session
        .metadata
        .retain(|(key, _)| key != "provider_id" && key != "provider_session_id");
    session
        .metadata
        .push(("provider_id".to_string(), provider_id.to_string()));
    session.metadata.push((
        "provider_session_id".to_string(),
        session.session_id.clone(),
    ));
}

fn snapshot_is_older(incoming: &AgentSession, existing: &AgentSession) -> bool {
    let incoming_at = incoming
        .updated_at
        .as_deref()
        .or(incoming.created_at.as_deref());
    let existing_at = existing
        .updated_at
        .as_deref()
        .or(existing.created_at.as_deref());
    match (incoming_at, existing_at) {
        (Some(incoming_at), Some(existing_at)) => {
            match (
                sdkwork_utils_rust::parse_datetime(incoming_at, None),
                sdkwork_utils_rust::parse_datetime(existing_at, None),
            ) {
                (Some(incoming_at), Some(existing_at)) => incoming_at < existing_at,
                _ => false,
            }
        }
        _ => false,
    }
}

fn normalize_session_timestamp(value: &mut Option<String>, field: &str) -> KernelResult<()> {
    let Some(raw) = value.as_deref().filter(|value| !value.trim().is_empty()) else {
        *value = None;
        return Ok(());
    };
    let parsed = sdkwork_utils_rust::parse_datetime(raw, None)
        .ok_or_else(|| KernelError::validation(format!("{field} must be an RFC 3339 timestamp")))?;
    *value = Some(sdkwork_utils_rust::format_datetime(
        parsed,
        Some(PROVIDER_SNAPSHOT_TIMESTAMP_FORMAT),
    ));
    Ok(())
}

fn validate_session_timestamp_order(session: &AgentSession) -> KernelResult<()> {
    let parsed = |value: Option<&str>| {
        value.and_then(|value| sdkwork_utils_rust::parse_datetime(value, None))
    };
    let created_at = parsed(session.created_at.as_deref());
    let updated_at = parsed(session.updated_at.as_deref());
    let ended_at = parsed(session.ended_at.as_deref());
    let archived_at = parsed(session.archived_at.as_deref());
    if matches!((created_at, updated_at), (Some(created), Some(updated)) if updated < created) {
        return Err(KernelError::validation(
            "updated_at must not be earlier than created_at",
        ));
    }
    if matches!((created_at, ended_at), (Some(created), Some(ended)) if ended < created) {
        return Err(KernelError::validation(
            "ended_at must not be earlier than created_at",
        ));
    }
    if matches!((created_at, archived_at), (Some(created), Some(archived)) if archived < created) {
        return Err(KernelError::validation(
            "archived_at must not be earlier than created_at",
        ));
    }
    if matches!((ended_at, archived_at), (Some(ended), Some(archived)) if archived < ended) {
        return Err(KernelError::validation(
            "archived_at must not be earlier than ended_at",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{AgentMessageRole, AgentPart, SessionKind, SessionSource};

    #[test]
    fn create_session_persists_and_lists_by_updated_at() {
        let store = InMemoryProviderSessionStore::new("codex");
        let config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main)
            .with_title("First");

        let first = store
            .create_session("agent.1", Some("user.1"), config.clone())
            .expect("first session");
        let second = store
            .create_session("agent.1", Some("user.1"), config.with_title("Second"))
            .expect("second session");

        store
            .append_conversation_message(
                &first.session_id,
                AgentMessage::new(
                    "msg.1",
                    AgentMessageRole::User,
                    vec![AgentPart::text("p1", "hello")],
                ),
            )
            .expect("append");

        let listed = store
            .list_sessions(&SessionListQuery::default())
            .expect("listed");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].session_id, first.session_id);
        assert_eq!(listed[1].session_id, second.session_id);
    }

    #[test]
    fn session_list_is_bounded_and_rejects_partial_cursors() {
        let store = InMemoryProviderSessionStore::new("bounded");
        for index in 0..=MAX_PROVIDER_SESSION_PAGE_SIZE {
            store
                .create_session(
                    "agent.bounded",
                    None,
                    SessionConfig::new().with_title(format!("Session {index}")),
                )
                .expect("session");
        }
        let listed = store
            .list_sessions(&SessionListQuery {
                limit: Some(usize::MAX),
                ..SessionListQuery::default()
            })
            .expect("bounded list");
        assert_eq!(listed.len(), MAX_PROVIDER_SESSION_PAGE_SIZE);
        assert!(store
            .list_sessions(&SessionListQuery {
                after_session_id: Some("bounded.cursor".to_string()),
                ..SessionListQuery::default()
            })
            .is_err());
        assert!(store
            .list_sessions(&SessionListQuery {
                after_updated_at: Some("2026-07-15T00:00:00Z".to_string()),
                ..SessionListQuery::default()
            })
            .is_err());
    }

    #[test]
    fn session_list_normalizes_timezone_cursor_before_keyset_comparison() {
        let store = InMemoryProviderSessionStore::new("cursor");
        for (session_id, updated_at) in [
            ("cursor.session.1", "2026-07-15T00:03:00Z"),
            ("cursor.session.2", "2026-07-15T00:02:00Z"),
            ("cursor.session.3", "2026-07-15T00:01:00Z"),
        ] {
            let mut session = AgentSession::new(session_id);
            session.updated_at = Some(updated_at.to_string());
            store.synchronize_session(session).expect("snapshot");
        }
        let next = store
            .list_sessions(&SessionListQuery {
                limit: Some(2),
                after_updated_at: Some("2026-07-15T08:02:00+08:00".to_string()),
                after_session_id: Some("cursor.session.2".to_string()),
                ..SessionListQuery::default()
            })
            .expect("cursor page");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].session_id, "cursor.session.3");
    }

    #[test]
    fn conversation_history_roundtrip() {
        let store = InMemoryProviderSessionStore::new("hermes");
        let session = store
            .create_session(
                "agent.1",
                None,
                SessionConfig::new()
                    .with_source(SessionSource::Cli)
                    .with_kind(SessionKind::Main),
            )
            .expect("created");

        store
            .append_conversation_message(
                &session.session_id,
                AgentMessage::new(
                    "msg.1",
                    AgentMessageRole::User,
                    vec![AgentPart::text("p1", "hello")],
                ),
            )
            .expect("append");

        let history = store
            .get_conversation_history(&session.session_id)
            .expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].parts[0].text.as_deref(), Some("hello"));
        assert_eq!(
            history[0].session_id.as_deref(),
            Some(session.session_id.as_str())
        );
    }

    #[test]
    fn conversation_append_is_idempotent_and_message_identity_is_immutable() {
        let store = InMemoryProviderSessionStore::new("messages");
        let first = store
            .create_session("agent.messages", None, SessionConfig::new())
            .expect("first session");
        let second = store
            .create_session("agent.messages", None, SessionConfig::new())
            .expect("second session");
        let message = AgentMessage::new(
            "msg.identity",
            AgentMessageRole::User,
            vec![AgentPart::text("part.identity", "original")],
        );
        store
            .append_conversation_message(&first.session_id, message.clone())
            .expect("first append");
        let change_count = store
            .changes_since(0, Some(20))
            .expect("changes")
            .changes
            .len();
        store
            .append_conversation_message(&first.session_id, message.clone())
            .expect("exact retry");
        assert_eq!(
            store
                .get_conversation_history(&first.session_id)
                .expect("first history")
                .len(),
            1
        );
        assert_eq!(
            store
                .get_session(&first.session_id)
                .expect("first")
                .message_count,
            1
        );
        assert_eq!(
            store
                .changes_since(0, Some(20))
                .expect("changes after retry")
                .changes
                .len(),
            change_count
        );

        let conflicting = AgentMessage::new(
            "msg.identity",
            AgentMessageRole::Agent,
            vec![AgentPart::text("part.identity", "changed")],
        );
        assert!(store
            .append_conversation_message(&first.session_id, conflicting)
            .is_err());
        assert!(store
            .append_conversation_message(
                &second.session_id,
                message.clone().for_session(second.session_id.clone()),
            )
            .is_err());
        assert!(store
            .get_conversation_history(&second.session_id)
            .expect("second history")
            .is_empty());

        store.close_session(&first.session_id).expect("close");
        store
            .append_conversation_message(&first.session_id, message)
            .expect("exact retry remains idempotent after close");
    }

    #[test]
    fn conversation_append_validates_message_binding_and_timestamp() {
        let store = InMemoryProviderSessionStore::new("message-validation");
        let session = store
            .create_session("agent.messages", None, SessionConfig::new())
            .expect("session");
        let wrong_session = AgentMessage::new(
            "msg.wrong-session",
            AgentMessageRole::User,
            vec![AgentPart::text("part.wrong-session", "wrong")],
        )
        .for_session("message-validation.other");
        assert!(store
            .append_conversation_message(&session.session_id, wrong_session)
            .is_err());
        let mut invalid_timestamp = AgentMessage::new(
            "msg.invalid-time",
            AgentMessageRole::User,
            vec![AgentPart::text("part.invalid-time", "invalid")],
        );
        invalid_timestamp.created_at = Some("eventually".to_string());
        assert!(store
            .append_conversation_message(&session.session_id, invalid_timestamp)
            .is_err());
        assert!(store
            .get_conversation_history(&session.session_id)
            .expect("history")
            .is_empty());
    }

    #[test]
    fn message_count_overflow_rejects_append_without_partial_mutation() {
        let store = InMemoryProviderSessionStore::new("overflow");
        let session = store
            .create_session("agent.overflow", None, SessionConfig::new())
            .expect("created");
        let mut maxed = session.clone();
        maxed.message_count = u32::MAX;
        store.update_session(maxed).expect("max count snapshot");
        let change_count = store
            .changes_since(0, Some(10))
            .expect("changes before append")
            .changes
            .len();

        let error = store
            .append_conversation_message(
                &session.session_id,
                AgentMessage::new(
                    "msg.overflow",
                    AgentMessageRole::User,
                    vec![AgentPart::text("part.overflow", "late")],
                ),
            )
            .expect_err("overflow must fail");
        assert!(error.to_string().contains("message count overflow"));
        assert!(store
            .get_conversation_history(&session.session_id)
            .expect("history")
            .is_empty());
        assert_eq!(
            store
                .changes_since(0, Some(10))
                .expect("changes after append")
                .changes
                .len(),
            change_count
        );
    }

    #[test]
    fn exhausted_change_sequence_rejects_mutation_without_partial_state() {
        let store = InMemoryProviderSessionStore::new("sequence");
        store.write_inner().expect("inner").next_change_sequence = u64::MAX;

        let error = store
            .create_session("agent.sequence", None, SessionConfig::new())
            .expect_err("sequence exhaustion must fail");
        assert!(error.to_string().contains("change sequence exhausted"));
        assert!(store
            .list_sessions(&SessionListQuery::default())
            .expect("sessions")
            .is_empty());
    }

    #[test]
    fn update_rejects_invalid_timestamp_without_overwriting_session() {
        let store = InMemoryProviderSessionStore::new("timestamps");
        let session = store
            .create_session("agent.timestamps", None, SessionConfig::new())
            .expect("created");
        let mut invalid = session.clone();
        invalid.updated_at = Some("eventually".to_string());

        assert!(store.update_session(invalid).is_err());
        assert_eq!(
            store.get_session(&session.session_id).expect("unchanged"),
            session
        );
        assert_eq!(
            store
                .changes_since(0, Some(10))
                .expect("changes")
                .changes
                .len(),
            1
        );
    }

    #[test]
    fn updates_preserve_created_at_and_monotonic_message_count() {
        let store = InMemoryProviderSessionStore::new("aggregates");
        let session = store
            .create_session("agent.aggregates", None, SessionConfig::new())
            .expect("created");
        store
            .append_conversation_message(
                &session.session_id,
                AgentMessage::new(
                    "msg.aggregate",
                    AgentMessageRole::User,
                    vec![AgentPart::text("part.aggregate", "hello")],
                ),
            )
            .expect("message");
        let original_created_at = session.created_at.clone();

        let mut ordinary_update = store.get_session(&session.session_id).expect("session");
        ordinary_update.created_at = Some("2027-01-01T00:00:00Z".to_string());
        ordinary_update.parent_session_id = Some("aggregates.parent".to_string());
        ordinary_update.forked_from_id = Some("aggregates.source".to_string());
        ordinary_update.ended_at = Some("2099-01-01T00:03:00Z".to_string());
        ordinary_update.archived_at = Some("2099-01-01T00:04:00Z".to_string());
        ordinary_update.message_count = 0;
        ordinary_update.token_usage.input_tokens = 11;
        ordinary_update.token_usage.output_tokens = 7;
        ordinary_update.token_usage.cached_tokens = 3;
        ordinary_update.token_usage.reasoning_tokens = 5;
        ordinary_update.token_usage.total_tokens = 18;
        ordinary_update.tool_call_count = 4;
        ordinary_update.compression_count = 2;
        ordinary_update.cost_cents = Some(99);
        ordinary_update.change_summary.additions = 21;
        ordinary_update.change_summary.deletions = 8;
        ordinary_update.change_summary.files_changed = 3;
        ordinary_update.title = Some("ordinary".to_string());
        let ordinary_update = store
            .update_session(ordinary_update)
            .expect("ordinary update");
        assert_eq!(ordinary_update.created_at, original_created_at);
        assert_eq!(ordinary_update.message_count, 1);

        let mut synchronized = ordinary_update;
        synchronized.created_at = Some("2028-01-01T00:00:00Z".to_string());
        synchronized.updated_at = Some("2030-01-01T00:00:00Z".to_string());
        synchronized.parent_session_id = None;
        synchronized.forked_from_id = None;
        synchronized.ended_at = None;
        synchronized.archived_at = None;
        synchronized.message_count = 0;
        synchronized.token_usage = Default::default();
        synchronized.tool_call_count = 0;
        synchronized.compression_count = 0;
        synchronized.cost_cents = None;
        synchronized.change_summary = Default::default();
        synchronized.title = Some("synchronized".to_string());
        let synchronized = store
            .synchronize_session(synchronized)
            .expect("synchronized update");
        assert_eq!(synchronized.created_at, original_created_at);
        assert_eq!(synchronized.message_count, 1);
        assert_eq!(synchronized.token_usage.input_tokens, 11);
        assert_eq!(synchronized.token_usage.output_tokens, 7);
        assert_eq!(synchronized.token_usage.cached_tokens, 3);
        assert_eq!(synchronized.token_usage.reasoning_tokens, 5);
        assert_eq!(synchronized.token_usage.total_tokens, 18);
        assert_eq!(synchronized.tool_call_count, 4);
        assert_eq!(synchronized.compression_count, 2);
        assert_eq!(synchronized.cost_cents, Some(99));
        assert_eq!(synchronized.change_summary.additions, 21);
        assert_eq!(synchronized.change_summary.deletions, 8);
        assert_eq!(synchronized.change_summary.files_changed, 3);
        assert_eq!(
            synchronized.parent_session_id.as_deref(),
            Some("aggregates.parent")
        );
        assert_eq!(
            synchronized.forked_from_id.as_deref(),
            Some("aggregates.source")
        );
        assert_eq!(
            synchronized.ended_at.as_deref(),
            Some("2099-01-01T00:03:00Z")
        );
        assert_eq!(
            synchronized.archived_at.as_deref(),
            Some("2099-01-01T00:04:00Z")
        );
        assert_eq!(synchronized.title.as_deref(), Some("synchronized"));

        let mut conflicting_lineage = synchronized;
        conflicting_lineage.parent_session_id = Some("aggregates.other".to_string());
        assert!(store.update_session(conflicting_lineage).is_err());
    }

    #[test]
    fn full_crud_and_incremental_changes_are_ordered() {
        let store = InMemoryProviderSessionStore::new("rig");
        let created = store
            .create_session(
                "agent.rig",
                None,
                SessionConfig::new().with_title("Initial"),
            )
            .expect("created");
        assert_eq!(store.get_session(&created.session_id).unwrap(), created);

        let mut updated = created.clone();
        updated.title = Some("Updated".to_string());
        let updated = store.update_session(updated).expect("updated");
        assert_eq!(updated.title.as_deref(), Some("Updated"));

        let closed = store.close_session(&created.session_id).expect("closed");
        assert_eq!(closed.state, SessionState::Closed);
        let deleted = store.delete_session(&created.session_id).expect("deleted");
        assert_eq!(deleted.session_id, created.session_id);
        assert!(store.get_session(&created.session_id).is_err());

        let first_page = store.changes_since(0, Some(2)).expect("first changes");
        assert_eq!(first_page.changes.len(), 2);
        assert!(first_page.has_more);
        assert_eq!(
            first_page.changes[0].kind,
            ProviderSessionChangeKind::Created
        );
        assert_eq!(
            first_page.changes[1].kind,
            ProviderSessionChangeKind::Updated
        );

        let second_page = store
            .changes_since(first_page.next_cursor, Some(10))
            .expect("remaining changes");
        assert_eq!(
            second_page
                .changes
                .iter()
                .map(|change| change.kind)
                .collect::<Vec<_>>(),
            vec![
                ProviderSessionChangeKind::Closed,
                ProviderSessionChangeKind::Deleted
            ]
        );
        assert!(!second_page.has_more);
    }

    #[test]
    fn terminal_sessions_cannot_be_resumed_and_repeated_close_is_idempotent() {
        let store = InMemoryProviderSessionStore::new("claude-code");
        let created = store
            .create_session("agent.claude", None, SessionConfig::new())
            .expect("created");
        let closed = store.close_session(&created.session_id).expect("closed");
        assert_eq!(closed.ended_at, closed.updated_at);
        store
            .close_session(&created.session_id)
            .expect("repeated close is idempotent");
        assert!(store.resume_session(&created.session_id).is_err());
        let changes = store.changes_since(0, Some(10)).expect("changes");
        assert_eq!(
            changes
                .changes
                .iter()
                .filter(|change| change.kind == ProviderSessionChangeKind::Closed)
                .count(),
            1
        );

        let mut reopened = store
            .get_session(&created.session_id)
            .expect("closed session");
        reopened.state = SessionState::Active;
        assert!(store.update_session(reopened.clone()).is_err());
        reopened.updated_at = Some("2099-01-01T00:00:00Z".to_string());
        assert!(store.synchronize_session(reopened).is_err());
        assert!(store
            .append_conversation_message(
                &created.session_id,
                AgentMessage::new(
                    "msg.after-close",
                    AgentMessageRole::User,
                    vec![AgentPart::text("part.after-close", "late")],
                ),
            )
            .is_err());
    }

    #[test]
    fn synchronize_session_preserves_native_identity() {
        let store = InMemoryProviderSessionStore::new("codex");
        let mut native = AgentSession::new("thread.native.1");
        native.state = SessionState::Working;

        let synchronized = store.synchronize_session(native).expect("synchronized");
        assert_eq!(synchronized.metadata_value("provider_id"), Some("codex"));
        assert_eq!(
            synchronized.metadata_value("provider_session_id"),
            Some("thread.native.1")
        );
        assert_eq!(synchronized.state, SessionState::Working);
    }

    #[test]
    fn synchronized_terminal_snapshots_receive_lifecycle_timestamps() {
        let store = InMemoryProviderSessionStore::new("openclaw");
        let mut closed = AgentSession::new("openclaw.session.closed");
        closed.state = SessionState::Closed;
        closed.updated_at = Some("2026-07-16T00:01:00Z".to_string());
        let closed = store.synchronize_session(closed).expect("closed snapshot");
        assert_eq!(closed.ended_at, closed.updated_at);

        let mut archived = AgentSession::new("openclaw.session.archived");
        archived.state = SessionState::Archived;
        archived.updated_at = Some("2026-07-16T00:02:00Z".to_string());
        let archived = store
            .synchronize_session(archived)
            .expect("archived snapshot");
        assert_eq!(archived.ended_at, archived.updated_at);
        assert_eq!(archived.archived_at, archived.updated_at);
    }

    #[test]
    fn stale_provider_snapshot_cannot_roll_back_newer_state() {
        let store = InMemoryProviderSessionStore::new("codex");
        let mut newer = AgentSession::new("thread.native.2");
        newer.updated_at = Some("2026-07-15T00:02:00Z".to_string());
        newer.state = SessionState::Working;
        store.synchronize_session(newer).expect("newer snapshot");

        let mut stale = AgentSession::new("thread.native.2");
        stale.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        stale.state = SessionState::Paused;
        let retained = store
            .synchronize_session(stale)
            .expect("stale snapshot ignored");
        assert_eq!(retained.state, SessionState::Working);
        assert_eq!(
            store
                .changes_since(0, Some(10))
                .expect("changes")
                .changes
                .len(),
            1
        );
    }

    #[test]
    fn replaying_identical_provider_snapshot_does_not_emit_duplicate_change() {
        let store = InMemoryProviderSessionStore::new("openclaw");
        let mut snapshot = AgentSession::new("openclaw.session.replay");
        snapshot.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        snapshot.state = SessionState::Working;

        store
            .synchronize_session(snapshot.clone())
            .expect("first snapshot");
        store
            .synchronize_session(snapshot)
            .expect("replayed snapshot");
        assert_eq!(
            store
                .changes_since(0, Some(10))
                .expect("changes")
                .changes
                .len(),
            1
        );
    }

    #[test]
    fn provider_timestamps_are_normalized_before_comparison_and_sorting() {
        let store = InMemoryProviderSessionStore::new("codex");
        let mut snapshot = AgentSession::new("codex.session.offset");
        snapshot.updated_at = Some("2026-07-15T08:01:00+08:00".to_string());
        let synchronized = store
            .synchronize_session(snapshot)
            .expect("offset snapshot");
        assert_eq!(
            synchronized.updated_at.as_deref(),
            Some("2026-07-15T00:01:00Z")
        );

        let mut replay = synchronized;
        replay.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        store.synchronize_session(replay).expect("UTC replay");
        assert_eq!(
            store
                .changes_since(0, Some(10))
                .expect("changes")
                .changes
                .len(),
            1
        );
    }

    #[test]
    fn provider_snapshot_finalizer_enforces_identity_time_and_metadata() {
        let mut snapshot = AgentSession::new("native.session.1");
        snapshot.created_at = Some("2026-07-15T08:00:00+08:00".to_string());
        snapshot.updated_at = Some("".to_string());
        snapshot.ended_at = Some("2026-07-15T08:03:00+08:00".to_string());
        snapshot.archived_at = Some("2026-07-15T08:04:00.120000000+08:00".to_string());
        snapshot.token_usage.input_tokens = 11;
        snapshot.token_usage.output_tokens = 7;
        snapshot.parent_session_id = Some(" native.parent ".to_string());
        snapshot.forked_from_id = Some(" native.source ".to_string());
        snapshot.child_session_ids = vec![" native.child ".to_string(), "native.child".to_string()];
        snapshot
            .metadata
            .push(("provider_id".to_string(), "stale".to_string()));

        let finalized = finalize_provider_session_snapshot("codex", snapshot).expect("finalized");
        assert_eq!(
            finalized.created_at.as_deref(),
            Some("2026-07-15T00:00:00Z")
        );
        assert_eq!(finalized.updated_at, None);
        assert_eq!(finalized.ended_at.as_deref(), Some("2026-07-15T00:03:00Z"));
        assert_eq!(
            finalized.archived_at.as_deref(),
            Some("2026-07-15T00:04:00.120Z")
        );
        assert_eq!(finalized.token_usage.total_tokens, 18);
        assert_eq!(
            finalized.parent_session_id.as_deref(),
            Some("native.parent")
        );
        assert_eq!(finalized.forked_from_id.as_deref(), Some("native.source"));
        assert_eq!(finalized.child_session_ids, vec!["native.child"]);
        assert_eq!(finalized.metadata_value("provider_id"), Some("codex"));
        assert_eq!(
            finalized.metadata_value("provider_session_id"),
            Some("native.session.1")
        );
        assert!(finalize_provider_session_snapshot("codex", AgentSession::new(" ")).is_err());

        let mut invalid = AgentSession::new("native.session.invalid");
        invalid.ended_at = Some("eventually".to_string());
        assert!(finalize_provider_session_snapshot("codex", invalid).is_err());

        let mut overflow = AgentSession::new("native.session.overflow");
        overflow.token_usage.input_tokens = u64::MAX;
        overflow.token_usage.output_tokens = 1;
        assert!(finalize_provider_session_snapshot("codex", overflow).is_err());

        let mut self_parent = AgentSession::new("native.session.self-parent");
        self_parent.parent_session_id = Some(self_parent.session_id.clone());
        assert!(finalize_provider_session_snapshot("codex", self_parent).is_err());

        let mut self_child = AgentSession::new("native.session.self-child");
        self_child
            .child_session_ids
            .push(self_child.session_id.clone());
        assert!(finalize_provider_session_snapshot("codex", self_child).is_err());

        let mut invalid_order = AgentSession::new("native.session.invalid-order");
        invalid_order.created_at = Some("2026-07-16T00:02:00Z".to_string());
        invalid_order.updated_at = Some("2026-07-16T00:01:00Z".to_string());
        assert!(finalize_provider_session_snapshot("codex", invalid_order).is_err());

        let mut invalid_archive_order = AgentSession::new("native.session.invalid-archive-order");
        invalid_archive_order.created_at = Some("2026-07-16T00:00:00Z".to_string());
        invalid_archive_order.ended_at = Some("2026-07-16T00:02:00Z".to_string());
        invalid_archive_order.archived_at = Some("2026-07-16T00:01:00Z".to_string());
        assert!(finalize_provider_session_snapshot("codex", invalid_archive_order).is_err());
    }

    #[test]
    fn provider_timestamp_normalization_preserves_meaningful_subseconds() {
        let mut snapshot = AgentSession::new("native.session.subseconds");
        snapshot.created_at = Some("2026-07-15T08:00:00.120000000+08:00".to_string());

        let finalized = finalize_provider_session_snapshot("codex", snapshot)
            .expect("subsecond timestamp finalizes");
        assert_eq!(
            finalized.created_at.as_deref(),
            Some("2026-07-15T00:00:00.120Z")
        );
    }

    #[test]
    fn future_change_cursor_is_rejected() {
        let store = InMemoryProviderSessionStore::new("rig");
        assert!(store
            .changes_since(1, Some(10))
            .expect_err("future cursor")
            .to_string()
            .contains("ahead"));
        assert!(store.changes_since(0, Some(10)).is_ok());
    }

    #[test]
    fn provider_snapshot_rejects_invalid_non_empty_timestamp() {
        let store = InMemoryProviderSessionStore::new("hermes");
        let mut snapshot = AgentSession::new("hermes.session.invalid-time");
        snapshot.updated_at = Some("not-a-timestamp".to_string());
        assert!(store
            .synchronize_session(snapshot)
            .expect_err("invalid timestamp")
            .to_string()
            .contains("RFC 3339"));
    }

    #[test]
    fn change_retention_is_bounded_and_expired_cursors_fail() {
        let store = InMemoryProviderSessionStore::new("load-test");
        let session = store
            .create_session("agent.load", None, SessionConfig::new())
            .expect("created");
        for index in 0..MAX_PROVIDER_SESSION_CHANGES {
            let mut updated = store.get_session(&session.session_id).expect("session");
            updated.title = Some(format!("update-{index}"));
            store.update_session(updated).expect("updated");
        }

        let retained = store
            .changes_since(1, Some(usize::MAX))
            .expect("oldest retained cursor remains valid");
        assert_eq!(
            retained.changes.len(),
            MAX_PROVIDER_SESSION_CHANGE_PAGE_SIZE
        );
        assert!(retained.has_more);
        assert_eq!(
            store.read_inner().expect("inner").changes.len(),
            MAX_PROVIDER_SESSION_CHANGES
        );
        assert!(store
            .changes_since(0, Some(1))
            .expect_err("zero cursor must not hide an evicted change")
            .to_string()
            .contains("expired"));

        let mut extra = store.get_session(&session.session_id).expect("session");
        extra.title = Some("evict-again".to_string());
        store.update_session(extra).expect("updated");
        assert!(store
            .changes_since(1, Some(1))
            .expect_err("evicted cursor must fail")
            .to_string()
            .contains("expired"));
    }
}
