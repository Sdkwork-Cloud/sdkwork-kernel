use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentPackageLifecycle, AgentPackageManifest,
    AgentPackageProviderBinding, AgentPackageSource,
};

use crate::{ids, CODEX_CLI_PACKAGE, CODEX_CLI_VERSION};

pub fn codex_package_manifest() -> AgentPackageManifest {
    AgentPackageManifest::new(
        ids::AGENT_ID,
        "0.2.0",
        AgentPackageSource::registry("npm", CODEX_CLI_PACKAGE, CODEX_CLI_VERSION),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("codex package lifecycle is valid")
    .with_provider_binding(AgentPackageProviderBinding::new(
        ids::INSTALLER_PROVIDER_ID,
        ids::CONFIGURATION_PROVIDER_ID,
    ))
    .expect("codex package provider binding is valid")
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .require_configuration_section(AgentConfigSectionKind::Runtime)
    .require_configuration_section(AgentConfigSectionKind::Security)
    .with_default_profile("profile.codex.local")
}
