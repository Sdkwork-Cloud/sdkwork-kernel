use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentProviderBindingMode, AgentProviderFamily, ProtocolAdapterRequest,
    ProtocolFamily, ProtocolObjectKind, RuntimeBuilder, RuntimeState, TraceContext,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_plugin_rig::{
    ids, rig_agent_definition, rig_agent_manifest, rig_kernel_plugin_manifest,
    rig_package_manifest, rig_provider_manifests, RigKernelPlugin,
};

#[test]
fn rig_uses_stable_standard_ids() {
    assert_eq!(ids::PLUGIN_ID, "plugin.intelligence.rig");
    assert_eq!(ids::AGENT_ID, "agent.intelligence.rig-general");
    assert_eq!(ids::MODEL_PROVIDER_ID, "provider.model.rig-rust");
    assert_eq!(ids::TOOL_PROVIDER_ID, "provider.tool.rig-rust");
    assert_eq!(ids::MEMORY_PROVIDER_ID, "provider.memory.rig-rust");
    assert_eq!(ids::KNOWLEDGE_PROVIDER_ID, "provider.knowledge.rig-rust");
    assert_eq!(ids::MCP_PROVIDER_ID, "provider.mcp.rig-rust");
    assert_eq!(ids::PLANNING_PROVIDER_ID, "provider.planning.rig-rust");
    assert_eq!(
        ids::INSTALLER_PROVIDER_ID,
        "provider.agent.installer.rig-rust"
    );
    assert_eq!(
        ids::CONFIGURATION_PROVIDER_ID,
        "provider.agent.configuration.rig-rust"
    );
    assert_eq!(ids::CHAT_RPC_ADAPTER_ID, "adapter.rpc.agent-chat");
}

#[test]
fn rig_exposes_canonical_kernel_plugin_names() {
    let manifest = rig_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, ids::PLUGIN_ID);
    assert_eq!(manifest.implementation_kind, "typed-local-provider");
    assert!(manifest.supports_profile("runtime-local"));

    let plugin = RigKernelPlugin::fail_closed();
    assert_kernel_plugin_trait(&plugin);
    assert_eq!(
        SdkworkKernelPlugin::plugin_manifest(&plugin).plugin_id,
        ids::PLUGIN_ID
    );
}

fn assert_kernel_plugin_trait<T: SdkworkKernelPlugin>(_plugin: &T) {}

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
    assert!(agent
        .optional_capabilities
        .contains(&"mcp.tools".to_string()));
    assert!(agent
        .optional_capabilities
        .contains(&"mcp.resources".to_string()));
    assert!(agent
        .optional_capabilities
        .contains(&"mcp.prompts".to_string()));

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
    assert!(
        !model.supports_capability("model.tool_call"),
        "Rig must not bind model.tool_call until model descriptors and backend execution support tool-call output"
    );
    assert!(
        !model.supports_capability("model.streaming"),
        "Rig must not declare streaming until it implements ModelProvider::stream"
    );
    assert!(
        !model.supports_capability("model.structured_output"),
        "Rig must not bind structured output until its catalog declares a structured response format"
    );

    let tool = definition
        .default_binding(AgentProviderFamily::Tool)
        .expect("Rig tool binding is explicit");
    assert_eq!(tool.provider_id, ids::TOOL_PROVIDER_ID);
    assert!(tool.supports_capability("tool.invoke"));
    assert!(definition.tool_call_policy.policy_required);
    assert!(definition
        .tool_call_policy
        .allows_tool(ids::DEFAULT_TOOL_ID));

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

    let knowledge = definition
        .default_binding(AgentProviderFamily::Knowledge)
        .expect("Rig knowledge binding is explicit");
    assert_eq!(knowledge.provider_id, ids::KNOWLEDGE_PROVIDER_ID);
    assert!(!knowledge.required);
    assert!(knowledge.supports_capability("knowledge.search"));
    assert!(knowledge.supports_capability("knowledge.read"));

    let mcp = definition
        .default_binding(AgentProviderFamily::Mcp)
        .expect("Rig MCP binding is explicit");
    assert_eq!(mcp.provider_id, ids::MCP_PROVIDER_ID);
    assert_eq!(mcp.mode, AgentProviderBindingMode::TypedLocal);
    assert!(!mcp.required);
    assert!(mcp.supports_capability("mcp.tools"));
    assert!(mcp.supports_capability("mcp.resources"));
    assert!(mcp.supports_capability("mcp.prompts"));

    let policy = definition
        .default_binding(AgentProviderFamily::Policy)
        .expect("Rig policy binding is explicit");
    assert_eq!(policy.provider_id, ids::POLICY_PROVIDER_ID);
    assert!(policy.required);

    let chat_rpc_adapter = definition
        .default_binding(AgentProviderFamily::ProtocolAdapter)
        .expect("Rig chat RPC adapter binding is explicit");
    assert_eq!(chat_rpc_adapter.provider_id, ids::CHAT_RPC_ADAPTER_ID);
    assert_eq!(chat_rpc_adapter.mode, AgentProviderBindingMode::TypedLocal);
    assert!(chat_rpc_adapter.required);
    assert!(chat_rpc_adapter.supports_capability("protocol.map"));
    assert!(chat_rpc_adapter.supports_capability("protocol.stream"));

    assert_eq!(
        definition.model_selection.default_provider_id.as_deref(),
        Some(ids::MODEL_PROVIDER_ID)
    );
    assert_eq!(
        definition.model_selection.default_model_id.as_deref(),
        Some(ids::DEFAULT_MODEL_ID)
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
    let model_provider = providers
        .iter()
        .find(|provider| provider.provider_id == ids::MODEL_PROVIDER_ID)
        .expect("Rig model provider manifest is present");
    assert!(
        !model_provider
            .capabilities
            .contains(&"model.tool_call".to_string()),
        "Rig model provider must not advertise tool-call output while its model catalog and backend do not support it"
    );
    assert!(
        !model_provider
            .capabilities
            .contains(&"model.streaming".to_string()),
        "Rig model provider must not advertise streaming while ModelProvider::stream is unsupported"
    );
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
        provider.provider_id == ids::KNOWLEDGE_PROVIDER_ID
            && provider
                .capabilities
                .contains(&"knowledge.search".to_string())
            && provider
                .capabilities
                .contains(&"knowledge.read".to_string())
            && provider
                .capabilities
                .contains(&"knowledge.list".to_string())
    }));
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::MCP_PROVIDER_ID
            && provider.provider_family == "mcp"
            && provider.capabilities.contains(&"mcp.tools".to_string())
            && provider.capabilities.contains(&"mcp.resources".to_string())
            && provider.capabilities.contains(&"mcp.prompts".to_string())
    }));
    assert!(providers.iter().any(|provider| {
        provider.provider_id == ids::PLANNING_PROVIDER_ID
            && provider
                .capabilities
                .contains(&"planning.create".to_string())
    }));
    let chat_rpc_adapter = providers
        .iter()
        .find(|provider| provider.provider_id == ids::CHAT_RPC_ADAPTER_ID)
        .expect("Rig chat RPC adapter provider manifest is present");
    assert_eq!(chat_rpc_adapter.provider_family, "protocol_adapter");
    assert!(chat_rpc_adapter
        .capabilities
        .contains(&"protocol.map".to_string()));
    assert!(chat_rpc_adapter
        .capabilities
        .contains(&"protocol.stream".to_string()));
    assert!(
        !chat_rpc_adapter
            .capabilities
            .contains(&"model.chat".to_string()),
        "protocol adapter manifest must not replace the model provider capability"
    );
}

#[test]
fn rig_plugin_assembles_runtime_with_typed_providers() {
    let plugin = RigKernelPlugin::fail_closed();
    let builder = RuntimeBuilder::new("runtime.rig.local", plugin.agent_manifest());
    let report = plugin
        .configure_runtime(builder)
        .bootstrap()
        .expect("rig runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Degraded);
    assert_eq!(report.runtime.diagnostics().state, "degraded");
    let model_diagnostic = report
        .runtime
        .diagnostics()
        .provider(ids::MODEL_PROVIDER_ID)
        .expect("Rig model diagnostics are present")
        .clone();
    assert!(model_diagnostic.typed_registered);
    assert!(model_diagnostic.health_is_degraded());
    assert_eq!(
        model_diagnostic
            .health
            .expect("Rig model health is reported")
            .status,
        "degraded"
    );
    let tool_diagnostic = report
        .runtime
        .diagnostics()
        .provider(ids::TOOL_PROVIDER_ID)
        .expect("Rig tool diagnostics are present")
        .clone();
    assert!(tool_diagnostic.typed_registered);
    assert!(tool_diagnostic.health_is_degraded());
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
    assert!(report
        .runtime
        .diagnostics()
        .provider_diagnostics
        .iter()
        .any(
            |provider| provider.provider_id == ids::KNOWLEDGE_PROVIDER_ID
                && provider.typed_registered
        ));
    assert!(report
        .runtime
        .diagnostics()
        .provider_diagnostics
        .iter()
        .any(|provider| provider.provider_id == ids::MCP_PROVIDER_ID
            && provider.provider_family == "mcp"
            && provider.typed_registered));
    assert_eq!(
        report.runtime.mcp_provider_ids(),
        [ids::MCP_PROVIDER_ID.to_string()]
    );
    assert!(report
        .runtime
        .mcp_provider_by_id(ids::MCP_PROVIDER_ID)
        .is_ok());
    assert!(report
        .runtime
        .diagnostics()
        .provider_diagnostics
        .iter()
        .any(|provider| provider.provider_id == ids::CHAT_RPC_ADAPTER_ID
            && provider.provider_family == "protocol_adapter"
            && provider.typed_registered));
    assert_eq!(
        report.runtime.protocol_adapter_ids(),
        [ids::CHAT_RPC_ADAPTER_ID.to_string()]
    );
    assert!(report
        .runtime
        .protocol_adapter_by_id(ids::CHAT_RPC_ADAPTER_ID)
        .is_ok());
    assert!(report
        .runtime
        .capability_manifest()
        .protocol_adapters
        .contains(&ids::CHAT_RPC_ADAPTER_ID.to_string()));
    let envelope = report
        .runtime
        .protocol_adapter_by_id(ids::CHAT_RPC_ADAPTER_ID)
        .expect("Rig chat RPC protocol adapter is registered by id")
        .handle_request(
            &report.runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.rig.fail-closed",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "summarize Rig runtime status",
            )
            .with_metadata("sdkwork.chat.provider_id", ids::MODEL_PROVIDER_ID)
            .with_trace_context(TraceContext::new("trace.rig.chat", "span.rig.chat")),
        )
        .expect("Rig chat RPC adapter maps fail-closed backend errors to an envelope");
    assert_eq!(envelope.object_kind, ProtocolObjectKind::KernelError);
    assert_eq!(
        envelope.metadata_value("sdkwork.error.kind"),
        Some("provider_unavailable")
    );
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
    let knowledge_diagnostic = report
        .runtime
        .diagnostics()
        .provider(ids::KNOWLEDGE_PROVIDER_ID)
        .expect("Rig knowledge diagnostics are present")
        .clone();
    assert!(knowledge_diagnostic
        .capabilities
        .contains(&"knowledge.search".to_string()));
    assert!(plugin.conformance_profile().requires("runtime-local"));
    assert_eq!(
        plugin
            .agent_definition()
            .model_selection
            .default_provider_id,
        Some(ids::MODEL_PROVIDER_ID.to_string())
    );
}

#[test]
fn rig_component_spec_declares_runtime_plugin_entrypoints() {
    let component_spec = include_str!("../specs/component.spec.json");

    assert!(
        component_spec.contains("RigKernelPlugin::configure_runtime"),
        "Rig component spec must expose the runtime assembly entrypoint"
    );
    assert!(
        component_spec.contains("rig_agent_definition"),
        "Rig component spec must expose the executable agent definition entrypoint"
    );
    assert!(
        component_spec.contains("rig_provider_manifests"),
        "Rig component spec must expose provider manifest discovery"
    );
    assert!(
        component_spec.contains("adapter.rpc.agent-chat"),
        "Rig component spec must declare the chat RPC protocol adapter"
    );
}
