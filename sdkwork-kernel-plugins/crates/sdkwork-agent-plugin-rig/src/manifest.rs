use sdkwork_agent_kernel::{
    agent_chat_rpc_adapter_manifest, AgentChatRpcAdapter, AgentDefinition, AgentManifest,
    AgentPackageManifest, ModelProvider, ProviderManifest, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkworkKernelPlugin,
};

use crate::{
    agent_definition::{rig_agent_definition, rig_agent_manifest},
    configuration::RigConfigurationProvider,
    conformance::rig_conformance_profile,
    ids,
    installer::RigAgentInstaller,
    package::rig_package_manifest,
    provider::{
        RigKnowledgeProvider, RigMcpProvider, RigMemoryProvider, RigModelProvider,
        RigPlanningProvider, RigPolicyProvider, RigToolProvider,
    },
};

#[derive(Debug, Clone, Default)]
pub struct RigKernelPlugin;

impl RigKernelPlugin {
    pub fn fail_closed() -> Self {
        Self
    }
}

pub fn rig_kernel_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(ids::PLUGIN_ID, "Rig", "0.1.0", "typed-local-provider")
        .with_source_reference("external/rig")
        .with_agent_id(ids::AGENT_ID)
        .with_provider_id(ids::MODEL_PROVIDER_ID)
        .with_provider_id(ids::TOOL_PROVIDER_ID)
        .with_provider_id(ids::MCP_PROVIDER_ID)
        .with_provider_id(ids::MEMORY_PROVIDER_ID)
        .with_provider_id(ids::KNOWLEDGE_PROVIDER_ID)
        .with_provider_id(ids::PLANNING_PROVIDER_ID)
        .with_provider_id(ids::POLICY_PROVIDER_ID)
        .with_provider_id(ids::CHAT_RPC_ADAPTER_ID)
        .with_provider_id(ids::INSTALLER_PROVIDER_ID)
        .with_provider_id(ids::CONFIGURATION_PROVIDER_ID)
        .with_supported_profile("runtime-manifest")
        .with_supported_profile("runtime-local")
        .with_supported_profile("agent-installation")
        .with_supported_profile("provider-model")
        .with_supported_profile("provider-tool")
        .with_supported_profile("provider-mcp")
        .with_supported_profile("provider-memory")
        .with_supported_profile("provider-knowledge")
        .with_supported_profile("security-baseline")
}

pub fn rig_provider_manifests() -> Vec<ProviderManifest> {
    vec![
        RigModelProvider::fail_closed().provider_manifest(),
        RigToolProvider::fail_closed().provider_manifest(),
        RigMcpProvider::fail_closed().provider_manifest(),
        RigMemoryProvider::new().provider_manifest(),
        RigKnowledgeProvider::new().provider_manifest(),
        RigPlanningProvider::new().provider_manifest(),
        RigPolicyProvider::new().provider_manifest(),
        chat_rpc_adapter_provider_manifest(),
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

fn chat_rpc_adapter_provider_manifest() -> ProviderManifest {
    let adapter_manifest = agent_chat_rpc_adapter_manifest();
    ProviderManifest::new(
        ids::CHAT_RPC_ADAPTER_ID,
        adapter_manifest.provider_family,
        adapter_manifest.adapter_id,
        "0.1.0",
        vec!["protocol.map".to_string(), "protocol.stream".to_string()],
    )
}

impl SdkworkKernelPlugin for RigKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        rig_kernel_plugin_manifest()
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
            .register_mcp_provider(ids::MCP_PROVIDER_ID, "0.1.0", RigMcpProvider::fail_closed())
            .register_memory_provider(ids::MEMORY_PROVIDER_ID, "0.1.0", RigMemoryProvider::new())
            .register_knowledge_provider(
                ids::KNOWLEDGE_PROVIDER_ID,
                "0.1.0",
                RigKnowledgeProvider::new(),
            )
            .register_planning_provider(
                ids::PLANNING_PROVIDER_ID,
                "0.1.0",
                RigPlanningProvider::new(),
            )
            .register_policy_provider(ids::POLICY_PROVIDER_ID, "0.1.0", RigPolicyProvider::new())
            .register_protocol_adapter(
                ids::CHAT_RPC_ADAPTER_ID,
                "0.1.0",
                AgentChatRpcAdapter::new(),
            )
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

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        rig_conformance_profile()
    }
}
