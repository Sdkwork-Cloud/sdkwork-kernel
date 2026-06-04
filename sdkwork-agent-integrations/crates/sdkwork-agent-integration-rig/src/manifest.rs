use sdkwork_agent_integration_core::{
    IntegrationConformanceProfile, IntegrationPluginManifest, SdkworkAgentIntegrationPlugin,
};
use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentPackageManifest, ModelProvider, ProviderManifest,
    RuntimeBuilder,
};

use crate::{
    agent_definition::{rig_agent_definition, rig_agent_manifest},
    configuration::RigConfigurationProvider,
    conformance::rig_conformance_profile,
    ids,
    installer::RigAgentInstaller,
    package::rig_package_manifest,
    provider::{
        RigMemoryProvider, RigModelProvider, RigPlanningProvider, RigPolicyProvider,
        RigToolProvider,
    },
};

#[derive(Debug, Clone, Default)]
pub struct RigIntegrationPlugin;

impl RigIntegrationPlugin {
    pub fn fail_closed() -> Self {
        Self
    }
}

pub fn rig_plugin_manifest() -> IntegrationPluginManifest {
    IntegrationPluginManifest::new(ids::PLUGIN_ID, "Rig", "0.1.0", "typed-local-provider")
        .with_source_reference("external/rig")
        .with_agent_id(ids::AGENT_ID)
        .with_provider_id(ids::MODEL_PROVIDER_ID)
        .with_provider_id(ids::TOOL_PROVIDER_ID)
        .with_provider_id(ids::MEMORY_PROVIDER_ID)
        .with_provider_id(ids::PLANNING_PROVIDER_ID)
        .with_provider_id(ids::POLICY_PROVIDER_ID)
        .with_provider_id(ids::INSTALLER_PROVIDER_ID)
        .with_provider_id(ids::CONFIGURATION_PROVIDER_ID)
        .with_supported_profile("runtime-manifest")
        .with_supported_profile("runtime-local")
        .with_supported_profile("agent-installation")
        .with_supported_profile("provider-model")
        .with_supported_profile("provider-tool")
        .with_supported_profile("provider-memory")
        .with_supported_profile("security-baseline")
}

pub fn rig_provider_manifests() -> Vec<ProviderManifest> {
    vec![
        RigModelProvider::fail_closed().provider_manifest(),
        RigToolProvider::fail_closed().provider_manifest(),
        RigMemoryProvider::new().provider_manifest(),
        RigPlanningProvider::new().provider_manifest(),
        ProviderManifest::new(
            ids::POLICY_PROVIDER_ID,
            "policy",
            "rig-standard-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        ),
        ProviderManifest::new(
            ids::INSTALLER_PROVIDER_ID,
            "agent_installer",
            "rig-rust-installer",
            "0.1.0",
            vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ],
        ),
        ProviderManifest::new(
            ids::CONFIGURATION_PROVIDER_ID,
            "agent_configuration",
            "rig-rust-configuration",
            "0.1.0",
            vec!["agent.configure".to_string()],
        ),
    ]
}

impl SdkworkAgentIntegrationPlugin for RigIntegrationPlugin {
    fn plugin_manifest(&self) -> IntegrationPluginManifest {
        rig_plugin_manifest()
    }

    fn agent_manifest(&self) -> AgentManifest {
        rig_agent_manifest()
    }

    fn agent_definition(&self) -> AgentDefinition {
        rig_agent_definition()
    }

    fn package_manifest(&self) -> AgentPackageManifest {
        rig_package_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        rig_provider_manifests()
    }

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder {
        builder
            .register_model_provider(
                ids::MODEL_PROVIDER_ID,
                "0.1.0",
                RigModelProvider::fail_closed(),
            )
            .register_tool_provider(
                ids::TOOL_PROVIDER_ID,
                "0.1.0",
                RigToolProvider::fail_closed(),
            )
            .register_memory_provider(ids::MEMORY_PROVIDER_ID, "0.1.0", RigMemoryProvider::new())
            .register_planning_provider(
                ids::PLANNING_PROVIDER_ID,
                "0.1.0",
                RigPlanningProvider::new(),
            )
            .register_policy_provider(ids::POLICY_PROVIDER_ID, "0.1.0", RigPolicyProvider::new())
            .register_agent_installer(
                ids::INSTALLER_PROVIDER_ID,
                "0.1.0",
                RigAgentInstaller::new(),
            )
            .register_agent_configuration(
                ids::CONFIGURATION_PROVIDER_ID,
                "0.1.0",
                RigConfigurationProvider::new(),
            )
    }

    fn conformance_profile(&self) -> IntegrationConformanceProfile {
        rig_conformance_profile()
    }
}
