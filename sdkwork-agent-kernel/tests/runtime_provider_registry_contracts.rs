use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigValue, AgentConfiguration,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentInstallPlan, AgentInstallReport,
    AgentInstallRequest, AgentInstallStep, AgentInstallStepKind, AgentInstaller,
    AgentPackageSource, AgentTask, AgentUninstallReport, AgentUninstallRequest, AgentUpgradePlan,
    AgentUpgradeReport, AgentUpgradeRequest, ContextFrame, ContextProvider, FilesystemRequest,
    FilesystemResult, HostProvider, KernelErrorKind, KernelEvent, KernelResult, MemoryProvider,
    MemoryRecord, MemoryScope, ModelProvider, ModelRequest, ModelResponse, Plan, PlanningProvider,
    PolicyCategory, PolicyDecision, PolicyProvider, PolicyRequest, ProtocolAdapter,
    ProtocolAdapterAuthMode, ProtocolAdapterManifest, ProtocolAdapterRequest,
    ProtocolAdapterResponse, ProtocolFamily, ProtocolStreamUpdate, ProtocolTransport,
    ProviderHealth, ProviderManifest, RedactionClassification, RuntimeBuilder, RuntimeState,
    SecretRef, SecretValue, TelemetryProvider, ToolCall, ToolDescriptor, ToolProvider, ToolResult,
    TrustLevel,
};

const INSTALLABLE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.registry",
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
  "agent_id": "agent.intelligence.core-spi",
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
        "agent.intelligence.registry",
        "0.1.0",
        AgentPackageSource::registry("sdkwork", "agent.intelligence.registry", "0.1.0"),
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
    assert_eq!(install_report.agent_id, "agent.intelligence.registry");

    let configuration_provider = report
        .runtime
        .agent_configuration_provider()
        .expect("typed configuration provider is registered");
    let spec = configuration_provider
        .configuration_spec("agent.intelligence.registry")
        .expect("typed configuration spec loads");
    assert!(spec.required_keys().contains(&"agent.display_name"));
    assert!(spec.required_keys().contains(&"auth.login.username"));
    assert!(spec.required_keys().contains(&"auth.login.password"));
    assert!(spec.required_keys().contains(&"llm.openai.api_key"));

    let validation = configuration_provider
        .validate_configuration(&valid_configuration())
        .expect("typed configuration provider validates");
    assert!(validation.is_valid());
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
        .register_model_provider("provider.model.typed", "0.1.0", FakeModelProvider)
        .register_tool_provider("provider.tool.typed", "0.1.0", FakeToolProvider)
        .register_policy_provider("provider.policy.typed", "0.1.0", FakePolicyProvider)
        .register_context_provider("provider.context.typed", "0.1.0", FakeContextProvider)
        .register_memory_provider("provider.memory.typed", "0.1.0", FakeMemoryProvider)
        .register_planning_provider("provider.planning.typed", "0.1.0", FakePlanningProvider)
        .register_host_provider("provider.host.typed", "0.1.0", FakeHostProvider)
        .register_protocol_adapter("adapter.protocol.typed", "0.1.0", FakeProtocolAdapter)
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
    assert_eq!(model_response.provider_id, "provider.model.typed");

    let tool_result = report
        .runtime
        .tool_provider()
        .expect("tool provider is registered")
        .invoke_tool(ToolCall::new("tool.1", "tool.echo", "{}"))
        .expect("tool provider invokes");
    assert_eq!(tool_result.tool_call_id, "tool.1");

    let decision = report
        .runtime
        .policy_provider()
        .expect("policy provider is registered")
        .evaluate(PolicyRequest::new("policy.1", "tool.invoke", "tool.echo"))
        .expect("policy provider evaluates");
    assert!(decision.is_allow());

    let context = report
        .runtime
        .context_provider()
        .expect("context provider is registered")
        .collect("session.1")
        .expect("context provider collects");
    assert_eq!(context[0].context_frame_id, "context.1");

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

    let plan = report
        .runtime
        .planning_provider()
        .expect("planning provider is registered")
        .create_plan("task.1", "run.1", "plan");
    assert_eq!(plan.plan_id, "plan.core");

    let file = report
        .runtime
        .host_provider()
        .expect("host provider is registered")
        .filesystem(FilesystemRequest::read("fs.1", "workspace/README.md"))
        .expect("host provider reads");
    assert_eq!(file.content.as_deref(), Some("readme"));

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

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_id == "provider.model.typed"
            && provider.provider_family == "model"));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "model.chat"
            && capability.provider_id == "provider.model.typed"));
}

#[test]
fn runtime_registry_reports_provider_unavailable_for_manifest_only_core_spi_provider() {
    let manifest =
        sdkwork_agent_kernel::AgentManifest::from_json(CORE_SPI_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.core", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider_manifest("provider.model.manifest", "0.1.0")
        .register_tool_provider_manifest("provider.tool.manifest", "0.1.0")
        .register_policy_provider_manifest("provider.policy.manifest", "0.1.0")
        .register_context_provider_manifest("provider.context.manifest", "0.1.0")
        .register_memory_provider_manifest("provider.memory.manifest", "0.1.0")
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
    assert_eq!(error.provider_id(), Some("provider.model.manifest"));

    let error = match report.runtime.protocol_adapter() {
        Ok(_) => panic!("typed protocol adapter instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("adapter.protocol.manifest"));
}

struct FakeRuntimeAgentConfigurationProvider;

impl AgentConfigurationProvider for FakeRuntimeAgentConfigurationProvider {
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
        provider("provider.model.typed", "model", vec!["model.chat"])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.typed",
            "model response",
        ))
    }
}

struct FakeToolProvider;

impl ToolProvider for FakeToolProvider {
    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.echo",
            "provider.tool.typed",
            "Echo",
            sdkwork_agent_kernel::SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, "tool response"))
    }
}

struct FakePolicyProvider;

impl PolicyProvider for FakePolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            "decision.1",
            request.policy_request_id,
            "provider.policy.typed",
        ))
    }
}

struct FakeContextProvider;

impl ContextProvider for FakeContextProvider {
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
}

struct FakePlanningProvider;

impl PlanningProvider for FakePlanningProvider {
    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> Plan {
        Plan::new("plan.core", task_id, run_id, summary)
    }
}

struct FakeHostProvider;

impl HostProvider for FakeHostProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        provider(
            "provider.host.typed",
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

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue> {
        Ok(SecretValue::new(secret_ref.secret_ref_id, "secret"))
    }
}

#[derive(Debug)]
struct FakeProtocolAdapter;

impl ProtocolAdapter for FakeProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        ProtocolAdapterManifest::new(
            "adapter.protocol.typed",
            ProtocolFamily::Http,
            "1.1",
            ProtocolTransport::Http,
            ProtocolAdapterAuthMode::LocalTrusted,
        )
        .with_exposed_capabilities(vec!["protocol.map".to_string()])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn map_request_to_task(&self, _request: ProtocolAdapterRequest) -> KernelResult<AgentTask> {
        Ok(AgentTask::new("task.protocol", "session.protocol", "hello"))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(ProtocolStreamUpdate::from_event(event, 1))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            "response.protocol",
            task.task_id,
        ))
    }
}

struct FakeTelemetryProvider;

impl TelemetryProvider for FakeTelemetryProvider {
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

struct FakeRuntimeAgentInstaller;

impl AgentInstaller for FakeRuntimeAgentInstaller {
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
    AgentConfiguration::new("agent.intelligence.registry", "profile.local")
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
