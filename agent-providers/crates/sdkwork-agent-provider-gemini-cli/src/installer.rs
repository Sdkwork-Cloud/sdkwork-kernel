use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const GEMINI_CLI_PACKAGE: &str = "@google/gemini-cli";
pub const GEMINI_CLI_VERSION: &str = "0.53.0";

pub fn gemini_cli_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(GEMINI_CLI_PACKAGE, GEMINI_CLI_VERSION),
    )
}
