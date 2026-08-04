mod runtime;

use crate::bridge::{
    AgentBridgeConfig, AgentBridgeHealth, AgentBridgeMetadata, AgentBridgePlugin,
    AgentBridgeProvider, AgentBridgeStatus, AgentBridgeType,
};
use crate::chat::ChatClient;
use crate::runtime_guard::lock_runtime_mutex;
use crate::session::BridgeSessionQuery;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};
use runtime::CodexRuntime;
use std::sync::{Arc, Mutex};

pub struct CodexProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<CodexRuntime>>,
    capabilities: Vec<String>,
}

impl CodexProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        let capabilities = vec![
            "chat".to_string(),
            "tool_call".to_string(),
            "session_history".to_string(),
        ];
        Ok(Self {
            config,
            runtime: Mutex::new(None),
            capabilities,
        })
    }
}

impl ChatClient for CodexProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }

    fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.list_sessions(query)
    }

    fn health(&self) -> Result<bool, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        match runtime.as_ref() {
            Some(rt) => Ok(rt.health_check().status == AgentBridgeStatus::Healthy),
            None => Ok(false),
        }
    }
}

impl AgentBridgeProvider for CodexProvider {
    fn bridge_id(&self) -> &str {
        &self.config.bridge_id
    }

    fn bridge_type(&self) -> &AgentBridgeType {
        &self.config.bridge_type
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn initialize(&self) -> Result<(), String> {
        let mut runtime = lock_runtime_mutex(&self.runtime)?;
        if runtime.is_none() {
            *runtime = Some(CodexRuntime::new(&self.config)?);
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), String> {
        let mut runtime = lock_runtime_mutex(&self.runtime)?;
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }

    fn health_check(&self) -> AgentBridgeHealth {
        let runtime = match lock_runtime_mutex(&self.runtime) {
            Ok(runtime) => runtime,
            Err(message) => {
                return AgentBridgeHealth {
                    status: AgentBridgeStatus::Unknown,
                    message: Some(message),
                    last_check: chrono::Utc::now(),
                };
            }
        };
        match runtime.as_ref() {
            Some(rt) => rt.health_check(),
            None => AgentBridgeHealth {
                status: AgentBridgeStatus::Unknown,
                message: Some("Runtime not initialized".to_string()),
                last_check: chrono::Utc::now(),
            },
        }
    }

    fn metadata(&self) -> AgentBridgeMetadata {
        AgentBridgeMetadata {
            bridge_id: self.config.bridge_id.clone(),
            bridge_type: self.config.bridge_type.clone(),
            version: "1.0.0".to_string(),
            description: "Codex agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities.clone(),
            config_schema: None,
        }
    }
}

pub struct CodexPlugin;

impl CodexPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBridgePlugin for CodexPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.codex"
    }

    fn name(&self) -> &str {
        "Codex Bridge Plugin"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::Codex]
    }

    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::Codex => Ok(Arc::new(CodexProvider::new(config)?)),
            _ => Err(format!("Unsupported bridge type: {:?}", bridge_type)),
        }
    }

    fn config_schema(&self) -> Option<String> {
        None
    }

    fn validate_config(&self, _config: &AgentBridgeConfig) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AgentBridgeConfig {
        AgentBridgeConfig::new("test.codex", AgentBridgeType::Codex)
    }

    #[test]
    #[ignore = "live test: requires a real Codex app-server install and OpenAI credentials"]
    fn provider_session_history_roundtrip() {
        // The Codex bridge drives the Rust-native codex app-server worker
        // (unlike TypeScript/Python runtimes there is no mockable sdk_probe
        // backend), so this test needs a working Codex install and
        // credentials. Gated as a live test.
        let provider = CodexProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let session = provider
            .create_session(SessionConfig::new("agent.1").with_title("Codex"))
            .expect("session");
        provider
            .send_message(ChatRequest {
                session_id: session.session_id.clone(),
                content: "hello codex".to_string(),
                model: None,
                stream: false,
            })
            .expect("send");
        let messages = provider
            .get_messages(&session.session_id, None)
            .expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, crate::types::MessageRole::User);
        assert!(
            !messages[1].content.contains("received:"),
            "stub echo must not be returned: {}",
            messages[1].content
        );
    }

    #[test]
    #[ignore = "live test: requires a real Codex app-server install and OpenAI credentials"]
    fn provider_send_message_routes_through_sdk_model_provider() {
        // Same live constraint as `provider_session_history_roundtrip`: the
        // Rust-native codex app-server worker is a real client, so this test
        // needs a working Codex install and credentials.
        let provider = CodexProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let session = provider
            .create_session(SessionConfig::new("agent.codex"))
            .expect("create session");
        let response = provider
            .send_message(ChatRequest {
                session_id: session.session_id,
                content: "hello".to_string(),
                model: None,
                stream: false,
            })
            .expect("send message");
        assert_eq!(response.status, crate::types::ChatStatus::Completed);
        assert!(
            !response.content.contains("received:"),
            "stub echo response must not be returned: {}",
            response.content
        );
        assert!(
            !response.content.contains("Mock response"),
            "direct mock fallback must not be returned in default fail-closed profile: {}",
            response.content
        );
    }

    #[test]
    fn provider_health_after_init() {
        let provider = CodexProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let health = provider.health_check();
        assert!(
            matches!(
                health.status,
                AgentBridgeStatus::Healthy | AgentBridgeStatus::Degraded
            ),
            "unexpected health status: {:?}",
            health.status
        );
    }
}
