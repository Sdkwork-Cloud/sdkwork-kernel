use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const MIMO_CODE_SDK_PACKAGE: &str = "@mimo-ai/sdk";
pub const MIMO_CODE_SDK_VERSION: &str = "0.1.9";

pub fn mimo_code_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(MIMO_CODE_SDK_PACKAGE, MIMO_CODE_SDK_VERSION),
    )
}
