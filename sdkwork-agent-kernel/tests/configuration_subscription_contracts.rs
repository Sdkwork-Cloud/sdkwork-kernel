//! Contract tests for configuration change subscription, settings scopes,
//! and optimistic version locking.
//!
//! The configuration store notifies subscribers of save/migrate/archive
//! changes; `AgentSettingSources` expresses the explicit settings scope
//! selection (`settingSources`); optimistic saves conflict on stale versions.

use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationChange, AgentConfigurationProfile,
    AgentConfigurationStore, AgentSettingSources, AgentSettingsScope,
    InMemoryAgentConfigurationStore, KernelErrorKind,
};
use std::sync::{Arc, Mutex};

fn profile(agent_id: &str, profile_id: &str, version: &str) -> AgentConfigurationProfile {
    AgentConfigurationProfile::new(
        profile_id,
        agent_id,
        version,
        AgentConfiguration::new(agent_id, profile_id),
    )
}

#[test]
fn settings_scope_vocabulary_is_stable() {
    assert_eq!(AgentSettingsScope::Enterprise.as_str(), "enterprise");
    assert_eq!(AgentSettingsScope::User.as_str(), "user");
    assert_eq!(AgentSettingsScope::Project.as_str(), "project");
    assert_eq!(AgentSettingsScope::Local.as_str(), "local");

    assert_eq!(
        AgentSettingsScope::from_str("project"),
        Some(AgentSettingsScope::Project)
    );
    assert_eq!(AgentSettingsScope::from_str("team"), None);
}

#[test]
fn setting_sources_express_explicit_scope_selection() {
    let all = AgentSettingSources::all();
    assert!(all.allows(AgentSettingsScope::Enterprise));
    assert!(all.allows(AgentSettingsScope::Local));

    let none = AgentSettingSources::none();
    assert!(none.is_empty());
    assert!(!none.allows(AgentSettingsScope::User));

    let user_only = AgentSettingSources::none().with_scope(AgentSettingsScope::User);
    assert!(user_only.allows(AgentSettingsScope::User));
    assert!(!user_only.allows(AgentSettingsScope::Project));

    // Duplicate scope insertion is idempotent.
    let deduped = AgentSettingSources::none()
        .with_scope(AgentSettingsScope::Project)
        .with_scope(AgentSettingsScope::Project);
    assert_eq!(deduped.scopes.len(), 1);
}

#[test]
fn store_notifies_subscribers_on_save() {
    let mut store = InMemoryAgentConfigurationStore::new();
    let notifications: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = notifications.clone();

    store.subscribe(Box::new(move |record, change| {
        captured.lock().unwrap().push((
            record.profile.profile_id.clone(),
            change.as_str().to_string(),
        ));
    }));

    store
        .save_profile(profile("agent.1", "profile.default", "v1"))
        .expect("save succeeds");

    let notifications = notifications.lock().unwrap();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0],
        ("profile.default".to_string(), "saved".to_string())
    );
}

#[test]
fn store_notifies_subscribers_on_migrate_and_archive() {
    let mut store = InMemoryAgentConfigurationStore::new();
    let notifications: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = notifications.clone();

    store.subscribe(Box::new(move |_record, change| {
        captured.lock().unwrap().push(change.as_str().to_string());
    }));

    store
        .save_profile(profile("agent.1", "profile.a", "v1"))
        .unwrap();

    let plan = sdkwork_agent_kernel::AgentConfigurationUpgradePlan::new(
        "plan.1",
        "agent.1",
        "profile.a",
        "v1",
        "v2",
    );
    store
        .migrate_profile(&plan, profile("agent.1", "profile.a", "v2"))
        .unwrap();
    store
        .archive_profile(&sdkwork_agent_kernel::AgentProfileArchiveRequest::new(
            "archive.1",
            "agent.1",
            "profile.a",
        ))
        .unwrap();

    let notifications = notifications.lock().unwrap();
    assert_eq!(
        *notifications,
        vec![
            "saved".to_string(),
            "migrated".to_string(),
            "archived".to_string()
        ]
    );
}

#[test]
fn subscription_unsubscribe_detaches() {
    let mut store = InMemoryAgentConfigurationStore::new();
    let notifications: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = notifications.clone();

    let subscription = store.subscribe(Box::new(move |_record, change| {
        captured.lock().unwrap().push(change.as_str().to_string());
    }));

    store
        .save_profile(profile("agent.1", "profile.a", "v1"))
        .unwrap();
    subscription.unsubscribe();
    store
        .save_profile(profile("agent.1", "profile.a", "v2"))
        .unwrap();

    let notifications = notifications.lock().unwrap();
    assert_eq!(
        notifications.len(),
        1,
        "unsubscribed subscriber stops receiving"
    );
}

#[test]
fn optimistic_save_succeeds_on_matching_version() {
    let mut store = InMemoryAgentConfigurationStore::new();
    store
        .save_profile(profile("agent.1", "profile.a", "v1"))
        .unwrap();

    let record = store
        .save_profile_if_version(profile("agent.1", "profile.a", "v2"), "v1")
        .expect("matching version saves");
    assert_eq!(record.profile.profile_id, "profile.a");
}

#[test]
fn optimistic_save_conflicts_on_stale_version() {
    let mut store = InMemoryAgentConfigurationStore::new();
    store
        .save_profile(profile("agent.1", "profile.a", "v2"))
        .unwrap();

    let error = store
        .save_profile_if_version(profile("agent.1", "profile.a", "v3"), "v1")
        .expect_err("stale version conflicts");

    assert_eq!(error.kind(), KernelErrorKind::Conflict);
    assert!(error.to_string().contains("expected v1"));
}

#[test]
fn optimistic_save_conflicts_when_profile_missing() {
    let mut store = InMemoryAgentConfigurationStore::new();

    let error = store
        .save_profile_if_version(profile("agent.1", "profile.missing", "v1"), "v1")
        .expect_err("missing profile conflicts");

    assert_eq!(error.kind(), KernelErrorKind::Conflict);
    assert!(error.to_string().contains("does not exist"));
}

#[test]
fn optimistic_save_allows_create_with_empty_version() {
    let mut store = InMemoryAgentConfigurationStore::new();

    // Empty/zero expected version permits first creation.
    store
        .save_profile_if_version(profile("agent.1", "profile.first", "v1"), "")
        .expect("empty expected version permits create");
    let stored = store.load_profile("agent.1", "profile.first").unwrap();
    assert_eq!(stored.configuration_version, "v1");
}

#[test]
fn configuration_change_vocabulary_is_stable() {
    assert_eq!(AgentConfigurationChange::Saved.as_str(), "saved");
    assert_eq!(AgentConfigurationChange::Migrated.as_str(), "migrated");
    assert_eq!(AgentConfigurationChange::Archived.as_str(), "archived");
}
