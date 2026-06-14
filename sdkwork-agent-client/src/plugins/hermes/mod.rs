mod runtime;

use std::sync::{Arc, Mutex};
use crate::chat::ChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{
    AgentBridgeProvider, AgentBridgeType, AgentBridgeHealth, AgentBridgeStatus,
    AgentBridgeMetadata, AgentBridgeConfig, AgentBridgePlugin,
};
use runtime::HermesRuntime;

pub struct HermesProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<HermesRuntime>>,
    capabilities: Vec<String>,
}

impl HermesProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        let capabilities = vec![
            "chat".to_string(),
            "tool_call".to_string(),
            "memory".to_string(),
        ];
        Ok(Self {
            config,
            runtime: Mutex::new(None),
            capabilities,
        })
    }
}

impl ChatClient for HermesProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }

    fn health(&self) -> Result<bool, String> {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => Ok(rt.health_check().status == AgentBridgeStatus::Healthy),
            None => Ok(false),
        }
    }
}

impl AgentBridgeProvider for HermesProvider {
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
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(HermesRuntime::new(&self.config)?);
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }

    fn health_check(&self) -> AgentBridgeHealth {
        let runtime = self.runtime.lock().unwrap();
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
            description: "Hermes agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities.clone(),
            config_schema: None,
        }
    }
}

pub struct HermesPlugin;

impl HermesPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HermesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBridgePlugin for HermesPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.hermes"
    }

    fn name(&self) -> &str {
        "Hermes Bridge Plugin"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::Hermes]
    }

    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::Hermes => Ok(Arc::new(HermesProvider::new(config)?)),
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
        AgentBridgeConfig::new("test.hermes", AgentBridgeType::Hermes)
    }

    #[test]
    fn plugin_metadata() {
        let plugin = HermesPlugin::new();
        assert_eq!(plugin.plugin_id(), "builtin.hermes");
        assert_eq!(plugin.name(), "Hermes Bridge Plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.supported_bridge_types(), vec![AgentBridgeType::Hermes]);
    }

    #[test]
    fn plugin_default() {
        let plugin = HermesPlugin;
        assert_eq!(plugin.plugin_id(), "builtin.hermes");
    }

    #[test]
    fn plugin_create_provider() {
        let plugin = HermesPlugin::new();
        let config = test_config();
        let provider = plugin
            .create_provider(AgentBridgeType::Hermes, config)
            .expect("create provider");
        assert_eq!(provider.bridge_id(), "test.hermes");
        assert_eq!(provider.bridge_type(), &AgentBridgeType::Hermes);
    }

    #[test]
    fn plugin_create_provider_unsupported_type() {
        let plugin = HermesPlugin::new();
        let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw);
        let result = plugin.create_provider(AgentBridgeType::OpenClaw, config);
        assert!(result.is_err());
    }

    #[test]
    fn plugin_validate_config() {
        let plugin = HermesPlugin::new();
        let config = test_config();
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[test]
    fn provider_capabilities() {
        let provider = HermesProvider::new(test_config()).unwrap();
        let caps = provider.capabilities();
        assert!(caps.contains(&"chat".to_string()));
        assert!(caps.contains(&"tool_call".to_string()));
        assert!(caps.contains(&"memory".to_string()));
    }

    #[test]
    fn provider_metadata() {
        let provider = HermesProvider::new(test_config()).unwrap();
        let meta = provider.metadata();
        assert_eq!(meta.bridge_id, "test.hermes");
        assert_eq!(meta.bridge_type, AgentBridgeType::Hermes);
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.author, "SDKWork");
    }

    #[test]
    fn provider_health_before_init() {
        let provider = HermesProvider::new(test_config()).unwrap();
        let health = provider.health_check();
        assert_eq!(health.status, AgentBridgeStatus::Unknown);
        assert_eq!(health.message.as_deref(), Some("Runtime not initialized"));
        assert!(!provider.health().unwrap());
    }

    #[test]
    fn provider_initialize_and_shutdown() {
        let provider = HermesProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        provider.shutdown().expect("shutdown");
    }

    #[test]
    fn provider_health_after_init() {
        let provider = HermesProvider::new(test_config()).unwrap();
        provider.initialize().expect("init");
        let health = provider.health_check();
        assert_eq!(health.status, AgentBridgeStatus::Unknown);
        assert_eq!(
            health.message.as_deref(),
            Some("Hermes runtime not implemented")
        );
    }

    #[test]
    fn provider_send_message_not_initialized() {
        let provider = HermesProvider::new(test_config()).unwrap();
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
        let provider = HermesProvider::new(test_config()).unwrap();
        let config = SessionConfig::new("agent.1");
        let err = provider.create_session(config).unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }

    #[test]
    fn provider_get_messages_not_initialized() {
        let provider = HermesProvider::new(test_config()).unwrap();
        let err = provider.get_messages("s1", None).unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }

    #[test]
    fn provider_close_session_not_initialized() {
        let provider = HermesProvider::new(test_config()).unwrap();
        let err = provider.close_session("s1").unwrap_err();
        assert_eq!(err, "Runtime not initialized");
    }
}
