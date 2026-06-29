//! Plugin system with lifecycle management for SDKWORK agent runtime.
//!
//! This module implements a complete plugin lifecycle following SDKWORK standards:
//! - Plugin initialization and shutdown
//! - Activation and deactivation
//! - Health checking
//! - Hot reload support
//! - Plugin registry for discovery and management

use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Helper function to handle RwLock poisoning errors
fn handle_lock_error<T>(result: Result<T, std::sync::PoisonError<T>>) -> KernelResult<T> {
    result.map_err(|_| KernelError::Internal {
        message: "Lock poisoned - a thread panicked while holding the lock".to_string(),
    })
}

/// Plugin context providing runtime information and services
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin ID
    pub plugin_id: String,
    /// Plugin version
    pub version: String,
    /// Configuration directory path
    pub config_dir: String,
    /// Data directory path
    pub data_dir: String,
    /// Runtime identifier
    pub runtime_id: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(
        plugin_id: impl Into<String>,
        version: impl Into<String>,
        runtime_id: impl Into<String>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            version: version.into(),
            config_dir: String::new(),
            data_dir: String::new(),
            runtime_id: runtime_id.into(),
            environment: "development".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Set configuration directory
    pub fn with_config_dir(mut self, dir: impl Into<String>) -> Self {
        self.config_dir = dir.into();
        self
    }

    /// Set data directory
    pub fn with_data_dir(mut self, dir: impl Into<String>) -> Self {
        self.data_dir = dir.into();
        self
    }

    /// Set environment
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is loaded but not initialized
    Loaded,
    /// Plugin is initializing
    Initializing,
    /// Plugin is initialized and ready
    Initialized,
    /// Plugin is activating
    Activating,
    /// Plugin is active and running
    Active,
    /// Plugin is deactivating
    Deactivating,
    /// Plugin is deactivated
    Deactivated,
    /// Plugin is shutting down
    ShuttingDown,
    /// Plugin is shut down
    Shutdown,
    /// Plugin encountered an error
    Error,
    /// Plugin is reloading
    Reloading,
}

impl PluginState {
    /// Check if plugin is in a runnable state
    pub fn is_runnable(&self) -> bool {
        matches!(self, PluginState::Active)
    }

    /// Check if plugin can be activated
    pub fn can_activate(&self) -> bool {
        matches!(self, PluginState::Initialized | PluginState::Deactivated)
    }

    /// Check if plugin can be deactivated
    pub fn can_deactivate(&self) -> bool {
        matches!(self, PluginState::Active)
    }

    /// Check if plugin can be reloaded
    pub fn can_reload(&self) -> bool {
        matches!(
            self,
            PluginState::Active | PluginState::Initialized | PluginState::Deactivated
        )
    }

    /// Get state as string
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginState::Loaded => "loaded",
            PluginState::Initializing => "initializing",
            PluginState::Initialized => "initialized",
            PluginState::Activating => "activating",
            PluginState::Active => "active",
            PluginState::Deactivating => "deactivating",
            PluginState::Deactivated => "deactivated",
            PluginState::ShuttingDown => "shutting_down",
            PluginState::Shutdown => "shutdown",
            PluginState::Error => "error",
            PluginState::Reloading => "reloading",
        }
    }
}

/// Plugin trait with full lifecycle management
///
/// All plugins must implement this trait to be managed by the plugin registry.
pub trait Plugin: Send + Sync {
    /// Get plugin manifest
    fn manifest(&self) -> ProviderManifest;

    /// Initialize plugin
    ///
    /// Called once when the plugin is first loaded.
    /// Plugins should perform one-time setup here (e.g., establish connections, load config).
    fn initialize(&mut self, context: &PluginContext) -> KernelResult<()>;

    /// Shutdown plugin
    ///
    /// Called when the plugin is being permanently unloaded.
    /// Plugins should release all resources here.
    fn shutdown(&mut self) -> KernelResult<()>;

    /// Activate plugin
    ///
    /// Called to make the plugin active and ready to handle requests.
    /// This is called after initialize or after deactivate.
    fn activate(&mut self) -> KernelResult<()>;

    /// Deactivate plugin
    ///
    /// Called to temporarily deactivate the plugin.
    /// The plugin may be activated again later.
    fn deactivate(&mut self) -> KernelResult<()>;

    /// Health check
    ///
    /// Return current health status of the plugin.
    fn health_check(&self) -> ProviderHealth;

    /// Reload plugin
    ///
    /// Called to reload plugin configuration or state without full shutdown.
    fn reload(&mut self) -> KernelResult<()> {
        // Default: no-op
        Ok(())
    }

    /// Get current state
    fn state(&self) -> PluginState;

    /// Get plugin ID
    fn plugin_id(&self) -> &str;
}

/// Plugin metadata for registry
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub plugin_id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
}

impl PluginMetadata {
    pub fn new(
        plugin_id: impl Into<String>,
        version: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            version: version.into(),
            name: name.into(),
            description: String::new(),
            author: String::new(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

/// Plugin entry in the registry
struct PluginEntry {
    plugin: Arc<RwLock<Box<dyn Plugin>>>,
    metadata: PluginMetadata,
    context: PluginContext,
}

/// Plugin registry for managing multiple plugins
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, PluginEntry>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin
    pub fn register(
        &self,
        plugin: Box<dyn Plugin>,
        metadata: PluginMetadata,
        context: PluginContext,
    ) -> KernelResult<()> {
        let plugin_id = plugin.plugin_id().to_string();

        let entry = PluginEntry {
            plugin: Arc::new(RwLock::new(plugin)),
            metadata,
            context,
        };

        let mut plugins = self.plugins.write().map_err(|_| {
            KernelError::Internal {
                message: "Plugin registry lock poisoned - a thread panicked while holding the lock".to_string(),
            }
        })?;
        if plugins.contains_key(&plugin_id) {
            return Err(KernelError::validation(format!(
                "Plugin '{}' is already registered",
                plugin_id
            )));
        }

        plugins.insert(plugin_id.clone(), entry);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&self, plugin_id: &str) -> KernelResult<()> {
        let mut plugins = self.plugins.write().map_err(|_| {
            KernelError::Internal {
                message: "Plugin registry lock poisoned - a thread panicked while holding the lock".to_string(),
            }
        })?;

        if let Some(entry) = plugins.remove(plugin_id) {
            // Shutdown plugin before unregistering
            let mut plugin = handle_lock_error(entry.plugin.write())?;
            plugin.shutdown()?;
        }

        Ok(())
    }

    /// Initialize a plugin
    pub fn initialize(&self, plugin_id: &str) -> KernelResult<()> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let mut plugin = handle_lock_error(entry.plugin.write())?;
        plugin.initialize(&entry.context)?;

        Ok(())
    }

    /// Initialize all plugins
    pub fn initialize_all(&self) -> KernelResult<Vec<String>> {
        let plugins = handle_lock_error(self.plugins.read())?;
        let mut initialized = Vec::new();
        let mut errors = Vec::new();

        for (plugin_id, entry) in plugins.iter() {
            let mut plugin = handle_lock_error(entry.plugin.write())?;
            match plugin.initialize(&entry.context) {
                Ok(()) => initialized.push(plugin_id.clone()),
                Err(e) => errors.push(format!("{}: {}", plugin_id, e)),
            }
        }

        if !errors.is_empty() {
            return Err(KernelError::Internal {
                message: format!("Initialization errors: {}", errors.join(", ")),
            });
        }

        Ok(initialized)
    }

    /// Activate a plugin
    pub fn activate(&self, plugin_id: &str) -> KernelResult<()> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let mut plugin = handle_lock_error(entry.plugin.write())?;

        if !plugin.state().can_activate() {
            return Err(KernelError::validation(format!(
                "Plugin '{}' cannot be activated in state {:?}",
                plugin_id,
                plugin.state()
            )));
        }

        plugin.activate()?;
        Ok(())
    }

    /// Activate all initialized plugins
    pub fn activate_all(&self) -> KernelResult<Vec<String>> {
        let plugins = handle_lock_error(self.plugins.read())?;
        let mut activated = Vec::new();

        for (plugin_id, entry) in plugins.iter() {
            let mut plugin = handle_lock_error(entry.plugin.write())?;
            if plugin.state().can_activate() {
                plugin.activate()?;
                activated.push(plugin_id.clone());
            }
        }

        Ok(activated)
    }

    /// Deactivate a plugin
    pub fn deactivate(&self, plugin_id: &str) -> KernelResult<()> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let mut plugin = handle_lock_error(entry.plugin.write())?;

        if !plugin.state().can_deactivate() {
            return Err(KernelError::validation(format!(
                "Plugin '{}' cannot be deactivated in state {:?}",
                plugin_id,
                plugin.state()
            )));
        }

        plugin.deactivate()?;
        Ok(())
    }

    /// Reload a plugin
    pub fn reload(&self, plugin_id: &str) -> KernelResult<()> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let mut plugin = handle_lock_error(entry.plugin.write())?;

        if !plugin.state().can_reload() {
            return Err(KernelError::validation(format!(
                "Plugin '{}' cannot be reloaded in state {:?}",
                plugin_id,
                plugin.state()
            )));
        }

        plugin.reload()?;
        Ok(())
    }

    /// Get plugin health
    pub fn health(&self, plugin_id: &str) -> KernelResult<ProviderHealth> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let plugin = handle_lock_error(entry.plugin.read())?;
        Ok(plugin.health_check())
    }

    /// Get all plugin healths
    pub fn health_all(&self) -> KernelResult<HashMap<String, ProviderHealth>> {
        let plugins = handle_lock_error(self.plugins.read())?;
        let mut healths = HashMap::new();

        for (plugin_id, entry) in plugins.iter() {
            let plugin = handle_lock_error(entry.plugin.read())?;
            healths.insert(plugin_id.clone(), plugin.health_check());
        }

        Ok(healths)
    }

    /// Get plugin state
    pub fn state(&self, plugin_id: &str) -> KernelResult<PluginState> {
        let plugins = handle_lock_error(self.plugins.read())?;

        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| KernelError::validation(format!("Plugin '{}' not found", plugin_id)))?;

        let plugin = handle_lock_error(entry.plugin.read())?;
        Ok(plugin.state())
    }

    /// List all registered plugins
    pub fn list(&self) -> KernelResult<Vec<PluginMetadata>> {
        let plugins = handle_lock_error(self.plugins.read())?;
        Ok(plugins.values().map(|entry| entry.metadata.clone()).collect())
    }

    /// List active plugins
    pub fn list_active(&self) -> KernelResult<Vec<String>> {
        let plugins = handle_lock_error(self.plugins.read())?;
        let mut active = Vec::new();

        for (plugin_id, entry) in plugins.iter() {
            let plugin = handle_lock_error(entry.plugin.read())?;
            if plugin.state().is_runnable() {
                active.push(plugin_id.clone());
            }
        }

        Ok(active)
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&self) -> KernelResult<()> {
        let plugins = handle_lock_error(self.plugins.read())?;
        let mut errors = Vec::new();

        for (plugin_id, entry) in plugins.iter() {
            let mut plugin = handle_lock_error(entry.plugin.write())?;
            if let Err(e) = plugin.shutdown() {
                errors.push(format!("{}: {}", plugin_id, e));
            }
        }

        if !errors.is_empty() {
            return Err(KernelError::Internal {
                message: format!("Shutdown errors: {}", errors.join(", ")),
            });
        }

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        state: PluginState,
        plugin_id: String,
    }

    impl TestPlugin {
        fn new(plugin_id: impl Into<String>) -> Self {
            Self {
                state: PluginState::Loaded,
                plugin_id: plugin_id.into(),
            }
        }
    }

    impl Plugin for TestPlugin {
        fn manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                &self.plugin_id,
                "test",
                "test-plugin",
                "1.0.0",
                vec!["test.capability".to_string()],
            )
        }

        fn initialize(&mut self, _context: &PluginContext) -> KernelResult<()> {
            self.state = PluginState::Initialized;
            Ok(())
        }

        fn shutdown(&mut self) -> KernelResult<()> {
            self.state = PluginState::Shutdown;
            Ok(())
        }

        fn activate(&mut self) -> KernelResult<()> {
            self.state = PluginState::Active;
            Ok(())
        }

        fn deactivate(&mut self) -> KernelResult<()> {
            self.state = PluginState::Deactivated;
            Ok(())
        }

        fn health_check(&self) -> ProviderHealth {
            if self.state.is_runnable() {
                ProviderHealth::available()
            } else {
                ProviderHealth::unavailable("not active")
            }
        }

        fn state(&self) -> PluginState {
            self.state
        }

        fn plugin_id(&self) -> &str {
            &self.plugin_id
        }
    }

    #[test]
    fn plugin_context_builder() {
        let ctx = PluginContext::new("plugin-1", "1.0.0", "runtime-1")
            .with_config_dir("/config")
            .with_data_dir("/data")
            .with_environment("production")
            .with_metadata("key1", "value1");

        assert_eq!(ctx.plugin_id, "plugin-1");
        assert_eq!(ctx.version, "1.0.0");
        assert_eq!(ctx.config_dir, "/config");
        assert_eq!(ctx.data_dir, "/data");
        assert_eq!(ctx.environment, "production");
        assert_eq!(ctx.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn plugin_state_transitions() {
        assert!(PluginState::Initialized.can_activate());
        assert!(!PluginState::Active.can_activate());

        assert!(PluginState::Active.can_deactivate());
        assert!(!PluginState::Initialized.can_deactivate());

        assert!(PluginState::Active.can_reload());
        assert!(PluginState::Initialized.can_reload());
        assert!(!PluginState::Loaded.can_reload());

        assert!(PluginState::Active.is_runnable());
        assert!(!PluginState::Initialized.is_runnable());
    }

    #[test]
    fn plugin_metadata_builder() {
        let meta = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin")
            .with_description("A test plugin")
            .with_author("Test Author")
            .with_capabilities(vec!["cap1".to_string(), "cap2".to_string()])
            .with_dependencies(vec!["dep1".to_string()]);

        assert_eq!(meta.plugin_id, "plugin-1");
        assert_eq!(meta.name, "Test Plugin");
        assert_eq!(meta.description, "A test plugin");
        assert_eq!(meta.author, "Test Author");
        assert_eq!(meta.capabilities.len(), 2);
        assert_eq!(meta.dependencies.len(), 1);
    }

    #[test]
    fn plugin_registry_register() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("plugin-1"));
        let metadata = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin");
        let context = PluginContext::new("plugin-1", "1.0.0", "runtime-1");

        assert!(registry.register(plugin, metadata, context).is_ok());
        assert_eq!(registry.list().unwrap().len(), 1);
    }

    #[test]
    fn plugin_registry_initialize() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("plugin-1"));
        let metadata = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin");
        let context = PluginContext::new("plugin-1", "1.0.0", "runtime-1");

        registry.register(plugin, metadata, context).unwrap();
        registry.initialize("plugin-1").unwrap();

        let state = registry.state("plugin-1").unwrap();
        assert_eq!(state, PluginState::Initialized);
    }

    #[test]
    fn plugin_registry_activate() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("plugin-1"));
        let metadata = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin");
        let context = PluginContext::new("plugin-1", "1.0.0", "runtime-1");

        registry.register(plugin, metadata, context).unwrap();
        registry.initialize("plugin-1").unwrap();
        registry.activate("plugin-1").unwrap();

        let state = registry.state("plugin-1").unwrap();
        assert_eq!(state, PluginState::Active);

        let active = registry.list_active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "plugin-1");
    }

    #[test]
    fn plugin_registry_health() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("plugin-1"));
        let metadata = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin");
        let context = PluginContext::new("plugin-1", "1.0.0", "runtime-1");

        registry.register(plugin, metadata, context).unwrap();
        registry.initialize("plugin-1").unwrap();
        registry.activate("plugin-1").unwrap();

        let health = registry.health("plugin-1").unwrap();
        assert!(health.is_available());
    }

    #[test]
    fn plugin_registry_deactivate() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("plugin-1"));
        let metadata = PluginMetadata::new("plugin-1", "1.0.0", "Test Plugin");
        let context = PluginContext::new("plugin-1", "1.0.0", "runtime-1");

        registry.register(plugin, metadata, context).unwrap();
        registry.initialize("plugin-1").unwrap();
        registry.activate("plugin-1").unwrap();
        registry.deactivate("plugin-1").unwrap();

        let state = registry.state("plugin-1").unwrap();
        assert_eq!(state, PluginState::Deactivated);

        let active = registry.list_active().unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn plugin_registry_shutdown_all() {
        let registry = PluginRegistry::new();

        for i in 1..=3 {
            let plugin = Box::new(TestPlugin::new(format!("plugin-{}", i)));
            let metadata = PluginMetadata::new(format!("plugin-{}", i), "1.0.0", "Test Plugin");
            let context = PluginContext::new(format!("plugin-{}", i), "1.0.0", "runtime-1");

            registry.register(plugin, metadata, context).unwrap();
        }

        registry.initialize_all().unwrap();
        registry.shutdown_all().unwrap();

        for i in 1..=3 {
            let state = registry.state(&format!("plugin-{}", i)).unwrap();
            assert_eq!(state, PluginState::Shutdown);
        }
    }
}
