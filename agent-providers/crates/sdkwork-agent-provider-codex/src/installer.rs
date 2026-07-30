use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const CODEX_SDK_PACKAGE: &str = "@openai/codex-sdk";
pub const CODEX_SDK_VERSION: &str = "0.146.0";

pub fn codex_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(CODEX_SDK_PACKAGE, CODEX_SDK_VERSION),
    )
}
