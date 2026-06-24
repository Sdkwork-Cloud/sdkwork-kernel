use crate::bridge::{AgentBridgeHealth, AgentBridgeStatus};
use crate::session::{BridgeSessionQuery, BridgeSessionStore};
use crate::types::*;

/// SQLite session store helper. Local chat must use [`SdkModelBridgeRuntime`] or Remote internal-api.
pub struct BridgeSessionRuntime {
    store: BridgeSessionStore,
}

impl BridgeSessionRuntime {
    pub fn new(provider_id: &str, bridge_id: &str) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::open_default(provider_id, bridge_id)?,
        })
    }

    pub fn memory(provider_id: &str, bridge_id: &str) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::memory(provider_id, bridge_id)?,
        })
    }

    pub fn session_store(&self) -> &BridgeSessionStore {
        &self.store
    }

    pub fn send_message(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err(
            "BridgeSessionRuntime is session-store only; use SdkModelBridgeRuntime or AgentClientMode::Remote"
                .to_string(),
        )
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.store.get_messages(session_id, limit)
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        self.store.create_session(config)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.store.close_session(session_id)
    }

    pub fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        self.store.list_sessions(query)
    }

    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Degraded,
            message: Some(
                "BridgeSessionRuntime provides session persistence only; chat is unavailable"
                    .to_string(),
            ),
            last_check: chrono::Utc::now(),
        }
    }
}
