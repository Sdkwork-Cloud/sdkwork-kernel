use crate::session::query::BridgeSessionQuery;
use crate::session::sqlite::SqliteBridgeSessionStore;
use crate::types::*;

/// SQLite-backed bridge session store facade for client runtimes.
#[derive(Clone)]
pub struct BridgeSessionStore {
    inner: SqliteBridgeSessionStore,
}

impl BridgeSessionStore {
    pub fn open_default(
        provider_id: impl Into<String>,
        bridge_id: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            inner: SqliteBridgeSessionStore::open_default(provider_id, bridge_id)?,
        })
    }

    pub fn memory(
        provider_id: impl Into<String>,
        bridge_id: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            inner: SqliteBridgeSessionStore::memory(provider_id, bridge_id)?,
        })
    }

    pub fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    pub fn bridge_id(&self) -> &str {
        self.inner.bridge_id()
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        self.inner.create_session(config)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.inner.close_session(session_id)
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.inner.get_messages(session_id, limit)
    }

    pub fn append_message(&self, session_id: &str, message: ChatMessage) -> Result<(), String> {
        self.inner.append_message(session_id, message)
    }

    pub fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        self.inner.list_sessions(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::query::BridgeSessionQuery;

    #[test]
    fn create_and_list_sessions_sorted_by_updated_at() {
        let store = BridgeSessionStore::memory("codex", "bridge.codex").expect("store");
        let first = store
            .create_session(SessionConfig::new("agent.1").with_title("First"))
            .expect("first");
        let _second = store
            .create_session(SessionConfig::new("agent.1").with_title("Second"))
            .expect("second");

        store
            .append_message(
                &first.session_id,
                ChatMessage {
                    id: "msg.1".to_string(),
                    role: MessageRole::User,
                    content: "hello".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    metadata: None,
                },
            )
            .expect("append");

        let listed = store
            .list_sessions(&BridgeSessionQuery::default())
            .expect("listed");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].session_id, first.session_id);
    }
}
