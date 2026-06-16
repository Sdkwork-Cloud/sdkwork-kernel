use super::plugin::AgentBridgePlugin;
use super::provider::AgentBridgeProvider;
use super::types::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeType};
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin registry
pub struct AgentBridgePluginRegistry {
    plugins: HashMap<String, Arc<dyn AgentBridgePlugin>>,
    providers: HashMap<String, Arc<dyn AgentBridgeProvider>>,
}

impl AgentBridgePluginRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            providers: HashMap::new(),
        }
    }

    /// Register plugin
    pub fn register_plugin(&mut self, plugin: Arc<dyn AgentBridgePlugin>) -> Result<(), String> {
        let plugin_id = plugin.plugin_id().to_string();
        if self.plugins.contains_key(&plugin_id) {
            return Err(format!("Plugin already registered: {}", plugin_id));
        }
        self.plugins.insert(plugin_id, plugin);
        Ok(())
    }

    /// Unregister plugin
    pub fn unregister_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(format!("Plugin not found: {}", plugin_id));
        }
        self.plugins.remove(plugin_id);
        Ok(())
    }

    /// Create provider instance
    pub fn create_provider(
        &mut self,
        plugin_id: &str,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<String, String> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

        // Validate config before creating provider
        plugin.validate_config(&config)?;

        let bridge_id = config.bridge_id.clone();

        // Check for duplicate bridge_id
        if self.providers.contains_key(&bridge_id) {
            return Err(format!("Provider already exists: {}", bridge_id));
        }

        let provider = plugin.create_provider(bridge_type, config)?;
        self.providers.insert(bridge_id.clone(), provider);
        Ok(bridge_id)
    }

    /// Get provider instance
    pub fn get_provider(&self, bridge_id: &str) -> Option<Arc<dyn AgentBridgeProvider>> {
        self.providers.get(bridge_id).cloned()
    }

    /// Remove provider instance
    pub fn remove_provider(
        &mut self,
        bridge_id: &str,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        self.providers
            .remove(bridge_id)
            .ok_or_else(|| format!("Provider not found: {}", bridge_id))
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<Arc<dyn AgentBridgePlugin>> {
        self.plugins.values().cloned().collect()
    }

    /// List all created providers
    pub fn list_providers(&self) -> Vec<Arc<dyn AgentBridgeProvider>> {
        self.providers.values().cloned().collect()
    }

    /// Health check all providers
    pub fn health_check_all(&self) -> HashMap<String, AgentBridgeHealth> {
        self.providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.health_check()))
            .collect()
    }
}

impl Default for AgentBridgePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
