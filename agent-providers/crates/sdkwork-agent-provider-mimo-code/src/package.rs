use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentPackageLifecycle, AgentPackageManifest,
    AgentPackageProviderBinding, AgentPackageSource,
};

use crate::{ids, MIMO_CODE_SDK_PACKAGE, MIMO_CODE_SDK_VERSION};

pub fn mimo_code_package_manifest() -> AgentPackageManifest {
    AgentPackageManifest::new(
        ids::AGENT_ID,
        env!("CARGO_PKG_VERSION"),
        AgentPackageSource::registry("npm", MIMO_CODE_SDK_PACKAGE, MIMO_CODE_SDK_VERSION),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("mimo-code package lifecycle is valid")
    .with_provider_binding(AgentPackageProviderBinding::new(
        ids::INSTALLER_PROVIDER_ID,
        ids::CONFIGURATION_PROVIDER_ID,
    ))
    .expect("mimo-code package provider binding is valid")
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .require_configuration_section(AgentConfigSectionKind::Runtime)
    .require_configuration_section(AgentConfigSectionKind::Security)
    .with_default_profile("profile.mimo-code.local")
}
