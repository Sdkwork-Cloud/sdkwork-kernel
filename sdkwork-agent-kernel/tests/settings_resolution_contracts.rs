//! Contract tests: layered settings resolution.
//!
//! Settings follow the agent SDK hierarchy (enterprise managed > user >
//! project > local) and honor explicit `settingSources` selection. The
//! service composes with `AgentSettingSources` from the configuration
//! SPI without importing filesystem settings.

use sdkwork_agent_kernel::{
    AgentSettingEntry, AgentSettingSources, AgentSettingsDocument, AgentSettingsScope,
    AgentSettingsService,
};

#[test]
fn resolves_highest_precedence_value_for_each_key() {
    let mut document = AgentSettingsDocument::new();
    document
        .set(AgentSettingsScope::Enterprise, "model", "opus")
        .set(AgentSettingsScope::User, "model", "sonnet")
        .set(AgentSettingsScope::Project, "model", "haiku")
        .set(AgentSettingsScope::Local, "model", "local-model")
        .set(AgentSettingsScope::User, "permission_mode", "default");

    let resolved = document.resolve(&AgentSettingSources::all());

    assert_eq!(
        resolved
            .iter()
            .find(|entry| entry.key == "model")
            .expect("model resolved"),
        &AgentSettingEntry::new("model", "opus", AgentSettingsScope::Enterprise)
    );
    assert_eq!(resolved.len(), 2, "one entry per unique key");
    assert_eq!(
        resolved
            .iter()
            .find(|entry| entry.key == "permission_mode")
            .expect("permission_mode resolved")
            .scope,
        AgentSettingsScope::User
    );
}

#[test]
fn get_returns_highest_precedence_entry() {
    let mut document = AgentSettingsDocument::new();
    document
        .set(AgentSettingsScope::Project, "sandbox", "off")
        .set(AgentSettingsScope::User, "sandbox", "read-only");

    let entry = document
        .get("sandbox", &AgentSettingSources::all())
        .expect("sandbox resolved");
    assert_eq!(entry.value, "read-only");
    assert_eq!(entry.scope, AgentSettingsScope::User);
}

#[test]
fn none_sources_load_no_settings() {
    let mut document = AgentSettingsDocument::new();
    document.set(AgentSettingsScope::User, "model", "sonnet");

    assert!(document.resolve(&AgentSettingSources::none()).is_empty());
    assert!(document
        .get("model", &AgentSettingSources::none())
        .is_none());
}

#[test]
fn partial_source_selection_filters_scopes() {
    let mut document = AgentSettingsDocument::new();
    document
        .set(AgentSettingsScope::Enterprise, "env", "prod")
        .set(AgentSettingsScope::User, "model", "sonnet")
        .set(AgentSettingsScope::Local, "theme", "dark");

    let sources = AgentSettingSources::none().with_scope(AgentSettingsScope::User);
    let resolved = document.resolve(&sources);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].key, "model");
    assert_eq!(resolved[0].scope, AgentSettingsScope::User);
}

#[test]
fn later_set_replaces_within_scope() {
    let mut document = AgentSettingsDocument::new();
    document
        .set(AgentSettingsScope::User, "model", "first")
        .set(AgentSettingsScope::User, "model", "second");

    assert_eq!(
        document.scope_entries(AgentSettingsScope::User).len(),
        1,
        "same-scope set replaces the prior value"
    );
    assert_eq!(
        document
            .get("model", &AgentSettingSources::all())
            .expect("model resolved")
            .value,
        "second"
    );
}

#[test]
fn merge_keeps_precedence_of_overlay() {
    let mut base = AgentSettingsDocument::new();
    base.set(AgentSettingsScope::User, "model", "sonnet").set(
        AgentSettingsScope::User,
        "theme",
        "light",
    );

    let mut overlay = AgentSettingsDocument::new();
    overlay.set(AgentSettingsScope::User, "model", "opus").set(
        AgentSettingsScope::Enterprise,
        "theme",
        "dark",
    );

    base.merge(&overlay);
    let resolved = base.resolve(&AgentSettingSources::all());

    assert_eq!(
        resolved
            .iter()
            .find(|entry| entry.key == "model")
            .unwrap()
            .value,
        "opus",
        "overlay same-scope set replaces the base value"
    );
    assert_eq!(
        resolved
            .iter()
            .find(|entry| entry.key == "theme")
            .unwrap()
            .value,
        "dark",
        "overlay higher-precedence scope wins over base lower scope"
    );
}

#[test]
fn entries_carry_optional_source_identifiers() {
    let mut document = AgentSettingsDocument::new();
    document.set_entry(
        AgentSettingEntry::new("model", "opus", AgentSettingsScope::Enterprise)
            .with_source("enterprise://managed/settings.json"),
    );

    let entry = document
        .get("model", &AgentSettingSources::all())
        .expect("model resolved");
    assert_eq!(
        entry.source.as_deref(),
        Some("enterprise://managed/settings.json")
    );
}

#[test]
fn typed_json_values_are_accessible() {
    let mut document = AgentSettingsDocument::new();
    document.set(
        AgentSettingsScope::User,
        "max_tokens",
        r#"{"value": 4096, "unit": "tokens"}"#,
    );

    let entry = document
        .get("max_tokens", &AgentSettingSources::all())
        .expect("max_tokens resolved");
    let json = entry.value_json().expect("JSON value parses");
    assert_eq!(json["value"], 4096);
    assert_eq!(json["unit"], "tokens");
}

#[test]
fn service_validates_keys() {
    assert!(AgentSettingsService::validate_key("model").is_ok());
    assert!(AgentSettingsService::validate_key("permission_mode").is_ok());
    assert!(AgentSettingsService::validate_key("").is_err());
    assert!(AgentSettingsService::validate_key("  ").is_err());
    assert!(AgentSettingsService::validate_key("bad\nkey").is_err());
}

#[test]
fn scopes_round_trip_through_strings() {
    for scope in [
        AgentSettingsScope::Enterprise,
        AgentSettingsScope::User,
        AgentSettingsScope::Project,
        AgentSettingsScope::Local,
    ] {
        assert_eq!(AgentSettingsScope::from_str(scope.as_str()), Some(scope));
    }
    assert_eq!(AgentSettingsScope::from_str("unknown"), None);
    assert_eq!(
        AgentSettingsScope::as_str(&AgentSettingsScope::User),
        "user"
    );
}
