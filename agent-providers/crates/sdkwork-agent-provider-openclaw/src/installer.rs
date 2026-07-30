use sdkwork_agent_plugin_core::{ProcessAdapterInstaller, ProcessAdapterPackage};

use crate::ids;

pub const OPENCLAW_PACKAGE: &str = "openclaw";
pub const OPENCLAW_PACKAGE_VERSION: &str = "2026.7.1-2";
pub const OPENAI_SDK_PACKAGE: &str = "openai";
pub const OPENAI_SDK_VERSION: &str = "7.1.0";

pub fn openclaw_agent_installer() -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        ids::AGENT_ID,
        ids::INSTALLER_PROVIDER_ID,
        env!("CARGO_PKG_VERSION"),
        ProcessAdapterPackage::npm(OPENCLAW_PACKAGE, OPENCLAW_PACKAGE_VERSION),
    )
    .with_dependency(ProcessAdapterPackage::npm(
        OPENAI_SDK_PACKAGE,
        OPENAI_SDK_VERSION,
    ))
    .with_install_scripts()
}
