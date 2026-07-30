use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const HERMES_PACKAGE: &str = "hermes-agent";
pub const HERMES_PACKAGE_VERSION: &str = "0.19.0";

pub fn hermes_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::pypi(HERMES_PACKAGE, HERMES_PACKAGE_VERSION),
    )
}
