//! Claude Code provider config materialization.
//!
//! The Claude Code CLI reads `~/.claude/settings.json`; its `env` section is
//! loaded into the CLI process environment. Applied model configurations are
//! materialized there (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`,
//! `ANTHROPIC_MODEL`) so the CLI actually routes through the configured relay
//! endpoint with the configured credential, exactly matching the upstream
//! config management surface.

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest,
    AgentModelSelectionRequest, KernelError, KernelResult,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config, merge_json_path, provider_user_home,
    update_provider_json_config,
};
use serde_json::{json, Value};

const ANTHROPIC_BASE_URL_ENV: &str = "ANTHROPIC_BASE_URL";
const ANTHROPIC_AUTH_TOKEN_ENV: &str = "ANTHROPIC_AUTH_TOKEN";
const ANTHROPIC_MODEL_ENV: &str = "ANTHROPIC_MODEL";

/// Resolves `~/.claude/settings.json` (no override when the home is unknown).
pub fn claude_code_settings_path() -> Option<std::path::PathBuf> {
    provider_user_home().map(|home| home.join(".claude").join("settings.json"))
}

/// Resolves the plaintext API key for materialization: the transient request
/// field first, then the host secret surface. Returns `None` when the key is
/// unavailable so the CLI keeps its own credential instead of failing.
fn resolve_materialization_api_key(request: &AgentModelConfigurationRequest) -> Option<String> {
    if let Some(api_key) = request
        .api_key_materialization
        .as_deref()
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty())
    {
        return Some(api_key.to_string());
    }
    sdkwork_agent_kernel::lookup_env_file_secret(&request.api_key_secret_ref)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_materialized_settings(
    current: Option<&Value>,
    request: &AgentModelConfigurationRequest,
    api_key: Option<&str>,
) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let mut env = serde_json::Map::new();
    if let Some(existing) = document.get("env").and_then(Value::as_object) {
        env = existing.clone();
    }
    env.insert(
        ANTHROPIC_BASE_URL_ENV.to_string(),
        Value::String(request.base_url.trim().to_string()),
    );
    if let Some(api_key) = api_key {
        env.insert(ANTHROPIC_AUTH_TOKEN_ENV.to_string(), Value::String(api_key.to_string()));
    }
    env.insert(
        ANTHROPIC_MODEL_ENV.to_string(),
        Value::String(request.default_model_id.trim().to_string()),
    );
    merge_json_path(
        &mut document,
        &["env"],
        Value::Object(env),
    );
    Ok(document)
}

fn update_selected_model(current: Option<&Value>, model_id: &str) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let mut env = document
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    env.insert(ANTHROPIC_MODEL_ENV.to_string(), Value::String(model_id.to_string()));
    merge_json_path(&mut document, &["env"], Value::Object(env));
    Ok(document)
}

/// Materializes a Claude Code model configuration into `~/.claude/settings.json`.
pub fn materialize_claude_code_model_configuration(
    request: &AgentModelConfigurationRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = claude_code_settings_path() else {
        return Ok(());
    };
    materialize_claude_code_model_configuration_at(&path, request, application)
}

/// Materializes a Claude Code model configuration into an explicit settings file.
pub(crate) fn materialize_claude_code_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_provider_json_config(path, |current| {
        build_materialized_settings(current, request, api_key.as_deref())
    })
}

/// Materializes a Claude Code model selection (updates `ANTHROPIC_MODEL`).
pub fn materialize_claude_code_model_selection(
    request: &AgentModelSelectionRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = claude_code_settings_path() else {
        return Ok(());
    };
    materialize_claude_code_model_selection_at(&path, request, application)
}

/// Materializes a Claude Code model selection into an explicit settings file.
pub(crate) fn materialize_claude_code_model_selection_at(
    path: &std::path::Path,
    request: &AgentModelSelectionRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    update_provider_json_config(path, |current| {
        update_selected_model(current, request.model_id.trim())
    })
}

/// Restores the pre-materialization `~/.claude/settings.json` backup.
pub fn dematerialize_claude_code_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = claude_code_settings_path() else {
        return Ok(());
    };
    dematerialize_provider_config(&path)
}

/// Restores the pre-materialization backup for an explicit settings file.
pub(crate) fn dematerialize_claude_code_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config(path)
}

/// Shared test fixtures and assertions for Claude-Code-shaped settings files.
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn request_with(
        api_key: Option<&str>,
        base_url: &str,
        model: &str,
    ) -> AgentModelConfigurationRequest {
        let mut request = AgentModelConfigurationRequest::new(
            "request-1",
            "agent.code-engine.claude-code",
            "profile.test",
            "anthropic",
            base_url,
            "secret.ref",
            model,
        );
        if let Some(api_key) = api_key {
            request = request.with_api_key_materialization(api_key);
        }
        request
    }

    #[test]
    fn build_settings_sets_env_variables() {
        let request = request_with(
            Some("token-abc"),
            "https://api.birdcoder.com",
            "claude-sonnet-4-5",
        );
        let document =
            build_materialized_settings(None, &request, Some("token-abc")).expect("build");
        let env = document["env"].as_object().expect("env object");
        assert_eq!(env["ANTHROPIC_BASE_URL"].as_str(), Some("https://api.birdcoder.com"));
        assert_eq!(env["ANTHROPIC_AUTH_TOKEN"].as_str(), Some("token-abc"));
        assert_eq!(env["ANTHROPIC_MODEL"].as_str(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn build_settings_merges_existing_environment() {
        let existing = json!({
            "env": { "ANTHROPIC_SMALL_FAST_MODEL": "claude-haiku", "OTHER": "keep" },
            "permissions": { "allow": ["Bash(*)" ] }
        });
        let request = request_with(None, "https://api.birdcoder.com", "claude-sonnet-4-5");
        let document =
            build_materialized_settings(Some(&existing), &request, None).expect("merge");
        let env = document["env"].as_object().expect("env object");
        assert_eq!(env["OTHER"].as_str(), Some("keep"));
        assert_eq!(env["ANTHROPIC_BASE_URL"].as_str(), Some("https://api.birdcoder.com"));
        assert!(document["permissions"].is_object());
    }

    #[test]
    fn update_selected_model_keeps_other_env() {
        let existing = json!({ "env": { "ANTHROPIC_BASE_URL": "https://x", "ANTHROPIC_MODEL": "a" } });
        let document = update_selected_model(Some(&existing), "claude-opus-4-5").expect("update");
        let env = document["env"].as_object().expect("env object");
        assert_eq!(env["ANTHROPIC_MODEL"].as_str(), Some("claude-opus-4-5"));
        assert_eq!(env["ANTHROPIC_BASE_URL"].as_str(), Some("https://x"));
    }
}
