use sdkwork_agent_kernel::{
    AgentDefinition, AgentInstaller, AgentManifest, AgentPackageManifest, ModelProvider,
    ProviderManifest, RuntimeBuilder, ToolProvider,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkStandardPolicyProvider,
    SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{mimo_code_agent_definition, mimo_code_agent_manifest},
    conformance::mimo_code_conformance_profile,
    ids, mimo_code_agent_installer,
    package::mimo_code_package_manifest,
    MiMoCodeConfigurationProvider, MiMoCodeSdkIntegration,
};

#[derive(Debug, Clone, Default)]
pub struct MiMoCodeKernelPlugin;

impl MiMoCodeKernelPlugin {
    pub fn new() -> Self {
        Self
    }
}

pub fn mimo_code_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(
        ids::PLUGIN_ID,
        "MiMo Code",
        env!("CARGO_PKG_VERSION"),
        "process-adapter",
    )
    .with_source_reference("external/mimo-code")
    .with_agent_id(ids::AGENT_ID)
    .with_provider_id(ids::MODEL_PROVIDER_ID)
    .with_provider_id(ids::TOOL_PROVIDER_ID)
    .with_provider_id(ids::POLICY_PROVIDER_ID)
    .with_provider_id(ids::INSTALLER_PROVIDER_ID)
    .with_provider_id(ids::CONFIGURATION_PROVIDER_ID)
    .with_supported_profile("runtime-manifest")
    .with_supported_profile("agent-installation")
    .with_supported_profile("provider-model")
    .with_supported_profile("provider-tool")
    .with_supported_profile("security-baseline")
}

pub fn mimo_code_provider_manifests() -> Vec<ProviderManifest> {
    let integration = MiMoCodeSdkIntegration::bootstrap().expect("mimo-code sdk integration");
    vec![
        integration.model.provider_manifest(),
        integration.tools.provider_manifest(),
        SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID).provider_manifest_for(),
        mimo_code_agent_installer().provider_manifest(),
        ProviderManifest::new(
            ids::CONFIGURATION_PROVIDER_ID,
            "agent_configuration",
            "mimo-code-configuration",
            env!("CARGO_PKG_VERSION"),
            vec!["agent.configure".to_string()],
        ),
    ]
}

impl SdkworkKernelPlugin for MiMoCodeKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        mimo_code_kernel_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        mimo_code_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        mimo_code_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        mimo_code_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        mimo_code_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        let integration = MiMoCodeSdkIntegration::bootstrap()
            .expect("mimo-code sdk integration must negotiate required capabilities");

        builder
            .register_model_provider(
                ids::MODEL_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                integration.model,
            )
            .register_tool_provider(
                ids::TOOL_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                integration.tools,
            )
            .register_policy_provider(
                ids::POLICY_PROVIDER_ID,
                "0.1.0",
                SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID),
            )
            .register_agent_installer(
                ids::INSTALLER_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                mimo_code_agent_installer(),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                env!("CARGO_PKG_VERSION"),
                MiMoCodeConfigurationProvider::new(),
            )
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        mimo_code_conformance_profile()
    }
}
