//! opencode provider config materialization.
//!
//! opencode reads `opencode.json` (`OPENCODE_CONFIG` override, else
//! `~/.config/opencode/opencode.json`, `~/.config/opencode/opencode.jsonc`,
//! or `~/.opencode/opencode.json`). Applied model configurations are
//! materialized as a provider entry with `options.baseURL` + `options.apiKey`
//! (the native relay surface) and the selected `model` (`sdkwork/<id>`),
//! exactly matching the upstream config management surface.

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest, AgentModelSelectionRequest,
    KernelError, KernelResult, ProviderModelConfigurationStatus, ProviderModelMaterializationState,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config_named, merge_json_path, provider_user_home, read_provider_config,
    update_provider_json_config_named,
};
use serde_json::{json, Value};

/// Provider id materialized into `provider` for SDKWork-managed configs.
pub const SDKWORK_OPENCODE_PROVIDER_ID: &str = "sdkwork";
const OPENCODE_PROVIDER_NPM: &str = "@ai-sdk/openai-compatible";
const OPENCODE_PROVIDER_NAME: &str = "SDKWork BirdCoder";

/// Resolves the opencode config file path (mirrors the upstream candidates).
/// When no config file exists yet, the canonical XDG default path is returned
/// so materialization still writes a fresh config instead of silently
/// skipping a fresh installation.
pub fn opencode_config_path() -> Option<std::path::PathBuf> {
    if let Some(config) = std::env::var_os("OPENCODE_CONFIG") {
        let config = std::path::PathBuf::from(config);
        if config.is_file() {
            return Some(config);
        }
    }
    let home = provider_user_home()?;
    #[cfg(windows)]
    let candidates = [
        // xdg-basedir maps XDG_CONFIG_HOME to %APPDATA% on Windows.
        home.join("AppData")
            .join("Roaming")
            .join("opencode")
            .join("opencode.json"),
        home.join("AppData")
            .join("Roaming")
            .join("opencode")
            .join("opencode.jsonc"),
        home.join(".config").join("opencode").join("opencode.json"),
        home.join(".config").join("opencode").join("opencode.jsonc"),
        home.join(".opencode").join("opencode.json"),
    ];
    #[cfg(not(windows))]
    let candidates = [
        home.join(".config").join("opencode").join("opencode.json"),
        home.join(".config").join("opencode").join("opencode.jsonc"),
        home.join(".opencode").join("opencode.json"),
    ];
    let default_path = candidates[0].clone();
    Some(
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .unwrap_or(default_path),
    )
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
        "npm": OPENCODE_PROVIDER_NPM,
        "name": OPENCODE_PROVIDER_NAME,
        "options": {
            "baseURL": request.base_url.trim(),
        },
        "models": {
            model_id: {}
        },
    });
    if let Some(api_key) = api_key {
        provider["options"]["apiKey"] = Value::String(api_key.to_string());
    }
    merge_json_path(
        &mut document,
        &["provider", SDKWORK_OPENCODE_PROVIDER_ID],
        provider,
    )?;
    merge_json_path(
        &mut document,
        &["model"],
        Value::String(format!("{SDKWORK_OPENCODE_PROVIDER_ID}/{model_id}")),
    )?;
    Ok(document)
}

fn update_selected_model(current: Option<&Value>, model_id: &str) -> KernelResult<Value> {
    let mut document = current.cloned().unwrap_or_else(|| json!({}));
    let provider = document
        .get_mut("provider")
        .and_then(Value::as_object_mut)
        .and_then(|providers| providers.get_mut(SDKWORK_OPENCODE_PROVIDER_ID))
        .and_then(Value::as_object_mut);
    match provider {
        Some(provider) => {
            provider.insert("models".to_string(), json!({ model_id: {} }));
        }
        // Fail closed: applying a selection with no SDKWork-managed provider
        // entry would silently leave the CLI on its previous model.
        None => {
            return Err(KernelError::provider_error(
                "opencode_model_selection",
                format!(
                    "no SDKWork-managed provider entry (provider.{SDKWORK_OPENCODE_PROVIDER_ID}) exists; materialize a model configuration before selecting"
                ),
            ));
        }
    }
    merge_json_path(
        &mut document,
        &["model"],
        Value::String(format!("{SDKWORK_OPENCODE_PROVIDER_ID}/{model_id}")),
    )?;
    Ok(document)
}

/// Materializes an opencode model configuration into the opencode config file.
pub fn materialize_opencode_model_configuration(
    request: &AgentModelConfigurationRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = opencode_config_path() else {
        return Err(KernelError::provider_error(
            "provider_config_path",
            "could not resolve the opencode config path: user home is unavailable",
        ));
    };
    materialize_opencode_model_configuration_at(&path, request, application)
}

/// Materializes an opencode model configuration into an explicit config file.
pub(crate) fn materialize_opencode_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_provider_json_config_named(path, "opencode", |current| {
        build_materialized_config(current, request, api_key.as_deref())
    })
}

/// Materializes an opencode model selection (updates the `model` key).
pub fn materialize_opencode_model_selection(
    request: &AgentModelSelectionRequest,
    application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = opencode_config_path() else {
        return Err(KernelError::provider_error(
            "provider_config_path",
            "could not resolve the opencode config path: user home is unavailable",
        ));
    };
    materialize_opencode_model_selection_at(&path, request, application)
}

/// Materializes an opencode model selection into an explicit config file.
pub(crate) fn materialize_opencode_model_selection_at(
    path: &std::path::Path,
    request: &AgentModelSelectionRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    update_provider_json_config_named(path, "opencode", |current| {
        update_selected_model(current, request.model_id.trim())
    })
}

/// Restores the pre-materialization opencode config backup.
pub fn dematerialize_opencode_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = opencode_config_path() else {
        return Err(KernelError::provider_error(
            "provider_config_path",
            "could not resolve the opencode config path: user home is unavailable",
        ));
    };
    dematerialize_opencode_model_configuration_at(&path)
}

/// Restores the pre-materialization backup for an explicit config file.
pub(crate) fn dematerialize_opencode_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config_named(path, "opencode")
}

/// Reads the currently effective opencode model configuration back from the
/// opencode config file and reports the materialization state.
pub fn read_opencode_model_configuration() -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(path) = opencode_config_path() else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "opencode",
        ));
    };
    read_opencode_model_configuration_at(&path)
}

/// Reads the effective opencode model configuration back from an explicit
/// config file (used by tests and by config surfaces with a known path).
pub(crate) fn read_opencode_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(content) = read_provider_config(path)? else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "opencode",
        ));
    };
    let document = match serde_json::from_str::<Value>(&content) {
        Ok(document) => document,
        Err(error) => {
            return Ok(ProviderModelConfigurationStatus {
                provider_scope: "opencode".to_string(),
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
    let sdkwork_entry = document
        .get("provider")
        .and_then(|providers| providers.get(SDKWORK_OPENCODE_PROVIDER_ID));
    let effective_base_url = sdkwork_entry
        .and_then(|entry| entry.get("options"))
        .and_then(|options| options.get("baseURL"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let credential_configured = sdkwork_entry
        .and_then(|entry| entry.get("options"))
        .and_then(|options| options.get("apiKey"))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty());
    // The materialized model is `sdkwork/<id>`; strip the provider prefix so
    // the effective model id is comparable with the applied profile.
    let effective_default_model = document
        .get("model")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|model| {
            model
                .strip_prefix(&format!("{SDKWORK_OPENCODE_PROVIDER_ID}/"))
                .unwrap_or(model)
                .to_string()
        });
    let materialized = effective_base_url.is_some();
    let mut status = ProviderModelConfigurationStatus {
        provider_scope: "opencode".to_string(),
        materialization: if materialized {
            ProviderModelMaterializationState::Materialized
        } else {
            ProviderModelMaterializationState::NotMaterialized
        },
        effective_base_url,
        effective_default_model,
        credential_configured,
        issues: Vec::new(),
    };
    if !materialized {
        status.issues.push(format!(
            "provider does not carry a {} entry",
            SDKWORK_OPENCODE_PROVIDER_ID
        ));
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_with(
        api_key: Option<&str>,
        base_url: &str,
        model: &str,
    ) -> AgentModelConfigurationRequest {
        let mut request = AgentModelConfigurationRequest::new(
            "request-1",
            "agent.agent-engine.opencode",
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
    fn build_config_sets_provider_entry_and_model() {
        let request = request_with(
            Some("token-abc"),
            "https://api.birdcoder.com/v1",
            "gpt-5.4-mini",
        );
        let document = build_materialized_config(None, &request, Some("token-abc")).expect("build");
        assert_eq!(document["model"].as_str(), Some("sdkwork/gpt-5.4-mini"));
        let provider = &document["provider"]["sdkwork"];
        assert_eq!(provider["npm"].as_str(), Some("@ai-sdk/openai-compatible"));
        assert_eq!(
            provider["options"]["baseURL"].as_str(),
            Some("https://api.birdcoder.com/v1")
        );
        assert_eq!(provider["options"]["apiKey"].as_str(), Some("token-abc"));
        assert!(provider["models"]["gpt-5.4-mini"].is_object());
    }

    #[test]
    fn build_config_merges_existing_providers() {
        let existing = json!({
            "provider": { "deepseek": { "npm": "@ai-sdk/deepseek", "options": { "baseURL": "https://x" } } },
            "model": "deepseek/deepseek-chat"
        });
        let request = request_with(None, "https://api.birdcoder.com/v1", "gpt-5");
        let document = build_materialized_config(Some(&existing), &request, None).expect("merge");
        assert!(document["provider"]["deepseek"].is_object());
        assert!(document["provider"]["sdkwork"].is_object());
        assert_eq!(document["model"].as_str(), Some("sdkwork/gpt-5"));
    }

    #[test]
    fn update_selected_model_changes_model_key() {
        let existing = json!({ "model": "sdkwork/gpt-5", "provider": { "sdkwork": { "models": { "gpt-5": {} } } } });
        let document = update_selected_model(Some(&existing), "gpt-5.5").expect("update");
        assert_eq!(document["model"].as_str(), Some("sdkwork/gpt-5.5"));
        assert!(document["provider"]["sdkwork"]["models"]["gpt-5.5"].is_object());
    }

    #[test]
    fn materialize_round_trip_writes_and_dematerializes() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-opencode-materialize-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("opencode.json");
        std::fs::write(&path, "{\"model\": \"openai/gpt-5\"}\n").expect("seed");
        let request = request_with(Some("token-xyz"), "https://api.birdcoder.com/v1", "gpt-5.4");
        materialize_opencode_model_configuration_at(
            &path,
            &request,
            &AgentModelConfigurationApplication::new(
                "request-1",
                "opencode",
                sdkwork_agent_kernel::AgentConfigurationProfile::new(
                    "profile.test",
                    "agent.agent-engine.opencode",
                    "0.2.0",
                    sdkwork_agent_kernel::AgentConfiguration::new(
                        "agent.agent-engine.opencode",
                        "profile.test",
                    ),
                ),
            ),
        )
        .expect("materialize");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("\"apiKey\": \"token-xyz\""));
        assert!(content.contains("\"model\": \"sdkwork/gpt-5.4\""));
        dematerialize_opencode_model_configuration_at(&path).expect("dematerialize");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read restored"),
            "{\"model\": \"openai/gpt-5\"}\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_materialized_provider_entry() {
        let dir =
            std::env::temp_dir().join(format!("sdkwork-opencode-readback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("opencode.json");
        std::fs::write(
            &path,
            json!({
                "provider": {
                    "sdkwork": {
                        "npm": "@ai-sdk/openai-compatible",
                        "options": {
                            "baseURL": "https://api.birdcoder.com",
                            "apiKey": "token-abc"
                        },
                        "models": { "gpt-5.4": {} }
                    }
                },
                "model": "sdkwork/gpt-5.4"
            })
            .to_string(),
        )
        .expect("seed");

        let status = read_opencode_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Materialized
        );
        assert_eq!(
            status.effective_base_url.as_deref(),
            Some("https://api.birdcoder.com")
        );
        // The `sdkwork/` prefix is stripped so the id compares with the profile.
        assert_eq!(status.effective_default_model.as_deref(), Some("gpt-5.4"));
        assert!(status.credential_configured);
        assert!(status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_not_materialized_without_provider_entry() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-opencode-readback-native-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("opencode.json");
        std::fs::write(&path, json!({ "model": "openai/gpt-5" }).to_string()).expect("seed");

        let status = read_opencode_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::NotMaterialized
        );
        assert!(!status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
