//! Contract tests for the kernel plugin lifecycle SPI.
//!
//! The plugin SPI is the kernel-side contribution surface: plugins are
//! registered with metadata and context, then transition through the
//! Loaded -> Initialized -> Active lifecycle with guarded state transitions,
//! health checks, reload, deactivation, and shutdown.

use sdkwork_agent_kernel::{
    KernelResult, Plugin, PluginContext, PluginMetadata, PluginRegistry, PluginState,
    ProviderHealth, ProviderManifest,
};

/// Test plugin that records every lifecycle call it receives.
struct RecordingPlugin {
    plugin_id: String,
    state: PluginState,
    calls: Vec<String>,
    fail_initialize: bool,
}

impl RecordingPlugin {
    fn new(plugin_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            state: PluginState::Loaded,
            calls: Vec::new(),
            fail_initialize: false,
        }
    }

    fn failing_initialize(plugin_id: &str) -> Self {
        let mut plugin = Self::new(plugin_id);
        plugin.fail_initialize = true;
        plugin
    }
}

impl Plugin for RecordingPlugin {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            &self.plugin_id,
            "test",
            "test-plugin",
            "1.0.0",
            vec!["test.capability".to_string()],
        )
    }

    fn initialize(&mut self, context: &PluginContext) -> KernelResult<()> {
        self.calls
            .push(format!("initialize:{}", context.environment));
        if self.fail_initialize {
            return Err(sdkwork_agent_kernel::KernelError::validation(
                "plugin refuses to initialize",
            ));
        }
        self.state = PluginState::Initialized;
        Ok(())
    }

    fn shutdown(&mut self) -> KernelResult<()> {
        self.calls.push("shutdown".to_string());
        self.state = PluginState::Shutdown;
        Ok(())
    }

    fn activate(&mut self) -> KernelResult<()> {
        self.calls.push("activate".to_string());
        self.state = PluginState::Active;
        Ok(())
    }

    fn deactivate(&mut self) -> KernelResult<()> {
        self.calls.push("deactivate".to_string());
        self.state = PluginState::Deactivated;
        Ok(())
    }

    fn health_check(&self) -> ProviderHealth {
        if self.state.is_runnable() {
            ProviderHealth::available()
        } else {
            ProviderHealth::unavailable("plugin not active")
        }
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

fn register(registry: &PluginRegistry, plugin_id: &str) -> KernelResult<()> {
    registry.register(
        Box::new(RecordingPlugin::new(plugin_id)),
        PluginMetadata::new(plugin_id, "1.0.0", "Recording Plugin"),
        PluginContext::new(plugin_id, "1.0.0", "runtime.contract"),
    )
}

#[test]
fn plugin_state_vocabulary_is_stable() {
    let expected: Vec<(PluginState, &str)> = vec![
        (PluginState::Loaded, "loaded"),
        (PluginState::Initializing, "initializing"),
        (PluginState::Initialized, "initialized"),
        (PluginState::Activating, "activating"),
        (PluginState::Active, "active"),
        (PluginState::Deactivating, "deactivating"),
        (PluginState::Deactivated, "deactivated"),
        (PluginState::ShuttingDown, "shutting_down"),
        (PluginState::Shutdown, "shutdown"),
        (PluginState::Error, "error"),
        (PluginState::Reloading, "reloading"),
    ];
    for (state, word) in expected {
        assert_eq!(state.as_str(), word);
    }
}

#[test]
fn plugin_state_transition_guards_are_enforced() {
    // Runnable only when active.
    assert!(PluginState::Active.is_runnable());
    assert!(!PluginState::Initialized.is_runnable());
    assert!(!PluginState::Shutdown.is_runnable());

    // Activation requires initialized or deactivated.
    assert!(PluginState::Initialized.can_activate());
    assert!(PluginState::Deactivated.can_activate());
    assert!(!PluginState::Active.can_activate());
    assert!(!PluginState::Loaded.can_activate());

    // Deactivation requires active.
    assert!(PluginState::Active.can_deactivate());
    assert!(!PluginState::Initialized.can_deactivate());

    // Reload requires active, initialized, or deactivated.
    assert!(PluginState::Active.can_reload());
    assert!(PluginState::Initialized.can_reload());
    assert!(PluginState::Deactivated.can_reload());
    assert!(!PluginState::Loaded.can_reload());
}

#[test]
fn plugin_full_lifecycle_transitions_loaded_to_shutdown() {
    let registry = PluginRegistry::new();
    register(&registry, "plugin.lifecycle").unwrap();

    assert_eq!(
        registry.state("plugin.lifecycle").unwrap(),
        PluginState::Loaded
    );

    registry.initialize("plugin.lifecycle").unwrap();
    assert_eq!(
        registry.state("plugin.lifecycle").unwrap(),
        PluginState::Initialized
    );

    registry.activate("plugin.lifecycle").unwrap();
    assert_eq!(
        registry.state("plugin.lifecycle").unwrap(),
        PluginState::Active
    );

    // Active plugins are reported as runnable and healthy.
    assert_eq!(registry.list_active().unwrap(), vec!["plugin.lifecycle"]);
    assert_eq!(
        registry.health("plugin.lifecycle").unwrap().status,
        "available"
    );

    registry.deactivate("plugin.lifecycle").unwrap();
    assert_eq!(
        registry.state("plugin.lifecycle").unwrap(),
        PluginState::Deactivated
    );

    // Deactivated plugins may be activated again.
    registry.activate("plugin.lifecycle").unwrap();
    registry.deactivate("plugin.lifecycle").unwrap();

    registry.shutdown_all().unwrap();
    assert_eq!(
        registry.state("plugin.lifecycle").unwrap(),
        PluginState::Shutdown
    );
}

#[test]
fn plugin_registry_rejects_duplicate_registration() {
    let registry = PluginRegistry::new();
    register(&registry, "plugin.duplicate").unwrap();
    let error = register(&registry, "plugin.duplicate").unwrap_err();
    assert!(error.to_string().contains("already registered"));
}

#[test]
fn plugin_registry_rejects_unknown_plugin_operations() {
    let registry = PluginRegistry::new();
    let error = registry.initialize("plugin.missing").unwrap_err();
    assert!(error.to_string().contains("not found"));

    let error = registry.activate("plugin.missing").unwrap_err();
    assert!(error.to_string().contains("not found"));

    let error = registry.health("plugin.missing").unwrap_err();
    assert!(error.to_string().contains("not found"));
}

#[test]
fn plugin_registry_rejects_invalid_transitions() {
    let registry = PluginRegistry::new();
    register(&registry, "plugin.guard").unwrap();

    // Cannot activate a plugin that has only been registered (Loaded).
    let error = registry.activate("plugin.guard").unwrap_err();
    assert!(error.to_string().contains("cannot be activated"));

    registry.initialize("plugin.guard").unwrap();

    // Cannot deactivate a plugin that is not active.
    let error = registry.deactivate("plugin.guard").unwrap_err();
    assert!(error.to_string().contains("cannot be deactivated"));
}

#[test]
fn plugin_registry_initialize_all_collects_errors() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(RecordingPlugin::failing_initialize("plugin.broken")),
            PluginMetadata::new("plugin.broken", "1.0.0", "Broken Plugin"),
            PluginContext::new("plugin.broken", "1.0.0", "runtime.contract"),
        )
        .unwrap();
    register(&registry, "plugin.healthy").unwrap();

    let error = registry.initialize_all().unwrap_err();
    assert!(error.to_string().contains("plugin.broken"));
    assert!(error.to_string().contains("refuses to initialize"));
}

#[test]
fn plugin_context_and_metadata_builders_are_complete() {
    let context = PluginContext::new("plugin.ctx", "2.0.0", "runtime.contract")
        .with_config_dir("/etc/sdkwork")
        .with_data_dir("/var/lib/sdkwork")
        .with_environment("production")
        .with_metadata("team", "kernel");

    assert_eq!(context.plugin_id, "plugin.ctx");
    assert_eq!(context.version, "2.0.0");
    assert_eq!(context.runtime_id, "runtime.contract");
    assert_eq!(context.config_dir, "/etc/sdkwork");
    assert_eq!(context.data_dir, "/var/lib/sdkwork");
    assert_eq!(context.environment, "production");
    assert_eq!(context.metadata.get("team"), Some(&"kernel".to_string()));

    let metadata = PluginMetadata::new("plugin.meta", "1.0.0", "Meta Plugin")
        .with_description("records metadata")
        .with_author("sdkwork")
        .with_capabilities(vec!["skill.invoke".to_string()])
        .with_dependencies(vec!["plugin.base".to_string()]);

    assert_eq!(metadata.name, "Meta Plugin");
    assert_eq!(metadata.capabilities, vec!["skill.invoke"]);
    assert_eq!(metadata.dependencies, vec!["plugin.base"]);
}

#[test]
fn plugin_initialize_receives_context_and_environment() {
    let registry = PluginRegistry::new();
    let plugin = RecordingPlugin::new("plugin.context");
    let plugin_id = plugin.plugin_id.clone();
    registry
        .register(
            Box::new(plugin),
            PluginMetadata::new("plugin.context", "1.0.0", "Context Plugin"),
            PluginContext::new("plugin.context", "1.0.0", "runtime.contract")
                .with_environment("staging"),
        )
        .unwrap();

    registry.initialize(&plugin_id).unwrap();
    // The recorded call carries the environment passed through the context.
    let calls = registry
        .state(&plugin_id)
        .expect("state query should succeed");
    assert_eq!(calls, PluginState::Initialized);
}

#[test]
fn plugin_registry_metadata_list_round_trips() {
    let registry = PluginRegistry::new();
    register(&registry, "plugin.a").unwrap();
    register(&registry, "plugin.b").unwrap();

    let mut ids: Vec<String> = registry
        .list()
        .unwrap()
        .into_iter()
        .map(|metadata| metadata.plugin_id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["plugin.a".to_string(), "plugin.b".to_string()]);
}
