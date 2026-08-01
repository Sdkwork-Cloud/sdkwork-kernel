//! Layered settings resolution service.
//!
//! Settings are organized by scope layer with strict precedence:
//! enterprise managed > user > project > local. `AgentSettingsDocument`
//! holds per-scope layers; `AgentSettingsService::resolve` merges them
//! honoring an explicit `AgentSettingSources` selection (empty selection
//! loads nothing, `all()` loads the full hierarchy), aligning with the
//! agent SDK `settingSources` semantics.

use crate::{
    AgentConfigValue, AgentConfiguration, AgentSettingSources, AgentSettingsScope, KernelError,
    KernelResult,
};

/// A single settings entry with its origin scope and optional source
/// identifier (file path or URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSettingEntry {
    pub key: String,
    pub value: String,
    pub scope: AgentSettingsScope,
    pub source: Option<String>,
}

impl AgentSettingEntry {
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        scope: AgentSettingsScope,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            scope,
            source: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Parse the value as a typed JSON value when possible.
    pub fn value_json(&self) -> Option<serde_json::Value> {
        serde_json::from_str(&self.value).ok()
    }
}

/// Settings document with per-scope layers. Within a scope, later `set`
/// calls replace earlier values for the same key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentSettingsDocument {
    layers: Vec<(AgentSettingsScope, Vec<AgentSettingEntry>)>,
}

impl AgentSettingsDocument {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &mut self,
        scope: AgentSettingsScope,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> &mut Self {
        let key = key.into();
        let value = value.into();
        let layer = self.layer_mut(scope);
        if let Some(entry) = layer.iter_mut().find(|entry| entry.key == key) {
            entry.value = value;
        } else {
            layer.push(AgentSettingEntry::new(key, value, scope));
        }
        self
    }

    pub fn set_entry(&mut self, entry: AgentSettingEntry) -> &mut Self {
        let scope = entry.scope;
        let layer = self.layer_mut(scope);
        if let Some(existing) = layer.iter_mut().find(|e| e.key == entry.key) {
            *existing = entry;
        } else {
            layer.push(entry);
        }
        self
    }

    pub fn scope_entries(&self, scope: AgentSettingsScope) -> &[AgentSettingEntry] {
        self.layers
            .iter()
            .find(|(layer_scope, _)| *layer_scope == scope)
            .map(|(_, entries)| entries.as_slice())
            .unwrap_or(&[])
    }

    pub fn scopes(&self) -> Vec<AgentSettingsScope> {
        self.layers.iter().map(|(scope, _)| *scope).collect()
    }

    /// Merge another document into this one. Precedence rules:
    /// - entries from `other` replace same-scope same-key entries here
    /// - higher-precedence scopes in `other` win over lower scopes here
    ///   when the same key exists in both.
    pub fn merge(&mut self, other: &AgentSettingsDocument) {
        for (_, entries) in &other.layers {
            for entry in entries {
                self.set_entry(entry.clone());
            }
        }
    }

    /// Resolve the merged view: entries from enabled scopes ordered by
    /// precedence (enterprise first), with the highest-precedence value for
    /// each key.
    pub fn resolve(&self, sources: &AgentSettingSources) -> Vec<AgentSettingEntry> {
        let mut ordered = Vec::new();
        for scope in [
            AgentSettingsScope::Enterprise,
            AgentSettingsScope::User,
            AgentSettingsScope::Project,
            AgentSettingsScope::Local,
        ] {
            if !sources.allows(scope) {
                continue;
            }
            for entry in self.scope_entries(scope) {
                // First occurrence wins: lower-precedence scopes only fill
                // keys that no higher-precedence scope has set.
                if !ordered
                    .iter()
                    .any(|e: &AgentSettingEntry| e.key == entry.key)
                {
                    ordered.push(entry.clone());
                }
            }
        }
        ordered
    }

    /// Resolved value for a key under the given sources.
    pub fn get(&self, key: &str, sources: &AgentSettingSources) -> Option<AgentSettingEntry> {
        self.resolve(sources)
            .into_iter()
            .find(|entry| entry.key == key)
    }

    /// Project a configuration snapshot into this document under `scope`.
    ///
    /// Bridges the configuration store and the settings hierarchy: every
    /// `AgentConfigEntry` becomes an `AgentSettingEntry` in the given
    /// scope layer (later loads replace same-key entries within the
    /// scope). `SecretRef` entries are **not** projected — secret values
    /// never flow through the settings value surface; they resolve only
    /// through the kernel secret providers.
    pub fn load_configuration(
        &mut self,
        configuration: &AgentConfiguration,
        scope: AgentSettingsScope,
    ) -> &mut Self {
        for entry in &configuration.entries {
            let Some(value) = project_config_value(&entry.value) else {
                continue;
            };
            self.set(scope, entry.key.clone(), value);
        }
        self
    }

    fn layer_mut(&mut self, scope: AgentSettingsScope) -> &mut Vec<AgentSettingEntry> {
        let index = self
            .layers
            .iter()
            .position(|(layer_scope, _)| *layer_scope == scope)
            .unwrap_or_else(|| {
                self.layers.push((scope, Vec::new()));
                self.layers.len() - 1
            });
        &mut self.layers[index].1
    }
}

/// Settings resolution service: pure functions over layered documents.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentSettingsService;

impl AgentSettingsService {
    pub fn new() -> Self {
        Self
    }

    /// Validate a settings key: non-empty, no control characters.
    pub fn validate_key(key: &str) -> KernelResult<()> {
        if key.trim().is_empty() {
            return Err(KernelError::validation("settings key must not be blank"));
        }
        if key.chars().any(char::is_control) {
            return Err(KernelError::validation(
                "settings key must not contain control characters",
            ));
        }
        Ok(())
    }
}

/// Project an `AgentConfigValue` to its settings string representation.
/// `SecretRef` values return `None`: secret references never enter the
/// settings value surface (kernel secrets resolve only via secret
/// providers). `StringList` projects to a JSON array string.
fn project_config_value(value: &AgentConfigValue) -> Option<String> {
    match value {
        AgentConfigValue::String(value) => Some(value.clone()),
        AgentConfigValue::Boolean(value) => Some(value.to_string()),
        AgentConfigValue::Integer(value) => Some(value.to_string()),
        AgentConfigValue::StringList(values) => serde_json::to_string(values).ok(),
        AgentConfigValue::Json(value) => Some(value.clone()),
        AgentConfigValue::SecretRef(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_resolves_with_precedence() {
        let mut document = AgentSettingsDocument::new();
        document
            .set(AgentSettingsScope::Enterprise, "model", "opus")
            .set(AgentSettingsScope::User, "model", "sonnet")
            .set(AgentSettingsScope::Project, "model", "haiku")
            .set(AgentSettingsScope::Project, "sandbox", "read-only");

        let resolved = document.resolve(&AgentSettingSources::all());
        let model = resolved
            .iter()
            .find(|entry| entry.key == "model")
            .expect("model present");
        assert_eq!(model.value, "opus");
        assert_eq!(model.scope, AgentSettingsScope::Enterprise);
    }

    #[test]
    fn resolve_respects_source_selection() {
        let mut document = AgentSettingsDocument::new();
        document
            .set(AgentSettingsScope::User, "model", "sonnet")
            .set(AgentSettingsScope::Local, "model", "local-model");

        let user_only = AgentSettingSources::none().with_scope(AgentSettingsScope::User);
        let resolved = document.resolve(&user_only);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].value, "sonnet");

        assert!(document
            .get("model", &AgentSettingSources::none())
            .is_none());
    }

    #[test]
    fn set_replaces_within_scope() {
        let mut document = AgentSettingsDocument::new();
        document
            .set(AgentSettingsScope::User, "model", "first")
            .set(AgentSettingsScope::User, "model", "second");
        assert_eq!(document.scope_entries(AgentSettingsScope::User).len(), 1);
        assert_eq!(
            document
                .get("model", &AgentSettingSources::all())
                .unwrap()
                .value,
            "second"
        );
    }

    #[test]
    fn merge_combines_layers() {
        let mut base = AgentSettingsDocument::new();
        base.set(AgentSettingsScope::User, "model", "sonnet");

        let mut overlay = AgentSettingsDocument::new();
        overlay.set(AgentSettingsScope::User, "model", "opus").set(
            AgentSettingsScope::Local,
            "theme",
            "dark",
        );

        base.merge(&overlay);
        let resolved = base.resolve(&AgentSettingSources::all());
        assert_eq!(resolved.len(), 2);
        assert_eq!(
            resolved.iter().find(|e| e.key == "model").unwrap().value,
            "opus"
        );
        assert_eq!(
            resolved.iter().find(|e| e.key == "theme").unwrap().value,
            "dark"
        );
    }

    #[test]
    fn entry_json_value_parses() {
        let entry =
            AgentSettingEntry::new("limits", r#"{"max_turns":10}"#, AgentSettingsScope::User);
        let value = entry.value_json().expect("json parses");
        assert_eq!(value["max_turns"], 10);
    }

    #[test]
    fn service_validates_keys() {
        assert!(AgentSettingsService::validate_key("model").is_ok());
        assert!(AgentSettingsService::validate_key(" ").is_err());
        assert!(AgentSettingsService::validate_key("bad\nkey").is_err());
    }

    #[test]
    fn load_configuration_projects_typed_values_into_scope() {
        let mut configuration = AgentConfiguration::new("agent-1", "profile-1");
        configuration = configuration
            .set("model", AgentConfigValue::string("opus"))
            .set("max_tokens", AgentConfigValue::integer(4096))
            .set("streaming", AgentConfigValue::boolean(true))
            .set(
                "extensions",
                AgentConfigValue::string_list(vec!["a".into(), "b".into()]),
            );

        let mut document = AgentSettingsDocument::new();
        document.load_configuration(&configuration, AgentSettingsScope::User);

        let sources = AgentSettingSources::all();
        assert_eq!(document.get("model", &sources).unwrap().value, "opus");
        assert_eq!(document.get("max_tokens", &sources).unwrap().value, "4096");
        assert_eq!(document.get("streaming", &sources).unwrap().value, "true");
        assert_eq!(
            document.get("extensions", &sources).unwrap().value,
            r#"["a","b"]"#
        );
    }

    #[test]
    fn load_configuration_skips_secret_refs() {
        let mut configuration = AgentConfiguration::new("agent-1", "profile-1");
        configuration = configuration
            .set("api_key", AgentConfigValue::secret_ref("secret://llm/acme"))
            .set("model", AgentConfigValue::string("opus"));

        let mut document = AgentSettingsDocument::new();
        document.load_configuration(&configuration, AgentSettingsScope::User);

        let sources = AgentSettingSources::all();
        assert!(
            document.get("api_key", &sources).is_none(),
            "secret refs must not flow into the settings value surface"
        );
        assert_eq!(document.get("model", &sources).unwrap().value, "opus");
    }

    #[test]
    fn load_configuration_reload_replaces_within_scope() {
        let mut first = AgentConfiguration::new("agent-1", "profile-1");
        first = first.set("model", AgentConfigValue::string("sonnet"));
        let mut second = AgentConfiguration::new("agent-1", "profile-1");
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
            "opus",
            "later configuration load replaces the earlier value in scope"
        );
    }

    #[test]
    fn configuration_projection_plays_with_layer_precedence() {
        let mut project_config = AgentConfiguration::new("agent-1", "project-profile");
        project_config = project_config.set("model", AgentConfigValue::string("haiku"));
        let mut user_config = AgentConfiguration::new("agent-1", "user-profile");
        user_config = user_config.set("model", AgentConfigValue::string("sonnet"));

        let mut document = AgentSettingsDocument::new();
        document
            .load_configuration(&project_config, AgentSettingsScope::Project)
            .load_configuration(&user_config, AgentSettingsScope::User);

        assert_eq!(
            document
                .get("model", &AgentSettingSources::all())
                .unwrap()
                .value,
            "sonnet",
            "higher-precedence user layer wins over project layer"
        );
    }
}
