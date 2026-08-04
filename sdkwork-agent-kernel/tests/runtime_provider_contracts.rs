use sdkwork_agent_kernel::{
    AgentManifest, AgentRuntime, CapabilityManifest, KernelResult, PolicyDecision, PolicyProvider,
    PolicyRequest, ProviderHealth, ProviderManifest, RuntimeBuilder, RuntimeState, SideEffectLevel,
    ToolCall, ToolDescriptor, ToolProvider, ToolResult,
};

#[test]
fn runtime_state_is_ready_degraded_or_failed_from_capability_manifest() {
    let ready = AgentRuntime::from_capability_manifest(capability_manifest(vec![], vec![]));
    let degraded = AgentRuntime::from_capability_manifest(capability_manifest(
        vec![],
        vec!["memory.query".to_string()],
    ));
    let failed = AgentRuntime::from_capability_manifest(capability_manifest(
        vec!["policy.evaluate".to_string()],
        vec![],
    ));

    assert_eq!(ready.state(), RuntimeState::Ready);
    assert_eq!(degraded.state(), RuntimeState::Degraded);
    assert_eq!(failed.state(), RuntimeState::Failed);
}

#[test]
fn side_effectful_tool_descriptor_requires_policy_categories() {
    let descriptor = ToolDescriptor::new(
        "tool.shell.run",
        "provider.tool.fake",
        "Run Shell Command",
        SideEffectLevel::SideEffectful,
    )
    .with_policy_categories(vec![
        "tool.invoke".to_string(),
        "host.process.execute".to_string(),
    ]);

    assert!(descriptor.requires_policy());
    assert_eq!(
        descriptor.policy_categories,
        ["tool.invoke", "host.process.execute"]
    );
}

#[test]
fn tool_provider_trait_supports_deterministic_fake_provider() {
    let provider = FakeToolProvider;
    let tools = provider.list_tools();

    assert_eq!(provider.health().status, "available");
    assert_eq!(tools[0].tool_id, "tool.echo");

    let result = provider
        .invoke_tool(ToolCall::new("tool-call.1", "tool.echo", "hello"))
        .expect("tool call succeeds");

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.output, "hello");
}

#[test]
fn policy_provider_trait_returns_decision_for_request() {
    let provider = FakePolicyProvider;
    let decision = provider
        .evaluate(PolicyRequest::new(
            "policy-request.1",
            "tool.invoke",
            "tool.echo",
        ))
        .expect("policy evaluates");

    assert!(decision.is_allow());
}

#[test]
fn schema_constants_expose_machine_readable_manifest_contracts() {
    assert!(sdkwork_agent_kernel::AGENT_MANIFEST_SCHEMA.contains("SDKWork Agent Manifest"));
    assert!(sdkwork_agent_kernel::AGENT_DEFINITION_SCHEMA.contains("SDKWork Agent Definition"));
    assert!(sdkwork_agent_kernel::PROVIDER_MANIFEST_SCHEMA.contains("SDKWork Provider Manifest"));
    assert!(
        sdkwork_agent_kernel::CAPABILITY_MANIFEST_SCHEMA.contains("SDKWork Capability Manifest")
    );
}

#[test]
fn capability_manifest_metadata_defines_tool_memory_and_knowledge_spi_operations() {
    let manifest = AgentManifest::from_json(
        r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.spi-metadata",
  "name": "spi-metadata",
  "display_name": "SPI Metadata",
  "description": "Agent used to prove capability metadata.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    { "capability_id": "tool.invoke", "min_version": "0.1.0" },
    { "capability_id": "memory.write", "min_version": "0.1.0" },
    { "capability_id": "memory.delete", "min_version": "0.1.0" },
    { "capability_id": "memory.export", "min_version": "0.1.0" },
    { "capability_id": "knowledge.search", "min_version": "0.1.0" },
    { "capability_id": "knowledge.read", "min_version": "0.1.0" },
    { "capability_id": "knowledge.list", "min_version": "0.1.0" }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.tool.*", "agent.memory.*", "agent.knowledge.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#,
    )
    .expect("agent manifest parses");

    let report = RuntimeBuilder::new("runtime.spi-metadata", manifest)
        .register_provider(ProviderManifest::new(
            "provider.tool.standard",
            "tool",
            "standard-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        ))
        .register_provider(ProviderManifest::new(
            "provider.memory.standard",
            "memory",
            "standard-memory",
            "0.1.0",
            vec![
                "memory.write".to_string(),
                "memory.delete".to_string(),
                "memory.export".to_string(),
            ],
        ))
        .register_provider(ProviderManifest::new(
            "provider.knowledge.standard",
            "knowledge",
            "standard-knowledge",
            "0.1.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        ))
        .bootstrap()
        .expect("runtime bootstraps");

    let capability = |capability_id: &str| {
        report
            .runtime
            .capability_manifest()
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == capability_id)
            .expect("capability exists")
    };

    let tool_invoke = capability("tool.invoke");
    assert_eq!(tool_invoke.operations, ["invoke_tool"]);
    assert_eq!(
        tool_invoke.side_effect_level.as_deref(),
        Some("side_effectful")
    );
    assert_eq!(tool_invoke.policy_categories, ["tool.invoke"]);

    let memory_write = capability("memory.write");
    assert_eq!(memory_write.operations, ["write", "health"]);
    assert_eq!(
        memory_write.side_effect_level.as_deref(),
        Some("side_effectful")
    );
    assert_eq!(memory_write.policy_categories, ["memory.write"]);

    let memory_delete = capability("memory.delete");
    assert_eq!(memory_delete.operations, ["delete", "health"]);
    assert_eq!(
        memory_delete.side_effect_level.as_deref(),
        Some("destructive")
    );
    assert_eq!(memory_delete.policy_categories, ["memory.delete"]);

    let memory_export = capability("memory.export");
    assert_eq!(memory_export.operations, ["export", "health"]);
    assert_eq!(
        memory_export.side_effect_level.as_deref(),
        Some("read_only")
    );
    assert_eq!(memory_export.policy_categories, ["memory.read"]);

    let knowledge_search = capability("knowledge.search");
    assert_eq!(knowledge_search.operations, ["search", "health"]);
    assert_eq!(
        knowledge_search.side_effect_level.as_deref(),
        Some("read_only")
    );
    assert_eq!(knowledge_search.policy_categories, ["knowledge.search"]);

    let knowledge_read = capability("knowledge.read");
    assert_eq!(knowledge_read.operations, ["read", "health"]);
    assert_eq!(
        knowledge_read.side_effect_level.as_deref(),
        Some("read_only")
    );
    assert_eq!(knowledge_read.policy_categories, ["knowledge.read"]);

    let knowledge_list = capability("knowledge.list");
    assert_eq!(knowledge_list.operations, ["list", "health"]);
    assert_eq!(
        knowledge_list.side_effect_level.as_deref(),
        Some("read_only")
    );
    assert_eq!(knowledge_list.policy_categories, ["knowledge.list"]);
}

struct FakeToolProvider;

impl ToolProvider for FakeToolProvider {
    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.echo",
            "provider.tool.fake",
            "Echo",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, call.arguments))
    }
}

struct FakePolicyProvider;

impl PolicyProvider for FakePolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            "policy-decision.1",
            request.policy_request_id,
            "provider.policy.fake",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn capability_manifest(
    missing_required_capabilities: Vec<String>,
    degraded_capabilities: Vec<String>,
) -> CapabilityManifest {
    CapabilityManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "capability".to_string(),
        runtime_id: "runtime.local".to_string(),
        agent_id: "agent.general".to_string(),
        kernel_version: "0.1.0".to_string(),
        providers: vec![],
        capabilities: vec![],
        missing_required_capabilities,
        degraded_capabilities,
        protocol_adapters: vec![],
        security_profile: "fail_closed=true".to_string(),
        generated_at: "2026-05-27T00:00:00Z".to_string(),
    }
}
