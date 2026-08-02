use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

/// Official Codex CLI distribution. It supplies the executable consumed by the
/// Rust `codex-app-server-client`; it is not a model SDK or runtime backend.
pub const CODEX_CLI_PACKAGE: &str = "@openai/codex";
pub const CODEX_CLI_VERSION: &str = "0.146.0";

pub fn codex_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(CODEX_CLI_PACKAGE, CODEX_CLI_VERSION),
    )
}
