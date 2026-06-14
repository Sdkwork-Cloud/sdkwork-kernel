use crate::chat::ChatClient;
use super::types::{AgentBridgeType, AgentBridgeHealth, AgentBridgeMetadata};

/// Unified Agent Bridge Provider trait
/// Inherits from ChatClient — the ChatClient implementation IS the bridge logic
pub trait AgentBridgeProvider: ChatClient + Send + Sync {
    /// Return bridge provider identifier
    fn bridge_id(&self) -> &str;

    /// Return bridge provider type
    fn bridge_type(&self) -> &AgentBridgeType;

    /// Return bridge provider version
    fn version(&self) -> &str;

    /// Return supported capabilities
    fn capabilities(&self) -> &[String];

    /// Check if specified capability is supported
    fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities().iter().any(|c| c == capability)
    }

    /// Initialize bridge provider (uses interior mutability)
    fn initialize(&self) -> Result<(), String>;

    /// Shutdown bridge provider (uses interior mutability)
    fn shutdown(&self) -> Result<(), String>;

    /// Health check
    fn health_check(&self) -> AgentBridgeHealth;

    /// Get provider metadata
    fn metadata(&self) -> AgentBridgeMetadata;
}
