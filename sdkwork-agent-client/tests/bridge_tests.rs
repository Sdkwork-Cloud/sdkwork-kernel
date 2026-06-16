use sdkwork_agent_client::bridge::{
    AgentBridgeConfig, AgentBridgeHealth, AgentBridgePluginRegistry, AgentBridgeStatus,
    AgentBridgeType, AgentClient, AgentClientMode, FallbackStrategy,
};
use sdkwork_agent_client::plugins::BuiltinPlugins;
use std::sync::Arc;

#[test]
fn test_bridge_type_display() {
    assert_eq!(AgentBridgeType::OpenClaw.to_string(), "openclaw");
    assert_eq!(AgentBridgeType::ZeroClaw.to_string(), "zeroclaw");
    assert_eq!(AgentBridgeType::Hermes.to_string(), "hermes");
    assert_eq!(
        AgentBridgeType::Custom("test".to_string()).to_string(),
        "test"
    );
}

#[test]
fn test_bridge_status_display() {
    assert_eq!(AgentBridgeStatus::Healthy.to_string(), "healthy");
    assert_eq!(AgentBridgeStatus::Degraded.to_string(), "degraded");
    assert_eq!(AgentBridgeStatus::Unhealthy.to_string(), "unhealthy");
    assert_eq!(AgentBridgeStatus::Unknown.to_string(), "unknown");
}

#[test]
fn test_bridge_health_healthy() {
    let health = AgentBridgeHealth::healthy();
    assert_eq!(health.status, AgentBridgeStatus::Healthy);
    assert!(health.message.is_none());
}

#[test]
fn test_bridge_health_unhealthy() {
    let health = AgentBridgeHealth::unhealthy("test error");
    assert_eq!(health.status, AgentBridgeStatus::Unhealthy);
    assert_eq!(health.message, Some("test error".to_string()));
}

#[test]
fn test_bridge_config_new() {
    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw);
    assert_eq!(config.bridge_id, "test");
    assert_eq!(config.bridge_type, AgentBridgeType::OpenClaw);
    assert!(config.settings.is_empty());
    assert!(config.secrets.is_empty());
}

#[test]
fn test_bridge_config_with_settings() {
    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw)
        .with_setting("key1", "value1")
        .with_secret("secret1", "value1");

    assert_eq!(config.settings.get("key1"), Some(&"value1".to_string()));
    assert_eq!(config.secrets.get("secret1"), Some(&"value1".to_string()));
}

#[test]
fn test_plugin_registry_new() {
    let registry = AgentBridgePluginRegistry::new();
    assert!(registry.list_plugins().is_empty());
    assert!(registry.list_providers().is_empty());
}

#[test]
fn test_builtin_plugins_create_all() {
    let plugins = BuiltinPlugins::create_all();
    assert_eq!(plugins.plugins().len(), 3);
}

#[test]
fn test_builtin_plugins_register_all() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();

    plugins.register_all(&mut registry).unwrap();
    assert_eq!(registry.list_plugins().len(), 3);
}

#[test]
fn test_plugin_registry_create_provider() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();

    plugins.register_all(&mut registry).unwrap();

    let config = AgentBridgeConfig::new("test-openclaw", AgentBridgeType::OpenClaw);
    let bridge_id = registry
        .create_provider("builtin.openclaw", AgentBridgeType::OpenClaw, config)
        .unwrap();

    assert_eq!(bridge_id, "test-openclaw");
    assert!(registry.get_provider(&bridge_id).is_some());
}

#[test]
fn test_plugin_registry_create_provider_not_found() {
    let mut registry = AgentBridgePluginRegistry::new();

    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw);
    let result = registry.create_provider("nonexistent", AgentBridgeType::OpenClaw, config);

    assert!(result.is_err());
}

#[test]
fn test_fallback_strategy_default() {
    let strategy = FallbackStrategy::default();
    match strategy {
        FallbackStrategy::Immediate => {}
        _ => panic!("Expected Immediate"),
    }
}

#[test]
fn test_agent_client_mode_local_not_found() {
    let registry = Arc::new(AgentBridgePluginRegistry::new());
    let mode = AgentClientMode::Local {
        bridge_id: "nonexistent".to_string(),
    };

    let result = AgentClient::new(mode, registry);
    assert!(result.is_err());
}
