use crate::OpenCodeSdkIntegration;
use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentPackageManifest, ModelProvider, ProviderManifest,
    RuntimeBuilder, ToolProvider,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, ProcessAdapterConfigurationProvider,
    ProcessAdapterInstaller, SdkStandardPolicyProvider, SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{opencode_agent_definition, opencode_agent_manifest},
    conformance::opencode_conformance_profile,
    ids,
    package::opencode_package_manifest,
};

#[derive(Debug, Clone, Default)]
pub struct OpenCodeKernelPlugin;

impl OpenCodeKernelPlugin {
    pub fn new() -> Self {
        Self
    }
}

pub fn opencode_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(ids::PLUGIN_ID, "OpenCode", "0.2.0", "process-adapter")
        .with_source_reference("external/opencode")
        .with_agent_id(ids::AGENT_ID)
        .with_provider_id(ids::MODEL_PROVIDER_ID)
        .with_provider_id(ids::TOOL_PROVIDER_ID)
        .with_provider_id(ids::POLICY_PROVIDER_ID)
        .with_provider_id(ids::INSTALLER_PROVIDER_ID)
        .with_provider_id(ids::CONFIGURATION_PROVIDER_ID)
        .with_supported_profile("runtime-manifest")
        .with_supported_profile("provider-model")
        .with_supported_profile("provider-tool")
        .with_supported_profile("security-baseline")
}

pub fn opencode_provider_manifests() -> Vec<ProviderManifest> {
    let integration = OpenCodeSdkIntegration::bootstrap().expect("opencode sdk integration");
    vec![
        integration.model.provider_manifest(),
        integration.tools.provider_manifest(),
        SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID).provider_manifest_for(),
    ]
}

impl SdkworkKernelPlugin for OpenCodeKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        opencode_kernel_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        opencode_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        opencode_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        opencode_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        opencode_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        let integration = OpenCodeSdkIntegration::bootstrap()
            .expect("opencode sdk integration must negotiate required capabilities");

        builder
            .register_model_provider(ids::MODEL_PROVIDER_ID, "0.2.0", integration.model)
            .register_tool_provider(ids::TOOL_PROVIDER_ID, "0.2.0", integration.tools)
            .register_policy_provider(
                ids::POLICY_PROVIDER_ID,
                "0.1.0",
                SdkStandardPolicyProvider::new(ids::POLICY_PROVIDER_ID),
            )
            .register_agent_installer(
                ids::INSTALLER_PROVIDER_ID,
                "0.1.0",
                ProcessAdapterInstaller::new(ids::AGENT_ID, "opencode"),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                "0.1.0",
                ProcessAdapterConfigurationProvider::new(ids::AGENT_ID),
            )
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        opencode_conformance_profile()
    }
}
