use crate::bridge::AgentBridgeHealth;
use crate::session::{BridgeSessionQuery, BridgeSessionStore};
use crate::types::*;

/// Shared bridge runtime that persists sessions and conversation history in SQLite.
pub struct BridgeSessionRuntime {
    store: BridgeSessionStore,
    provider_label: String,
}

impl BridgeSessionRuntime {
    pub fn new(
        provider_id: &str,
        bridge_id: &str,
        provider_label: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::open_default(provider_id, bridge_id)?,
            provider_label: provider_label.to_string(),
        })
    }

    pub fn memory(
        provider_id: &str,
        bridge_id: &str,
        provider_label: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::memory(provider_id, bridge_id)?,
            provider_label: provider_label.to_string(),
        })
    }

    pub fn session_store(&self) -> &BridgeSessionStore {
        &self.store
    }

    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let user_message = ChatMessage {
            id: format!("msg.{}", uuid_simple()),
            role: MessageRole::User,
            content: request.content.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        };
        self.store.append_message(&request.session_id, user_message)?;

        let assistant_content = format!(
            "[{}] {}",
            self.provider_label,
            if request.content.is_empty() {
                "ready".to_string()
            } else {
                format!("received: {}", request.content)
            }
        );
        let assistant_message = ChatMessage {
            id: format!("msg.{}", uuid_simple()),
            role: MessageRole::Assistant,
            content: assistant_content.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        };
        self.store
            .append_message(&request.session_id, assistant_message)?;

        Ok(ChatResponse {
            message_id: format!("msg.{}", uuid_simple()),
            session_id: request.session_id,
            content: assistant_content,
            status: ChatStatus::Completed,
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            }),
        })
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
        AgentBridgeHealth::healthy()
    }
}

fn uuid_simple() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
