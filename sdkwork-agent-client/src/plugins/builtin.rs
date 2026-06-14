use std::sync::Arc;
use crate::bridge::{AgentBridgePlugin, AgentBridgePluginRegistry};
use super::{OpenClawPlugin, ZeroClawPlugin, HermesPlugin};

pub struct BuiltinPlugins {
    plugins: Vec<Arc<dyn AgentBridgePlugin>>,
}

impl BuiltinPlugins {
    pub fn create_all() -> Self {
        Self {
            plugins: vec![
                Arc::new(OpenClawPlugin::new()),
                Arc::new(ZeroClawPlugin::new()),
                Arc::new(HermesPlugin::new()),
            ],
        }
    }

    pub fn plugins(&self) -> &[Arc<dyn AgentBridgePlugin>] {
        &self.plugins
    }

    pub fn register_all(&self, registry: &mut AgentBridgePluginRegistry) -> Result<(), String> {
        for plugin in &self.plugins {
            registry.register_plugin(plugin.clone())?;
        }
        Ok(())
    }
}
