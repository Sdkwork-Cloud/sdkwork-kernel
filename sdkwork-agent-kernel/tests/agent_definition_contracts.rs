use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentProviderBinding, AgentProviderBindingMode,
    AgentProviderFamily, MemoryScope, MemoryStrategy, ModelSelectionPolicy, ToolCallPolicy,
};

const STANDARD_AGENT_DEFINITION_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_definition",
  "definition_id": "definition.intelligence.research",
  "agent": {
    "schema_version": "0.1.0",
    "manifest_type": "agent",
    "agent_id": "agent.research",
    "name": "research-agent",
    "display_name": "Research Agent",
    "description": "Agent used to prove provider binding standards.",
    "version": "0.1.0",
    "domain": "intelligence",
    "required_capabilities": [
      {
        "capability_id": "model.chat",
        "min_version": "0.1.0"
      },
      {
        "capability_id": "tool.invoke",
        "min_version": "0.1.0"
      },
      {
        "capability_id": "memory.query",
        "min_version": "0.1.0"
      }
    ],
    "optional_capabilities": [
      {
        "capability_id": "model.streaming",
        "min_version": "0.1.0"
      },
      {
        "capability_id": "memory.write",
        "min_version": "0.1.0"
      }
    ],
    "event_families": ["agent.runtime.*", "agent.model.*", "agent.tool.*", "agent.memory.*"],
    "owner": {
      "name": "sdkwork-platform"
    },
    "status": "candidate"
  },
  "provider_bindings": [
    {
      "binding_id": "binding.model.primary",
      "family": "model",
      "provider_id": "provider.openai",
      "required": true,
      "default": true,
      "mode": "typed_local",
      "capabilities": ["model.catalog", "model.chat", "model.streaming"],
      "min_version": "0.1.0"
    },
    {
      "binding_id": "binding.tool.primary",
      "family": "tool",
      "provider_id": "provider.tool.mcp",
      "required": true,
      "default": true,
      "mode": "manifest_or_typed",
      "capabilities": ["tool.invoke", "tool.cancellation"],
      "min_version": "0.1.0"
    },
    {
      "binding_id": "binding.memory.primary",
      "family": "memory",
      "provider_id": "provider.memory.vector",
      "required": false,
      "default": true,
      "mode": "manifest_or_typed",
      "capabilities": ["memory.query", "memory.write", "memory.delete", "memory.export"],
      "min_version": "0.1.0"
    }
  ],
  "model_selection": {
    "default_provider_id": "provider.openai",
    "default_model_id": "gpt-4.1",
    "required_capabilities": ["model.chat"],
    "allow_provider_fallback": true
  },
  "tool_call_policy": {
    "default_provider_id": "provider.tool.mcp",
    "policy_required": true,
    "allowed_tool_ids": ["tool.web.search", "tool.repo.read"],
    "denied_tool_ids": ["tool.shell.rm"],
    "max_parallel_calls": 4
  },
  "memory_strategy": {
    "default_provider_id": "provider.memory.vector",
    "enabled_scopes": ["session", "user", "tenant"],
    "write_policy_required": true,
    "read_policy_required_for_sensitive": true,
    "retention_required": true
  },
  "extensions": {
    "sdkwork.routing.note": "provider bindings are selected by provider id"
  }
}
"#;

#[test]
fn agent_definition_makes_model_tool_and_memory_bindings_explicit() {
    let manifest = AgentManifest::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("nested agent manifest parses from agent definition");
    let definition = AgentDefinition::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("agent definition parses");

    assert_eq!(definition.manifest.agent_id, manifest.agent_id);
    assert_eq!(definition.definition_id, "definition.intelligence.research");
    assert!(definition.requires_provider_family(AgentProviderFamily::Model));
    assert!(definition.requires_provider_family(AgentProviderFamily::Tool));
    assert!(!definition.requires_provider_family(AgentProviderFamily::Memory));

    let model = definition
        .default_binding(AgentProviderFamily::Model)
        .expect("default model binding is explicit");
    assert_eq!(model.provider_id, "provider.openai");
    assert_eq!(model.mode, AgentProviderBindingMode::TypedLocal);
    assert!(model.supports_capability("model.chat"));
    assert!(model.satisfies_version("0.1.0"));

    let tool = definition
        .default_binding(AgentProviderFamily::Tool)
        .expect("default tool binding is explicit");
    assert_eq!(tool.provider_id, "provider.tool.mcp");
    assert_eq!(tool.mode, AgentProviderBindingMode::ManifestOrTyped);
    assert!(tool.supports_capability("tool.invoke"));

    let memory = definition
        .default_binding(AgentProviderFamily::Memory)
        .expect("default memory binding is explicit");
    assert_eq!(memory.provider_id, "provider.memory.vector");
    assert!(!memory.required);
    assert!(memory.supports_capability("memory.export"));
}

#[test]
fn agent_definition_preserves_llm_tool_call_and_memory_policies() {
    let definition = AgentDefinition::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("agent definition parses");

    assert_eq!(
        definition.model_selection.default_provider_id.as_deref(),
        Some("provider.openai")
    );
    assert_eq!(
        definition.model_selection.default_model_id.as_deref(),
        Some("gpt-4.1")
    );
    assert!(definition.model_selection.requires_capability("model.chat"));
    assert!(definition.model_selection.allow_provider_fallback);

    assert_eq!(
        definition.tool_call_policy.default_provider_id.as_deref(),
        Some("provider.tool.mcp")
    );
    assert!(definition.tool_call_policy.policy_required);
    assert!(definition.tool_call_policy.allows_tool("tool.web.search"));
    assert!(!definition.tool_call_policy.allows_tool("tool.shell.rm"));
    assert_eq!(definition.tool_call_policy.max_parallel_calls, Some(4));

    assert_eq!(
        definition.memory_strategy.default_provider_id.as_deref(),
        Some("provider.memory.vector")
    );
    assert!(definition
        .memory_strategy
        .scope_enabled(MemoryScope::Session));
    assert!(definition
        .memory_strategy
        .scope_enabled(MemoryScope::Tenant));
    assert!(!definition
        .memory_strategy
        .scope_enabled(MemoryScope::Organization));
    assert!(definition.memory_strategy.write_policy_required);
    assert!(
        definition
            .memory_strategy
            .read_policy_required_for_sensitive
    );
    assert!(definition.memory_strategy.retention_required);
}

#[test]
fn agent_definition_rejects_ambiguous_default_provider_bindings() {
    let manifest = AgentManifest::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("nested agent manifest parses from agent definition");
    let binding_a = AgentProviderBinding::new(
        "binding.model.a",
        AgentProviderFamily::Model,
        "provider.a",
        true,
    )
    .as_default();
    let binding_b = AgentProviderBinding::new(
        "binding.model.b",
        AgentProviderFamily::Model,
        "provider.b",
        true,
    )
    .as_default();

    let error = AgentDefinition::new("definition.intelligence.ambiguous", manifest)
        .with_provider_binding(binding_a)
        .with_provider_binding(binding_b)
        .with_model_selection(ModelSelectionPolicy::default_provider("provider.a"))
        .with_tool_call_policy(ToolCallPolicy::default())
        .with_memory_strategy(MemoryStrategy::disabled())
        .validate()
        .expect_err("two default model bindings must be rejected");

    assert!(error
        .to_string()
        .contains("multiple default provider bindings"));
}

#[test]
fn agent_definition_rejects_policy_references_without_matching_bindings() {
    let manifest = AgentManifest::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("nested agent manifest parses from agent definition");

    let error = AgentDefinition::new("definition.intelligence.missing", manifest)
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.tool.primary",
                AgentProviderFamily::Tool,
                "provider.tool.mcp",
                true,
            )
            .as_default(),
        )
        .with_model_selection(ModelSelectionPolicy::default_provider("provider.missing"))
        .with_tool_call_policy(ToolCallPolicy::default_provider("provider.tool.mcp"))
        .with_memory_strategy(MemoryStrategy::disabled())
        .validate()
        .expect_err("model policy must reference a model binding");

    assert!(error
        .to_string()
        .contains("model selection references unknown provider"));
}

#[test]
fn agent_definition_accepts_explicit_knowledge_provider_binding() {
    let manifest = AgentManifest::from_json(STANDARD_AGENT_DEFINITION_JSON)
        .expect("nested agent manifest parses from agent definition");

    let definition = AgentDefinition::new("definition.intelligence.knowledge", manifest)
        .with_provider_binding(
            AgentProviderBinding::new(
                "binding.knowledge.primary",
                AgentProviderFamily::Knowledge,
                "provider.knowledge.wiki",
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
        .validate()
        .expect("knowledge provider binding is standard");

    let knowledge = definition
        .default_binding(AgentProviderFamily::Knowledge)
        .expect("default knowledge binding is explicit");
    assert_eq!(knowledge.provider_id, "provider.knowledge.wiki");
    assert!(knowledge.supports_capability("knowledge.search"));
    assert!(knowledge.supports_capability("knowledge.read"));
    assert!(!definition.requires_provider_family(AgentProviderFamily::Knowledge));
}
