use crate::{hermes_agent_installer, HermesConfigurationProvider, HermesSdkIntegration};
use sdkwork_agent_kernel::{
    AgentDefinition, AgentInstaller, AgentManifest, AgentPackageManifest, ModelProvider,
    ProviderManifest, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkStandardPolicyProvider,
    SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{hermes_agent_definition, hermes_agent_manifest},
    conformance::hermes_conformance_profile,
    ids,
    package::hermes_package_manifest,
};

#[derive(Debug, Clone, Default)]
pub struct HermesKernelPlugin;

impl HermesKernelPlugin {
    pub fn new() -> Self {
        Self
    }
}

pub fn hermes_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(ids::PLUGIN_ID, "Hermes Agent", "0.2.0", "process-adapter")
        .with_source_reference("external/hermes-agent")
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

pub fn hermes_provider_manifests() -> Vec<ProviderManifest> {
    let integration = HermesSdkIntegration::bootstrap().expect("hermes sdk integration");
    vec![
        integration.model.provider_manifest(),
        SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID).provider_manifest_for(),
        hermes_agent_installer().provider_manifest(),
        ProviderManifest::new(
            ids::CONFIGURATION_PROVIDER_ID,
            "agent_configuration",
            "hermes-configuration",
            env!("CARGO_PKG_VERSION"),
            vec!["agent.configure".to_string()],
        ),
    ]
}

impl SdkworkKernelPlugin for HermesKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        hermes_kernel_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        hermes_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        hermes_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        hermes_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        hermes_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        let integration = HermesSdkIntegration::bootstrap()
            .expect("hermes sdk integration must negotiate required capabilities");

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
                hermes_agent_installer(),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                "0.1.0",
                HermesConfigurationProvider::new(),
            )
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        hermes_conformance_profile()
    }
}
