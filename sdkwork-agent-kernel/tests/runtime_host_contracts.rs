use sdkwork_agent_kernel::{
    AgentKernelHost, AgentManifest, AgentRuntimeConformanceProfile, AgentRuntimeRegistration,
    KernelErrorKind, ProviderManifest, RuntimeBuilder,
};

const CODE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.code",
  "name": "sdkwork-code-agent",
  "display_name": "SDKWork Code Agent",
  "description": "Code agent loaded into a multi-agent kernel host.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const RESEARCH_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.research",
  "name": "sdkwork-research-agent",
  "display_name": "SDKWork Research Agent",
  "description": "Research agent loaded into a multi-agent kernel host.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn kernel_host_loads_multiple_agent_runtime_implementations_and_aggregates_diagnostics() {
    let code_runtime = bootstrap_manifest_runtime(
        "runtime.agent.code",
        CODE_AGENT_MANIFEST_JSON,
        "provider.model.code",
    );
    let research_runtime = bootstrap_manifest_runtime(
        "runtime.agent.research",
        RESEARCH_AGENT_MANIFEST_JSON,
        "provider.model.research",
    );

    let mut host = AgentKernelHost::new("host.local");
    host.load_runtime(AgentRuntimeRegistration::new(
        "implementation.code.local",
        code_runtime,
    ))
    .expect("code runtime loads");
    host.load_runtime(AgentRuntimeRegistration::new(
        "implementation.research.remote",
        research_runtime,
    ))
    .expect("research runtime loads");

    assert_eq!(host.host_id(), "host.local");
    assert_eq!(host.runtime_count(), 2);
    assert_eq!(
        host.runtime_ids(),
        ["runtime.agent.code", "runtime.agent.research"]
    );
    assert_eq!(
        host.runtime_slot("runtime.agent.code")
            .expect("code runtime slot exists")
            .implementation_id,
        "implementation.code.local"
    );
    assert_eq!(
        host.runtime("runtime.agent.research")
            .expect("research runtime exists")
            .capability_manifest()
            .agent_id,
        "agent.intelligence.research"
    );

    let diagnostics = host.diagnostics();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].runtime_id, "runtime.agent.code");
    assert_eq!(diagnostics[1].runtime_id, "runtime.agent.research");

    let reports = host.conformance_reports(AgentRuntimeConformanceProfile::Manifest);
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| report.is_passed()));
}

#[test]
fn kernel_host_rejects_duplicate_runtime_ids_and_can_unload_runtime() {
    let code_runtime = bootstrap_manifest_runtime(
        "runtime.agent.duplicate",
        CODE_AGENT_MANIFEST_JSON,
        "provider.model.code",
    );
    let duplicate_runtime = bootstrap_manifest_runtime(
        "runtime.agent.duplicate",
        RESEARCH_AGENT_MANIFEST_JSON,
        "provider.model.research",
    );

    let mut host = AgentKernelHost::new("host.local");
    host.load_runtime(AgentRuntimeRegistration::new(
        "implementation.code.local",
        code_runtime,
    ))
    .expect("first runtime loads");

    let duplicate_error = host
        .load_runtime(AgentRuntimeRegistration::new(
            "implementation.research.remote",
            duplicate_runtime,
        ))
        .expect_err("duplicate runtime id is rejected");
    assert_eq!(duplicate_error.kind(), KernelErrorKind::ValidationError);

    let removed = host
        .unload_runtime("runtime.agent.duplicate")
        .expect("runtime unloads");
    assert_eq!(
        removed.runtime.capability_manifest().agent_id,
        "agent.intelligence.code"
    );
    assert_eq!(host.runtime_count(), 0);
    assert!(host.runtime("runtime.agent.duplicate").is_none());
}

fn bootstrap_manifest_runtime(
    runtime_id: &str,
    manifest_json: &str,
    provider_id: &str,
) -> sdkwork_agent_kernel::AgentRuntime {
    let manifest = AgentManifest::from_json(manifest_json).expect("manifest parses");
    RuntimeBuilder::new(runtime_id, manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_provider(ProviderManifest::new(
            provider_id,
            "model",
            provider_id,
            "0.1.0",
            vec!["model.chat".to_string()],
        ))
        .bootstrap()
        .expect("runtime bootstraps")
        .runtime
}
