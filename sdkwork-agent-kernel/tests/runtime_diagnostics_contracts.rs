use sdkwork_agent_kernel::{
    AgentManifest, ContextFrame, ContextProvider, KernelResult, KnowledgeDocument,
    KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod,
    KnowledgeSearchRequest, KnowledgeSearchResult, MemoryProvider, MemoryRecord, MemoryScope,
    ModelProvider, ModelRequest, ModelResponse, Plan, PlanningProvider, PolicyDecision,
    PolicyProvider, PolicyRequest, ProviderHealth, ProviderManifest, RedactionClassification,
    RuntimeBuilder, TrustLevel, AGENT_RUNTIME_DIAGNOSTICS_SCHEMA,
};

const DIAGNOSTICS_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.diagnostics",
  "name": "sdkwork-diagnostics-agent",
  "display_name": "SDKWork Diagnostics Agent",
  "description": "Agent used to prove runtime diagnostics contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    },
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

const CORE_PROVIDER_HEALTH_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.provider-health",
  "name": "sdkwork-provider-health-agent",
  "display_name": "SDKWork Provider Health Agent",
  "description": "Agent used to prove all core provider SPI expose health.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "context.collect",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "memory.query",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "knowledge.search",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "planning.create",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.provider.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const MINIMAL_READY_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.minimal-ready",
  "name": "sdkwork-minimal-ready-agent",
  "display_name": "SDKWork Minimal Ready Agent",
  "description": "Agent used to prove missing full-profile provider families do not degrade a partial runtime.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.provider.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn runtime_diagnostics_report_typed_manifest_only_health_and_missing_standard_families() {
    let manifest = AgentManifest::from_json(DIAGNOSTICS_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.agent.diagnostics", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.model.typed", "0.1.0", DegradedModelProvider)
        .register_policy_provider_manifest("provider.policy.manifest", "0.1.0")
        .bootstrap()
        .expect("runtime bootstraps");

    let diagnostics = report.runtime.diagnostics();

    assert_eq!(diagnostics.runtime_id, "runtime.agent.diagnostics");
    assert_eq!(diagnostics.agent_id, "agent.intelligence.diagnostics");
    assert_eq!(diagnostics.state, "degraded");
    assert_eq!(diagnostics.provider_count, 2);
    assert_eq!(diagnostics.capability_count, 2);
    assert_eq!(diagnostics.typed_provider_count, 1);
    assert_eq!(diagnostics.manifest_only_provider_count, 1);
    assert_eq!(
        diagnostics.missing_required_capabilities,
        Vec::<String>::new()
    );
    assert_eq!(diagnostics.degraded_capabilities, ["memory.query"]);
    assert!(diagnostics.is_degraded());

    let model = diagnostics
        .provider("provider.model.typed")
        .expect("model provider diagnostic exists");
    assert_eq!(model.provider_family, "model");
    assert_eq!(model.provider_version, "0.1.0");
    assert!(model.typed_registered);
    assert_eq!(
        model.health,
        Some(ProviderHealth {
            status: "degraded".to_string()
        })
    );
    assert!(model.health_is_degraded());
    assert_eq!(model.capabilities, ["model.chat"]);

    let policy = diagnostics
        .provider("provider.policy.manifest")
        .expect("policy provider diagnostic exists");
    assert_eq!(policy.provider_family, "policy");
    assert!(!policy.typed_registered);
    assert_eq!(policy.health, None);
    assert_eq!(
        diagnostics.manifest_only_provider_ids(),
        ["provider.policy.manifest"]
    );

    assert_eq!(
        diagnostics.missing_standard_provider_families(),
        [
            "tool",
            "context",
            "memory",
            "knowledge",
            "planning",
            "host",
            "protocol_adapter",
            "mcp",
            "skill",
            "collaboration",
            "telemetry",
            "agent_installer",
            "agent_configuration"
        ]
    );
}

#[test]
fn runtime_diagnostics_reads_health_from_policy_context_memory_and_planning_providers() {
    let manifest = AgentManifest::from_json(CORE_PROVIDER_HEALTH_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.agent.provider-health", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_policy_provider("provider.policy.typed", "0.1.0", DegradedPolicyProvider)
        .register_context_provider("provider.context.typed", "0.1.0", DegradedContextProvider)
        .register_memory_provider("provider.memory.typed", "0.1.0", DegradedMemoryProvider)
        .register_knowledge_provider(
            "provider.knowledge.typed",
            "0.1.0",
            DegradedKnowledgeProvider,
        )
        .register_planning_provider("provider.planning.typed", "0.1.0", DegradedPlanningProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    let diagnostics = report.runtime.diagnostics();

    assert_eq!(diagnostics.provider_count, 5);
    assert_eq!(diagnostics.typed_provider_count, 5);
    assert_eq!(diagnostics.manifest_only_provider_count, 0);

    for provider_id in [
        "provider.policy.typed",
        "provider.context.typed",
        "provider.memory.typed",
        "provider.knowledge.typed",
        "provider.planning.typed",
    ] {
        let provider = diagnostics
            .provider(provider_id)
            .unwrap_or_else(|| panic!("missing provider diagnostic: {provider_id}"));
        assert_eq!(
            provider.health,
            Some(ProviderHealth {
                status: "degraded".to_string()
            })
        );
        assert!(provider.health_is_degraded());
    }
}

#[test]
fn runtime_diagnostics_do_not_degrade_partial_runtimes_for_missing_full_profile_families() {
    let manifest = AgentManifest::from_json(MINIMAL_READY_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.agent.minimal-ready", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.model.typed", "0.1.0", HealthyModelProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    let diagnostics = report.runtime.diagnostics();

    assert_eq!(diagnostics.state, "ready");
    assert_eq!(diagnostics.provider_count, 1);
    assert_eq!(diagnostics.typed_provider_count, 1);
    assert_eq!(diagnostics.manifest_only_provider_count, 0);
    assert!(!diagnostics.missing_standard_provider_families().is_empty());
    assert!(!diagnostics.is_degraded());
}

#[test]
fn runtime_diagnostics_schema_is_exported_for_standard_clients() {
    assert!(AGENT_RUNTIME_DIAGNOSTICS_SCHEMA.contains("SDKWork Agent Runtime Diagnostics"));
    assert!(AGENT_RUNTIME_DIAGNOSTICS_SCHEMA.contains("agent_runtime_diagnostics"));
    assert!(AGENT_RUNTIME_DIAGNOSTICS_SCHEMA.contains("provider_diagnostics"));
}

struct DegradedModelProvider;

impl ModelProvider for DegradedModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.typed",
            "model",
            "provider.model.typed",
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
            "provider.model.typed",
            "diagnostic response",
        ))
    }
}

struct HealthyModelProvider;

impl ModelProvider for HealthyModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.typed",
            "model",
            "provider.model.typed",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.typed",
            "healthy response",
        ))
    }
}

struct DegradedPolicyProvider;

impl PolicyProvider for DegradedPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            "decision.diagnostics",
            request.policy_request_id,
            "provider.policy.typed",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}

struct DegradedContextProvider;

impl ContextProvider for DegradedContextProvider {
    fn collect(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>> {
        Ok(vec![ContextFrame::new(
            "context.diagnostics",
            session_id,
            "diagnostics",
            "context",
            TrustLevel::TrustedHost,
            RedactionClassification::Internal,
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}

struct DegradedMemoryProvider;

impl MemoryProvider for DegradedMemoryProvider {
    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        Ok(vec![MemoryRecord::new(
            "memory.diagnostics",
            scope,
            owner_context,
            "memory",
            TrustLevel::TrustedHost,
            RedactionClassification::Internal,
        )])
    }

    fn write(&mut self, _record: MemoryRecord) -> KernelResult<()> {
        Ok(())
    }

    fn delete(&mut self, _memory_record_id: &str) -> KernelResult<()> {
        Ok(())
    }

    fn export(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        self.query(scope, owner_context)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}

struct DegradedKnowledgeProvider;

impl KnowledgeProvider for DegradedKnowledgeProvider {
    fn search(&self, _request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![KnowledgeSearchResult::new(
            "knowledge.diagnostics",
            KnowledgeDocumentKind::WikiSection,
            "diagnostics knowledge",
            KnowledgeRetrievalMethod::Keyword,
        )])
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::WikiSection,
            "diagnostics knowledge",
            "knowledge",
        ))
    }

    fn list(&self, _filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(vec![KnowledgeDocument::new(
            "knowledge.diagnostics",
            KnowledgeDocumentKind::WikiSection,
            "diagnostics knowledge",
            "knowledge",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}

struct DegradedPlanningProvider;

impl PlanningProvider for DegradedPlanningProvider {
    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> Plan {
        Plan::new("plan.diagnostics", task_id, run_id, summary)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}
