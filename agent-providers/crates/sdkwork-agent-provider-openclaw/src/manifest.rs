use crate::{
    openclaw_agent_installer, OpenClawConfigurationProvider, OpenClawSdkIntegration,
};
use sdkwork_agent_kernel::{
    AgentDefinition, AgentInstaller, AgentManifest, AgentPackageManifest, ModelProvider,
    ProviderManifest, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkStandardPolicyProvider,
    SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{openclaw_agent_definition, openclaw_agent_manifest},
    conformance::openclaw_conformance_profile,
    ids,
    package::openclaw_package_manifest,
};

#[derive(Debug, Clone, Default)]
pub struct OpenClawKernelPlugin;

impl OpenClawKernelPlugin {
    pub fn new() -> Self {
        Self
    }
}

pub fn openclaw_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(ids::PLUGIN_ID, "OpenClaw", "0.2.0", "process-adapter")
        .with_source_reference("external/openclaw")
        .with_agent_id(ids::AGENT_ID)
        .with_provider_id(ids::MODEL_PROVIDER_ID)
        .with_provider_id(ids::POLICY_PROVIDER_ID)
        .with_provider_id(ids::INSTALLER_PROVIDER_ID)
        .with_provider_id(ids::CONFIGURATION_PROVIDER_ID)
        .with_supported_profile("runtime-manifest")
        .with_supported_profile("agent-installation")
        .with_supported_profile("provider-model")
        .with_supported_profile("security-baseline")
}

pub fn openclaw_provider_manifests() -> Vec<ProviderManifest> {
    let integration = OpenClawSdkIntegration::bootstrap().expect("openclaw sdk integration");
    vec![
        integration.model.provider_manifest(),
        SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID).provider_manifest_for(),
        openclaw_agent_installer().provider_manifest(),
        ProviderManifest::new(
            ids::CONFIGURATION_PROVIDER_ID,
            "agent_configuration",
            "openclaw-configuration",
            env!("CARGO_PKG_VERSION"),
            vec!["agent.configure".to_string()],
        ),
    ]
}

impl SdkworkKernelPlugin for OpenClawKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        openclaw_kernel_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        openclaw_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        openclaw_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        openclaw_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        openclaw_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        let integration = OpenClawSdkIntegration::bootstrap()
            .expect("openclaw sdk integration must negotiate required capabilities");

        builder
            .register_model_provider(ids::MODEL_PROVIDER_ID, "0.2.0", integration.model)
            .register_policy_provider(
                ids::POLICY_PROVIDER_ID,
                "0.1.0",
                SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID),
            )
            .register_agent_installer(
                ids::INSTALLER_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                openclaw_agent_installer(),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                "0.1.0",
                OpenClawConfigurationProvider::new(),
            )
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        openclaw_conformance_profile()
    }
}
