use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentProviderBinding, AgentProviderBindingMode,
    AgentProviderFamily, CapabilityRequirement, MemoryStrategy, ModelSelectionPolicy,
    ToolCallPolicy,
};

use crate::ids;

pub fn opencode_agent_manifest() -> AgentManifest {
    AgentManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "agent".to_string(),
        agent_id: ids::AGENT_ID.to_string(),
        name: "opencode".to_string(),
        display_name: "OpenCode".to_string(),
        description: "SDKWork-standard OpenCode TypeScript SDK backed code-agent runtime."
            .to_string(),
        version: "0.2.0".to_string(),
        domain: "intelligence".to_string(),
        required_capabilities: vec!["model.chat".to_string(), "policy.evaluate".to_string()],
        optional_capabilities: vec!["tool.invoke".to_string()],
        required_capability_requirements: vec![
            CapabilityRequirement::new("model.chat").with_min_version("0.1.0"),
            CapabilityRequirement::new("policy.evaluate").with_min_version("0.1.0"),
        ],
        optional_capability_requirements: vec![
            CapabilityRequirement::new("tool.invoke").with_min_version("0.1.0")
        ],
        event_families: vec![
            "agent.runtime.*".to_string(),
            "agent.session.*".to_string(),
            "agent.model.*".to_string(),
            "agent.tool.*".to_string(),
            "agent.policy.*".to_string(),
            "code.terminal.*".to_string(),
            "code.patch.*".to_string(),
        ],
        owner_name: "sdkwork-platform".to_string(),
        status: "experimental".to_string(),
    }
}

pub fn opencode_agent_definition() -> AgentDefinition {
    AgentDefinition::new(
        "definition.intelligence.opencode",
        opencode_agent_manifest(),
    )
    .with_provider_binding(
        AgentProviderBinding::new(
            "binding.opencode.model",
            AgentProviderFamily::Model,
            ids::MODEL_PROVIDER_ID,
            true,
        )
        .as_default()
        .with_mode(AgentProviderBindingMode::TypedLocal)
        .with_min_version("0.2.0")
        .with_capability("model.chat"),
    )
    .with_provider_binding(
        AgentProviderBinding::new(
            "binding.opencode.tool",
            AgentProviderFamily::Tool,
            ids::TOOL_PROVIDER_ID,
            false,
        )
        .as_default()
        .with_mode(AgentProviderBindingMode::TypedLocal)
        .with_min_version("0.2.0")
        .with_capability("tool.invoke"),
    )
    .with_provider_binding(
        AgentProviderBinding::new(
            "binding.opencode.policy",
            AgentProviderFamily::Policy,
            ids::POLICY_PROVIDER_ID,
            true,
        )
        .as_default()
        .with_mode(AgentProviderBindingMode::TypedLocal)
        .with_min_version("0.1.0")
        .with_capability("policy.evaluate"),
    )
    .with_model_selection(
        ModelSelectionPolicy::default_provider(ids::MODEL_PROVIDER_ID)
            .with_default_model_id("opencode-default")
            .with_required_capability("model.chat")
            .allow_provider_fallback(false),
    )
    .with_tool_call_policy(
        ToolCallPolicy::default_provider(ids::TOOL_PROVIDER_ID).with_policy_required(true),
    )
    .with_memory_strategy(MemoryStrategy::disabled())
}
