use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};

pub struct OpenClawRuntime {
    config: AgentBridgeConfig,
}

impl OpenClawRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
        })
    }

    pub fn send_message(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err("OpenClaw runtime not implemented".to_string())
    }

    pub fn get_messages(
        &self,
        _session_id: &str,
        _limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        Err("OpenClaw runtime not implemented".to_string())
    }

    pub fn create_session(&self, _config: SessionConfig) -> Result<SessionInfo, String> {
        Err("OpenClaw runtime not implemented".to_string())
    }

    pub fn close_session(&self, _session_id: &str) -> Result<(), String> {
        Err("OpenClaw runtime not implemented".to_string())
    }

    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Unknown,
            message: Some("OpenClaw runtime not implemented".to_string()),
            last_check: chrono::Utc::now(),
        }
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
