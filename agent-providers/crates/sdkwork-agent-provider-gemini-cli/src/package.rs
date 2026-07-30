use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentPackageLifecycle, AgentPackageManifest,
    AgentPackageProviderBinding, AgentPackageSource,
};

use crate::{ids, GEMINI_CLI_PACKAGE, GEMINI_CLI_VERSION};

pub fn gemini_cli_package_manifest() -> AgentPackageManifest {
    AgentPackageManifest::new(
        ids::AGENT_ID,
        "0.2.0",
        AgentPackageSource::registry("npm", GEMINI_CLI_PACKAGE, GEMINI_CLI_VERSION),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("gemini-cli package lifecycle is valid")
    .with_provider_binding(AgentPackageProviderBinding::new(
        ids::INSTALLER_PROVIDER_ID,
        ids::CONFIGURATION_PROVIDER_ID,
    ))
    .expect("gemini-cli package provider binding is valid")
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .require_configuration_section(AgentConfigSectionKind::Runtime)
    .require_configuration_section(AgentConfigSectionKind::Security)
    .with_default_profile("profile.gemini-cli.local")
}
