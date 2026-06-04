use sdkwork_agent_integration_core::SdkworkAgentIntegrationPlugin;
use sdkwork_agent_integration_rig::{
    ids, rig_agent_definition, rig_agent_manifest, rig_package_manifest, rig_provider_manifests,
    RigIntegrationPlugin,
};
use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentProviderBindingMode, AgentProviderFamily, RuntimeBuilder,
    RuntimeState,
};

#[test]
fn rig_uses_stable_standard_ids() {
    assert_eq!(ids::PLUGIN_ID, "plugin.intelligence.rig");
    assert_eq!(ids::AGENT_ID, "agent.intelligence.rig-general");
    assert_eq!(ids::MODEL_PROVIDER_ID, "provider.model.rig-rust");
    assert_eq!(ids::TOOL_PROVIDER_ID, "provider.tool.rig-rust");
    assert_eq!(ids::MEMORY_PROVIDER_ID, "provider.memory.rig-rust");
    assert_eq!(ids::PLANNING_PROVIDER_ID, "provider.planning.rig-rust");
    assert_eq!(
        ids::INSTALLER_PROVIDER_ID,
        "provider.agent.installer.rig-rust"
    );
    assert_eq!(
        ids::CONFIGURATION_PROVIDER_ID,
        "provider.agent.configuration.rig-rust"
    );
}

#[test]
fn rig_agent_and_package_manifests_declare_installable_standard_surface() {
    let agent = rig_agent_manifest();
    assert_eq!(agent.agent_id, ids::AGENT_ID);
    assert!(agent
        .required_capabilities
        .contains(&"model.chat".to_string()));
    assert!(agent
        .required_capabilities
        .contains(&"policy.evaluate".to_string()));
    assert!(agent.event_families.contains(&"agent.model.*".to_string()));

    let package = rig_package_manifest();
    assert_eq!(package.agent_id, ids::AGENT_ID);
    assert_eq!(
        package.provider_binding.installer_provider_id,
        ids::INSTALLER_PROVIDER_ID
    );
    assert_eq!(
        package.provider_binding.configuration_provider_id,
        ids::CONFIGURATION_PROVIDER_ID
    );
    assert!(package.lifecycle.supports_install);
    assert!(package.requires_llm_api_key());
    assert!(package
        .required_configuration_sections()
        .contains(&AgentConfigSectionKind::Security));
}

#[test]
fn rig_agent_definition_declares_model_tool_policy_and_memory_strategy() {
    let definition = rig_agent_definition();

    assert_eq!(
        definition.definition_id,
        "definition.intelligence.rig-general"
    );
    assert_eq!(definition.manifest.agent_id, ids::AGENT_ID);

    let model = definition
        .default_binding(AgentProviderFamily::Model)
        .expect("Rig model binding is explicit");
    assert_eq!(model.provider_id, ids::MODEL_PROVIDER_ID);
    assert_eq!(model.mode, AgentProviderBindingMode::TypedLocal);
    assert!(model.required);
    assert!(model.supports_capability("model.chat"));
    assert!(model.supports_capability("model.tool_call"));

    let tool = definition
        .default_binding(AgentProviderFamily::Tool)
        .expect("Rig tool binding is explicit");
    assert_eq!(tool.provider_id, ids::TOOL_PROVIDER_ID);
    assert!(tool.supports_capability("tool.invoke"));
    assert!(definition.tool_call_policy.policy_required);
    assert!(definition.tool_call_policy.allows_tool("tool.rig.retrieve"));

    let memory = definition
        .default_binding(AgentProviderFamily::Memory)
        .expect("Rig memory binding is explicit");
    assert_eq!(memory.provider_id, ids::MEMORY_PROVIDER_ID);
    assert!(!memory.required);
    assert!(memory.supports_capability("memory.query"));
    assert!(memory.supports_capability("memory.write"));
    assert_eq!(
        definition.memory_strategy.default_provider_id.as_deref(),
        Some(ids::MEMORY_PROVIDER_ID)
    );
    assert!(definition
        .memory_strategy
        .scope_enabled(sdkwork_agent_kernel::MemoryScope::Session));
    assert!(definition
        .memory_strategy
        .scope_enabled(sdkwork_agent_kernel::MemoryScope::Agent));

    let policy = definition
        .default_binding(AgentProviderFamily::Policy)
        .expect("Rig policy binding is explicit");
    assert_eq!(policy.provider_id, ids::POLICY_PROVIDER_ID);
    assert!(policy.required);

    assert_eq!(
        definition.model_selection.default_provider_id.as_deref(),
        Some(ids::MODEL_PROVIDER_ID)
    );
    assert_eq!(
        definition.model_selection.default_model_id.as_deref(),
        Some("rig.default")
    );
    assert!(definition.memory_strategy.write_policy_required);
}

#[test]
fn rig_provider_manifests_cover_model_tool_planning_and_lifecycle() {
    let providers = rig_provider_manifests();
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::MODEL_PROVIDER_ID
            && provider.capabilities.contains(&"model.catalog".to_string())
            && provider.capabilities.contains(&"model.chat".to_string())
    }));
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::TOOL_PROVIDER_ID
            && provider.capabilities.contains(&"tool.invoke".to_string())
    }));
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::MEMORY_PROVIDER_ID
            && provider.capabilities.contains(&"memory.query".to_string())
            && provider.capabilities.contains(&"memory.write".to_string())
            && provider.capabilities.contains(&"memory.delete".to_string())
            && provider.capabilities.contains(&"memory.export".to_string())
    }));
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::PLANNING_PROVIDER_ID
            && provider
                .capabilities
                .contains(&"planning.create".to_string())
    }));
}

#[test]
fn rig_plugin_assembles_runtime_with_typed_providers() {
    let plugin = RigIntegrationPlugin::fail_closed();
    let builder = RuntimeBuilder::new("runtime.rig.local", plugin.agent_manifest());
    let report = plugin
        .configure_runtime(builder)
        .bootstrap()
        .expect("rig runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    assert!(report
        .runtime
        .diagnostics()
        .provider_diagnostics
        .iter()
        .any(
            |provider| provider.provider_id == ids::MODEL_PROVIDER_ID && provider.typed_registered
        ));
    assert!(report
        .runtime
        .diagnostics()
        .provider_diagnostics
        .iter()
        .any(
            |provider| provider.provider_id == ids::MEMORY_PROVIDER_ID && provider.typed_registered
        ));
    let memory_diagnostic = report
        .runtime
        .diagnostics()
        .provider(ids::MEMORY_PROVIDER_ID)
        .expect("Rig memory diagnostics are present")
        .clone();
    assert!(memory_diagnostic
        .capabilities
        .contains(&"memory.query".to_string()));
    assert!(memory_diagnostic
        .capabilities
        .contains(&"memory.write".to_string()));
    assert!(memory_diagnostic
        .capabilities
        .contains(&"memory.delete".to_string()));
    assert!(memory_diagnostic
        .capabilities
        .contains(&"memory.export".to_string()));
    assert!(plugin.conformance_profile().requires("runtime-local"));
    assert_eq!(
        plugin
            .agent_definition()
            .model_selection
            .default_provider_id,
        Some(ids::MODEL_PROVIDER_ID.to_string())
    );
}
