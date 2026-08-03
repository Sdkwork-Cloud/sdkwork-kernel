//! OpenClaw provider config materialization.
//!
//! OpenClaw reads `~/.openclaw/openclaw.json` (`OPENCLAW_CONFIG_PATH` /
//! `OPENCLAW_STATE_DIR` overrides, with the legacy `.clawdbot` dir fallback).
//! Applied model configurations are materialized as a
//! `models.providers.sdkwork` entry with `baseUrl` + `apiKey` (the native
//! relay surface) and the selected model, exactly matching the upstream config
//! management surface. OpenClaw configs are natively JSON5 (comments and
//! trailing commas), so the existing file is parsed as JSON5 and written back
//! as strict JSON; the pre-mutation backup preserves the original formatting.

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest,
    AgentModelSelectionRequest, KernelError, KernelResult,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config, merge_json_path, provider_user_home,
    update_provider_config_file,
};
use serde_json::{json, Value};

/// Provider id materialized into `models.providers` for SDKWork-managed configs.
pub const SDKWORK_OPENCLAW_PROVIDER_ID: &str = "sdkwork";
const OPENCLAW_PROVIDER_NAME: &str = "SDKWork BirdCoder";

/// Mutates an OpenClaw config file (JSON5 in, strict JSON out) with
/// backup/verify/rollback semantics.
fn update_openclaw_config_file(
    path: &std::path::Path,
    transform: impl FnOnce(Option<&Value>) -> KernelResult<Value>,
) -> KernelResult<()> {
    update_provider_config_file(path, |current| {
        let current = match current {
            Some(content) => Some(json5::from_str(content).map_err(|error| {
                KernelError::provider_error(
                    "openclaw_config_parse",
                    format!("{} could not be parsed as JSON5: {error}", path.display()),
                )
            })?),
            None => None,
        };
        let next = transform(current.as_ref())?;
        serde_json::to_string_pretty(&next).map_err(|error| {
            KernelError::provider_error(
                "openclaw_config_serialize",
                format!("{} could not be serialized: {error}", path.display()),
            )
        })
    })
}

/// Resolves the OpenClaw config file path (mirrors the upstream candidates:
/// `OPENCLAW_CONFIG_PATH` override, then `OPENCLAW_STATE_DIR` (or the legacy
/// `.clawdbot` dir), then `~/.openclaw`).
pub fn openclaw_config_path() -> Option<std::path::PathBuf> {
    if let Some(config) = std::env::var_os("OPENCLAW_CONFIG_PATH") {
        let config = std::path::PathBuf::from(config);
        if config.is_file() {
            return Some(config);
        }
    }
    if let Some(state_dir) = std::env::var_os("OPENCLAW_STATE_DIR") {
        let state_dir = std::path::PathBuf::from(state_dir);
        if state_dir.is_dir() {
            return Some(state_dir.join("openclaw.json"));
        }
    }
    let home = provider_user_home()?;
    let state_dir = home.join(".openclaw");
    if state_dir.is_dir() {
        return Some(state_dir.join("openclaw.json"));
    }
    let legacy_dir = home.join(".clawdbot");
    if legacy_dir.is_dir() {
        return Some(legacy_dir.join("clawdbot.json"));
    }
    Some(home.join(".openclaw").join("openclaw.json"))
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

fn build_materialized_config(
    current: Option<&Value>,
    request: &AgentModelConfigurationRequest,
    api_key: Option<&str>,
) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let model_id = request.default_model_id.trim();
    let mut provider = json!({
        "name": OPENCLAW_PROVIDER_NAME,
        "baseUrl": request.base_url.trim(),
        "models": {
            model_id: { "name": model_id }
        },
    });
    if let Some(api_key) = api_key {
        provider["apiKey"] = Value::String(api_key.to_string());
    }
    merge_json_path(
        &mut document,
        &["models", "providers", SDKWORK_OPENCLAW_PROVIDER_ID],
        provider,
    );
    Ok(document)
}

fn update_selected_model(current: Option<&Value>, model_id: &str) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let provider = document
        .get_mut("models")
        .and_then(Value::as_object_mut)
        .and_then(|models| models.get_mut("providers"))
        .and_then(Value::as_object_mut)
        .and_then(|providers| providers.get_mut(SDKWORK_OPENCLAW_PROVIDER_ID))
        .and_then(Value::as_object_mut);
    if let Some(provider) = provider {
        provider.insert(
            "models".to_string(),
            json!({ model_id: { "name": model_id } }),
        );
    }
    Ok(document)
}

/// Materializes an OpenClaw model configuration into the OpenClaw config file.
pub fn materialize_openclaw_model_configuration(
    request: &AgentModelConfigurationRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = openclaw_config_path() else {
        return Ok(());
    };
    materialize_openclaw_model_configuration_at(&path, request, application)
}

/// Materializes an OpenClaw model configuration into an explicit config file.
pub(crate) fn materialize_openclaw_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_openclaw_config_file(path, |current| {
        build_materialized_config(current, request, api_key.as_deref())
    })
}

/// Materializes an OpenClaw model selection (updates the provider models map).
pub fn materialize_openclaw_model_selection(
    request: &AgentModelSelectionRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = openclaw_config_path() else {
        return Ok(());
    };
    materialize_openclaw_model_selection_at(&path, request, application)
}

/// Materializes an OpenClaw model selection into an explicit config file.
pub(crate) fn materialize_openclaw_model_selection_at(
    path: &std::path::Path,
    request: &AgentModelSelectionRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    update_openclaw_config_file(path, |current| {
        update_selected_model(current, request.model_id.trim())
    })
}

/// Restores the pre-materialization OpenClaw config backup.
pub fn dematerialize_openclaw_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = openclaw_config_path() else {
        return Ok(());
    };
    dematerialize_provider_config(&path)
}

/// Restores the pre-materialization backup for an explicit config file.
pub(crate) fn dematerialize_openclaw_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_with(api_key: Option<&str>, base_url: &str, model: &str) -> AgentModelConfigurationRequest {
        let mut request = AgentModelConfigurationRequest::new(
            "request-1",
            "agent.code-engine.openclaw",
            "profile.test",
            "openai",
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
    fn build_config_sets_provider_entry() {
        let request = request_with(
            Some("token-abc"),
            "https://api.birdcoder.com/v1",
            "gpt-5.4-mini",
        );
        let document =
            build_materialized_config(None, &request, Some("token-abc")).expect("build");
        let provider = &document["models"]["providers"]["sdkwork"];
        assert_eq!(provider["baseUrl"].as_str(), Some("https://api.birdcoder.com/v1"));
        assert_eq!(provider["apiKey"].as_str(), Some("token-abc"));
        assert!(provider["models"]["gpt-5.4-mini"].is_object());
    }

    #[test]
    fn build_config_merges_existing_providers() {
        let existing = json!({
            "models": { "providers": { "deepseek": { "baseUrl": "https://x" } } },
            "agents": { "main": { "provider": "deepseek" } }
        });
        let request = request_with(None, "https://api.birdcoder.com/v1", "gpt-5");
        let document = build_materialized_config(Some(&existing), &request, None).expect("merge");
        assert!(document["models"]["providers"]["deepseek"].is_object());
        assert!(document["models"]["providers"]["sdkwork"].is_object());
        assert!(document["agents"]["main"].is_object());
    }

    #[test]
    fn update_selected_model_keeps_other_keys() {
        let existing = json!({
            "models": { "providers": { "sdkwork": { "baseUrl": "https://x", "apiKey": "k", "models": { "gpt-5": {} } } } }
        });
        let document = update_selected_model(Some(&existing), "gpt-5.5").expect("update");
        let provider = &document["models"]["providers"]["sdkwork"];
        assert_eq!(provider["baseUrl"].as_str(), Some("https://x"));
        assert!(provider["models"]["gpt-5.5"].is_object());
    }

    #[test]
    fn materialization_merges_into_existing_json5_config() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-openclaw-json5-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("openclaw.json");
        // OpenClaw configs are natively JSON5: comments and trailing commas.
        std::fs::write(
            &path,
            "{\n  // user comment\n  \"agents\": { \"main\": { \"provider\": \"deepseek\" }, },\n}\n",
        )
        .expect("seed");
        let request = request_with(Some("token-xyz"), "https://api.birdcoder.com/v1", "gpt-5.4");
        materialize_openclaw_model_configuration_at(&path, &request, &AgentModelConfigurationApplication::new(
            "request-1",
            "openclaw",
            sdkwork_agent_kernel::AgentConfigurationProfile::new(
                "profile.test",
                "agent.code-engine.openclaw",
                "0.2.0",
                sdkwork_agent_kernel::AgentConfiguration::new(
                    "agent.code-engine.openclaw",
                    "profile.test",
                ),
            ),
        ))
        .expect("materialize");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("\"apiKey\": \"token-xyz\""));
        let document: Value = serde_json::from_str(&content).expect("strict json out");
        assert!(document["agents"]["main"].is_object(), "user config survives");
        dematerialize_openclaw_model_configuration_at(&path).expect("dematerialize");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read restored"),
            "{\n  // user comment\n  \"agents\": { \"main\": { \"provider\": \"deepseek\" }, },\n}\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialize_round_trip_writes_and_dematerializes() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-openclaw-materialize-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("openclaw.json");
        std::fs::write(&path, "{\"agents\":{\"main\":{\"provider\":\"deepseek\"}}}\n").expect("seed");
        let request = request_with(Some("token-xyz"), "https://api.birdcoder.com/v1", "gpt-5.4");
        materialize_openclaw_model_configuration_at(&path, &request, &AgentModelConfigurationApplication::new(
            "request-1",
            "openclaw",
            sdkwork_agent_kernel::AgentConfigurationProfile::new(
                "profile.test",
                "agent.code-engine.openclaw",
                "0.2.0",
                sdkwork_agent_kernel::AgentConfiguration::new(
                    "agent.code-engine.openclaw",
                    "profile.test",
                ),
            ),
        ))
        .expect("materialize");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("\"apiKey\": \"token-xyz\""));
        dematerialize_openclaw_model_configuration_at(&path).expect("dematerialize");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read restored"),
            "{\"agents\":{\"main\":{\"provider\":\"deepseek\"}}}\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
