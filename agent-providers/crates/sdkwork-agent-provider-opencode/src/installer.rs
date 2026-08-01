use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const OPENCODE_SDK_PACKAGE: &str = "@opencode-ai/sdk";
pub const OPENCODE_SDK_VERSION: &str = "1.18.11";

pub fn opencode_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(OPENCODE_SDK_PACKAGE, OPENCODE_SDK_VERSION),
    )
}
