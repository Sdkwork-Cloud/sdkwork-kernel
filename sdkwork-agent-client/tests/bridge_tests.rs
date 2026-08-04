use sdkwork_agent_client::bridge::{
    AgentBridgeConfig, AgentBridgeHealth, AgentBridgePluginRegistry, AgentBridgeStatus,
    AgentBridgeType, AgentClient, AgentClientMode, FallbackStrategy,
};
use sdkwork_agent_client::plugins::BuiltinPlugins;
use sdkwork_agent_client::session::BridgeSessionQuery;
use sdkwork_agent_client::{ChatRequest, SessionConfig};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_bridge_type_display() {
    assert_eq!(AgentBridgeType::OpenClaw.to_string(), "openclaw");
    assert_eq!(AgentBridgeType::ZeroClaw.to_string(), "zeroclaw");
    assert_eq!(AgentBridgeType::Hermes.to_string(), "hermes");
    assert_eq!(AgentBridgeType::Codex.to_string(), "codex");
    assert_eq!(AgentBridgeType::ClaudeCode.to_string(), "claude-code");
    assert_eq!(AgentBridgeType::OpenCode.to_string(), "opencode");
    assert_eq!(AgentBridgeType::GeminiCli.to_string(), "gemini-cli");
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
    assert_eq!(plugins.plugins().len(), 7);
}

#[test]
fn test_builtin_plugins_register_all() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();

    plugins.register_all(&mut registry).unwrap();
    assert_eq!(registry.list_plugins().len(), 7);
    let plugin_ids = registry
        .list_plugins()
        .into_iter()
        .map(|plugin| plugin.plugin_id().to_string())
        .collect::<Vec<_>>();
    assert!(plugin_ids.contains(&"builtin.claude-code".to_string()));
    assert!(plugin_ids.contains(&"builtin.opencode".to_string()));
    assert!(plugin_ids.contains(&"builtin.gemini-cli".to_string()));
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
fn sdk_backed_plugins_initialize_and_release_owned_runtimes() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();
    plugins.register_all(&mut registry).unwrap();

    for (index, (plugin_id, bridge_type)) in [
        ("builtin.claude-code", AgentBridgeType::ClaudeCode),
        ("builtin.opencode", AgentBridgeType::OpenCode),
        ("builtin.gemini-cli", AgentBridgeType::GeminiCli),
    ]
    .into_iter()
    .enumerate()
    {
        let bridge_id = format!("test.sdk-backed.{index}");
        registry
            .create_provider(
                plugin_id,
                bridge_type.clone(),
                AgentBridgeConfig::new(&bridge_id, bridge_type.clone()),
            )
            .expect("provider should create");
        let provider = registry
            .get_provider(&bridge_id)
            .expect("provider should be registered");

        provider.initialize().expect("runtime should initialize");
        assert_eq!(provider.bridge_type(), &bridge_type);
        provider.shutdown().expect("runtime should shut down");
        assert_eq!(provider.health_check().status, AgentBridgeStatus::Unknown);
    }
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

#[test]
#[ignore = "live test: requires real Codex/Hermes installs to create sessions"]
fn test_registry_list_all_sessions_sorted_by_updated_at() {
    let database_path = std::env::temp_dir().join(format!(
        "sdkwork-agent-client-bridge-test-{}.sqlite",
        Uuid::new_v4()
    ));
    std::env::set_var(
        "SDKWORK_DATABASE_FILE",
        database_path.to_string_lossy().as_ref(),
    );

    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();
    plugins.register_all(&mut registry).unwrap();

    let codex_id = registry
        .create_provider(
            "builtin.codex",
            AgentBridgeType::Codex,
            AgentBridgeConfig::new("bridge.codex", AgentBridgeType::Codex),
        )
        .unwrap();
    let hermes_id = registry
        .create_provider(
            "builtin.hermes",
            AgentBridgeType::Hermes,
            AgentBridgeConfig::new("bridge.hermes", AgentBridgeType::Hermes),
        )
        .unwrap();

    let codex = registry.get_provider(&codex_id).unwrap();
    let hermes = registry.get_provider(&hermes_id).unwrap();
    codex.initialize().unwrap();
    hermes.initialize().unwrap();

    let codex_session = codex
        .create_session(SessionConfig::new("agent.1").with_title("Codex"))
        .unwrap();
    let _hermes_session = hermes
        .create_session(SessionConfig::new("agent.1").with_title("Hermes"))
        .unwrap();
    codex
        .send_message(ChatRequest {
            session_id: codex_session.session_id.clone(),
            content: "ping".to_string(),
            model: None,
            stream: false,
        })
        .unwrap();

    let listed = registry.list_all_sessions(&BridgeSessionQuery::default());
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].session_id, codex_session.session_id);
    assert_eq!(listed[0].provider_id, "codex");
}

#[test]
fn internal_runtime_mount_prefix_matches_internal_api_authority() {
    use sdkwork_agent_client::INTERNAL_RUNTIME_MOUNT_PREFIX;

    assert_eq!(
        INTERNAL_RUNTIME_MOUNT_PREFIX,
        "/internal/v3/api/intelligence/runtime"
    );
    assert!(!INTERNAL_RUNTIME_MOUNT_PREFIX.contains("/api/kernel"));
    assert!(!INTERNAL_RUNTIME_MOUNT_PREFIX.contains("/api/sessions"));
    assert!(!INTERNAL_RUNTIME_MOUNT_PREFIX.contains("/api/chat"));
}
