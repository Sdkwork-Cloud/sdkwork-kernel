use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfiguration,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentManifest, AgentPackageLifecycle,
    AgentPackageManifest, AgentPackageProviderBinding, AgentPackageSource,
    AgentPackageVersionCompatibility, KernelResult, ProviderHealth, RuntimeBootstrapReport,
    RuntimeBuilder, RuntimeState, AGENT_KERNEL_SPEC_VERSION,
};

const BASE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.general",
  "name": "sdkwork-general-agent",
  "display_name": "SDKWork General Agent",
  "description": "Provider-neutral agent runtime.",
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
  "event_families": ["agent.runtime.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const INSTALLABLE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.installable",
  "name": "sdkwork-installable-agent",
  "display_name": "SDKWork Installable Agent",
  "description": "Agent that requires kernel-owned installation and configuration capabilities.",
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

const VERSIONED_MODEL_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.versioned-model",
  "name": "sdkwork-versioned-model-agent",
  "display_name": "SDKWork Versioned Model Agent",
  "description": "Agent that requires a minimum model provider version.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "2.0.0"
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
fn runtime_builder_bootstraps_ready_when_required_capabilities_are_available() {
    let manifest = AgentManifest::from_json(BASE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_provider(provider("provider.fake", "model", vec!["model.chat"]))
        .register_provider(provider(
            "provider.policy.fake",
            "policy",
            vec!["policy.evaluate"],
        ))
        .register_provider(provider(
            "provider.memory.fake",
            "memory",
            vec!["memory.query"],
        ))
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    assert!(report.runtime.capability_manifest().is_ready());
    assert_eq!(report.runtime.capability_manifest().capabilities.len(), 3);
    assert_eq!(
        report.events[0].event_type,
        "agent.runtime.bootstrap.started"
    );
    assert_eq!(
        report.events.last().unwrap().event_type,
        "agent.runtime.ready"
    );
}

#[test]
fn runtime_builder_bootstraps_degraded_when_only_optional_capabilities_are_missing() {
    let manifest = AgentManifest::from_json(BASE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_provider(provider("provider.fake", "model", vec!["model.chat"]))
        .register_provider(provider(
            "provider.policy.fake",
            "policy",
            vec!["policy.evaluate"],
        ))
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Degraded);
    assert_eq!(
        report.runtime.capability_manifest().degraded_capabilities,
        ["memory.query"]
    );
    assert_eq!(
        report.events.last().unwrap().event_type,
        "agent.runtime.degraded"
    );
}

#[test]
fn runtime_builder_bootstraps_failed_when_required_capabilities_are_missing() {
    let manifest = AgentManifest::from_json(BASE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_provider(provider("provider.fake", "model", vec!["model.chat"]))
        .bootstrap()
        .expect("runtime bootstrap report is still returned");

    assert_eq!(report.runtime.state(), RuntimeState::Failed);
    assert_eq!(
        report
            .runtime
            .capability_manifest()
            .missing_required_capabilities,
        ["policy.evaluate"]
    );
    assert_eq!(
        report.events.last().unwrap().event_type,
        "agent.runtime.failed"
    );
}

#[test]
fn runtime_builder_selects_provider_that_satisfies_required_capability_min_version() {
    let manifest = AgentManifest::from_json(VERSIONED_MODEL_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.versioned", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_provider(provider_with_version(
            "provider.legacy",
            "model",
            "1.9.0",
            vec!["model.chat"],
        ))
        .register_provider(provider_with_version(
            "provider.current",
            "model",
            "2.1.0",
            vec!["model.chat"],
        ))
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    let model_capability = capability(&report, "model.chat");
    assert_eq!(model_capability.provider_id, "provider.current");
    assert_eq!(model_capability.version, "2.1.0");
}

#[test]
fn runtime_builder_fails_when_required_capability_min_version_is_not_satisfied() {
    let manifest = AgentManifest::from_json(VERSIONED_MODEL_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.versioned", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_provider(provider_with_version(
            "provider.legacy",
            "model",
            "1.9.0",
            vec!["model.chat"],
        ))
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Failed);
    assert_eq!(
        report
            .runtime
            .capability_manifest()
            .missing_required_capabilities,
        ["model.chat"]
    );
}

#[test]
fn runtime_builder_fails_closed_on_security_profile_mismatch() {
    let manifest = AgentManifest::from_json(BASE_AGENT_MANIFEST_JSON).unwrap();
    let error = RuntimeBuilder::new("runtime.local", manifest)
        .with_required_security_profile("fail_closed=true")
        .with_security_profile("fail_closed=false")
        .bootstrap()
        .expect_err("security profile mismatch fails");

    assert!(error.to_string().contains("security profile"));
}

#[test]
fn runtime_bootstrap_report_preserves_kernel_version_and_registered_providers() {
    let manifest = AgentManifest::from_json(BASE_AGENT_MANIFEST_JSON).unwrap();
    let RuntimeBootstrapReport { runtime, events } = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_provider(provider("provider.fake", "model", vec!["model.chat"]))
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(
        runtime.capability_manifest().kernel_version,
        AGENT_KERNEL_SPEC_VERSION
    );
    assert_eq!(runtime.capability_manifest().providers.len(), 1);
    assert!(events
        .iter()
        .any(|event| event.event_type == "agent.runtime.providers.registered"));
}

#[test]
fn runtime_builder_registers_agent_installer_and_configuration_providers_as_kernel_capabilities() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_agent_installer_provider("provider.agent.installer.local", "0.1.0")
        .register_agent_configuration_provider("provider.agent.configuration.local", "0.1.0")
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let manifest = report.runtime.capability_manifest();
    assert!(manifest.providers.iter().any(|provider| {
        provider.provider_id == "provider.agent.installer.local"
            && provider.provider_family == "agent_installer"
    }));
    assert!(manifest.providers.iter().any(|provider| {
        provider.provider_id == "provider.agent.configuration.local"
            && provider.provider_family == "agent_configuration"
    }));

    let install = capability(&report, "agent.install");
    assert!(install.required);
    assert_eq!(
        install.operations,
        [
            "detect_installation",
            "configuration_spec",
            "plan_install",
            "install",
            "health"
        ]
    );
    assert_eq!(install.side_effect_level.as_deref(), Some("side_effectful"));
    assert_eq!(install.policy_categories, ["agent.install"]);

    let uninstall = capability(&report, "agent.uninstall");
    assert!(!uninstall.required);
    assert_eq!(
        uninstall.operations,
        [
            "detect_installation",
            "plan_uninstall",
            "uninstall",
            "health"
        ]
    );
    assert_eq!(uninstall.side_effect_level.as_deref(), Some("destructive"));
    assert_eq!(uninstall.policy_categories, ["agent.uninstall"]);

    let upgrade = capability(&report, "agent.upgrade");
    assert!(!upgrade.required);
    assert_eq!(
        upgrade.operations,
        ["detect_installation", "plan_upgrade", "upgrade", "health"]
    );
    assert_eq!(upgrade.side_effect_level.as_deref(), Some("side_effectful"));
    assert_eq!(upgrade.policy_categories, ["agent.upgrade"]);

    let configure = capability(&report, "agent.configure");
    assert!(configure.required);
    assert_eq!(
        configure.operations,
        ["configuration_spec", "validate_configuration", "health"]
    );
    assert_eq!(
        configure.side_effect_level.as_deref(),
        Some("side_effectful")
    );
    assert_eq!(configure.policy_categories, ["agent.configure"]);

    assert!(report
        .events
        .iter()
        .any(|event| event.event_type == "agent.install.provider.registered"));
    assert!(report
        .events
        .iter()
        .any(|event| event.event_type == "agent.configure.provider.registered"));
}

#[test]
fn runtime_builder_registers_agent_package_manifest_provider_bindings() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .with_agent_package_manifest(installable_agent_package())
        .bootstrap()
        .expect("agent package manifest bootstraps provider bindings");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);

    let install = capability(&report, "agent.install");
    assert_eq!(
        install.provider_id,
        "provider.agent.installer.package-binding"
    );

    let configure = capability(&report, "agent.configure");
    assert_eq!(
        configure.provider_id,
        "provider.agent.configuration.package-binding"
    );

    assert!(report
        .events
        .iter()
        .any(|event| event.event_type == "agent.install.provider.registered"));
    assert!(report
        .events
        .iter()
        .any(|event| event.event_type == "agent.configure.provider.registered"));
}

#[test]
fn runtime_builder_rejects_agent_package_when_configuration_provider_omits_required_section() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let error = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .with_agent_package_manifest(installable_agent_package())
        .register_agent_configuration(
            "provider.agent.configuration.package-binding",
            "0.1.0",
            MissingLlmApiKeyConfigurationProvider,
        )
        .bootstrap()
        .expect_err("runtime fails closed when package configuration contract is incomplete");

    assert!(error.to_string().contains("required configuration section"));
}

#[test]
fn runtime_builder_rejects_agent_package_for_a_different_agent_manifest() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let error = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .with_agent_package_manifest(installable_agent_package().for_agent("agent.different"))
        .bootstrap()
        .expect_err("runtime rejects package bound to a different agent id");

    assert!(error.to_string().contains("agent id"));
}

#[test]
fn runtime_builder_rejects_agent_package_incompatible_with_kernel_version() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let error = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .with_agent_package_manifest(installable_agent_package().with_kernel_compatibility(
            AgentPackageVersionCompatibility::new("9.0.0", None::<String>),
        ))
        .bootstrap()
        .expect_err("runtime rejects package incompatible with agent kernel");

    assert!(error.to_string().contains("kernel version"));
}

#[test]
fn runtime_builder_fails_closed_when_required_agent_installer_capability_is_missing() {
    let manifest = AgentManifest::from_json(INSTALLABLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.local", manifest)
        .with_generated_at("2026-05-27T00:00:00Z")
        .register_agent_configuration_provider("provider.agent.configuration.local", "0.1.0")
        .bootstrap()
        .expect("runtime bootstrap report is still returned");

    assert_eq!(report.runtime.state(), RuntimeState::Failed);
    assert_eq!(
        report
            .runtime
            .capability_manifest()
            .missing_required_capabilities,
        ["agent.install"]
    );
    assert_eq!(
        report.events.last().unwrap().event_type,
        "agent.runtime.failed"
    );
}

struct MissingLlmApiKeyConfigurationProvider;

impl AgentConfigurationProvider for MissingLlmApiKeyConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        Ok(AgentConfigurationSpec::new(agent_id)
            .add_section(
                AgentConfigSection::base("base", "Base").add_field(
                    AgentConfigField::text("agent.display_name", "Display name").required(),
                ),
            )
            .add_section(
                AgentConfigSection::login_auth("login", "Login").add_field(
                    AgentConfigField::secret("auth.login.password", "Password").required(),
                ),
            ))
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationValidation> {
        Ok(self
            .configuration_spec(&configuration.agent_id)?
            .validate(configuration))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn installable_agent_package() -> AgentPackageManifest {
    AgentPackageManifest::new(
        "agent.installable",
        "0.1.0",
        AgentPackageSource::registry("sdkwork", "agent.installable", "0.1.0"),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("installable lifecycle")
    .with_provider_binding(AgentPackageProviderBinding::new(
        "provider.agent.installer.package-binding",
        "provider.agent.configuration.package-binding",
    ))
    .expect("provider binding")
    .with_kernel_compatibility(AgentPackageVersionCompatibility::new(
        "0.1.0",
        Some("0.2.0"),
    ))
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LoginAuth)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
}

trait TestAgentPackageMutations {
    fn for_agent(self, agent_id: &str) -> Self;
}

impl TestAgentPackageMutations for AgentPackageManifest {
    fn for_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = agent_id.to_string();
        self
    }
}

fn provider(
    provider_id: &str,
    provider_family: &str,
    capabilities: Vec<&str>,
) -> sdkwork_agent_kernel::ProviderManifest {
    provider_with_version(provider_id, provider_family, "0.1.0", capabilities)
}

fn provider_with_version(
    provider_id: &str,
    provider_family: &str,
    version: &str,
    capabilities: Vec<&str>,
) -> sdkwork_agent_kernel::ProviderManifest {
    sdkwork_agent_kernel::ProviderManifest::new(
        provider_id,
        provider_family,
        provider_id,
        version,
        capabilities
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect(),
    )
}

fn capability<'a>(
    report: &'a RuntimeBootstrapReport,
    capability_id: &str,
) -> &'a sdkwork_agent_kernel::Capability {
    report
        .runtime
        .capability_manifest()
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == capability_id)
        .unwrap_or_else(|| panic!("missing capability: {capability_id}"))
}
