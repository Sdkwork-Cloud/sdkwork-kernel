use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentPackageLifecycle, AgentPackageManifest,
    AgentPackageProviderBinding, AgentPackageSource,
};

use crate::{ids, OPENCODE_SDK_PACKAGE, OPENCODE_SDK_VERSION};

pub fn opencode_package_manifest() -> AgentPackageManifest {
    AgentPackageManifest::new(
        ids::AGENT_ID,
        "0.2.0",
        AgentPackageSource::registry("npm", OPENCODE_SDK_PACKAGE, OPENCODE_SDK_VERSION),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("opencode package lifecycle is valid")
    .with_provider_binding(AgentPackageProviderBinding::new(
        ids::INSTALLER_PROVIDER_ID,
        ids::CONFIGURATION_PROVIDER_ID,
    ))
    .expect("opencode package provider binding is valid")
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .require_configuration_section(AgentConfigSectionKind::Runtime)
    .require_configuration_section(AgentConfigSectionKind::Security)
    .with_default_profile("profile.opencode.local")
}
