mod runtime;

use crate::bridge::{
    AgentBridgeConfig, AgentBridgeHealth, AgentBridgeMetadata, AgentBridgePlugin,
    AgentBridgeProvider, AgentBridgeStatus, AgentBridgeType,
};
use crate::chat::ChatClient;
use crate::runtime_guard::lock_runtime_mutex;
use crate::session::BridgeSessionQuery;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};
use runtime::ZeroCloudRuntime;
use std::sync::{Arc, Mutex};

pub struct ZeroCloudProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<ZeroCloudRuntime>>,
    capabilities: Vec<String>,
}

impl ZeroCloudProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        let capabilities = vec!["chat".to_string(), "tool_call".to_string()];
        Ok(Self {
            config,
            runtime: Mutex::new(None),
            capabilities,
        })
    }
}

impl ChatClient for ZeroCloudProvider {
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

impl AgentBridgeProvider for ZeroCloudProvider {
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
            *runtime = Some(ZeroCloudRuntime::new(&self.config)?);
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
            description: "ZeroCloud agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities.clone(),
            config_schema: None,
        }
    }
}

pub struct ZeroCloudPlugin;

impl ZeroCloudPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZeroCloudPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBridgePlugin for ZeroCloudPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.zeroclaw"
    }

    fn name(&self) -> &str {
        "ZeroCloud Bridge Plugin"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::ZeroCloud]
    }

    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::ZeroCloud => Ok(Arc::new(ZeroCloudProvider::new(config)?)),
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
        AgentBridgeConfig::new("test.zeroclaw", AgentBridgeType::ZeroCloud)
    }

    #[test]
    fn plugin_metadata() {
        let plugin = ZeroCloudPlugin::new();
        assert_eq!(plugin.plugin_id(), "builtin.zeroclaw");
        assert_eq!(plugin.name(), "ZeroCloud Bridge Plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(
            plugin.supported_bridge_types(),
            vec![AgentBridgeType::ZeroCloud]
        );
    }

    #[test]
    fn plugin_default() {
        let plugin = ZeroCloudPlugin;
        assert_eq!(plugin.plugin_id(), "builtin.zeroclaw");
    }

    #[test]
    fn plugin_create_provider() {
        let plugin = ZeroCloudPlugin::new();
        let config = test_config();
        let provider = plugin
            .create_provider(AgentBridgeType::ZeroCloud, config)
            .expect("create provider");
        assert_eq!(provider.bridge_id(), "test.zeroclaw");
        assert_eq!(provider.bridge_type(), &AgentBridgeType::ZeroCloud);
    }

    #[test]
    fn plugin_create_provider_unsupported_type() {
        let plugin = ZeroCloudPlugin::new();
        let config = AgentBridgeConfig::new("test", AgentBridgeType::Hermes);
        let result = plugin.create_provider(AgentBridgeType::Hermes, config);
        assert!(result.is_err());
    }

    #[test]
    fn plugin_validate_config() {
        let plugin = ZeroCloudPlugin::new();
        let config = test_config();
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[test]
    fn provider_capabilities() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let caps = provider.capabilities();
        assert!(caps.contains(&"chat".to_string()));
        assert!(caps.contains(&"tool_call".to_string()));
    }

    #[test]
    fn provider_metadata() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let meta = provider.metadata();
        assert_eq!(meta.bridge_id, "test.zeroclaw");
        assert_eq!(meta.bridge_type, AgentBridgeType::ZeroCloud);
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.author, "SDKWork");
    }

    #[test]
    fn provider_health_before_init() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let health = provider.health_check();
        assert_eq!(health.status, AgentBridgeStatus::Unknown);
        assert_eq!(health.message.as_deref(), Some("Runtime not initialized"));
        assert!(!provider.health().unwrap());
    }

    #[test]
    fn provider_initialize_and_shutdown() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        provider.shutdown().expect("shutdown");
    }

    #[test]
    fn provider_health_after_init() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let health = provider.health_check();
        assert_eq!(health.status, AgentBridgeStatus::Degraded);
        assert!(health
            .message
            .as_deref()
            .unwrap()
            .contains("not SDK-backed"));
    }

    #[test]
    fn provider_send_message_is_fail_closed_after_init() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let session = provider
            .create_session(SessionConfig::new("agent.1"))
            .expect("session");
        let err = provider
            .send_message(ChatRequest {
                session_id: session.session_id,
                content: "hello".to_string(),
                model: None,
                stream: false,
            })
            .unwrap_err();
        assert!(err.contains("not SDK-backed"));
    }

    #[test]
    fn provider_send_message_not_initialized() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let request = ChatRequest {
            session_id: "s1".to_string(),
            content: "hello".to_string(),
            model: None,
            stream: false,
        };
        let err = provider.send_message(request).unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }

    #[test]
    fn provider_create_session_not_initialized() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let config = SessionConfig::new("agent.1");
        let err = provider.create_session(config).unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }

    #[test]
    fn provider_get_messages_not_initialized() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let err = provider.get_messages("s1", None).unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }

    #[test]
    fn provider_close_session_not_initialized() {
        let provider = ZeroCloudProvider::new(test_config()).unwrap();
        let err = provider.close_session("s1").unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }
}
