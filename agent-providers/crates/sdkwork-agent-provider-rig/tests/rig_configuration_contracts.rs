use sdkwork_agent_kernel::{
    AgentConfigValue, AgentConfiguration, AgentConfigurationProvider, ProviderHealth,
};
use sdkwork_agent_provider_rig::{
    ids, RigBackendBootstrapState, RigBackendConfig, RigBackendExecutionState, RigBackendMode,
    RigConfigurationProvider, RigPluginDiagnostics,
};

#[test]
fn rig_configuration_spec_requires_secret_refs_and_security_defaults() {
    let provider = RigConfigurationProvider::new();
    let spec = provider
        .configuration_spec(ids::AGENT_ID)
        .expect("rig configuration spec loads");

    assert!(spec.required_keys().contains(&"agent.display_name"));
    assert!(spec.required_keys().contains(&"llm.rig.provider_id"));
    assert!(spec.required_keys().contains(&"llm.rig.api_key"));
    assert!(spec.required_keys().contains(&"runtime.rig.backend_mode"));
    assert!(spec.required_keys().contains(&"security.fail_closed"));
    assert_eq!(provider.health(), ProviderHealth::available());
}

#[test]
fn rig_configuration_rejects_raw_api_keys() {
    let provider = RigConfigurationProvider::new();
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.local")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set("llm.rig.api_key", AgentConfigValue::string("raw-secret"))
        .set(
            "runtime.rig.backend_mode",
            AgentConfigValue::string("fail_closed"),
        )
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let validation = provider
        .validate_configuration(&configuration)
        .expect("validation report is produced");

    assert!(!validation.is_valid());
    assert!(validation
        .invalid_fields
        .iter()
        .any(|field| field.field_key == "llm.rig.api_key"));
}

#[test]
fn rig_configuration_accepts_secret_ref_profile() {
    let provider = RigConfigurationProvider::new();
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.local")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set(
            "llm.rig.api_key",
            AgentConfigValue::secret_ref("secret://rig/openai"),
        )
        .set(
            "runtime.rig.backend_mode",
            AgentConfigValue::string("fail_closed"),
        )
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let validation = provider
        .validate_configuration(&configuration)
        .expect("validation report is produced");

    assert!(validation.is_valid());
}

#[test]
fn rig_backend_config_resolves_mode_without_raw_secrets() {
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.live")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set(
            "llm.rig.api_key",
            AgentConfigValue::secret_ref("secret://rig/openai"),
        )
        .set("runtime.rig.backend_mode", AgentConfigValue::string("live"))
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let config = RigBackendConfig::from_configuration(&configuration)
        .expect("live Rig backend config is parsed from secret-ref configuration");

    assert_eq!(config.mode, RigBackendMode::Live);
    assert_eq!(config.provider_id.as_deref(), Some("openai"));
    assert_eq!(
        config.api_key_secret_ref.as_deref(),
        Some("secret://rig/openai")
    );
}

#[test]
fn rig_plugin_diagnostics_reports_live_pending_execution_state() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };

    let diagnostics = RigPluginDiagnostics::from_backend_config(&config);

    assert_eq!(diagnostics.backend_mode, RigBackendMode::Live);
    assert!(diagnostics.live_backend_configured);
    assert!(diagnostics.fail_closed);
    assert_eq!(
        diagnostics.backend_execution_status().state,
        RigBackendExecutionState::LivePending
    );
    assert!(diagnostics
        .backend_execution_status()
        .safe_reason
        .contains("not connected"));
}

#[test]
fn rig_plugin_diagnostics_exposes_secret_safe_bootstrap_readiness() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };

    let readiness = RigPluginDiagnostics::backend_bootstrap_readiness_from_config(&config);

    assert_eq!(readiness.backend_mode, RigBackendMode::Live);
    assert_eq!(readiness.state, RigBackendBootstrapState::LivePending);
    assert_eq!(readiness.provider_id.as_deref(), Some("openai"));
    assert_eq!(readiness.required_secret_refs, vec!["llm.rig.api_key"]);
    assert!(readiness.missing_secret_refs.is_empty());
    assert!(readiness
        .policy_categories
        .contains(&"host.secrets.read".to_string()));
    assert!(readiness
        .policy_categories
        .contains(&"model.invoke".to_string()));
    assert!(readiness.fail_closed);
    assert!(readiness.safe_summary.contains("live-pending"));
    assert!(
        !readiness.safe_summary.contains("secret://rig/openai"),
        "diagnostics readiness summaries must not echo secret reference values"
    );
}

#[test]
fn rig_plugin_diagnostics_reports_missing_live_secret_refs_in_readiness() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: None,
    };

    let readiness = RigPluginDiagnostics::backend_bootstrap_readiness_from_config(&config);

    assert_eq!(readiness.backend_mode, RigBackendMode::Live);
    assert_eq!(readiness.state, RigBackendBootstrapState::LivePending);
    assert_eq!(readiness.provider_id.as_deref(), Some("openai"));
    assert_eq!(readiness.required_secret_refs, vec!["llm.rig.api_key"]);
    assert_eq!(readiness.missing_secret_refs, vec!["llm.rig.api_key"]);
    assert!(readiness.fail_closed);
    assert!(readiness.safe_summary.contains("live-pending"));
}

#[test]
fn rig_backend_config_builds_secret_safe_live_bootstrap_plan() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };

    let plan = config.bootstrap_plan();

    assert_eq!(plan.backend_mode, RigBackendMode::Live);
    assert_eq!(plan.state, RigBackendBootstrapState::LivePending);
    assert_eq!(
        plan.execution_status().state,
        RigBackendExecutionState::LivePending
    );
    assert_eq!(plan.provider_id.as_deref(), Some("openai"));
    assert_eq!(plan.required_secret_refs, vec!["llm.rig.api_key"]);
    assert_eq!(
        plan.secret_ref_value("llm.rig.api_key"),
        Some("secret://rig/openai")
    );
    assert!(plan
        .policy_categories
        .contains(&"host.secrets.read".to_string()));
    assert!(plan.policy_categories.contains(&"model.invoke".to_string()));
    assert!(plan.fail_closed);
    assert!(plan.safe_summary.contains("live-pending"));
    assert!(
        !plan.safe_summary.contains("secret://rig/openai"),
        "bootstrap diagnostics must not echo secret reference values in summaries"
    );
}

#[test]
fn rig_backend_config_builds_minimal_fail_closed_bootstrap_plan() {
    let plan = RigBackendConfig::fail_closed().bootstrap_plan();

    assert_eq!(plan.backend_mode, RigBackendMode::FailClosed);
    assert_eq!(plan.state, RigBackendBootstrapState::FailClosed);
    assert_eq!(
        plan.execution_status().state,
        RigBackendExecutionState::FailClosed
    );
    assert_eq!(plan.provider_id, None);
    assert!(plan.required_secret_refs.is_empty());
    assert_eq!(plan.secret_ref_value("llm.rig.api_key"), None);
    assert!(plan.policy_categories.contains(&"model.invoke".to_string()));
    assert!(!plan
        .policy_categories
        .contains(&"host.secrets.read".to_string()));
    assert!(plan.fail_closed);
    assert!(plan.safe_summary.contains("fail-closed"));
}

#[test]
fn rig_backend_config_rejects_unknown_backend_modes() {
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.invalid")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set(
            "llm.rig.api_key",
            AgentConfigValue::secret_ref("secret://rig/openai"),
        )
        .set(
            "runtime.rig.backend_mode",
            AgentConfigValue::string("experimental"),
        )
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let error = RigBackendConfig::from_configuration(&configuration)
        .expect_err("unknown Rig backend mode must fail closed");

    assert_eq!(
        error.kind(),
        sdkwork_agent_kernel::KernelErrorKind::ValidationError
    );
}
