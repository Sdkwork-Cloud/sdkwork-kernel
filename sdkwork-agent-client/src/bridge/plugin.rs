use std::sync::Arc;
use super::types::{AgentBridgeType, AgentBridgeConfig};
use super::provider::AgentBridgeProvider;

/// Agent Bridge Plugin trait
/// Supports runtime dynamic loading via dynamic libraries
pub trait AgentBridgePlugin: Send + Sync {
    /// Return plugin unique identifier
    fn plugin_id(&self) -> &str;

    /// Return plugin name
    fn name(&self) -> &str;

    /// Return plugin version
    fn version(&self) -> &str;

    /// Return supported bridge types
    fn supported_bridge_types(&self) -> Vec<AgentBridgeType>;

    /// Create bridge provider instance
    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String>;

    /// Return plugin configuration schema
    fn config_schema(&self) -> Option<String>;

    /// Validate configuration
    fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String>;
}
