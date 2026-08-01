//! Layered settings resolution service.
//!
//! Settings are organized by scope layer with strict precedence:
//! enterprise managed > user > project > local. `AgentSettingsDocument`
//! holds per-scope layers; `AgentSettingsService::resolve` merges them
//! honoring an explicit `AgentSettingSources` selection (empty selection
//! loads nothing, `all()` loads the full hierarchy), aligning with the
//! agent SDK `settingSources` semantics.

use crate::{AgentSettingSources, AgentSettingsScope, KernelError, KernelResult};

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
}
