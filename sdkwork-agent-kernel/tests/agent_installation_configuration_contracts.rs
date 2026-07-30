use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfigValue,
    AgentConfiguration, AgentConfigurationProfile, AgentConfigurationProfileStatus,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentConfigurationStore,
    AgentConfigurationStoreRecord, AgentConfigurationUpgradePlan, AgentConfigurationUpgradeRequest,
    AgentInstallPlan, AgentInstallReport, AgentInstallRequest, AgentInstallStatus,
    AgentInstallStep, AgentInstallStepKind, AgentInstallation, AgentInstallationDependency,
    AgentInstallationState, AgentInstaller, AgentPackageSource, AgentProfileArchiveRequest,
    AgentSecretBinding, AgentSecretBindingKind, AgentUninstallPlan, AgentUninstallReport,
    AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest,
    ConfigurationMigrationStep, ConfigurationMigrationStepKind, KernelEventRedaction,
    KernelEventSource, KernelResult, PolicyCategory, ProviderHealth, SideEffectLevel,
    AGENT_CONFIGURATION_MIGRATION_SCHEMA, AGENT_CONFIGURATION_PROFILE_SCHEMA,
    AGENT_CONFIGURATION_SPEC_SCHEMA,
};

const AGENT_CONFIGURATION_SPEC_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_configuration_spec",
  "agent_id": "agent.code",
  "sections": [
    {
      "section_id": "base",
      "title": "Base",
      "kind": "base",
      "fields": [
        {
          "key": "agent.display_name",
          "label": "Display name",
          "value_kind": "string",
          "required": true,
          "secret_ref_required": false,
          "redaction_classification": "public",
          "description": "Agent display name",
          "default_value": {
            "value_kind": "string",
            "value": "Code Agent"
          }
        }
      ]
    },
    {
      "section_id": "login",
      "title": "Login",
      "kind": "login_auth",
      "fields": [
        {
          "key": "auth.login.username",
          "label": "Username",
          "value_kind": "string",
          "required": true,
          "secret_ref_required": false,
          "redaction_classification": "public"
        },
        {
          "key": "auth.login.password",
          "label": "Password",
          "value_kind": "secret_ref",
          "required": true,
          "secret_ref_required": true,
          "redaction_classification": "secret"
        }
      ]
    },
    {
      "section_id": "llm",
      "title": "LLM",
      "kind": "llm_api_key",
      "fields": [
        {
          "key": "llm.openai.api_key",
          "label": "OpenAI API key",
          "value_kind": "secret_ref",
          "required": true,
          "secret_ref_required": true,
          "redaction_classification": "secret",
          "description": "LLM API keys must be provided as host secret references"
        }
      ]
    }
  ]
}
"#;

const AGENT_CONFIGURATION_PROFILE_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_configuration_profile",
  "profile_id": "profile.local",
  "agent_id": "agent.code",
  "configuration_version": "0.1.0",
  "status": "active",
  "configuration": {
    "entries": [
      {
        "key": "agent.display_name",
        "value_kind": "string",
        "value": "Code Agent"
      },
      {
        "key": "auth.login.username",
        "value_kind": "string",
        "value": "alice"
      },
      {
        "key": "auth.login.password",
        "value_kind": "secret_ref",
        "value": "secret://login/password"
      },
      {
        "key": "llm.openai.api_key",
        "value_kind": "secret_ref",
        "value": "secret://llm/openai"
      }
    ]
  },
  "secret_bindings": [
    {
      "field_key": "auth.login.password",
      "kind": "login_password",
      "secret_ref": "secret://login/password"
    },
    {
      "field_key": "llm.openai.api_key",
      "kind": "llm_api_key",
      "secret_ref": "secret://llm/openai",
      "provider_hint": "openai"
    }
  ]
}
"#;

const AGENT_CONFIGURATION_MIGRATION_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_configuration_migration",
  "plan_id": "config.migration.1",
  "agent_id": "agent.code",
  "profile_id": "profile.local",
  "from_configuration_version": "0.1.0",
  "to_configuration_version": "0.2.0",
  "required_policy_categories": [
    "agent.configure"
  ],
  "steps": [
    {
      "step_id": "preserve.auth.login.password",
      "kind": "preserve_secret_ref",
      "field_key": "auth.login.password",
      "secret_binding_kind": "login_password"
    },
    {
      "step_id": "rebind.llm.openai.api_key",
      "kind": "rebind_secret_ref",
      "field_key": "llm.openai.api_key",
      "secret_binding_kind": "llm_api_key",
      "provider_hint": "openai"
    }
  ]
}
"#;

#[test]
fn agent_configuration_spec_parses_machine_readable_json_schema() {
    assert!(AGENT_CONFIGURATION_SPEC_SCHEMA.contains("agent_configuration_spec"));

    let spec = AgentConfigurationSpec::from_json(AGENT_CONFIGURATION_SPEC_JSON)
        .expect("configuration spec parses");

    assert_eq!(spec.schema_version, "0.1.0");
    assert_eq!(spec.agent_id, "agent.code");
    assert_eq!(spec.sections.len(), 3);
    assert_eq!(spec.sections[0].kind, AgentConfigSectionKind::Base);
    assert_eq!(spec.sections[1].kind, AgentConfigSectionKind::LoginAuth);
    assert_eq!(spec.sections[2].kind, AgentConfigSectionKind::LlmApiKey);

    let display_name = spec
        .field("agent.display_name")
        .expect("base display name field exists");
    assert!(display_name.required);
    assert_eq!(
        display_name.default_value,
        Some(AgentConfigValue::string("Code Agent"))
    );

    let api_key = spec
        .field("llm.openai.api_key")
        .expect("llm api key field exists");
    assert!(api_key.required);
    assert!(api_key.secret_ref_required);
    assert_eq!(
        api_key.redaction_classification,
        KernelEventRedaction::Secret
    );

    assert!(spec.validate(&valid_agent_configuration()).is_valid());
}

#[test]
fn agent_configuration_spec_rejects_secret_ref_fields_without_secret_ref_requirement() {
    let invalid = AGENT_CONFIGURATION_SPEC_JSON.replace(
        r#""key": "auth.login.password",
          "label": "Password",
          "value_kind": "secret_ref",
          "required": true,
          "secret_ref_required": true,"#,
        r#""key": "auth.login.password",
          "label": "Password",
          "value_kind": "secret_ref",
          "required": true,
          "secret_ref_required": false,"#,
    );

    let error = AgentConfigurationSpec::from_json(&invalid)
        .expect_err("secret_ref fields must require secret references");
    assert!(error.to_string().contains("secret_ref_required"));
}

#[test]
fn agent_configuration_spec_models_base_login_and_llm_api_key_sections() {
    let provider = FakeAgentConfigurationProvider;
    let spec = provider
        .configuration_spec("agent.code")
        .expect("configuration spec loads");

    assert_eq!(spec.agent_id, "agent.code");
    assert_eq!(spec.sections.len(), 3);
    assert!(spec.required_keys().contains(&"agent.display_name"));
    assert!(spec.required_keys().contains(&"auth.login.username"));
    assert!(spec.required_keys().contains(&"auth.login.password"));
    assert!(spec.required_keys().contains(&"llm.openai.api_key"));

    let api_key = spec
        .field("llm.openai.api_key")
        .expect("llm api key field exists");
    assert_eq!(
        api_key.redaction_classification,
        KernelEventRedaction::Secret
    );
    assert!(api_key.secret_ref_required);
    assert_eq!(api_key.section_kind, AgentConfigSectionKind::LlmApiKey);

    let valid_config = AgentConfiguration::new("agent.code", "profile.local")
        .set("agent.display_name", AgentConfigValue::string("Code Agent"))
        .set("auth.login.username", AgentConfigValue::string("alice"))
        .set(
            "auth.login.password",
            AgentConfigValue::secret_ref("secret://login/password"),
        )
        .set(
            "llm.openai.api_key",
            AgentConfigValue::secret_ref("secret://llm/openai"),
        );

    let valid = provider
        .validate_configuration(&valid_config)
        .expect("configuration validates");
    assert!(valid.is_valid());

    let raw_secret_config = AgentConfiguration::new("agent.code", "profile.local")
        .set("agent.display_name", AgentConfigValue::string("Code Agent"))
        .set("auth.login.username", AgentConfigValue::string("alice"))
        .set(
            "auth.login.password",
            AgentConfigValue::secret_ref("secret://login/password"),
        )
        .set("llm.openai.api_key", AgentConfigValue::string("sk-raw"));

    let invalid = provider
        .validate_configuration(&raw_secret_config)
        .expect("configuration validation returns report");
    assert!(!invalid.is_valid());
    assert_eq!(invalid.invalid_fields[0].field_key, "llm.openai.api_key");
    assert_eq!(invalid.invalid_fields[0].reason_code, "secret_ref_required");
}

#[test]
fn agent_configuration_profile_parses_machine_readable_json_profile() {
    assert!(AGENT_CONFIGURATION_PROFILE_SCHEMA.contains("agent_configuration_profile"));

    let profile = AgentConfigurationProfile::from_json(AGENT_CONFIGURATION_PROFILE_JSON)
        .expect("configuration profile parses");

    assert_eq!(profile.profile_id, "profile.local");
    assert_eq!(profile.agent_id, "agent.code");
    assert_eq!(profile.configuration_version, "0.1.0");
    assert_eq!(profile.status, AgentConfigurationProfileStatus::Active);
    assert_eq!(profile.configuration.entries.len(), 4);
    assert_eq!(
        profile.configuration.value("agent.display_name"),
        Some(&AgentConfigValue::string("Code Agent"))
    );
    assert!(profile.requires_secret("auth.login.password"));
    assert!(profile.requires_secret("llm.openai.api_key"));
    assert_eq!(
        profile.secret_bindings[1].provider_hint.as_deref(),
        Some("openai")
    );
    assert!(profile
        .validate_against(&agent_configuration_spec("agent.code"))
        .is_valid());
}

#[test]
fn agent_configuration_profile_rejects_json_secret_refs_without_bindings() {
    let invalid = AGENT_CONFIGURATION_PROFILE_JSON.replace(
        r#",
    {
      "field_key": "llm.openai.api_key",
      "kind": "llm_api_key",
      "secret_ref": "secret://llm/openai",
      "provider_hint": "openai"
    }"#,
        "",
    );

    let error = AgentConfigurationProfile::from_json(&invalid)
        .expect_err("profile secret refs require bindings");
    assert!(error.to_string().contains("secret binding"));
}

#[test]
fn agent_configuration_upgrade_plan_parses_machine_readable_json_migration() {
    assert!(AGENT_CONFIGURATION_MIGRATION_SCHEMA.contains("agent_configuration_migration"));

    let plan = AgentConfigurationUpgradePlan::from_json(AGENT_CONFIGURATION_MIGRATION_JSON)
        .expect("configuration migration parses");

    assert_eq!(plan.plan_id, "config.migration.1");
    assert_eq!(plan.agent_id, "agent.code");
    assert_eq!(plan.profile_id, "profile.local");
    assert_eq!(plan.from_configuration_version, "0.1.0");
    assert_eq!(plan.to_configuration_version, "0.2.0");
    assert!(plan.requires_policy());
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentConfigure.as_str().to_string()]
    );
    assert_eq!(
        plan.steps[0].kind,
        ConfigurationMigrationStepKind::PreserveSecretRef
    );
    assert_eq!(plan.steps[0].field_key, "auth.login.password");
    assert_eq!(
        plan.steps[1].kind,
        ConfigurationMigrationStepKind::RebindSecretRef
    );
    assert_eq!(plan.steps[1].field_key, "llm.openai.api_key");
    assert_eq!(plan.steps[1].provider_hint.as_deref(), Some("openai"));
}

#[test]
fn agent_configuration_profile_tracks_version_status_configuration_and_secret_bindings() {
    let profile = AgentConfigurationProfile::new(
        "profile.local",
        "agent.code",
        "0.1.0",
        valid_agent_configuration(),
    )
    .activate()
    .add_secret_binding(AgentSecretBinding::login_password(
        "auth.login.password",
        "secret://login/password",
    ))
    .add_secret_binding(AgentSecretBinding::llm_api_key(
        "llm.openai.api_key",
        "openai",
        "secret://llm/openai",
    ));

    assert_eq!(profile.profile_id, "profile.local");
    assert_eq!(profile.agent_id, "agent.code");
    assert_eq!(profile.configuration_version, "0.1.0");
    assert_eq!(profile.status, AgentConfigurationProfileStatus::Active);
    assert_eq!(profile.configuration.profile_id, "profile.local");
    assert!(profile.requires_secret("auth.login.password"));
    assert!(profile.requires_secret("llm.openai.api_key"));
    assert!(profile
        .validate_against(&agent_configuration_spec("agent.code"))
        .is_valid());

    let invalid_profile = AgentConfigurationProfile::new(
        "profile.local",
        "agent.code",
        "0.1.0",
        valid_agent_configuration().set(
            "llm.openai.api_key",
            AgentConfigValue::secret_ref("secret://llm/missing-binding"),
        ),
    )
    .add_secret_binding(AgentSecretBinding::login_password(
        "auth.login.password",
        "secret://login/password",
    ));

    let invalid = invalid_profile.validate_against(&agent_configuration_spec("agent.code"));
    assert!(!invalid.is_valid());
    assert_eq!(invalid.invalid_fields[0].field_key, "llm.openai.api_key");
    assert_eq!(
        invalid.invalid_fields[0].reason_code,
        "secret_binding_missing"
    );
}

#[test]
fn agent_configuration_upgrade_plan_declares_migration_steps_for_profile_versions_and_secrets() {
    let plan = AgentConfigurationUpgradePlan::new(
        "config.migration.1",
        "agent.code",
        "profile.local",
        "0.1.0",
        "0.2.0",
    )
    .add_step(ConfigurationMigrationStep::preserve_secret_ref(
        "auth.login.password",
        AgentSecretBindingKind::LoginPassword,
    ))
    .add_step(ConfigurationMigrationStep::rebind_secret_ref(
        "llm.openai.api_key",
        AgentSecretBindingKind::LlmApiKey,
        "openai",
    ))
    .require_policy(PolicyCategory::AgentConfigure);

    assert_eq!(plan.profile_id, "profile.local");
    assert_eq!(plan.from_configuration_version, "0.1.0");
    assert_eq!(plan.to_configuration_version, "0.2.0");
    assert!(plan.requires_policy());
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentConfigure.as_str().to_string()]
    );
    assert_eq!(
        plan.steps[0].kind,
        ConfigurationMigrationStepKind::PreserveSecretRef
    );
    assert_eq!(
        plan.steps[1].kind,
        ConfigurationMigrationStepKind::RebindSecretRef
    );

    let policy = plan.to_policy_request("policy.configure.migration.1");
    assert_eq!(policy.category, "agent.configure");
    assert_eq!(policy.typed_category, Some(PolicyCategory::AgentConfigure));
    assert_eq!(policy.resource, "agent.code/profile.local");
    assert_eq!(
        policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
}

#[test]
fn agent_configuration_provider_plans_profile_upgrade_migrations() {
    let provider = FakeAgentConfigurationProvider;
    let request = AgentConfigurationUpgradeRequest::new(
        "config.upgrade.1",
        "agent.code",
        "profile.local",
        "0.1.0",
        "0.2.0",
    )
    .with_current_profile(
        AgentConfigurationProfile::new(
            "profile.local",
            "agent.code",
            "0.1.0",
            valid_agent_configuration(),
        )
        .activate()
        .add_secret_binding(AgentSecretBinding::login_password(
            "auth.login.password",
            "secret://login/password",
        ))
        .add_secret_binding(AgentSecretBinding::llm_api_key(
            "llm.openai.api_key",
            "openai",
            "secret://llm/openai",
        )),
    );

    let plan = provider
        .plan_configuration_upgrade(&request)
        .expect("configuration provider plans profile migration");

    assert_eq!(plan.agent_id, "agent.code");
    assert_eq!(plan.profile_id, "profile.local");
    assert_eq!(plan.from_configuration_version, "0.1.0");
    assert_eq!(plan.to_configuration_version, "0.2.0");
    assert!(plan
        .steps
        .iter()
        .any(|step| step.field_key == "auth.login.password"
            && step.kind == ConfigurationMigrationStepKind::PreserveSecretRef));
    assert!(plan
        .steps
        .iter()
        .any(|step| step.field_key == "llm.openai.api_key"
            && step.kind == ConfigurationMigrationStepKind::RebindSecretRef));
}

#[test]
fn agent_configuration_store_persists_lists_migrates_and_archives_profiles() {
    let mut store = FakeAgentConfigurationStore::default();
    let profile = active_configuration_profile();

    let created = store
        .save_profile(profile.clone())
        .expect("profile is saved");
    assert_eq!(created.profile.profile_id, "profile.local");
    assert_eq!(
        created.to_event("event.profile.created").event_type,
        "agent.configure.profile.created"
    );

    let loaded = store
        .load_profile("agent.code", "profile.local")
        .expect("profile can be loaded");
    assert_eq!(loaded.configuration_version, "0.1.0");

    let profiles = store
        .list_profiles("agent.code")
        .expect("profiles can be listed");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].profile_id, "profile.local");

    let migration = AgentConfigurationUpgradePlan::new(
        "config.migration.1",
        "agent.code",
        "profile.local",
        "0.1.0",
        "0.2.0",
    )
    .add_step(ConfigurationMigrationStep::preserve_secret_ref(
        "auth.login.password",
        AgentSecretBindingKind::LoginPassword,
    ))
    .add_step(ConfigurationMigrationStep::rebind_secret_ref(
        "llm.openai.api_key",
        AgentSecretBindingKind::LlmApiKey,
        "openai",
    ))
    .require_policy(PolicyCategory::AgentConfigure);

    let migrated = store
        .migrate_profile(&migration, profile)
        .expect("profile migration is saved");
    assert_eq!(migrated.profile.configuration_version, "0.2.0");
    assert_eq!(
        migrated.to_event("event.profile.migrated").event_type,
        "agent.configure.profile.migrated"
    );

    let archived = store
        .archive_profile(&AgentProfileArchiveRequest::new(
            "archive.1",
            "agent.code",
            "profile.local",
        ))
        .expect("profile is archived");
    assert_eq!(
        archived.profile.status,
        AgentConfigurationProfileStatus::Archived
    );
    assert_eq!(
        archived.to_event("event.profile.archived").event_type,
        "agent.configure.profile.archived"
    );
}

#[test]
fn agent_installer_plans_and_executes_install_with_policy_and_events() {
    let installer = FakeAgentInstaller;
    let detection = installer
        .detect_installation("agent.code")
        .expect("agent installation is detected");
    assert_eq!(detection.state, AgentInstallationState::Installed);
    assert_eq!(detection.installed_version.as_deref(), Some("0.1.0"));
    assert!(detection.is_installed());
    assert_eq!(detection.dependencies[0].package_id, "agent.code.runtime");
    assert!(detection.dependencies[0].version_matches());

    let request = AgentInstallRequest::new(
        "install.1",
        "agent.code",
        "0.2.0",
        AgentPackageSource::registry("sdkwork", "agent.code", "0.2.0"),
    )
    .with_configuration(valid_agent_configuration())
    .requested_by("user.1");

    let plan = installer
        .plan_install(&request)
        .expect("install plan is generated");
    assert_eq!(plan.agent_id, "agent.code");
    assert_eq!(plan.target_version, "0.2.0");
    assert!(plan.requires_policy());
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentInstall.as_str().to_string()]
    );
    assert_eq!(plan.steps[0].kind, AgentInstallStepKind::VerifyPackage);

    let policy_request = plan.to_policy_request("policy.install.1");
    assert_eq!(policy_request.category, "agent.install");
    assert_eq!(
        policy_request.typed_category,
        Some(PolicyCategory::AgentInstall)
    );
    assert_eq!(policy_request.resource, "agent.code@0.2.0");
    assert_eq!(
        policy_request.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );

    let report = installer.install(request).expect("agent installs");
    assert_eq!(report.status, AgentInstallStatus::Installed);
    assert_eq!(report.installed_version.as_deref(), Some("0.2.0"));
    assert_eq!(report.agent_id, "agent.code");

    let event = report.to_event("event.install.1");
    assert_eq!(event.event_type, "agent.install.installed");
    assert_eq!(event.source, KernelEventSource::Runtime);
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.installation.report.v1")
    );
}

#[test]
fn agent_installer_supports_upgrade_and_uninstall_lifecycle() {
    let installer = FakeAgentInstaller;
    let upgrade_request = AgentUpgradeRequest::new("upgrade.1", "agent.code", "0.1.0", "0.2.0")
        .with_configuration(valid_agent_configuration())
        .with_rollback_required();

    let upgrade_plan = installer
        .plan_upgrade(&upgrade_request)
        .expect("upgrade plan is generated");
    assert_eq!(upgrade_plan.from_version, "0.1.0");
    assert_eq!(upgrade_plan.to_version, "0.2.0");
    assert!(upgrade_plan.rollback_required);
    assert!(upgrade_plan.requires_policy());
    assert_eq!(
        upgrade_plan.required_policy_categories,
        [PolicyCategory::AgentUpgrade.as_str().to_string()]
    );
    let upgrade_policy = upgrade_plan.to_policy_request("policy.upgrade.1");
    assert_eq!(upgrade_policy.category, "agent.upgrade");
    assert_eq!(
        upgrade_policy.typed_category,
        Some(PolicyCategory::AgentUpgrade)
    );
    assert_eq!(upgrade_policy.action.as_deref(), Some("agent.upgrade"));
    assert_eq!(
        upgrade_policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );

    let upgrade_report = installer.upgrade(upgrade_request).expect("agent upgrades");
    assert_eq!(upgrade_report.status, AgentInstallStatus::Upgraded);
    assert!(!upgrade_report.safe_summary.is_empty());
    assert_eq!(
        upgrade_report.rollback_token.as_deref(),
        Some("rollback.agent.code.0.1.0")
    );
    let upgrade_event = upgrade_report.to_event("event.upgrade.1");
    assert_eq!(upgrade_event.event_type, "agent.install.upgraded");
    assert!(upgrade_event.payload.contains("status=upgraded"));
    assert!(upgrade_event.payload.contains("summary="));

    let uninstall_request =
        AgentUninstallRequest::new("uninstall.1", "agent.code").remove_configuration();
    let uninstall_report = installer
        .uninstall(uninstall_request)
        .expect("agent uninstalls");
    assert_eq!(uninstall_report.status, AgentInstallStatus::Uninstalled);
    assert!(uninstall_report.configuration_removed);
    assert!(!uninstall_report.safe_summary.is_empty());
    assert_eq!(
        uninstall_report.to_event("event.uninstall.1").event_type,
        "agent.install.uninstalled"
    );
}

#[test]
fn planned_upgrade_and_uninstall_reports_never_emit_success_events() {
    let install = AgentInstallReport::planned("install.dry-run", "agent.code", "0.2.0");
    assert_eq!(install.target_version, "0.2.0");
    assert_eq!(install.installed_version, None);
    let install_event = install.to_event("event.install.dry-run");
    assert_eq!(install_event.event_type, "agent.install.planned");
    assert!(install_event.payload.contains("target_version=0.2.0"));
    assert!(install_event.payload.contains("installed_version="));

    let upgrade = AgentUpgradeReport::planned(
        "upgrade.dry-run",
        "agent.code",
        "0.1.0",
        "0.2.0",
    );
    let upgrade_event = upgrade.to_event("event.upgrade.dry-run");
    assert_eq!(upgrade_event.event_type, "agent.install.planned");
    assert!(upgrade_event.payload.contains("status=planned"));
    assert!(upgrade_event.payload.contains("summary=agent upgrade planned"));

    let uninstall = AgentUninstallReport::planned("uninstall.dry-run", "agent.code");
    let uninstall_event = uninstall.to_event("event.uninstall.dry-run");
    assert_eq!(uninstall_event.event_type, "agent.install.planned");
    assert!(uninstall_event.payload.contains("status=planned"));
    assert!(uninstall_event
        .payload
        .contains("summary=agent uninstall planned"));
}

#[test]
fn installation_events_encode_untrusted_field_delimiters() {
    let report = AgentInstallReport::planned(
        "install.1;status=installed\nforged=true",
        "agent.code",
        "0.2.0",
    );

    let event = report.to_event("event.install.safe");
    assert!(!event.payload.contains(";status=installed"));
    assert!(!event.payload.contains('\n'));
    assert!(event
        .payload
        .contains("request_id=install.1%3Bstatus%3Dinstalled%0Aforged%3Dtrue"));
}

#[test]
fn upgrade_events_never_expose_opaque_rollback_tokens() {
    let secret = "rollback-token-must-not-leak";
    let report = AgentUpgradeReport::upgraded(
        "upgrade.secure-event",
        "agent.code",
        "0.1.0",
        "0.2.0",
    )
    .with_rollback_token(secret);

    let event = report.to_event("event.upgrade.secure");
    assert!(event.payload.contains("rollback_available=true"));
    assert!(!event.payload.contains(secret));
}

struct FakeAgentConfigurationProvider;

impl AgentConfigurationProvider for FakeAgentConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        Ok(agent_configuration_spec(agent_id))
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationValidation> {
        Ok(agent_configuration_spec(&configuration.agent_id).validate(configuration))
    }

    fn plan_configuration_upgrade(
        &self,
        request: &AgentConfigurationUpgradeRequest,
    ) -> KernelResult<AgentConfigurationUpgradePlan> {
        Ok(AgentConfigurationUpgradePlan::new(
            "config.migration.fake",
            request.agent_id.clone(),
            request.profile_id.clone(),
            request.from_configuration_version.clone(),
            request.to_configuration_version.clone(),
        )
        .add_step(ConfigurationMigrationStep::preserve_secret_ref(
            "auth.login.password",
            AgentSecretBindingKind::LoginPassword,
        ))
        .add_step(ConfigurationMigrationStep::rebind_secret_ref(
            "llm.openai.api_key",
            AgentSecretBindingKind::LlmApiKey,
            "openai",
        ))
        .require_policy(PolicyCategory::AgentConfigure))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeAgentInstaller;

impl AgentInstaller for FakeAgentInstaller {
    fn detect_installation(&self, agent_id: &str) -> KernelResult<AgentInstallation> {
        Ok(
            AgentInstallation::installed(agent_id, "0.1.0").with_dependency(
                AgentInstallationDependency::installed(
                    "sdkwork",
                    "agent.code.runtime",
                    "0.1.0",
                    "0.1.0",
                ),
            ),
        )
    }

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        Ok(agent_configuration_spec(agent_id))
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        Ok(AgentInstallPlan::new(
            "plan.install.1",
            request.agent_id.clone(),
            request.target_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.verify",
            AgentInstallStepKind::VerifyPackage,
            "verify package signature",
        ))
        .add_step(AgentInstallStep::new(
            "step.register",
            AgentInstallStepKind::RegisterAgent,
            "register agent manifest",
        ))
        .require_policy(PolicyCategory::AgentInstall))
    }

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport> {
        Ok(AgentInstallReport::installed(
            request.request_id,
            request.agent_id,
            request.target_version,
        ))
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        Ok(AgentUpgradePlan::new(
            "plan.upgrade.1",
            request.agent_id.clone(),
            request.from_version.clone(),
            request.to_version.clone(),
        )
        .with_rollback_required(request.rollback_required)
        .add_step(AgentInstallStep::new(
            "step.backup",
            AgentInstallStepKind::BackupCurrentVersion,
            "backup current agent version",
        ))
        .add_step(AgentInstallStep::new(
            "step.upgrade",
            AgentInstallStepKind::ReplaceVersion,
            "replace agent version",
        ))
        .require_policy(PolicyCategory::AgentUpgrade))
    }

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport> {
        Ok(AgentUpgradeReport::upgraded(
            request.request_id,
            request.agent_id,
            request.from_version,
            request.to_version,
        )
        .with_rollback_token("rollback.agent.code.0.1.0"))
    }

    fn plan_uninstall(&self, request: &AgentUninstallRequest) -> KernelResult<AgentUninstallPlan> {
        Ok(
            AgentUninstallPlan::new("plan.uninstall.1", request.agent_id.clone())
                .add_step(AgentInstallStep::new(
                    "step.remove",
                    AgentInstallStepKind::RemoveFiles,
                    "remove agent package",
                ))
                .require_policy(PolicyCategory::AgentUninstall),
        )
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        Ok(
            AgentUninstallReport::uninstalled(request.request_id, request.agent_id)
                .with_configuration_removed(request.remove_configuration),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Default)]
struct FakeAgentConfigurationStore {
    records: Vec<AgentConfigurationProfile>,
}

impl AgentConfigurationStore for FakeAgentConfigurationStore {
    fn save_profile(
        &mut self,
        profile: AgentConfigurationProfile,
    ) -> KernelResult<AgentConfigurationStoreRecord> {
        self.records.retain(|record| {
            record.agent_id != profile.agent_id || record.profile_id != profile.profile_id
        });
        self.records.push(profile.clone());
        Ok(AgentConfigurationStoreRecord::created(profile))
    }

    fn load_profile(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<AgentConfigurationProfile> {
        self.records
            .iter()
            .find(|record| record.agent_id == agent_id && record.profile_id == profile_id)
            .cloned()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::validation("profile not found"))
    }

    fn list_profiles(&self, agent_id: &str) -> KernelResult<Vec<AgentConfigurationProfile>> {
        Ok(self
            .records
            .iter()
            .filter(|record| record.agent_id == agent_id)
            .cloned()
            .collect())
    }

    fn migrate_profile(
        &mut self,
        plan: &AgentConfigurationUpgradePlan,
        current_profile: AgentConfigurationProfile,
    ) -> KernelResult<AgentConfigurationStoreRecord> {
        let mut migrated = current_profile;
        migrated.configuration_version = plan.to_configuration_version.clone();
        let record =
            AgentConfigurationStoreRecord::migrated(migrated.clone(), plan.plan_id.clone());
        self.save_profile(migrated)?;
        Ok(record)
    }

    fn archive_profile(
        &mut self,
        request: &AgentProfileArchiveRequest,
    ) -> KernelResult<AgentConfigurationStoreRecord> {
        let mut profile = self.load_profile(&request.agent_id, &request.profile_id)?;
        profile.status = AgentConfigurationProfileStatus::Archived;
        let record =
            AgentConfigurationStoreRecord::archived(profile.clone(), request.request_id.clone());
        self.save_profile(profile)?;
        Ok(record)
    }
}

fn agent_configuration_spec(agent_id: &str) -> AgentConfigurationSpec {
    AgentConfigurationSpec::new(agent_id)
        .add_section(
            AgentConfigSection::base("base", "Base")
                .add_field(AgentConfigField::text("agent.display_name", "Display name").required()),
        )
        .add_section(
            AgentConfigSection::login_auth("login", "Login")
                .add_field(AgentConfigField::text("auth.login.username", "Username").required())
                .add_field(AgentConfigField::secret("auth.login.password", "Password").required()),
        )
        .add_section(AgentConfigSection::llm_api_key("llm", "LLM").add_field(
            AgentConfigField::llm_api_key("llm.openai.api_key", "OpenAI API key"),
        ))
}

fn valid_agent_configuration() -> AgentConfiguration {
    AgentConfiguration::new("agent.code", "profile.local")
        .set("agent.display_name", AgentConfigValue::string("Code Agent"))
        .set("auth.login.username", AgentConfigValue::string("alice"))
        .set(
            "auth.login.password",
            AgentConfigValue::secret_ref("secret://login/password"),
        )
        .set(
            "llm.openai.api_key",
            AgentConfigValue::secret_ref("secret://llm/openai"),
        )
}

fn active_configuration_profile() -> AgentConfigurationProfile {
    AgentConfigurationProfile::new(
        "profile.local",
        "agent.code",
        "0.1.0",
        valid_agent_configuration(),
    )
    .activate()
    .add_secret_binding(AgentSecretBinding::login_password(
        "auth.login.password",
        "secret://login/password",
    ))
    .add_secret_binding(AgentSecretBinding::llm_api_key(
        "llm.openai.api_key",
        "openai",
        "secret://llm/openai",
    ))
}
