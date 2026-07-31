use sdkwork_agent_kernel::{
    AgentManifest, AgentPackageLifecycle, AgentPackageManifest, AgentProviderFamily,
    ProviderManifest, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginDeploymentSnapshot, KernelPluginManifest,
    KernelProviderBinding, LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest,
    LocalPluginLoadError, LocalPluginLoadErrorKind, LocalPluginProvider, LocalPluginSource,
    LocalPluginStatus, SdkworkKernelFoundationPlugin, SdkworkKernelPlugin, StandardPluginIds,
};

struct TestLocalProvider;

impl LocalPluginProvider for TestLocalProvider {
    fn provider_id(&self) -> &str {
        "provider.plugin.test"
    }

    fn discover(&self, _request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog {
        let mut catalog = LocalPluginCatalog::new(self.provider_id());
        catalog.plugins.push(LocalPluginDescriptor {
            plugin_id: "plugin.test.sample".to_string(),
            name: "sample".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Sample".to_string()),
            root_path: "sample".into(),
            manifest_path: "sample/.codex-plugin/plugin.json".into(),
            source: LocalPluginSource::User,
            status: LocalPluginStatus::ManifestOnly,
            skills: vec![],
            mcp_servers: vec![],
        });
        catalog.errors.push(LocalPluginLoadError {
            provider_id: self.provider_id().to_string(),
            path: None,
            kind: LocalPluginLoadErrorKind::InvalidManifest,
            message: "broken plugin skipped".to_string(),
        });
        catalog
    }
}

#[test]
fn local_plugin_discovery_is_partial_success_and_provider_neutral() {
    let catalog = TestLocalProvider.discover(&LocalPluginDiscoveryRequest::default());
    assert_eq!(catalog.provider_id, "provider.plugin.test");
    assert!(catalog.is_partial());
    assert_eq!(catalog.plugins[0].status, LocalPluginStatus::ManifestOnly);
    assert_eq!(
        catalog.errors[0].kind,
        LocalPluginLoadErrorKind::InvalidManifest
    );
}

#[test]
fn plugin_manifest_preserves_standard_identity_and_provider_ids() {
    let manifest = KernelPluginManifest::new(
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
fn canonical_plugin_manifest_uses_plugin_as_top_level_name() {
    let manifest = KernelPluginManifest::new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .with_source_reference("external/rig")
    .with_agent_id("agent.intelligence.rig-general")
    .with_provider_id("provider.model.rig-rust")
    .with_provider_id("adapter.rpc.agent-chat")
    .with_supported_profile("runtime-local");

    assert_eq!(manifest.plugin_id, "plugin.intelligence.rig");
    assert_eq!(manifest.provider_ids.len(), 2);
    assert!(manifest.supports_profile("runtime-local"));
    assert!(StandardPluginIds::validate_plugin_id("plugin.intelligence.rig").is_ok());
    assert!(StandardPluginIds::validate_provider_or_adapter_id("adapter.rpc.agent-chat").is_ok());
}

#[test]
fn canonical_plugin_binding_and_deployment_snapshot_keep_runtime_evidence() {
    let binding = KernelProviderBinding::new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .with_capability("model.chat")
    .activate();

    let deployment = KernelPluginDeploymentSnapshot::from_binding(
        "deployment.rig.1",
        "tenant.1",
        &binding,
        "2026-06-10T00:00:00Z",
    );

    assert_eq!(deployment.provider_id_snapshot, "provider.model.rig-rust");
    assert_eq!(deployment.capabilities_snapshot, ["model.chat"]);
}

#[test]
fn canonical_plugin_conformance_profile_records_required_profiles() {
    let profile = KernelPluginConformanceProfile::new("plugin.rig.local")
        .require_profile("runtime-manifest")
        .require_profile("runtime-local")
        .require_profile("provider-knowledge");

    assert!(profile.requires("runtime-local"));
    assert!(profile.requires("provider-knowledge"));
    assert!(!profile.requires("provider-host"));
}

#[test]
fn plugin_manifest_accepts_protocol_adapter_ids_as_runtime_providers() {
    let manifest = KernelPluginManifest::new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .with_agent_id("agent.intelligence.rig-general")
    .with_provider_id("provider.model.rig-rust")
    .with_provider_id("adapter.rpc.agent-chat");

    assert_eq!(
        manifest.provider_ids,
        ["provider.model.rig-rust", "adapter.rpc.agent-chat"]
    );
}

#[test]
fn plugin_manifest_rejects_non_standard_identity_and_duplicate_provider_ids() {
    let error =
        KernelPluginManifest::try_new("intelligence.rig", "Rig", "0.1.0", "typed-local-provider")
            .expect_err("plugin id without standard prefix should fail");
    assert!(error.contains("pluginId"));

    let error = KernelPluginManifest::try_new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect("standard plugin id should be accepted")
    .try_with_agent_id("intelligence.rig-general")
    .expect_err("agent id without standard prefix should fail");
    assert!(error.contains("agentId"));

    let error = KernelPluginManifest::try_new(
        "plugin.intelligence.rig",
        "Rig",
        "0.1.0",
        "typed-local-provider",
    )
    .expect("standard plugin id should be accepted")
    .try_with_provider_id("model.rig-rust")
    .expect_err("provider id without standard prefix should fail");
    assert!(error.contains("providerId"));

    let error = KernelPluginManifest::try_new(
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
    let binding = KernelProviderBinding::new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .with_capability("model.chat")
    .activate();

    let deployment = KernelPluginDeploymentSnapshot::from_binding(
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
fn plugin_core_rejects_non_standard_provider_binding_contracts() {
    let error = KernelProviderBinding::try_new(
        "rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect_err("binding id without standard prefix should fail");
    assert!(error.contains("bindingId"));

    let error = KernelProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect_err("provider id without standard prefix should fail");
    assert!(error.contains("providerId"));

    let error = KernelProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "config.rig.local",
    )
    .expect_err("profile id without standard prefix should fail");
    assert!(error.contains("configurationProfileId"));

    let error = KernelProviderBinding::try_new(
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
fn plugin_core_rejects_non_standard_deployment_snapshots() {
    let binding = KernelProviderBinding::try_new(
        "binding.rig.default",
        "agent.intelligence.rig-general",
        "provider.model.rig-rust",
        "typed-local-provider",
        "profile.rig.local",
    )
    .expect("standard binding should be accepted")
    .try_with_capability("model.chat")
    .expect("standard capability should be accepted");

    let error = KernelPluginDeploymentSnapshot::try_from_binding(
        "rig.1",
        "tenant.1",
        &binding,
        "2026-06-04T00:00:00Z",
    )
    .expect_err("deployment id without standard prefix should fail");
    assert!(error.contains("deploymentId"));
}

#[test]
fn standard_plugin_ids_match_kernel_standard_patterns() {
    assert!(StandardPluginIds::validate_provider_id("provider.model.rig-rust").is_ok());
    assert!(StandardPluginIds::validate_binding_id("binding.rig.default").is_ok());
    assert!(StandardPluginIds::validate_profile_id("profile.rig.local").is_ok());
    assert!(StandardPluginIds::validate_deployment_id("deployment.rig.1").is_ok());
    assert!(StandardPluginIds::validate_capability_id("model.chat").is_ok());
    assert!(StandardPluginIds::validate_provider_id("provider..rig").is_err());
    assert!(StandardPluginIds::validate_capability_id("chat").is_err());
}

#[test]
fn conformance_profile_records_required_standard_profiles() {
    let profile = KernelPluginConformanceProfile::new("rig-local")
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
    assert_kernel_plugin_trait(&plugin);

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

fn assert_kernel_plugin_trait<T: SdkworkKernelPlugin>(_plugin: &T) {}

#[test]
fn foundation_plugin_trait_does_not_require_agent_manifests() {
    let plugin = StaticFoundationPlugin;
    assert_foundation_plugin_trait(&plugin);

    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.sdkwork.knowledgebase"
    );
    assert_eq!(
        plugin.provider_manifests()[0].provider_id,
        "provider.knowledge.sdkwork-knowledgebase"
    );
    assert!(plugin.conformance_profile().requires("provider-knowledge"));
}

fn assert_foundation_plugin_trait<T: SdkworkKernelFoundationPlugin>(_plugin: &T) {}

struct StaticPlugin;

impl SdkworkKernelPlugin for StaticPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        KernelPluginManifest::new(
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

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        KernelPluginConformanceProfile::new("static").require_profile("runtime-local")
    }
}

struct StaticFoundationPlugin;

impl SdkworkKernelFoundationPlugin for StaticFoundationPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        KernelPluginManifest::new(
            "plugin.sdkwork.knowledgebase",
            "SDKWork Knowledgebase",
            "0.1.0",
            "official-foundation-plugin",
        )
        .with_provider_id("provider.knowledge.sdkwork-knowledgebase")
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        vec![ProviderManifest::new(
            "provider.knowledge.sdkwork-knowledgebase",
            "knowledge",
            "sdkwork-knowledgebase-provider",
            "0.1.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )]
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        KernelPluginConformanceProfile::new("sdkwork-knowledgebase")
            .require_profile("provider-knowledge")
    }
}
