use sdkwork_agent_kernel::{
    AgentManifest, Capability, CapabilityManifest, KernelEvent, KernelEventSeverity, KernelResult,
    ModelProvider, ModelRequest, ModelResponse, PolicyDecision, PolicyDecisionValue,
    ProviderHealth, ProviderManifest,
};

const AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.general",
  "name": "sdkwork-general-agent",
  "display_name": "SDKWork General Agent",
  "description": "Provider-neutral agent runtime.",
  "version": "0.1.0",
  "domain": "intelligence",
  "kernel_compatibility": {
    "agent_kernel": ">=0.1.0 <0.2.0"
  },
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "provider_requirements": [],
  "protocol_adapters": [],
  "security_profile": {
    "fail_closed": true,
    "required_policy_categories": ["model.invoke"],
    "redaction_required": true
  },
  "runtime_profile": {
    "modes": ["local", "desktop"]
  },
  "event_families": ["agent.session.*", "agent.task.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn parses_agent_manifest_and_reports_required_capabilities() {
    let manifest = AgentManifest::from_json(AGENT_MANIFEST_JSON).expect("manifest parses");

    assert_eq!(manifest.agent_id, "agent.general");
    assert!(manifest.requires_capability("model.chat"));
    assert!(manifest.requires_capability("policy.evaluate"));
    assert!(!manifest.requires_capability("memory.query"));
}

#[test]
fn capability_manifest_reports_missing_required_capabilities() {
    let manifest = CapabilityManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "capability".to_string(),
        runtime_id: "runtime.local".to_string(),
        agent_id: "agent.general".to_string(),
        kernel_version: "0.1.0".to_string(),
        providers: vec![],
        capabilities: vec![Capability {
            capability_id: "model.chat".to_string(),
            version: "0.1.0".to_string(),
            provider_id: "provider.fake".to_string(),
            status: "available".to_string(),
            required: true,
            operations: vec!["invoke".to_string()],
            side_effect_level: None,
            policy_categories: vec![],
        }],
        missing_required_capabilities: vec!["policy.evaluate".to_string()],
        degraded_capabilities: vec![],
        protocol_adapters: vec![],
        security_profile: "fail_closed=true".to_string(),
        generated_at: "2026-05-27T00:00:00Z".to_string(),
    };

    assert!(!manifest.is_ready());
    assert_eq!(
        manifest.missing_required_capabilities(),
        ["policy.evaluate"]
    );
}

#[test]
fn policy_decision_helpers_distinguish_allow_deny_and_approval() {
    let allow = PolicyDecision::allow("decision.allow", "request.1", "policy.test");
    let deny = PolicyDecision::deny(
        "decision.deny",
        "request.2",
        "policy.test",
        "tool.not_allowed",
    );
    let approval = PolicyDecision::needs_approval(
        "decision.approval",
        "request.3",
        "policy.test",
        "destructive_action",
    );

    assert!(allow.is_allow());
    assert!(!deny.is_allow());
    assert_eq!(deny.decision, PolicyDecisionValue::Deny);
    assert!(approval.is_needs_approval());
}

#[test]
fn kernel_event_exposes_type_and_trace_metadata() {
    let event = KernelEvent::new(
        "event.1",
        "agent.task.created",
        KernelEventSeverity::Info,
        "task_id=task.1",
    )
    .with_trace("trace.1", "span.1");

    assert_eq!(event.event_type, "agent.task.created");
    assert_eq!(event.trace_context.as_ref().unwrap().trace_id, "trace.1");
}

struct FakeModelProvider;

impl ModelProvider for FakeModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.fake",
            "model",
            "sdkwork-fake-model",
            "0.1.0",
            vec!["chat".to_string(), "streaming".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.fake",
            "hello from fake model",
        ))
    }
}

#[test]
fn model_provider_trait_supports_deterministic_fake_provider() {
    let provider = FakeModelProvider;
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_id, "provider.fake");
    assert_eq!(provider.health().status, "available");

    let response = provider
        .invoke(ModelRequest::new(
            "model-request.1",
            vec!["hello".to_string()],
        ))
        .expect("fake model responds");

    assert_eq!(response.provider_id, "provider.fake");
    assert_eq!(response.messages, ["hello from fake model"]);
}
