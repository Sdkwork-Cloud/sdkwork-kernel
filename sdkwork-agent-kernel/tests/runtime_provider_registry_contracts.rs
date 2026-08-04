use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigValue, AgentConfiguration,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentInstallPlan, AgentInstallReport,
    AgentInstallRequest, AgentInstallStep, AgentInstallStepKind, AgentInstallation, AgentInstaller,
    AgentPackageSource, AgentTask, AgentUninstallPlan, AgentUninstallReport, AgentUninstallRequest,
    AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest, ContextFrame, ContextProvider,
    FilesystemRequest, FilesystemResult, HostProvider, KernelErrorKind, KernelEvent, KernelResult,
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult, MemoryProvider,
    MemoryRecord, MemoryScope, ModelProvider, ModelRequest, ModelResponse, Plan, PlanningProvider,
    PolicyCategory, PolicyDecision, PolicyProvider, PolicyRequest, ProtocolAdapter,
    ProtocolAdapterAuthMode, ProtocolAdapterManifest, ProtocolAdapterRequest,
    ProtocolAdapterResponse, ProtocolFamily, ProtocolStreamUpdate, ProtocolTransport,
    ProviderHealth, ProviderManifest, ProviderSecretValue, RedactionClassification, RuntimeBuilder,
    RuntimeState, SecretRef, TelemetryProvider, ToolCall, ToolDescriptor, ToolProvider, ToolResult,
    TrustLevel,
};
use std::sync::{Arc, Mutex};

const INSTALLABLE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.registry",
  "name": "sdkwork-registry-agent",
  "display_name": "SDKWork Registry Agent",
  "description": "Agent used to prove runtime typed provider registry contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "agent.install",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "agent.configure",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [
    {
      "capability_id": "agent.uninstall",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "agent.upgrade",
      "min_version": "0.1.0"
    }
  ],
  "event_families": ["agent.runtime.*", "agent.install.*", "agent.configure.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const CORE_SPI_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.core-spi",
  "name": "sdkwork-core-spi-agent",
  "display_name": "SDKWork Core SPI Agent",
  "description": "Agent used to prove runtime typed core provider registry contracts.",
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
    },
    {
      "capability_id": "host.filesystem",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "protocol.map",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [
    {
      "capability_id": "telemetry.record",
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
fn runtime_registry_invokes_typed_agent_installer_and_configuration_provider() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_agent_installer(
            "provider.agent.installer.typed",
            "0.1.0",
            FakeRuntimeAgentInstaller,
        )
        .register_agent_configuration(
            "provider.agent.configuration.typed",
            "0.1.0",
            FakeRuntimeAgentConfigurationProvider,
        )
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let install_request = AgentInstallRequest::new(
        "install.registry.1",
        "agent.registry",
        "0.1.0",
        AgentPackageSource::registry("sdkwork", "agent.registry", "0.1.0"),
    )
    .with_configuration(valid_configuration());

    let installer = report
        .runtime
        .agent_installer()
        .expect("typed installer provider is registered");
    let plan = installer
        .plan_install(&install_request)
        .expect("typed installer plans install");
    assert_eq!(plan.plan_id, "plan.runtime.install");
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentInstall.as_str().to_string()]
    );

    let install_report = installer
        .install(install_request)
        .expect("typed installer installs");
    assert_eq!(install_report.agent_id, "agent.registry");

    let configuration_provider = report
        .runtime
        .agent_configuration_provider()
        .expect("typed configuration provider is registered");
    let spec = configuration_provider
        .configuration_spec("agent.registry")
        .expect("typed configuration spec loads");
    assert!(spec.required_keys().contains(&"agent.display_name"));
    assert!(spec.required_keys().contains(&"auth.login.username"));
    assert!(spec.required_keys().contains(&"auth.login.password"));
    assert!(spec.required_keys().contains(&"llm.openai.api_key"));

    let validation = configuration_provider
        .validate_configuration(&valid_configuration())
        .expect("typed configuration provider validates");
    assert!(validation.is_valid());

    let capability_manifest = report.runtime.capability_manifest();
    let installer_manifest = capability_manifest
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.agent.installer.typed")
        .expect("installer provider manifest is registered");
    assert_eq!(installer_manifest.provider_family, "agent_installer");
    assert_eq!(installer_manifest.name, "typed-agent-installer");
    assert_eq!(installer_manifest.version, "0.1.0");
    assert_eq!(
        installer_manifest.capabilities,
        ["agent.install", "agent.uninstall", "agent.upgrade"]
    );

    let configuration_manifest = capability_manifest
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.agent.configuration.typed")
        .expect("configuration provider manifest is registered");
    assert_eq!(
        configuration_manifest.provider_family,
        "agent_configuration"
    );
    assert_eq!(configuration_manifest.name, "typed-agent-configuration");
    assert_eq!(configuration_manifest.version, "0.1.0");
    assert_eq!(configuration_manifest.capabilities, ["agent.configure"]);
}

#[test]
fn runtime_registry_reports_provider_unavailable_when_only_manifest_provider_is_registered() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_agent_installer_provider("provider.agent.installer.local", "0.1.0")
        .register_agent_configuration_provider("provider.agent.configuration.local", "0.1.0")
        .bootstrap()
        .expect("manifest-only runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let installer_error = match report.runtime.agent_installer() {
        Ok(_) => panic!("typed installer instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(installer_error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(
        installer_error.provider_id(),
        Some("provider.agent.installer.local")
    );

    let configuration_error = match report.runtime.agent_configuration_provider() {
        Ok(_) => panic!("typed configuration instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(
        configuration_error.kind(),
        KernelErrorKind::ProviderUnavailable
    );
    assert_eq!(
        configuration_error.provider_id(),
        Some("provider.agent.configuration.local")
    );
}

#[test]
fn runtime_registry_invokes_typed_core_spi_providers() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.typed", "0.1.0", FakeModelProvider)
        .register_tool_provider(
            "provider.tool.typed",
            "0.1.0",
            FakeToolProvider::new("provider.tool.typed", "tool.echo", "tool response"),
        )
        .register_policy_provider(
            "provider.policy.typed",
            "0.1.0",
            FakePolicyProvider::new("provider.policy.typed"),
        )
        .register_context_provider("provider.context.typed", "0.1.0", FakeContextProvider)
        .register_memory_provider("provider.memory.typed", "0.1.0", FakeMemoryProvider)
        .register_knowledge_provider("provider.knowledge.typed", "0.1.0", FakeKnowledgeProvider)
        .register_planning_provider("provider.planning.typed", "0.1.0", FakePlanningProvider)
        .register_host_provider("provider.host.typed", "0.1.0", FakeHostProvider)
        .register_protocol_adapter(
            "adapter.protocol.typed",
            "0.1.0",
            FakeProtocolAdapter::new(
                "adapter.protocol.typed",
                "task.protocol",
                "response.protocol",
            ),
        )
        .register_telemetry_provider("provider.telemetry.typed", "0.1.0", FakeTelemetryProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let model_response = report
        .runtime
        .model_provider()
        .expect("model provider is registered")
        .invoke(ModelRequest::new("model.1", vec!["hello".to_string()]))
        .expect("model provider invokes");
    assert_eq!(model_response.provider_id, "provider.typed");

    let tool_result = report
        .runtime
        .tool_provider()
        .expect("tool provider is registered")
        .invoke_tool(ToolCall::new("tool.1", "tool.echo", "{}"))
        .expect("tool provider invokes");
    assert_eq!(tool_result.tool_call_id, "tool.1");
    let tool_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.tool.typed")
        .expect("tool provider manifest is registered");
    assert_eq!(tool_manifest.provider_family, "tool");
    assert_eq!(tool_manifest.version, "0.1.0");
    assert_eq!(
        tool_manifest.capabilities,
        ["tool.invoke", "tool.streaming", "tool.cancellation"]
    );

    let decision = report
        .runtime
        .policy_provider()
        .expect("policy provider is registered")
        .evaluate(PolicyRequest::new("policy.1", "tool.invoke", "tool.echo"))
        .expect("policy provider evaluates");
    assert!(decision.is_allow());
    let policy_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.policy.typed")
        .expect("policy provider manifest is registered");
    assert_eq!(policy_manifest.provider_family, "policy");
    assert_eq!(policy_manifest.name, "typed-policy-provider");
    assert_eq!(policy_manifest.version, "0.1.0");
    assert_eq!(policy_manifest.capabilities, ["policy.evaluate"]);

    let context = report
        .runtime
        .context_provider()
        .expect("context provider is registered")
        .collect("session.1")
        .expect("context provider collects");
    assert_eq!(context[0].context_frame_id, "context.1");
    let context_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.context.typed")
        .expect("context provider manifest is registered");
    assert_eq!(context_manifest.provider_family, "context");
    assert_eq!(context_manifest.name, "typed-context-provider");
    assert_eq!(context_manifest.version, "0.1.0");
    assert_eq!(context_manifest.capabilities, ["context.collect"]);

    let memory_provider = report
        .runtime
        .memory_provider()
        .expect("memory provider is registered");
    let memory = memory_provider
        .lock()
        .expect("memory provider lock is available")
        .query(MemoryScope::Session, "session.1")
        .expect("memory provider queries");
    assert_eq!(memory[0].memory_record_id, "memory.1");

    let knowledge = report
        .runtime
        .knowledge_provider()
        .expect("knowledge provider is registered")
        .search(
            KnowledgeSearchRequest::new("agent spi").with_method(KnowledgeRetrievalMethod::Keyword),
        )
        .expect("knowledge provider searches");
    assert_eq!(knowledge[0].document_id, "knowledge.1");
    assert_eq!(
        report.runtime.knowledge_provider_ids(),
        ["provider.knowledge.typed"]
    );

    let plan = report
        .runtime
        .planning_provider()
        .expect("planning provider is registered")
        .create_plan("task.1", "run.1", "plan")
        .expect("plan creation succeeds");
    assert_eq!(plan.plan_id, "plan.core");
    let planning_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.planning.typed")
        .expect("planning provider manifest is registered");
    assert_eq!(planning_manifest.provider_family, "planning");
    assert_eq!(planning_manifest.name, "typed-planning-provider");
    assert_eq!(planning_manifest.version, "0.1.0");
    assert_eq!(planning_manifest.capabilities, ["planning.create"]);

    let file = report
        .runtime
        .host_provider()
        .expect("host provider is registered")
        .filesystem(FilesystemRequest::read("fs.1", "workspace/README.md"))
        .expect("host provider reads");
    assert_eq!(file.content.as_deref(), Some("readme"));
    let host_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.host.typed")
        .expect("host provider manifest is registered");
    assert_eq!(host_manifest.provider_family, "host");
    assert_eq!(host_manifest.version, "0.1.0");
    assert_eq!(host_manifest.capabilities, ["host.filesystem"]);

    let task = report
        .runtime
        .protocol_adapter()
        .expect("protocol adapter is registered")
        .map_request_to_task(ProtocolAdapterRequest::new(
            "protocol.1",
            ProtocolFamily::Http,
            "task.create",
            "hello",
        ))
        .expect("protocol adapter maps request");
    assert_eq!(task.task_id, "task.protocol");
    let protocol_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "adapter.protocol.typed")
        .expect("protocol adapter manifest is registered");
    assert_eq!(protocol_manifest.provider_family, "protocol_adapter");
    assert_eq!(protocol_manifest.name, "adapter.protocol.typed");
    assert_eq!(protocol_manifest.version, "0.1.0");
    assert_eq!(
        protocol_manifest.capabilities,
        ["protocol.map", "protocol.stream"]
    );

    let telemetry_provider = report
        .runtime
        .telemetry_provider()
        .expect("telemetry provider is registered");
    assert_eq!(
        telemetry_provider
            .lock()
            .expect("telemetry provider lock is available")
            .health()
            .status,
        "available"
    );
    let telemetry_manifest = report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .find(|provider| provider.provider_id == "provider.telemetry.typed")
        .expect("telemetry provider manifest is registered");
    assert_eq!(telemetry_manifest.provider_family, "telemetry");
    assert_eq!(telemetry_manifest.name, "typed-telemetry-provider");
    assert_eq!(telemetry_manifest.version, "0.1.0");
    assert_eq!(telemetry_manifest.capabilities, ["telemetry.record"]);

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_id == "provider.typed"
            && provider.provider_family == "model"));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "model.chat"
            && capability.provider_id == "provider.typed"));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "knowledge.search"
            && capability.provider_id == "provider.knowledge.typed"
            && capability.side_effect_level.as_deref() == Some("read_only")));
}

#[test]
fn runtime_registry_supports_multiple_tool_policy_and_protocol_adapter_providers() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core.multi-family", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.typed", "0.1.0", FakeModelProvider)
        .register_tool_provider(
            "provider.tool.alpha",
            "0.1.0",
            FakeToolProvider::new("provider.tool.alpha", "tool.alpha", "tool alpha response"),
        )
        .register_tool_provider(
            "provider.tool.beta",
            "0.1.0",
            FakeToolProvider::new("provider.tool.beta", "tool.beta", "tool beta response"),
        )
        .register_policy_provider(
            "provider.policy.alpha",
            "0.1.0",
            FakePolicyProvider::new("provider.policy.alpha"),
        )
        .register_policy_provider(
            "provider.policy.beta",
            "0.1.0",
            FakePolicyProvider::new("provider.policy.beta"),
        )
        .register_context_provider("provider.context.typed", "0.1.0", FakeContextProvider)
        .register_memory_provider("provider.memory.typed", "0.1.0", FakeMemoryProvider)
        .register_knowledge_provider("provider.knowledge.typed", "0.1.0", FakeKnowledgeProvider)
        .register_planning_provider("provider.planning.typed", "0.1.0", FakePlanningProvider)
        .register_host_provider("provider.host.typed", "0.1.0", FakeHostProvider)
        .register_protocol_adapter(
            "adapter.protocol.alpha",
            "0.1.0",
            FakeProtocolAdapter::new("adapter.protocol.alpha", "task.alpha", "response.alpha"),
        )
        .register_protocol_adapter(
            "adapter.protocol.beta",
            "0.1.0",
            FakeProtocolAdapter::new("adapter.protocol.beta", "task.beta", "response.beta"),
        )
        .register_telemetry_provider("provider.telemetry.typed", "0.1.0", FakeTelemetryProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(
        report.runtime.tool_provider_ids(),
        ["provider.tool.alpha", "provider.tool.beta"]
    );
    assert_eq!(
        report
            .runtime
            .tool_provider()
            .expect("default tool provider")
            .list_tools()[0]
            .tool_id,
        "tool.alpha"
    );
    assert_eq!(
        report
            .runtime
            .tool_provider_by_id("provider.tool.beta")
            .expect("tool provider by id")
            .list_tools()[0]
            .tool_id,
        "tool.beta"
    );

    assert_eq!(
        report.runtime.policy_provider_ids(),
        ["provider.policy.alpha", "provider.policy.beta"]
    );
    assert_eq!(
        report
            .runtime
            .policy_provider()
            .expect("default policy provider")
            .evaluate(PolicyRequest::new(
                "policy.multi.1",
                "tool.invoke",
                "tool.alpha"
            ))
            .expect("default policy decision")
            .policy_provider_id,
        "provider.policy.alpha"
    );
    assert_eq!(
        report
            .runtime
            .policy_provider_by_id("provider.policy.beta")
            .expect("policy provider by id")
            .evaluate(PolicyRequest::new(
                "policy.multi.2",
                "tool.invoke",
                "tool.beta"
            ))
            .expect("policy decision by id")
            .policy_provider_id,
        "provider.policy.beta"
    );

    assert_eq!(
        report.runtime.protocol_adapter_ids(),
        ["adapter.protocol.alpha", "adapter.protocol.beta"]
    );
    assert_eq!(
        report
            .runtime
            .protocol_adapter()
            .expect("default protocol adapter")
            .manifest()
            .adapter_id,
        "adapter.protocol.alpha"
    );
    assert_eq!(
        report
            .runtime
            .protocol_adapter_by_id("adapter.protocol.beta")
            .expect("protocol adapter by id")
            .manifest()
            .adapter_id,
        "adapter.protocol.beta"
    );

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "tool"
            && provider.provider_id == "provider.tool.alpha"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "tool"
            && provider.provider_id == "provider.tool.beta"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "policy"
            && provider.provider_id == "provider.policy.alpha"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "policy"
            && provider.provider_id == "provider.policy.beta"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "protocol_adapter"
            && provider.provider_id == "adapter.protocol.alpha"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "protocol_adapter"
            && provider.provider_id == "adapter.protocol.beta"));
}

#[test]
fn runtime_registry_supports_multiple_context_and_planning_providers() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core.multi-context-planning", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.typed", "0.1.0", FakeModelProvider)
        .register_tool_provider(
            "provider.tool.typed",
            "0.1.0",
            FakeToolProvider::new("provider.tool.typed", "tool.echo", "tool response"),
        )
        .register_policy_provider(
            "provider.policy.typed",
            "0.1.0",
            FakePolicyProvider::new("provider.policy.typed"),
        )
        .register_context_provider(
            "provider.context.vector",
            "0.1.0",
            NamedContextProvider::new("context.vector"),
        )
        .register_context_provider(
            "provider.context.workspace",
            "0.1.0",
            NamedContextProvider::new("context.workspace"),
        )
        .register_memory_provider("provider.memory.typed", "0.1.0", FakeMemoryProvider)
        .register_knowledge_provider("provider.knowledge.typed", "0.1.0", FakeKnowledgeProvider)
        .register_planning_provider(
            "provider.planning.model",
            "0.1.0",
            NamedPlanningProvider::new("plan.model"),
        )
        .register_planning_provider(
            "provider.planning.rules",
            "0.1.0",
            NamedPlanningProvider::new("plan.rules"),
        )
        .register_host_provider("provider.host.typed", "0.1.0", FakeHostProvider)
        .register_protocol_adapter(
            "adapter.protocol.typed",
            "0.1.0",
            FakeProtocolAdapter::new(
                "adapter.protocol.typed",
                "task.protocol",
                "response.protocol",
            ),
        )
        .register_telemetry_provider("provider.telemetry.typed", "0.1.0", FakeTelemetryProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(
        report.runtime.context_provider_ids(),
        ["provider.context.vector", "provider.context.workspace"]
    );
    assert_eq!(
        report
            .runtime
            .context_provider()
            .expect("default context provider")
            .collect("session.1")
            .expect("default context collects")[0]
            .context_frame_id,
        "context.vector"
    );
    assert_eq!(
        report
            .runtime
            .context_provider_by_id("provider.context.workspace")
            .expect("context provider by id")
            .collect("session.1")
            .expect("selected context collects")[0]
            .context_frame_id,
        "context.workspace"
    );

    assert_eq!(
        report.runtime.planning_provider_ids(),
        ["provider.planning.model", "provider.planning.rules"]
    );
    assert_eq!(
        report
            .runtime
            .planning_provider()
            .expect("default planning provider")
            .create_plan("task.1", "run.1", "plan")
            .expect("plan creation succeeds")
            .plan_id,
        "plan.model"
    );
    assert_eq!(
        report
            .runtime
            .planning_provider_by_id("provider.planning.rules")
            .expect("planning provider by id")
            .create_plan("task.1", "run.1", "plan")
            .expect("plan creation succeeds")
            .plan_id,
        "plan.rules"
    );

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "context"
            && provider.provider_id == "provider.context.vector"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "context"
            && provider.provider_id == "provider.context.workspace"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "planning"
            && provider.provider_id == "provider.planning.model"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "planning"
            && provider.provider_id == "provider.planning.rules"));
}

#[test]
fn runtime_registry_supports_multiple_memory_host_and_telemetry_providers() {
    let telemetry_events = Arc::new(Mutex::new(Vec::new()));
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core.multi-stateful", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider("provider.typed", "0.1.0", FakeModelProvider)
        .register_tool_provider(
            "provider.tool.typed",
            "0.1.0",
            FakeToolProvider::new("provider.tool.typed", "tool.echo", "tool response"),
        )
        .register_policy_provider(
            "provider.policy.typed",
            "0.1.0",
            FakePolicyProvider::new("provider.policy.typed"),
        )
        .register_context_provider("provider.context.typed", "0.1.0", FakeContextProvider)
        .register_memory_provider(
            "provider.memory.session",
            "0.1.0",
            NamedMemoryProvider::new("memory.session"),
        )
        .register_memory_provider(
            "provider.memory.vector",
            "0.1.0",
            NamedMemoryProvider::new("memory.vector"),
        )
        .register_knowledge_provider("provider.knowledge.typed", "0.1.0", FakeKnowledgeProvider)
        .register_planning_provider("provider.planning.typed", "0.1.0", FakePlanningProvider)
        .register_host_provider(
            "provider.host.local",
            "0.1.0",
            NamedHostProvider::new("provider.host.local", "local readme"),
        )
        .register_host_provider(
            "provider.host.remote",
            "0.1.0",
            NamedHostProvider::new("provider.host.remote", "remote readme"),
        )
        .register_protocol_adapter(
            "adapter.protocol.typed",
            "0.1.0",
            FakeProtocolAdapter::new(
                "adapter.protocol.typed",
                "task.protocol",
                "response.protocol",
            ),
        )
        .register_telemetry_provider(
            "provider.telemetry.audit",
            "0.1.0",
            RecordingTelemetryProvider::new("provider.telemetry.audit", telemetry_events.clone()),
        )
        .register_telemetry_provider(
            "provider.telemetry.otlp",
            "0.1.0",
            RecordingTelemetryProvider::new("provider.telemetry.otlp", telemetry_events.clone()),
        )
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(
        report.runtime.memory_provider_ids(),
        ["provider.memory.session", "provider.memory.vector"]
    );
    assert_eq!(
        report
            .runtime
            .memory_provider()
            .expect("default memory provider")
            .lock()
            .expect("default memory lock")
            .query(MemoryScope::Session, "session.1")
            .expect("default memory query")[0]
            .memory_record_id,
        "memory.session"
    );
    assert_eq!(
        report
            .runtime
            .memory_provider_by_id("provider.memory.vector")
            .expect("memory provider by id")
            .lock()
            .expect("selected memory lock")
            .query(MemoryScope::Session, "session.1")
            .expect("selected memory query")[0]
            .memory_record_id,
        "memory.vector"
    );

    assert_eq!(
        report.runtime.host_provider_ids(),
        ["provider.host.local", "provider.host.remote"]
    );
    assert_eq!(
        report
            .runtime
            .host_provider()
            .expect("default host provider")
            .filesystem(FilesystemRequest::read("fs.default", "workspace/README.md"))
            .expect("default host reads")
            .content
            .as_deref(),
        Some("local readme")
    );
    assert_eq!(
        report
            .runtime
            .host_provider_by_id("provider.host.remote")
            .expect("host provider by id")
            .filesystem(FilesystemRequest::read(
                "fs.selected",
                "workspace/README.md"
            ))
            .expect("selected host reads")
            .content
            .as_deref(),
        Some("remote readme")
    );

    assert_eq!(
        report.runtime.telemetry_provider_ids(),
        ["provider.telemetry.audit", "provider.telemetry.otlp"]
    );
    report
        .runtime
        .telemetry_provider()
        .expect("default telemetry provider")
        .lock()
        .expect("default telemetry lock")
        .record_event(KernelEvent::new(
            "event.default",
            "agent.test.default",
            sdkwork_agent_kernel::KernelEventSeverity::Info,
            "default",
        ))
        .expect("default telemetry records");
    report
        .runtime
        .telemetry_provider_by_id("provider.telemetry.otlp")
        .expect("telemetry provider by id")
        .lock()
        .expect("selected telemetry lock")
        .record_event(KernelEvent::new(
            "event.selected",
            "agent.test.selected",
            sdkwork_agent_kernel::KernelEventSeverity::Info,
            "selected",
        ))
        .expect("selected telemetry records");
    assert_eq!(
        telemetry_events
            .lock()
            .expect("telemetry event sink lock")
            .as_slice(),
        [
            "provider.telemetry.audit:event.default",
            "provider.telemetry.otlp:event.selected"
        ]
    );

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "memory"
            && provider.provider_id == "provider.memory.session"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "memory"
            && provider.provider_id == "provider.memory.vector"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "host"
            && provider.provider_id == "provider.host.local"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "host"
            && provider.provider_id == "provider.host.remote"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "telemetry"
            && provider.provider_id == "provider.telemetry.audit"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "telemetry"
            && provider.provider_id == "provider.telemetry.otlp"));
}

#[test]
fn runtime_registry_reports_provider_unavailable_for_manifest_only_core_spi_provider() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider_manifest("provider.manifest", "0.1.0")
        .register_tool_provider_manifest("provider.tool.manifest", "0.1.0")
        .register_policy_provider_manifest("provider.policy.manifest", "0.1.0")
        .register_context_provider_manifest("provider.context.manifest", "0.1.0")
        .register_memory_provider_manifest("provider.memory.manifest", "0.1.0")
        .register_knowledge_provider_manifest("provider.knowledge.manifest", "0.1.0")
        .register_planning_provider_manifest("provider.planning.manifest", "0.1.0")
        .register_host_provider_manifest("provider.host.manifest", "0.1.0")
        .register_protocol_adapter_manifest("adapter.protocol.manifest", "0.1.0")
        .register_telemetry_provider_manifest("provider.telemetry.manifest", "0.1.0")
        .bootstrap()
        .expect("manifest-only runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let error = match report.runtime.model_provider() {
        Ok(_) => panic!("typed model instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("provider.manifest"));

    let error = match report.runtime.protocol_adapter() {
        Ok(_) => panic!("typed protocol adapter instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("adapter.protocol.manifest"));
    assert!(report
        .runtime
        .capability_manifest()
        .protocol_adapters
        .contains(&"adapter.protocol.manifest".to_string()));

    let error = match report.runtime.knowledge_provider() {
        Ok(_) => panic!("typed knowledge instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("provider.knowledge.manifest"));
}

struct FakeRuntimeAgentConfigurationProvider;

impl AgentConfigurationProvider for FakeRuntimeAgentConfigurationProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.agent.configuration.typed",
            "agent_configuration",
            "typed-agent-configuration",
            "9.9.9",
            vec!["agent.configure".to_string()],
        )
    }

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        Ok(configuration_spec(agent_id))
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationValidation> {
        Ok(configuration_spec(&configuration.agent_id).validate(configuration))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeModelProvider;

impl ModelProvider for FakeModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider("provider.typed", "model", vec!["model.chat"])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.typed",
            "model response",
        ))
    }
}

struct FakeToolProvider {
    provider_id: &'static str,
    tool_id: &'static str,
    response: &'static str,
}

impl FakeToolProvider {
    fn new(provider_id: &'static str, tool_id: &'static str, response: &'static str) -> Self {
        Self {
            provider_id,
            tool_id,
            response,
        }
    }
}

impl ToolProvider for FakeToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider(
            self.provider_id,
            "tool",
            vec!["tool.invoke", "tool.streaming", "tool.cancellation"],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            self.tool_id,
            self.provider_id,
            "Echo",
            sdkwork_agent_kernel::SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, self.response))
    }
}

struct FakePolicyProvider {
    provider_id: &'static str,
}

impl FakePolicyProvider {
    fn new(provider_id: &'static str) -> Self {
        Self { provider_id }
    }
}

impl PolicyProvider for FakePolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id,
            "policy",
            "typed-policy-provider",
            "9.9.9",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            "decision.1",
            request.policy_request_id,
            self.provider_id,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeContextProvider;

impl ContextProvider for FakeContextProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.context.typed",
            "context",
            "typed-context-provider",
            "9.9.9",
            vec!["context.collect".to_string()],
        )
    }

    fn collect(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>> {
        Ok(vec![ContextFrame::new(
            "context.1",
            session_id,
            "fake",
            "context",
            TrustLevel::TrustedHost,
            RedactionClassification::Public,
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct NamedContextProvider {
    context_frame_id: &'static str,
}

impl NamedContextProvider {
    fn new(context_frame_id: &'static str) -> Self {
        Self { context_frame_id }
    }
}

impl ContextProvider for NamedContextProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.context.named",
            "context",
            "named-context-provider",
            "9.9.9",
            vec!["context.collect".to_string()],
        )
    }

    fn collect(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>> {
        Ok(vec![ContextFrame::new(
            self.context_frame_id,
            session_id,
            "fake",
            "context",
            TrustLevel::TrustedHost,
            RedactionClassification::Public,
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeMemoryProvider;

impl MemoryProvider for FakeMemoryProvider {
    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        Ok(vec![MemoryRecord::new(
            "memory.1",
            scope,
            owner_context,
            "memory",
            TrustLevel::TrustedHost,
            RedactionClassification::Public,
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
        ProviderHealth::available()
    }
}

struct FakeKnowledgeProvider;

impl KnowledgeProvider for FakeKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider(
            "provider.knowledge.typed",
            "knowledge",
            vec!["knowledge.search", "knowledge.read", "knowledge.list"],
        )
    }

    fn search(&self, _request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![KnowledgeSearchResult::new(
            "knowledge.1",
            KnowledgeDocumentKind::WikiSection,
            "Agent SPI",
            KnowledgeRetrievalMethod::Keyword,
        )])
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::WikiPage,
            "Agent SPI",
            "knowledge document",
        ))
    }

    fn list(&self, _filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(vec![KnowledgeDocument::new(
            "knowledge.1",
            KnowledgeDocumentKind::WikiPage,
            "Agent SPI",
            "knowledge document",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct NamedMemoryProvider {
    memory_record_id: &'static str,
}

impl NamedMemoryProvider {
    fn new(memory_record_id: &'static str) -> Self {
        Self { memory_record_id }
    }
}

impl MemoryProvider for NamedMemoryProvider {
    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        Ok(vec![MemoryRecord::new(
            self.memory_record_id,
            scope,
            owner_context,
            "memory",
            TrustLevel::TrustedHost,
            RedactionClassification::Public,
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
        ProviderHealth::available()
    }
}

struct FakePlanningProvider;

impl PlanningProvider for FakePlanningProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.planning.typed",
            "planning",
            "typed-planning-provider",
            "9.9.9",
            vec!["planning.create".to_string()],
        )
    }

    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> KernelResult<Plan> {
        Ok(Plan::new("plan.core", task_id, run_id, summary))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct NamedPlanningProvider {
    plan_id: &'static str,
}

impl NamedPlanningProvider {
    fn new(plan_id: &'static str) -> Self {
        Self { plan_id }
    }
}

impl PlanningProvider for NamedPlanningProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.planning.named",
            "planning",
            "named-planning-provider",
            "9.9.9",
            vec!["planning.create".to_string()],
        )
    }

    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> KernelResult<Plan> {
        Ok(Plan::new(self.plan_id, task_id, run_id, summary))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeHostProvider;

impl HostProvider for FakeHostProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider("provider.host.typed", "host", vec!["host.filesystem"])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn filesystem(&self, request: FilesystemRequest) -> KernelResult<FilesystemResult> {
        Ok(FilesystemResult::read(request.operation_id, "readme"))
    }

    fn process(
        &self,
        request: sdkwork_agent_kernel::ProcessRequest,
    ) -> KernelResult<sdkwork_agent_kernel::ProcessResult> {
        Ok(sdkwork_agent_kernel::ProcessResult::exited(
            request.operation_id,
            0,
            "",
            "",
        ))
    }

    fn network(
        &self,
        request: sdkwork_agent_kernel::NetworkRequest,
    ) -> KernelResult<sdkwork_agent_kernel::NetworkResult> {
        Ok(sdkwork_agent_kernel::NetworkResult::response(
            request.operation_id,
            200,
            "",
        ))
    }

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<ProviderSecretValue> {
        Ok(ProviderSecretValue::new(secret_ref.secret_ref_id, "secret"))
    }

    fn storage(
        &self,
        request: sdkwork_agent_kernel::StorageRequest,
    ) -> KernelResult<sdkwork_agent_kernel::StorageResult> {
        Ok(sdkwork_agent_kernel::StorageResult::stored(
            request.operation_id,
        ))
    }

    fn time(
        &self,
        request: sdkwork_agent_kernel::TimeRequest,
    ) -> KernelResult<sdkwork_agent_kernel::TimeResult> {
        Ok(sdkwork_agent_kernel::TimeResult::now(
            request.operation_id,
            "2026-01-01T00:00:00Z",
        ))
    }

    fn environment(
        &self,
        request: sdkwork_agent_kernel::EnvironmentRequest,
    ) -> KernelResult<sdkwork_agent_kernel::EnvironmentResult> {
        Ok(sdkwork_agent_kernel::EnvironmentResult::not_found(
            request.operation_id,
            request.variable_name,
        ))
    }

    fn executor(
        &self,
        request: sdkwork_agent_kernel::ExecutorRequest,
    ) -> KernelResult<sdkwork_agent_kernel::ExecutorResult> {
        Ok(sdkwork_agent_kernel::ExecutorResult::completed(
            request.operation_id,
            request.action_id,
            "",
        ))
    }
}

struct NamedHostProvider {
    provider_id: &'static str,
    read_content: &'static str,
}

impl NamedHostProvider {
    fn new(provider_id: &'static str, read_content: &'static str) -> Self {
        Self {
            provider_id,
            read_content,
        }
    }
}

impl HostProvider for NamedHostProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider(
            self.provider_id,
            "host",
            vec![
                "host.filesystem",
                "host.process",
                "host.network",
                "host.secrets",
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn filesystem(&self, request: FilesystemRequest) -> KernelResult<FilesystemResult> {
        Ok(FilesystemResult::read(
            request.operation_id,
            self.read_content,
        ))
    }

    fn process(
        &self,
        request: sdkwork_agent_kernel::ProcessRequest,
    ) -> KernelResult<sdkwork_agent_kernel::ProcessResult> {
        Ok(sdkwork_agent_kernel::ProcessResult::exited(
            request.operation_id,
            0,
            "",
            "",
        ))
    }

    fn network(
        &self,
        request: sdkwork_agent_kernel::NetworkRequest,
    ) -> KernelResult<sdkwork_agent_kernel::NetworkResult> {
        Ok(sdkwork_agent_kernel::NetworkResult::response(
            request.operation_id,
            200,
            "",
        ))
    }

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<ProviderSecretValue> {
        Ok(ProviderSecretValue::new(secret_ref.secret_ref_id, "secret"))
    }

    fn storage(
        &self,
        request: sdkwork_agent_kernel::StorageRequest,
    ) -> KernelResult<sdkwork_agent_kernel::StorageResult> {
        Ok(sdkwork_agent_kernel::StorageResult::stored(
            request.operation_id,
        ))
    }

    fn time(
        &self,
        request: sdkwork_agent_kernel::TimeRequest,
    ) -> KernelResult<sdkwork_agent_kernel::TimeResult> {
        Ok(sdkwork_agent_kernel::TimeResult::now(
            request.operation_id,
            "2026-01-01T00:00:00Z",
        ))
    }

    fn environment(
        &self,
        request: sdkwork_agent_kernel::EnvironmentRequest,
    ) -> KernelResult<sdkwork_agent_kernel::EnvironmentResult> {
        Ok(sdkwork_agent_kernel::EnvironmentResult::not_found(
            request.operation_id,
            request.variable_name,
        ))
    }

    fn executor(
        &self,
        request: sdkwork_agent_kernel::ExecutorRequest,
    ) -> KernelResult<sdkwork_agent_kernel::ExecutorResult> {
        Ok(sdkwork_agent_kernel::ExecutorResult::completed(
            request.operation_id,
            request.action_id,
            "",
        ))
    }
}

#[derive(Debug)]
struct FakeProtocolAdapter {
    adapter_id: &'static str,
    task_id: &'static str,
    response_id: &'static str,
}

impl FakeProtocolAdapter {
    fn new(adapter_id: &'static str, task_id: &'static str, response_id: &'static str) -> Self {
        Self {
            adapter_id,
            task_id,
            response_id,
        }
    }
}

impl ProtocolAdapter for FakeProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        ProtocolAdapterManifest::new(
            self.adapter_id,
            ProtocolFamily::Http,
            "1.1",
            ProtocolTransport::Http,
            ProtocolAdapterAuthMode::LocalTrusted,
        )
        .with_exposed_capabilities(vec![
            "protocol.map".to_string(),
            "protocol.stream".to_string(),
        ])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn map_request_to_task(&self, _request: ProtocolAdapterRequest) -> KernelResult<AgentTask> {
        Ok(AgentTask::new(self.task_id, "session.protocol", "hello"))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(ProtocolStreamUpdate::from_event(event, 1))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            self.response_id,
            task.task_id,
        ))
    }
}

struct FakeTelemetryProvider;

impl TelemetryProvider for FakeTelemetryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.telemetry.typed",
            "telemetry",
            "typed-telemetry-provider",
            "9.9.9",
            vec!["telemetry.record".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn record_event(&mut self, _event: KernelEvent) -> KernelResult<()> {
        Ok(())
    }

    fn record_metric(
        &mut self,
        _metric: sdkwork_agent_kernel::TelemetryMetric,
    ) -> KernelResult<()> {
        Ok(())
    }

    fn record_log(&mut self, _log: sdkwork_agent_kernel::TelemetryLogRecord) -> KernelResult<()> {
        Ok(())
    }

    fn record_audit(&mut self, _audit: sdkwork_agent_kernel::AuditRecord) -> KernelResult<()> {
        Ok(())
    }

    fn start_span(&mut self, _span: sdkwork_agent_kernel::TelemetrySpan) -> KernelResult<()> {
        Ok(())
    }

    fn finish_span(&mut self, _span: sdkwork_agent_kernel::TelemetrySpan) -> KernelResult<()> {
        Ok(())
    }
}

struct RecordingTelemetryProvider {
    provider_id: &'static str,
    event_ids: Arc<Mutex<Vec<String>>>,
}

impl RecordingTelemetryProvider {
    fn new(provider_id: &'static str, event_ids: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            provider_id,
            event_ids,
        }
    }
}

impl TelemetryProvider for RecordingTelemetryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id,
            "telemetry",
            "recording-telemetry-provider",
            "9.9.9",
            vec!["telemetry.record".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn record_event(&mut self, event: KernelEvent) -> KernelResult<()> {
        self.event_ids
            .lock()
            .expect("recording telemetry sink lock")
            .push(format!("{}:{}", self.provider_id, event.event_id));
        Ok(())
    }

    fn record_metric(
        &mut self,
        _metric: sdkwork_agent_kernel::TelemetryMetric,
    ) -> KernelResult<()> {
        Ok(())
    }

    fn record_log(&mut self, _log: sdkwork_agent_kernel::TelemetryLogRecord) -> KernelResult<()> {
        Ok(())
    }

    fn record_audit(&mut self, _audit: sdkwork_agent_kernel::AuditRecord) -> KernelResult<()> {
        Ok(())
    }

    fn start_span(&mut self, _span: sdkwork_agent_kernel::TelemetrySpan) -> KernelResult<()> {
        Ok(())
    }

    fn finish_span(&mut self, _span: sdkwork_agent_kernel::TelemetrySpan) -> KernelResult<()> {
        Ok(())
    }
}

struct FakeRuntimeAgentInstaller;

impl AgentInstaller for FakeRuntimeAgentInstaller {
    fn detect_installation(&self, agent_id: &str) -> KernelResult<AgentInstallation> {
        Ok(AgentInstallation::installed(agent_id, "0.1.0"))
    }

    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.agent.installer.typed",
            "agent_installer",
            "typed-agent-installer",
            "9.9.9",
            vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ],
        )
    }

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        Ok(configuration_spec(agent_id))
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        Ok(AgentInstallPlan::new(
            "plan.runtime.install",
            request.agent_id.clone(),
            request.target_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.verify",
            AgentInstallStepKind::VerifyPackage,
            "verify package",
        ))
        .add_step(AgentInstallStep::new(
            "step.register",
            AgentInstallStepKind::RegisterAgent,
            "register agent",
        ))
        .require_policy(PolicyCategory::AgentInstall))
    }

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport> {
        Ok(AgentInstallReport::installed(
            request.request_id,
            request.agent_id,
            request.target_version,
        ))
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        Ok(AgentUpgradePlan::new(
            "plan.runtime.upgrade",
            request.agent_id.clone(),
            request.from_version.clone(),
            request.to_version.clone(),
        )
        .with_rollback_required(request.rollback_required)
        .require_policy(PolicyCategory::AgentUpgrade))
    }

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport> {
        Ok(AgentUpgradeReport::upgraded(
            request.request_id,
            request.agent_id,
            request.from_version,
            request.to_version,
        ))
    }

    fn plan_uninstall(&self, request: &AgentUninstallRequest) -> KernelResult<AgentUninstallPlan> {
        Ok(AgentUninstallPlan::new(
            "plan.runtime.uninstall",
            request.agent_id.clone(),
        ))
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        Ok(AgentUninstallReport::uninstalled(
            request.request_id,
            request.agent_id,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn configuration_spec(agent_id: &str) -> AgentConfigurationSpec {
    AgentConfigurationSpec::new(agent_id)
        .add_section(
            AgentConfigSection::base("base", "Base")
                .add_field(AgentConfigField::text("agent.display_name", "Display name").required()),
        )
        .add_section(
            AgentConfigSection::login_auth("login", "Login")
                .add_field(AgentConfigField::text("auth.login.username", "Username").required())
                .add_field(AgentConfigField::secret("auth.login.password", "Password").required()),
        )
        .add_section(AgentConfigSection::llm_api_key("llm", "LLM").add_field(
            AgentConfigField::llm_api_key("llm.openai.api_key", "OpenAI API key"),
        ))
}

fn valid_configuration() -> AgentConfiguration {
    AgentConfiguration::new("agent.registry", "profile.local")
        .set(
            "agent.display_name",
            AgentConfigValue::string("Registry Agent"),
        )
        .set("auth.login.username", AgentConfigValue::string("alice"))
        .set(
            "auth.login.password",
            AgentConfigValue::secret_ref("secret://login/password"),
        )
        .set(
            "llm.openai.api_key",
            AgentConfigValue::secret_ref("secret://llm/openai"),
        )
}

fn provider(provider_id: &str, provider_family: &str, capabilities: Vec<&str>) -> ProviderManifest {
    ProviderManifest::new(
        provider_id,
        provider_family,
        provider_id,
        "0.1.0",
        capabilities
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect(),
    )
}
