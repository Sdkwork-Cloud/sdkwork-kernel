use sdkwork_agent_kernel::{
    AgentConfigSectionKind, AgentConfiguration, AgentPackageLifecycle, AgentPackageManifest,
    AgentPackageProviderBinding, AgentPackageSource, AgentPackageVersionCompatibility,
};

const AGENT_PACKAGE_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_package",
  "agent_id": "agent.code",
  "version": "0.2.0",
  "source": {
    "type": "registry",
    "registry_id": "sdkwork",
    "package_id": "agent.code",
    "version": "0.2.0"
  },
  "lifecycle": {
    "install": true,
    "uninstall": true,
    "upgrade": true
  },
  "provider_binding": {
    "installer_provider_id": "provider.agent.installer.local",
    "configuration_provider_id": "provider.agent.configuration.local"
  },
  "kernel_compatibility": {
    "agent_kernel_min": "0.1.0",
    "agent_kernel_max_exclusive": "0.2.0"
  },
  "required_configuration_sections": [
    "base",
    "login_auth",
    "llm_api_key"
  ],
  "default_profile_id": "profile.local"
}
"#;

#[test]
fn agent_package_manifest_declares_lifecycle_providers_and_configuration_sections() {
    let package = installable_agent_package();

    assert_eq!(package.agent_id, "agent.code");
    assert_eq!(package.version, "0.2.0");
    assert!(package.lifecycle.supports_install);
    assert!(package.lifecycle.supports_uninstall);
    assert!(package.lifecycle.supports_upgrade);
    assert_eq!(
        package.provider_binding.installer_provider_id,
        "provider.agent.installer.local"
    );
    assert_eq!(
        package.provider_binding.configuration_provider_id,
        "provider.agent.configuration.local"
    );
    assert_eq!(package.default_profile_id.as_deref(), Some("profile.local"));
    assert_eq!(
        package.required_configuration_sections(),
        [
            AgentConfigSectionKind::Base,
            AgentConfigSectionKind::LoginAuth,
            AgentConfigSectionKind::LlmApiKey
        ]
    );
    assert!(package.requires_login_auth());
    assert!(package.requires_llm_api_key());
    assert!(package.is_compatible_with_agent_kernel("0.1.0"));
    assert!(!package.is_compatible_with_agent_kernel("0.2.0"));
}

#[test]
fn agent_package_manifest_builds_standard_lifecycle_requests() {
    let package = installable_agent_package();
    let configuration = AgentConfiguration::new("agent.code", "profile.local");

    let install = package
        .install_request("install.1")
        .with_configuration(configuration.clone())
        .requested_by("user.1");
    assert_eq!(install.agent_id, "agent.code");
    assert_eq!(install.target_version, "0.2.0");
    assert_eq!(install.profile_id.as_deref(), Some("profile.local"));
    assert_eq!(install.requested_by.as_deref(), Some("user.1"));
    assert_eq!(
        install.source,
        AgentPackageSource::registry("sdkwork", "agent.code", "0.2.0")
    );

    let upgrade = package
        .upgrade_request("upgrade.1", "0.1.0")
        .with_configuration(configuration)
        .with_rollback_required();
    assert_eq!(upgrade.agent_id, "agent.code");
    assert_eq!(upgrade.from_version, "0.1.0");
    assert_eq!(upgrade.to_version, "0.2.0");
    assert!(upgrade.rollback_required);

    let uninstall = package
        .uninstall_request("uninstall.1")
        .remove_configuration()
        .remove_data();
    assert_eq!(uninstall.agent_id, "agent.code");
    assert!(uninstall.remove_configuration);
    assert!(!uninstall.preserve_data);
}

#[test]
fn agent_package_manifest_rejects_incomplete_lifecycle_provider_bindings() {
    let invalid = AgentPackageManifest::new(
        "agent.code",
        "0.2.0",
        AgentPackageSource::registry("sdkwork", "agent.code", "0.2.0"),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("lifecycle can be attached")
    .with_provider_binding(AgentPackageProviderBinding::new(
        "",
        "provider.agent.configuration.local",
    ));

    let error = invalid.expect_err("installer provider id is required");
    assert!(error.to_string().contains("installer provider"));
}

#[test]
fn agent_package_manifest_parses_machine_readable_json_manifest() {
    let package = AgentPackageManifest::from_json(AGENT_PACKAGE_MANIFEST_JSON)
        .expect("agent package manifest parses");

    assert_eq!(package.schema_version, "0.1.0");
    assert_eq!(package.agent_id, "agent.code");
    assert_eq!(package.version, "0.2.0");
    assert_eq!(
        package.source,
        AgentPackageSource::registry("sdkwork", "agent.code", "0.2.0")
    );
    assert!(package.lifecycle.supports_install);
    assert!(package.lifecycle.supports_uninstall);
    assert!(package.lifecycle.supports_upgrade);
    assert_eq!(
        package.provider_binding.installer_provider_id,
        "provider.agent.installer.local"
    );
    assert_eq!(
        package.provider_binding.configuration_provider_id,
        "provider.agent.configuration.local"
    );
    assert_eq!(
        package.required_configuration_sections(),
        [
            AgentConfigSectionKind::Base,
            AgentConfigSectionKind::LoginAuth,
            AgentConfigSectionKind::LlmApiKey
        ]
    );
    assert_eq!(package.default_profile_id.as_deref(), Some("profile.local"));
    assert!(package.is_compatible_with_agent_kernel("0.1.0"));
    assert!(!package.is_compatible_with_agent_kernel("0.2.0"));
}

#[test]
fn agent_package_manifest_rejects_json_without_required_configuration_sections() {
    let invalid = AGENT_PACKAGE_MANIFEST_JSON.replace(
        r#""required_configuration_sections": [
    "base",
    "login_auth",
    "llm_api_key"
  ],"#,
        r#""required_configuration_sections": [],"#,
    );

    let error = AgentPackageManifest::from_json(&invalid)
        .expect_err("required configuration sections are enforced");
    assert!(error.to_string().contains("configuration section"));
}

fn installable_agent_package() -> AgentPackageManifest {
    AgentPackageManifest::new(
        "agent.code",
        "0.2.0",
        AgentPackageSource::registry("sdkwork", "agent.code", "0.2.0"),
    )
    .with_lifecycle(AgentPackageLifecycle::installable())
    .expect("valid lifecycle")
    .with_provider_binding(AgentPackageProviderBinding::new(
        "provider.agent.installer.local",
        "provider.agent.configuration.local",
    ))
    .expect("valid provider binding")
    .with_kernel_compatibility(AgentPackageVersionCompatibility::new(
        "0.1.0",
        Some("0.2.0"),
    ))
    .require_configuration_section(AgentConfigSectionKind::Base)
    .require_configuration_section(AgentConfigSectionKind::LoginAuth)
    .require_configuration_section(AgentConfigSectionKind::LlmApiKey)
    .with_default_profile("profile.local")
}
