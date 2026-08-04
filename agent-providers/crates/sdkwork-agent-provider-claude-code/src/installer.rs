use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const CLAUDE_AGENT_SDK_PACKAGE: &str = "@anthropic-ai/claude-agent-sdk";
pub const CLAUDE_AGENT_SDK_VERSION: &str = "0.3.221";
pub const ANTHROPIC_SDK_PACKAGE: &str = "@anthropic-ai/sdk";
pub const ANTHROPIC_SDK_VERSION: &str = "0.115.0";
pub const MCP_SDK_PACKAGE: &str = "@modelcontextprotocol/sdk";
pub const MCP_SDK_VERSION: &str = "1.30.0";
pub const ZOD_PACKAGE: &str = "zod";
pub const ZOD_VERSION: &str = "4.4.3";

pub fn claude_code_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(CLAUDE_AGENT_SDK_PACKAGE, CLAUDE_AGENT_SDK_VERSION),
    )
    // The Agent SDK declares these as peers and requires them at runtime.
    .with_dependency(ProcessAdapterPackage::npm(
        ANTHROPIC_SDK_PACKAGE,
        ANTHROPIC_SDK_VERSION,
    ))
    .with_dependency(ProcessAdapterPackage::npm(MCP_SDK_PACKAGE, MCP_SDK_VERSION))
    .with_dependency(ProcessAdapterPackage::npm(ZOD_PACKAGE, ZOD_VERSION))
}
