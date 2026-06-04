use sdkwork_agent_integration_core::{
    DeploymentSnapshot, IntegrationConformanceProfile, IntegrationPluginManifest,
    IntegrationProviderBinding, SdkworkAgentIntegrationPlugin, StandardIntegrationIds,
};
use sdkwork_agent_kernel::{
    AgentManifest, AgentPackageLifecycle, AgentPackageManifest, AgentProviderFamily,
    ProviderManifest, RuntimeBuilder,
};

#[test]
fn plugin_manifest_preserves_standard_identity_and_provider_ids() {
    let manifest = IntegrationPluginManifest::new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .with_source_reference("external/rig")
    .with_agent_id("agent.intelligence.rig-general")
    .with_provider_id("provider.model.rig-rust")
    .with_supported_profile("runtime-local");

    assert_eq!(manifest.plugin_id, "plugin.intelligence.rig");
    assert_eq!(manifest.implementation_kind, "typed-local-provider");
    assert_eq!(manifest.source_reference.as_deref(), Some("external/rig"));
    assert_eq!(
        manifest.agent_id.as_deref(),
        Some("agent.intelligence.rig-general")
    );
    assert_eq!(manifest.provider_ids, ["provider.model.rig-rust"]);
    assert!(manifest.supports_profile("runtime-local"));
}

#[test]
fn plugin_manifest_rejects_non_standard_identity_and_duplicate_provider_ids() {
    let error = IntegrationPluginManifest::try_new(
        "intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect_err("plugin id without standard prefix should fail");
    assert!(error.contains("pluginId"));

    let error = IntegrationPluginManifest::try_new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect("standard plugin id should be accepted")
    .try_with_agent_id("intelligence.rig-general")
    .expect_err("agent id without standard prefix should fail");
    assert!(error.contains("agentId"));

    let error = IntegrationPluginManifest::try_new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect("standard plugin id should be accepted")
    .try_with_provider_id("model.rig-rust")
    .expect_err("provider id without standard prefix should fail");
    assert!(error.contains("providerId"));

    let error = IntegrationPluginManifest::try_new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect("standard plugin id should be accepted")
    .try_with_provider_id("provider.model.rig-rust")
    .expect("standard provider id should be accepted")
    .try_with_provider_id("provider.model.rig-rust")
    .expect_err("duplicate provider id should fail");
    assert!(error.contains("duplicate providerId"));
}

#[test]
fn provider_binding_can_be_activated_without_mutating_deployment_snapshot() {
    let binding = IntegrationProviderBinding::new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .with_capability("model.chat")
    .activate();

    let deployment = DeploymentSnapshot::from_binding(
        "deployment.rig.1",
        "tenant.1",
        &binding,
        "2026-06-04T00:00:00Z",
    );

    let switched = binding
        .clone()
        .with_provider_id("provider.model.other")
        .deactivate();

    assert!(binding.active);
    assert!(!switched.active);
    assert_eq!(deployment.provider_id_snapshot, "provider.model.rig-rust");
    assert_eq!(deployment.binding_id, "binding.rig.default");
    assert_eq!(deployment.capabilities_snapshot, ["model.chat"]);
}

#[test]
fn integration_core_rejects_non_standard_provider_binding_contracts() {
    let error = IntegrationProviderBinding::try_new(
        "rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect_err("binding id without standard prefix should fail");
    assert!(error.contains("bindingId"));

    let error = IntegrationProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect_err("provider id without standard prefix should fail");
    assert!(error.contains("providerId"));

    let error = IntegrationProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "config.rig.local",
    )
    .expect_err("profile id without standard prefix should fail");
    assert!(error.contains("configurationProfileId"));

    let error = IntegrationProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect("standard binding should be accepted")
    .try_with_capability("model.")
    .expect_err("capability with empty segment should fail");
    assert!(error.contains("capabilities"));
}

#[test]
fn integration_core_rejects_non_standard_deployment_snapshots() {
    let binding = IntegrationProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect("standard binding should be accepted")
    .try_with_capability("model.chat")
    .expect("standard capability should be accepted");

    let error =
        DeploymentSnapshot::try_from_binding("rig.1", "tenant.1", &binding, "2026-06-04T00:00:00Z")
            .expect_err("deployment id without standard prefix should fail");
    assert!(error.contains("deploymentId"));
}

#[test]
fn standard_integration_ids_match_kernel_standard_patterns() {
    assert!(StandardIntegrationIds::validate_provider_id("provider.model.rig-rust").is_ok());
    assert!(StandardIntegrationIds::validate_binding_id("binding.rig.default").is_ok());
    assert!(StandardIntegrationIds::validate_profile_id("profile.rig.local").is_ok());
    assert!(StandardIntegrationIds::validate_deployment_id("deployment.rig.1").is_ok());
    assert!(StandardIntegrationIds::validate_capability_id("model.chat").is_ok());
    assert!(StandardIntegrationIds::validate_provider_id("provider..rig").is_err());
    assert!(StandardIntegrationIds::validate_capability_id("chat").is_err());
}

#[test]
fn conformance_profile_records_required_standard_profiles() {
    let profile = IntegrationConformanceProfile::new("rig-local")
        .require_profile("runtime-manifest")
        .require_profile("runtime-local")
        .require_profile("agent-installation")
        .require_profile("provider-model");

    assert!(profile.requires("runtime-local"));
    assert!(profile.requires("agent-installation"));
    assert!(!profile.requires("process-adapter"));
}

#[test]
fn plugin_trait_exposes_agent_package_provider_and_runtime_assembly_contracts() {
    let plugin = StaticPlugin;

    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.static"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.static"
    );
    assert_eq!(
        plugin.agent_definition().manifest.agent_id,
        "agent.intelligence.static"
    );
    assert_eq!(
        plugin.package_manifest().agent_id,
        "agent.intelligence.static"
    );
    assert!(plugin
        .agent_definition()
        .default_binding(AgentProviderFamily::Model)
        .is_none());
    assert_eq!(
        plugin.provider_manifests()[0].provider_id,
        "provider.model.static"
    );
    assert!(plugin.conformance_profile().requires("runtime-local"));

    let builder = RuntimeBuilder::new("runtime.static", plugin.agent_manifest());
    let assembled = plugin.configure_runtime(builder);
    let report = assembled
        .bootstrap()
        .expect("static plugin runtime bootstraps");

    assert!(report
        .runtime
        .capability_manifest()
        .providers
        .iter()
        .any(|provider| provider.provider_id == "provider.model.static"));
}

struct StaticPlugin;

impl SdkworkAgentIntegrationPlugin for StaticPlugin {
    fn plugin_manifest(&self) -> IntegrationPluginManifest {
        IntegrationPluginManifest::new(
            "plugin.intelligence.static",
            "Static",
            "0.1.0",
            "manifest-only",
        )
    }

    fn agent_manifest(&self) -> AgentManifest {
        AgentManifest {
            schema_version: "0.1.0".to_string(),
            manifest_type: "agent".to_string(),
            agent_id: "agent.intelligence.static".to_string(),
            name: "static-agent".to_string(),
            display_name: "Static Agent".to_string(),
            description: "Static test agent".to_string(),
            version: "0.1.0".to_string(),
            domain: "intelligence".to_string(),
            required_capabilities: vec!["model.chat".to_string()],
            optional_capabilities: Vec::new(),
            required_capability_requirements: vec![],
            optional_capability_requirements: vec![],
            event_families: vec!["agent.runtime.*".to_string()],
            owner_name: "sdkwork-platform".to_string(),
            status: "candidate".to_string(),
        }
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        AgentPackageManifest::new(
            "agent.intelligence.static",
            "0.1.0",
            sdkwork_agent_kernel::AgentPackageSource::registry(
                "sdkwork",
                "agent.intelligence.static",
                "0.1.0",
            ),
        )
        .with_lifecycle(AgentPackageLifecycle::installable())
        .expect("installable lifecycle is valid")
        .with_provider_binding(sdkwork_agent_kernel::AgentPackageProviderBinding::new(
            "provider.agent.installer.static",
            "provider.agent.configuration.static",
        ))
        .expect("provider binding is valid")
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        vec![ProviderManifest::new(
            "provider.model.static",
            "model",
            "static",
            "0.1.0",
            vec!["model.chat".to_string()],
        )]
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        builder.register_model_provider_manifest("provider.model.static", "0.1.0")
    }

    fn conformance_profile(&self) -> IntegrationConformanceProfile {
        IntegrationConformanceProfile::new("static").require_profile("runtime-local")
    }
}
