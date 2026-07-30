use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const CLAUDE_AGENT_SDK_PACKAGE: &str = "@anthropic-ai/claude-agent-sdk";
pub const CLAUDE_AGENT_SDK_VERSION: &str = "0.3.220";

pub fn claude_code_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(CLAUDE_AGENT_SDK_PACKAGE, CLAUDE_AGENT_SDK_VERSION),
    )
}
