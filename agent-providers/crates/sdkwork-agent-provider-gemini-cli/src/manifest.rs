use crate::{gemini_cli_agent_installer, GeminiCliConfigurationProvider, GeminiCliSdkIntegration};
use sdkwork_agent_kernel::{
    AgentDefinition, AgentInstaller, AgentManifest, AgentPackageManifest, ModelProvider,
    ProviderManifest, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkStandardPolicyProvider,
    SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{gemini_cli_agent_definition, gemini_cli_agent_manifest},
    conformance::gemini_cli_conformance_profile,
    ids,
    package::gemini_cli_package_manifest,
};

#[derive(Debug, Clone, Default)]
pub struct GeminiCliKernelPlugin;

impl GeminiCliKernelPlugin {
    pub fn new() -> Self {
        Self
    }
}

pub fn gemini_cli_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(ids::PLUGIN_ID, "Gemini CLI", "0.2.0", "process-adapter")
        .with_source_reference("external/gemini-cli")
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

pub fn gemini_cli_provider_manifests() -> Vec<ProviderManifest> {
    let integration = GeminiCliSdkIntegration::bootstrap().expect("gemini-cli sdk integration");
    vec![
        integration.model.provider_manifest(),
        SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID).provider_manifest_for(),
        gemini_cli_agent_installer().provider_manifest(),
        configuration_provider_manifest(ids::CONFIGURATION_PROVIDER_ID, "gemini-cli-configuration"),
    ]
}

fn configuration_provider_manifest(provider_id: &str, name: &str) -> ProviderManifest {
    ProviderManifest::new(
        provider_id,
        "agent_configuration",
        name,
        env!("CARGO_PKG_VERSION"),
        vec!["agent.configure".to_string()],
    )
}

impl SdkworkKernelPlugin for GeminiCliKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        gemini_cli_kernel_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        gemini_cli_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        gemini_cli_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        gemini_cli_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        gemini_cli_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        let integration = GeminiCliSdkIntegration::bootstrap()
            .expect("gemini-cli sdk integration must negotiate required capabilities");
        let activity = integration.provider_session_activity_provider();

        builder
            .register_model_provider(ids::MODEL_PROVIDER_ID, "0.2.0", integration.model)
            .register_provider_session_activity_provider(ids::MODEL_PROVIDER_ID, activity)
            .register_policy_provider(
                ids::POLICY_PROVIDER_ID,
                "0.1.0",
                SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID),
            )
            .register_agent_installer(
                ids::INSTALLER_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                gemini_cli_agent_installer(),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                "0.1.0",
                GeminiCliConfigurationProvider::new(),
            )
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        gemini_cli_conformance_profile()
    }
}
