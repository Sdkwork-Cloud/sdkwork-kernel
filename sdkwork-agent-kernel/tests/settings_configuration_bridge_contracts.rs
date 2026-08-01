//! Contract tests: configuration -> settings bridge.
//!
//! `AgentSettingsDocument::load_configuration` projects configuration
//! snapshots into the layered settings hierarchy. Secret references are
//! never projected (kernel secrets resolve only through secret
//! providers), and the resulting layers participate in normal settings
//! precedence under explicit `settingSources` selection.

use sdkwork_agent_kernel::{
    AgentConfigValue, AgentConfiguration, AgentSettingSources, AgentSettingsDocument,
    AgentSettingsScope,
};

#[test]
fn projects_full_configuration_snapshot_into_scope_layer() {
    let mut configuration = AgentConfiguration::new("agent-1", "profile-1");
    configuration = configuration
        .set("model", AgentConfigValue::string("opus"))
        .set("temperature", AgentConfigValue::integer(7))
        .set("streaming", AgentConfigValue::boolean(true))
        .set(
            "extensions",
            AgentConfigValue::string_list(vec!["mcp".into(), "skills".into()]),
        )
        .set("policy", AgentConfigValue::json(r#"{"mode":"default"}"#));

    let mut document = AgentSettingsDocument::new();
    document.load_configuration(&configuration, AgentSettingsScope::User);

    let resolved = document.resolve(&AgentSettingSources::all());
    assert_eq!(resolved.len(), 5, "every non-secret entry projects once");
    assert_eq!(
        document
            .get("model", &AgentSettingSources::all())
            .unwrap()
            .value,
        "opus"
    );
    assert_eq!(
        document
            .get("temperature", &AgentSettingSources::all())
            .unwrap()
            .value,
        "7"
    );
    assert_eq!(
        document
            .get("extensions", &AgentSettingSources::all())
            .unwrap()
            .value,
        r#"["mcp","skills"]"#
    );
    assert_eq!(
        document
            .get("policy", &AgentSettingSources::all())
            .unwrap()
            .value,
        r#"{"mode":"default"}"#
    );
}

#[test]
fn secret_refs_never_enter_the_settings_value_surface() {
    let mut configuration = AgentConfiguration::new("agent-1", "profile-1");
    configuration = configuration
        .set("api_key", AgentConfigValue::secret_ref("secret://llm/acme"))
        .set("model", AgentConfigValue::string("opus"));

    let mut document = AgentSettingsDocument::new();
    document.load_configuration(&configuration, AgentSettingsScope::User);

    let sources = AgentSettingSources::all();
    assert!(document.get("api_key", &sources).is_none());
    assert_eq!(document.resolve(&sources).len(), 1);
}

#[test]
fn later_load_replaces_same_key_within_scope() {
    let mut first = AgentConfiguration::new("agent-1", "profile-1");
    first = first.set("model", AgentConfigValue::string("sonnet"));
    let mut second = AgentConfiguration::new("agent-1", "profile-2");
    second = second.set("model", AgentConfigValue::string("opus"));

    let mut document = AgentSettingsDocument::new();
    document
        .load_configuration(&first, AgentSettingsScope::User)
        .load_configuration(&second, AgentSettingsScope::User);

    assert_eq!(
        document
            .get("model", &AgentSettingSources::all())
            .unwrap()
            .value,
        "opus"
    );
}

#[test]
fn projected_layers_follow_settings_precedence() {
    let mut enterprise = AgentConfiguration::new("agent-1", "enterprise-profile");
    enterprise = enterprise.set("model", AgentConfigValue::string("opus"));
    let mut user = AgentConfiguration::new("agent-1", "user-profile");
    user = user
        .set("model", AgentConfigValue::string("sonnet"))
        .set("theme", AgentConfigValue::string("dark"));

    let mut document = AgentSettingsDocument::new();
    document
        .load_configuration(&enterprise, AgentSettingsScope::Enterprise)
        .load_configuration(&user, AgentSettingsScope::User);

    let sources = AgentSettingSources::all();
    assert_eq!(document.get("model", &sources).unwrap().value, "opus");
    assert_eq!(document.get("theme", &sources).unwrap().value, "dark");

    let user_only = AgentSettingSources::none().with_scope(AgentSettingsScope::User);
    assert_eq!(document.get("model", &user_only).unwrap().value, "sonnet");
    assert!(document.get("theme", &user_only).is_some());
}

#[test]
fn configuration_projection_does_not_mutate_the_source() {
    let mut configuration = AgentConfiguration::new("agent-1", "profile-1");
    configuration = configuration.set("model", AgentConfigValue::string("opus"));

    let mut document = AgentSettingsDocument::new();
    document.load_configuration(&configuration, AgentSettingsScope::User);

    assert_eq!(
        configuration.entries.len(),
        1,
        "source configuration is untouched"
    );
    assert_eq!(
        configuration.value("model").unwrap(),
        &AgentConfigValue::string("opus")
    );
}
