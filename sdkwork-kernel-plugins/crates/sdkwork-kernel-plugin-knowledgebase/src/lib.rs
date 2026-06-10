//! SDKWork Knowledgebase kernel plugin.

mod ids;
mod manifest;
mod provider;

pub use ids::{SDKWORK_KNOWLEDGEBASE_PLUGIN_ID, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID};
pub use manifest::{
    sdkwork_knowledgebase_conformance_profile, sdkwork_knowledgebase_plugin_manifest,
    sdkwork_knowledgebase_provider_manifest, sdkwork_knowledgebase_provider_manifests,
    SdkworkKnowledgebasePlugin,
};
pub use provider::{KnowledgebaseRetrievalClient, SdkworkKnowledgebaseProvider};
