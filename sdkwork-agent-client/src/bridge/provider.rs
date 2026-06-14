use crate::chat::ChatClient;
use super::types::{AgentBridgeType, AgentBridgeHealth, AgentBridgeMetadata};

pub trait AgentBridgeProvider: ChatClient {
    fn bridge_id(&self) -> &str;

    fn bridge_type(&self) -> AgentBridgeType;

    fn version(&self) -> &str;

    fn capabilities(&self) -> Vec<String>;

    fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities().iter().any(|c| c == capability)
    }

    fn initialize(&self) -> Result<(), String>;

    fn shutdown(&self) -> Result<(), String>;

    fn health_check(&self) -> AgentBridgeHealth;

    fn metadata(&self) -> AgentBridgeMetadata;
}
