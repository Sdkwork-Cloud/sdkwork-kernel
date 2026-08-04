use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};
use crate::session::BridgeSessionRuntime;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};

const ZEROCLOUD_LOCAL_CHAT_UNAVAILABLE: &str = "ZeroCloud local bridge is not SDK-backed yet; use AgentClientMode::Remote or select openclaw, hermes, or codex";

/// ZeroCloud runtime handle: session persistence only until an upstream adapter exists.
pub struct ZeroCloudRuntime {
    inner: BridgeSessionRuntime,
}

impl ZeroCloudRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            inner: BridgeSessionRuntime::new("zeroclaw", &config.bridge_id)?,
        })
    }

    pub fn send_message(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err(ZEROCLOUD_LOCAL_CHAT_UNAVAILABLE.to_string())
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.inner.get_messages(session_id, limit)
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        self.inner.create_session(config)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.inner.close_session(session_id)
    }

    pub fn list_sessions(
        &self,
        query: &crate::session::BridgeSessionQuery,
    ) -> Result<Vec<SessionInfo>, String> {
        self.inner.list_sessions(query)
    }

    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Degraded,
            message: Some(ZEROCLOUD_LOCAL_CHAT_UNAVAILABLE.to_string()),
            last_check: chrono::Utc::now(),
        }
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
