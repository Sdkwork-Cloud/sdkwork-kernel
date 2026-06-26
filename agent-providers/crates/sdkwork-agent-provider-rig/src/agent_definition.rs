use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentProviderBinding, AgentProviderBindingMode,
    AgentProviderFamily, CapabilityRequirement, MemoryStrategy, ModelSelectionPolicy,
    ToolCallPolicy,
};

use crate::ids;

pub fn rig_agent_manifest() -> AgentManifest {
    AgentManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "agent".to_string(),
        agent_id: ids::AGENT_ID.to_string(),
        name: "rig-general-agent".to_string(),
        display_name: "Rig General Agent".to_string(),
        description: "SDKWork-standard Rig-backed general agent runtime.".to_string(),
        version: "0.1.0".to_string(),
        domain: "intelligence".to_string(),
        required_capabilities: vec![
            "model.catalog".to_string(),
            "model.chat".to_string(),
            "policy.evaluate".to_string(),
            "protocol.map".to_string(),
            "agent.install".to_string(),
            "agent.configure".to_string(),
        ],
        optional_capabilities: vec![
            "tool.invoke".to_string(),
            "memory.query".to_string(),
            "memory.write".to_string(),
            "memory.delete".to_string(),
            "memory.export".to_string(),
            "knowledge.search".to_string(),
            "knowledge.read".to_string(),
            "knowledge.list".to_string(),
            "mcp.tools".to_string(),
            "mcp.resources".to_string(),
            "mcp.prompts".to_string(),
            "planning.create".to_string(),
        ],
        required_capability_requirements: vec![
            CapabilityRequirement::new("model.catalog").with_min_version("0.1.0"),
            CapabilityRequirement::new("model.chat").with_min_version("0.1.0"),
            CapabilityRequirement::new("policy.evaluate").with_min_version("0.1.0"),
            CapabilityRequirement::new("protocol.map").with_min_version("0.1.0"),
            CapabilityRequirement::new("agent.install").with_min_version("0.1.0"),
            CapabilityRequirement::new("agent.configure").with_min_version("0.1.0"),
        ],
        optional_capability_requirements: vec![
            CapabilityRequirement::new("tool.invoke").with_min_version("0.1.0"),
            CapabilityRequirement::new("memory.query").with_min_version("0.1.0"),
            CapabilityRequirement::new("memory.write").with_min_version("0.1.0"),
            CapabilityRequirement::new("memory.delete").with_min_version("0.1.0"),
            CapabilityRequirement::new("memory.export").with_min_version("0.1.0"),
            CapabilityRequirement::new("knowledge.search").with_min_version("0.1.0"),
            CapabilityRequirement::new("knowledge.read").with_min_version("0.1.0"),
            CapabilityRequirement::new("knowledge.list").with_min_version("0.1.0"),
            CapabilityRequirement::new("mcp.tools").with_min_version("0.1.0"),
            CapabilityRequirement::new("mcp.resources").with_min_version("0.1.0"),
            CapabilityRequirement::new("mcp.prompts").with_min_version("0.1.0"),
            CapabilityRequirement::new("planning.create").with_min_version("0.1.0"),
        ],
        event_families: vec![
            "agent.runtime.*".to_string(),
            "agent.model.*".to_string(),
            "agent.tool.*".to_string(),
            "agent.memory.*".to_string(),
            "agent.knowledge.*".to_string(),
            "agent.mcp.*".to_string(),
            "agent.policy.*".to_string(),
            "agent.protocol.*".to_string(),
            "agent.install.*".to_string(),
            "agent.configure.*".to_string(),
        ],
        owner_name: "sdkwork-platform".to_string(),
        status: "candidate".to_string(),
    }
}

pub fn rig_agent_definition() -> AgentDefinition {
    AgentDefinition::new("definition.intelligence.rig-general", rig_agent_manifest())
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.model",
                AgentProviderFamily::Model,
                ids::MODEL_PROVIDER_ID,
                true,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec!["model.catalog".to_string(), "model.chat".to_string()]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.tool",
                AgentProviderFamily::Tool,
                ids::TOOL_PROVIDER_ID,
                false,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capability("tool.invoke"),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.memory",
                AgentProviderFamily::Memory,
                ids::MEMORY_PROVIDER_ID,
                false,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec![
                "memory.query".to_string(),
                "memory.write".to_string(),
                "memory.delete".to_string(),
                "memory.export".to_string(),
            ]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.knowledge",
                AgentProviderFamily::Knowledge,
                ids::KNOWLEDGE_PROVIDER_ID,
                false,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.mcp",
                AgentProviderFamily::Mcp,
                ids::MCP_PROVIDER_ID,
                false,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec![
                "mcp.tools".to_string(),
                "mcp.resources".to_string(),
                "mcp.prompts".to_string(),
            ]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.planning",
                AgentProviderFamily::Planning,
                ids::PLANNING_PROVIDER_ID,
                false,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capability("planning.create"),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.policy",
                AgentProviderFamily::Policy,
                ids::POLICY_PROVIDER_ID,
                true,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capability("policy.evaluate"),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.chat-rpc",
                AgentProviderFamily::ProtocolAdapter,
                ids::CHAT_RPC_ADAPTER_ID,
                true,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec![
                "protocol.map".to_string(),
                "protocol.stream".to_string(),
            ]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.installer",
                AgentProviderFamily::AgentInstaller,
                ids::INSTALLER_PROVIDER_ID,
                true,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capabilities(vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ]),
        )
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.rig.configuration",
                AgentProviderFamily::AgentConfiguration,
                ids::CONFIGURATION_PROVIDER_ID,
                true,
            )
            .as_default()
            .with_mode(AgentProviderBindingMode::TypedLocal)
            .with_min_version("0.1.0")
            .with_capability("agent.configure"),
        )
        .with_model_selection(
            ModelSelectionPolicy::default_provider(ids::MODEL_PROVIDER_ID)
                .with_default_model_id(ids::DEFAULT_MODEL_ID)
                .with_required_capability("model.chat")
                .allow_provider_fallback(false),
        )
        .with_tool_call_policy(
            ToolCallPolicy::default_provider(ids::TOOL_PROVIDER_ID)
                .with_policy_required(true)
                .with_allowed_tool_id(ids::DEFAULT_TOOL_ID)
                .with_max_parallel_calls(4),
        )
        .with_memory_strategy(
            MemoryStrategy::default_provider(ids::MEMORY_PROVIDER_ID)
                .with_scope(sdkwork_agent_kernel::MemoryScope::Session)
                .with_scope(sdkwork_agent_kernel::MemoryScope::Agent)
                .with_write_policy_required(true)
                .with_read_policy_required_for_sensitive(true),
        )
        .validate()
        .expect("Rig agent definition must satisfy SDKWork provider binding standards")
}
