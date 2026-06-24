mod agent_definition;
mod conformance;
pub mod ids;
mod manifest;
mod package;

pub use agent_definition::{openclaw_agent_definition, openclaw_agent_manifest};
pub use conformance::openclaw_conformance_profile;
pub use manifest::{
    openclaw_kernel_plugin_manifest, openclaw_provider_manifests, OpenClawKernelPlugin,
};
pub use package::openclaw_package_manifest;
