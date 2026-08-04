use sdkwork_agent_kernel::{
    AgentManifest, AgentRuntime, AgentRuntimeConformanceProfile, Capability, CapabilityManifest,
    KernelResult, ModelProvider, ModelRequest, ModelResponse, ProviderHealth, ProviderManifest,
    RuntimeBuilder,
};

const RUNTIME_CONFORMANCE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.runtime-conformance",
  "name": "sdkwork-runtime-conformance-agent",
  "display_name": "SDKWork Runtime Conformance Agent",
  "description": "Agent used to prove runtime conformance report contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
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
  "optional_capabilities": [
    {
      "capability_id": "memory.query",
      "min_version": "0.1.0"
    }
  ],
  "event_families": ["agent.runtime.*", "agent.provider.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn manifest_runtime_conformance_reports_capability_negotiation_without_requiring_typed_providers() {
    let manifest = AgentManifest::from_json(RUNTIME_CONFORMANCE_AGENT_MANIFEST_JSON).unwrap();
    let runtime = RuntimeBuilder::new("runtime.agent.conformance", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider_manifest("provider.manifest", "0.1.0")
        .register_policy_provider_manifest("provider.policy.manifest", "0.1.0")
        .bootstrap()
        .expect("runtime bootstraps")
        .runtime;

    let report = runtime.conformance_report(AgentRuntimeConformanceProfile::Manifest);

    assert_eq!(report.profile_id, "runtime-manifest");
    assert_eq!(report.implementation_id, "runtime.agent.conformance");
    assert_eq!(
        report.required_capabilities,
        ["model.chat", "policy.evaluate"]
    );
    assert!(report.is_passed());
    assert_eq!(
        report
            .case("agent.conformance.runtime.required_capabilities.available")
            .expect("required capability case exists")
            .status
            .as_str(),
        "passed"
    );
    assert_eq!(
        report
            .case("agent.conformance.runtime.local_providers.typed")
            .expect("local provider case exists")
            .status
            .as_str(),
        "skipped"
    );
}

#[test]
fn local_runtime_conformance_detects_manifest_only_missing_and_unhealthy_providers() {
    let manifest = AgentManifest::from_json(RUNTIME_CONFORMANCE_AGENT_MANIFEST_JSON).unwrap();
    let runtime = RuntimeBuilder::new("runtime.agent.conformance", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.typed", "0.1.0", DegradedModelProvider)
        .register_policy_provider_manifest("provider.policy.manifest", "0.1.0")
        .bootstrap()
        .expect("runtime bootstraps")
        .runtime;

    let report = runtime.conformance_report(AgentRuntimeConformanceProfile::LocalRuntime);

    assert_eq!(report.profile_id, "runtime-local");
    assert!(!report.is_passed());
    assert_eq!(report.failed_count(), 2);
    assert_eq!(
        report.failed_case_ids(),
        [
            "agent.conformance.runtime.local_providers.typed",
            "agent.conformance.runtime.local_providers.health_available"
        ]
    );
    assert_eq!(
        report
            .case("agent.conformance.runtime.optional_capabilities.available")
            .expect("optional capability case exists")
            .status
            .as_str(),
        "skipped"
    );
    assert_eq!(
        report
            .case("agent.conformance.runtime.local_providers.typed")
            .expect("typed provider case exists")
            .message,
        "manifest-only providers: provider.policy.manifest"
    );
}

#[test]
fn runtime_conformance_rejects_capability_ids_that_are_not_lowercase_namespaces() {
    let runtime = AgentRuntime::from_capability_manifest(CapabilityManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "capability".to_string(),
        runtime_id: "runtime.agent.invalid-capability".to_string(),
        agent_id: "agent.invalid-capability".to_string(),
        kernel_version: "0.1.0".to_string(),
        providers: vec![ProviderManifest::new(
            "provider.invalid",
            "model",
            "provider.invalid",
            "0.1.0",
            vec!["Model.Chat".to_string()],
        )],
        capabilities: vec![Capability {
            capability_id: "Model.Chat".to_string(),
            version: "0.1.0".to_string(),
            provider_id: "provider.invalid".to_string(),
            status: "available".to_string(),
            required: true,
            operations: Vec::new(),
            side_effect_level: None,
            policy_categories: Vec::new(),
        }],
        missing_required_capabilities: Vec::new(),
        degraded_capabilities: Vec::new(),
        protocol_adapters: Vec::new(),
        security_profile: "fail_closed=true".to_string(),
        generated_at: "2026-05-29T00:00:00Z".to_string(),
    });

    let report = runtime.conformance_report(AgentRuntimeConformanceProfile::Manifest);

    let namespace_case = report
        .case("agent.conformance.runtime.capabilities.namespaced")
        .expect("namespace case exists");
    assert_eq!(namespace_case.status.as_str(), "failed");
    assert!(namespace_case.message.contains("Model.Chat"));
}

#[test]
fn runtime_conformance_rejects_protocol_adapter_exposure_outside_effective_capabilities() {
    let runtime = AgentRuntime::from_capability_manifest(CapabilityManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "capability".to_string(),
        runtime_id: "runtime.agent.invalid-adapter-exposure".to_string(),
        agent_id: "agent.invalid-adapter-exposure".to_string(),
        kernel_version: "0.1.0".to_string(),
        providers: vec![ProviderManifest::new(
            "adapter.rpc.invalid",
            "protocol_adapter",
            "adapter.rpc.invalid",
            "0.1.0",
            vec!["protocol.map".to_string(), "model.chat".to_string()],
        )],
        capabilities: vec![Capability {
            capability_id: "protocol.map".to_string(),
            version: "0.1.0".to_string(),
            provider_id: "adapter.rpc.invalid".to_string(),
            status: "available".to_string(),
            required: true,
            operations: Vec::new(),
            side_effect_level: None,
            policy_categories: Vec::new(),
        }],
        missing_required_capabilities: Vec::new(),
        degraded_capabilities: Vec::new(),
        protocol_adapters: vec!["adapter.rpc.invalid".to_string()],
        security_profile: "fail_closed=true".to_string(),
        generated_at: "2026-06-09T00:00:00Z".to_string(),
    });

    let report = runtime.conformance_report(AgentRuntimeConformanceProfile::Manifest);

    let exposure_case = report
        .case("agent.conformance.runtime.protocol_adapters.exposure_subset")
        .expect("protocol adapter exposure subset case exists");
    assert_eq!(exposure_case.status.as_str(), "failed");
    assert!(exposure_case
        .message
        .contains("adapter.rpc.invalid:model.chat"));
}

struct DegradedModelProvider;

impl ModelProvider for DegradedModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.typed",
            "model",
            "provider.typed",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.typed",
            "runtime conformance response",
        ))
    }
}
