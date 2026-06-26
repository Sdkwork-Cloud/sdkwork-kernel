use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentPackageLifecycle, AgentPackageManifest, AgentPackageProviderBinding,
    AgentPackageSource,
};

use crate::ids;

pub fn hermes_package_manifest() -> AgentPackageManifest {
    AgentPackageManifest::new(
        ids::AGENT_ID,
        "0.2.0",
        AgentPackageSource::registry("pypi", "hermes-agent", "0.17.0"),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("hermes package lifecycle is valid")
    .with_provider_binding(AgentPackageProviderBinding::new(
        ids::INSTALLER_PROVIDER_ID,
        ids::CONFIGURATION_PROVIDER_ID,
    ))
    .expect("hermes package provider binding is valid")
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .require_configuration_section(AgentConfigSectionKind::Runtime)
    .require_configuration_section(AgentConfigSectionKind::Security)
    .with_default_profile("profile.hermes.local")
}
