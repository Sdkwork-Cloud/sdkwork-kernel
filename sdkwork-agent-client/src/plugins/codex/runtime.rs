use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth};
use crate::session::BridgeSessionRuntime;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};

pub struct CodexRuntime {
    inner: BridgeSessionRuntime,
}

impl CodexRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            inner: BridgeSessionRuntime::new("codex", &config.bridge_id, "Codex")?,
        })
    }

    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        self.inner.send_message(request)
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
        self.inner.health_check()
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
