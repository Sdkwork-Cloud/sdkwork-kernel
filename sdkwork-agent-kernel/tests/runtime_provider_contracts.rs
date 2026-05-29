use sdkwork_agent_kernel::{
    AgentRuntime, CapabilityManifest, KernelResult, PolicyDecision, PolicyProvider, PolicyRequest,
    ProviderHealth, RuntimeState, SideEffectLevel, ToolCall, ToolDescriptor, ToolProvider,
    ToolResult,
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
    assert!(sdkwork_agent_kernel::PROVIDER_MANIFEST_SCHEMA.contains("SDKWork Provider Manifest"));
    assert!(
        sdkwork_agent_kernel::CAPABILITY_MANIFEST_SCHEMA.contains("SDKWork Capability Manifest")
    );
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
}

fn capability_manifest(
    missing_required_capabilities: Vec<String>,
    degraded_capabilities: Vec<String>,
) -> CapabilityManifest {
    CapabilityManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "capability".to_string(),
        runtime_id: "runtime.local".to_string(),
        agent_id: "agent.intelligence.general".to_string(),
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
