//! Mimo Code provider config materialization.
//!
//! Mimo Code is a Claude Code fork and consumes the same settings surface: `~/.claude/settings.json (Mimo Code)`; its `env` section is
//! loaded into the CLI process environment. Applied model configurations are
//! materialized there (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`,
//! `ANTHROPIC_MODEL`) so the CLI actually routes through the configured relay
//! endpoint with the configured credential, exactly matching the upstream
//! config management surface (Mimo Code reads the same file).

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest, AgentModelSelectionRequest,
    KernelError, KernelResult, ProviderModelConfigurationStatus, ProviderModelMaterializationState,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config_named, merge_json_path, provider_user_home, read_provider_config,
    update_provider_json_config_named,
};
use serde_json::{json, Value};

const ANTHROPIC_BASE_URL_ENV: &str = "ANTHROPIC_BASE_URL";
const ANTHROPIC_AUTH_TOKEN_ENV: &str = "ANTHROPIC_AUTH_TOKEN";
const ANTHROPIC_MODEL_ENV: &str = "ANTHROPIC_MODEL";
/// Marker written next to the materialized values so read-back can tell
/// "SDKWork materialized this env block" apart from a user-configured relay.
/// The CLI ignores unknown environment names, so the marker is inert at
/// request time.
const SDKWORK_MANAGED_MARKER_ENV: &str = "SDKWORK_MANAGED";

/// Resolves `~/.claude/settings.json (Mimo Code)` (no override when the home is unknown).
pub fn mimo_code_settings_path() -> Option<std::path::PathBuf> {
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
        env.insert(
            ANTHROPIC_AUTH_TOKEN_ENV.to_string(),
            Value::String(api_key.to_string()),
        );
    }
    env.insert(
        ANTHROPIC_MODEL_ENV.to_string(),
        Value::String(request.default_model_id.trim().to_string()),
    );
    env.insert(
        SDKWORK_MANAGED_MARKER_ENV.to_string(),
        Value::String("true".to_string()),
    );
    merge_json_path(&mut document, &["env"], Value::Object(env))?;
    Ok(document)
}

fn update_selected_model(current: Option<&Value>, model_id: &str) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let mut env = document
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    env.insert(
        ANTHROPIC_MODEL_ENV.to_string(),
        Value::String(model_id.to_string()),
    );
    merge_json_path(&mut document, &["env"], Value::Object(env))?;
    Ok(document)
}

/// Materializes a Claude Code model configuration into `~/.claude/settings.json (Mimo Code)`.
pub fn materialize_mimo_code_model_configuration(
    request: &AgentModelConfigurationRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = mimo_code_settings_path() else {
        return Err(KernelError::provider_error(
                "provider_config_path",
                "could not resolve the ~/.claude/settings.json (Mimo Code) path: user home is unavailable",
            ));
    };
    materialize_mimo_code_model_configuration_at(&path, request, application)
}

/// Materializes a Claude Code model configuration into an explicit settings file.
pub(crate) fn materialize_mimo_code_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_provider_json_config_named(path, "mimo-code", |current| {
        build_materialized_settings(current, request, api_key.as_deref())
    })
}

/// Materializes a Claude Code model selection (updates `ANTHROPIC_MODEL`).
pub fn materialize_mimo_code_model_selection(
    request: &AgentModelSelectionRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = mimo_code_settings_path() else {
        return Err(KernelError::provider_error(
                "provider_config_path",
                "could not resolve the ~/.claude/settings.json (Mimo Code) path: user home is unavailable",
            ));
    };
    materialize_mimo_code_model_selection_at(&path, request, application)
}

/// Materializes a Claude Code model selection into an explicit settings file.
pub(crate) fn materialize_mimo_code_model_selection_at(
    path: &std::path::Path,
    request: &AgentModelSelectionRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    update_provider_json_config_named(path, "mimo-code", |current| {
        update_selected_model(current, request.model_id.trim())
    })
}

/// Restores the pre-materialization `~/.claude/settings.json (Mimo Code)` backup.
pub fn dematerialize_mimo_code_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = mimo_code_settings_path() else {
        return Err(KernelError::provider_error(
                "provider_config_path",
                "could not resolve the ~/.claude/settings.json (Mimo Code) path: user home is unavailable",
            ));
    };
    dematerialize_mimo_code_model_configuration_at(&path)
}

/// Restores the pre-materialization backup for an explicit settings file.
pub(crate) fn dematerialize_mimo_code_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config_named(path, "mimo-code")
}

/// Reads the currently effective Mimo Code model configuration back from
/// `~/.claude/settings.json` (the shared Claude-Code-shaped settings surface)
/// and reports the materialization state.
pub fn read_mimo_code_model_configuration() -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(path) = mimo_code_settings_path() else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "mimo-code",
        ));
    };
    read_mimo_code_model_configuration_at(&path)
}

/// Reads the effective Mimo Code model configuration back from an explicit
/// settings file (used by tests and by config surfaces with a known path).
pub(crate) fn read_mimo_code_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(content) = read_provider_config(path)? else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "mimo-code",
        ));
    };
    let document = match serde_json::from_str::<Value>(&content) {
        Ok(document) => document,
        Err(error) => {
            return Ok(ProviderModelConfigurationStatus {
                provider_scope: "mimo-code".to_string(),
                materialization: ProviderModelMaterializationState::Diverged,
                effective_base_url: None,
                effective_default_model: None,
                credential_configured: false,
                issues: vec![format!(
                    "{} could not be parsed as JSON: {error}",
                    path.display()
                )],
            });
        }
    };
    let env = document.get("env").and_then(Value::as_object);
    let sdkwork_managed = env
        .and_then(|env| env.get(SDKWORK_MANAGED_MARKER_ENV))
        .and_then(Value::as_str)
        .is_some_and(|value| value == "true");
    let effective_base_url = env
        .and_then(|env| env.get(ANTHROPIC_BASE_URL_ENV))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let effective_default_model = env
        .and_then(|env| env.get(ANTHROPIC_MODEL_ENV))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let credential_configured = env
        .and_then(|env| env.get(ANTHROPIC_AUTH_TOKEN_ENV))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty());
    // Only the SDKWork-managed marker proves the entry was materialized by
    // this platform; a user-configured relay must not be reported as
    // materialized, and a marker without its values is drift.
    let (materialization, issues) = if !sdkwork_managed {
        (
            ProviderModelMaterializationState::NotMaterialized,
            Vec::new(),
        )
    } else if effective_base_url.is_none() {
        (
            ProviderModelMaterializationState::Diverged,
            vec![format!(
                "SDKWork-managed marker exists but {ANTHROPIC_BASE_URL_ENV} is missing"
            )],
        )
    } else {
        (ProviderModelMaterializationState::Materialized, Vec::new())
    };
    Ok(ProviderModelConfigurationStatus {
        provider_scope: "mimo-code".to_string(),
        materialization,
        effective_base_url,
        effective_default_model,
        credential_configured,
        issues,
    })
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
            "agent.code-engine.mimo-code",
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
        assert_eq!(
            env["ANTHROPIC_BASE_URL"].as_str(),
            Some("https://api.birdcoder.com")
        );
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
        let document = build_materialized_settings(Some(&existing), &request, None).expect("merge");
        let env = document["env"].as_object().expect("env object");
        assert_eq!(env["OTHER"].as_str(), Some("keep"));
        assert_eq!(
            env["ANTHROPIC_BASE_URL"].as_str(),
            Some("https://api.birdcoder.com")
        );
        assert!(document["permissions"].is_object());
    }

    #[test]
    fn update_selected_model_keeps_other_env() {
        let existing =
            json!({ "env": { "ANTHROPIC_BASE_URL": "https://x", "ANTHROPIC_MODEL": "a" } });
        let document = update_selected_model(Some(&existing), "claude-opus-4-5").expect("update");
        let env = document["env"].as_object().expect("env object");
        assert_eq!(env["ANTHROPIC_MODEL"].as_str(), Some("claude-opus-4-5"));
        assert_eq!(env["ANTHROPIC_BASE_URL"].as_str(), Some("https://x"));
    }

    #[test]
    fn read_back_reports_materialized_settings_env() {
        let dir =
            std::env::temp_dir().join(format!("sdkwork-mimo-code-readback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            json!({
                "env": {
                    "SDKWORK_MANAGED": "true",
                    "ANTHROPIC_BASE_URL": "https://api.birdcoder.com",
                    "ANTHROPIC_AUTH_TOKEN": "token-abc",
                    "ANTHROPIC_MODEL": "claude-sonnet-4-5"
                }
            })
            .to_string(),
        )
        .expect("seed");

        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Materialized
        );
        assert_eq!(
            status.effective_base_url.as_deref(),
            Some("https://api.birdcoder.com")
        );
        assert_eq!(
            status.effective_default_model.as_deref(),
            Some("claude-sonnet-4-5")
        );
        assert!(status.credential_configured);
        assert!(status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_not_materialized_when_file_absent() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-mimo-code-readback-absent-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.json");

        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::NotMaterialized
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_never_reports_user_relay_without_marker_as_materialized() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-mimo-code-readback-user-relay-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://user-relay.example.com",
                    "ANTHROPIC_MODEL": "claude-sonnet-4-5"
                }
            })
            .to_string(),
        )
        .expect("seed");

        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::NotMaterialized,
            "a user-configured relay without the SDKWork marker must not be reported as materialized"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_diverged_when_marker_exists_but_values_missing() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-mimo-code-readback-diverged-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            json!({ "env": { "SDKWORK_MANAGED": "true" } }).to_string(),
        )
        .expect("seed");

        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Diverged,
            "a marker without its values is drift"
        );
        assert!(!status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialize_then_read_back_round_trip_on_the_same_file() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-mimo-code-roundtrip-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            json!({ "permissions": { "allow": ["Bash(*)"] } }).to_string(),
        )
        .expect("seed");
        let request = request_with(
            Some("token-abc"),
            "https://api.birdcoder.com",
            "claude-sonnet-4-5",
        );
        let application = AgentModelConfigurationApplication::new(
            "request-1",
            "mimo-code",
            sdkwork_agent_kernel::AgentConfigurationProfile::new(
                "profile.test",
                "agent.test",
                "1",
                sdkwork_agent_kernel::AgentConfiguration::new("agent.test", "profile.test"),
            ),
        );

        materialize_mimo_code_model_configuration_at(&path, &request, &application)
            .expect("materialize");
        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Materialized
        );
        assert_eq!(
            status.effective_base_url.as_deref(),
            Some("https://api.birdcoder.com")
        );
        assert!(status.credential_configured);
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(
            content.contains("\"Bash(*)\""),
            "user permissions must be preserved"
        );

        dematerialize_mimo_code_model_configuration_at(&path).expect("dematerialize");
        let status = read_mimo_code_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::NotMaterialized,
            "dematerialize must remove the SDKWork marker"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
