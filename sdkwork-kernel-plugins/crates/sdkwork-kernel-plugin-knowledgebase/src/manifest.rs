use crate::ids::{SDKWORK_KNOWLEDGEBASE_PLUGIN_ID, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID};
use sdkwork_agent_kernel::ProviderManifest;
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkworkKernelFoundationPlugin,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SdkworkKnowledgebasePlugin;

impl SdkworkKnowledgebasePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl SdkworkKernelFoundationPlugin for SdkworkKnowledgebasePlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        sdkwork_knowledgebase_plugin_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        sdkwork_knowledgebase_provider_manifests()
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        sdkwork_knowledgebase_conformance_profile()
    }
}

pub fn sdkwork_knowledgebase_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(
        SDKWORK_KNOWLEDGEBASE_PLUGIN_ID,
        "SDKWork Knowledgebase",
        env!("CARGO_PKG_VERSION"),
        "official-foundation-plugin",
    )
    .with_provider_id(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
    .with_supported_profile("plugin-manifest")
    .with_supported_profile("provider-knowledge")
}

pub fn sdkwork_knowledgebase_provider_manifest() -> ProviderManifest {
    ProviderManifest::new(
        SDKWORK_KNOWLEDGEBASE_PROVIDER_ID,
        "knowledge",
        "sdkwork-knowledgebase-provider",
        env!("CARGO_PKG_VERSION"),
        vec![
            "knowledge.search".to_string(),
            "knowledge.read".to_string(),
            "knowledge.list".to_string(),
        ],
    )
}

pub fn sdkwork_knowledgebase_provider_manifests() -> Vec<ProviderManifest> {
    vec![sdkwork_knowledgebase_provider_manifest()]
}

pub fn sdkwork_knowledgebase_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("sdkwork-knowledgebase")
        .require_profile("plugin-manifest")
        .require_profile("provider-knowledge")
}
