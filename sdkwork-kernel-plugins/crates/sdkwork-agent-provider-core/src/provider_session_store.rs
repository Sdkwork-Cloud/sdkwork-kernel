use crate::{create_session_from_config, now_iso, uuid_simple, SessionConfig};
use sdkwork_agent_kernel::{AgentMessage, AgentSession, KernelError, KernelResult, SessionState};
use sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE;
use std::collections::HashMap;
use std::sync::Mutex;

/// Maximum provider-local sessions retained in process memory per adapter.
const MAX_PROVIDER_SESSIONS: usize = 10_000;

/// Maximum conversation messages retained per provider-local session.
const MAX_PROVIDER_CONVERSATION_MESSAGES: usize = 10_000;

/// Filters for listing persisted provider sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionListQuery {
    pub agent_id: Option<String>,
    pub active_only: bool,
    pub limit: Option<usize>,
}

/// In-memory session + conversation store for a single provider adapter.
pub struct InMemoryProviderSessionStore {
    provider_id: String,
    inner: Mutex<ProviderSessionInner>,
}

struct ProviderSessionInner {
    sessions: HashMap<String, AgentSession>,
    conversations: HashMap<String, Vec<AgentMessage>>,
}

impl InMemoryProviderSessionStore {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            inner: Mutex::new(ProviderSessionInner {
                sessions: HashMap::new(),
                conversations: HashMap::new(),
            }),
        }
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
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
        session
            .metadata
            .push(("provider_id".to_string(), self.provider_id.clone()));

        let mut inner = self.lock_inner()?;
        if inner.sessions.len() >= MAX_PROVIDER_SESSIONS {
            return Err(KernelError::validation(format!(
                "provider session store capacity exceeded ({MAX_PROVIDER_SESSIONS})"
            )));
        }
        inner.sessions.insert(session_id, session.clone());
        inner
            .conversations
            .entry(session.session_id.clone())
            .or_default();
        Ok(session)
    }

    pub fn resume_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        let mut inner = self.lock_inner()?;
        let session = inner
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))?;
        session.state = SessionState::Active;
        touch_session(session);
        Ok(session.clone())
    }

    pub fn close_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        let mut inner = self.lock_inner()?;
        let session = inner
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))?;
        session.state = SessionState::Closed;
        touch_session(session);
        Ok(session.clone())
    }

    pub fn list_active_sessions(&self) -> KernelResult<Vec<AgentSession>> {
        self.list_sessions(&SessionListQuery {
            active_only: true,
            ..SessionListQuery::default()
        })
    }

    pub fn list_sessions(&self, query: &SessionListQuery) -> KernelResult<Vec<AgentSession>> {
        let inner = self.lock_inner()?;
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

        let limit = query.limit.unwrap_or(DEFAULT_LIST_PAGE_SIZE as usize);
        if sessions.len() > limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }

    pub fn get_conversation_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        let inner = self.lock_inner()?;
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
        message: AgentMessage,
    ) -> KernelResult<()> {
        let mut inner = self.lock_inner()?;
        if !inner.sessions.contains_key(session_id) {
            return Err(KernelError::validation(format!(
                "session not found: {session_id}"
            )));
        }
        inner
            .conversations
            .entry(session_id.to_string())
            .or_default()
            .push(message);
        let conversation = inner
            .conversations
            .get_mut(session_id)
            .expect("conversation exists after push");
        if conversation.len() > MAX_PROVIDER_CONVERSATION_MESSAGES {
            let overflow = conversation.len() - MAX_PROVIDER_CONVERSATION_MESSAGES;
            conversation.drain(0..overflow);
        }
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.message_count = session.message_count.saturating_add(1);
            touch_session(session);
        }
        Ok(())
    }

    fn lock_inner(&self) -> KernelResult<std::sync::MutexGuard<'_, ProviderSessionInner>> {
        self.inner.lock().map_err(|e| KernelError::Internal {
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
    }
}
