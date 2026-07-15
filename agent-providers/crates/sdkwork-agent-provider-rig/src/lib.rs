mod agent_definition;
mod backend;
mod configuration;
mod conformance;
mod deployment;
mod diagnostics;
pub mod ids;
mod installer;
mod manifest;
mod package;
mod provider;
#[cfg(feature = "rig-core-adapter")]
mod rig_core_adapter;
mod sdk_integration;
mod session;

pub use agent_definition::{rig_agent_definition, rig_agent_manifest};
pub use backend::{
    RigBackend, RigBackendBootstrapPlan, RigBackendBootstrapState, RigBackendConfig,
    RigBackendExecutionState, RigBackendExecutionStatus, RigBackendExecutor, RigBackendMode,
};
pub use configuration::RigConfigurationProvider;
pub use conformance::rig_conformance_profile;
pub use deployment::{RigDeploymentSpec, RigProviderBindingSpec};
pub use diagnostics::{RigBackendBootstrapReadiness, RigPluginDiagnostics};
pub use installer::RigAgentInstaller;
pub use manifest::{rig_kernel_plugin_manifest, rig_provider_manifests, RigKernelPlugin};
pub use package::rig_package_manifest;
pub use provider::{
    RigKnowledgeProvider, RigMcpProvider, RigMemoryProvider, RigModelProvider, RigPlanningProvider,
    RigPolicyProvider,
};
#[cfg(feature = "rig-core-adapter")]
pub use rig_core_adapter::{RigCoreKnowledgeAdapter, RigCoreOpenAiExecutor, RigVectorSearchPlan};
pub use sdk_integration::{rig_binding_manifest, RigSdkIntegration};
pub use session::{RigLifecycleProvider, RigSessionAdapter, RigSessionSnapshot};
