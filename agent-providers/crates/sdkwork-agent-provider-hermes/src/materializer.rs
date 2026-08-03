//! Hermes Agent provider config materialization.
//!
//! Hermes reads `~/.hermes/config.yaml` (`HERMES_HOME` override) with API keys
//! resolved from `~/.hermes/.env` and per-provider `key_env` entries. Applied
//! model configurations are materialized as a `providers.sdkwork` entry (the
//! native relay surface: `api`, `transport`, `key_env`) plus the key in
//! `~/.hermes/.env`, exactly matching the upstream config management surface.

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest, AgentModelSelectionRequest,
    KernelError, KernelResult,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config, provider_user_home, update_provider_config_file,
};

const HERMES_PROVIDER_ID: &str = "sdkwork";
const HERMES_PROVIDER_NAME: &str = "SDKWork BirdCoder";
const HERMES_API_KEY_ENV: &str = "SDKWORK_BIRDOODER_RELAY_API_KEY";

/// Resolves `~/.hermes/config.yaml` (mirrors the upstream candidates).
pub fn hermes_config_path() -> Option<std::path::PathBuf> {
    if let Some(home) = std::env::var_os("HERMES_HOME") {
        let home = std::path::PathBuf::from(home);
        if home.is_dir() {
            return Some(home.join("config.yaml"));
        }
    }
    let home = provider_user_home()?;
    #[cfg(windows)]
    let candidates = [
        home.join("AppData")
            .join("Local")
            .join("hermes")
            .join("config.yaml"),
        home.join(".hermes").join("config.yaml"),
    ];
    #[cfg(not(windows))]
    let candidates = [home.join(".hermes").join("config.yaml")];
    // When no config file exists yet, the canonical default path is returned
    // so materialization still writes a fresh config instead of silently
    // skipping a fresh installation.
    let default_path = candidates[0].clone();
    Some(
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .unwrap_or(default_path),
    )
}

/// Resolves `~/.hermes/.env` (the env file Hermes loads for API keys).
fn hermes_env_path() -> Option<std::path::PathBuf> {
    provider_user_home().map(|home| home.join(".hermes").join(".env"))
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
    current: Option<&str>,
    request: &AgentModelConfigurationRequest,
    api_key: Option<&str>,
) -> KernelResult<String> {
    let mut document: serde_yaml::Mapping = match current {
        Some(content) => serde_yaml::from_str(content).map_err(|error| {
            KernelError::provider_error(
                "hermes_config_parse",
                format!("~/.hermes/config.yaml could not be parsed: {error}"),
            )
        })?,
        None => serde_yaml::Mapping::new(),
    };
    let model_id = request.default_model_id.trim();
    // Hermes canonicalizes the chat model into `model.default` (the runtime
    // resolver and the kernel model catalog both read it); preserve any
    // existing model dict keys (provider, base_url, ...).
    let mut model = match document.get("model") {
        Some(serde_yaml::Value::Mapping(existing)) => existing.clone(),
        Some(serde_yaml::Value::String(existing)) if !existing.is_empty() => {
            serde_yaml::Mapping::from_iter([(
                serde_yaml::Value::String("default".to_string()),
                serde_yaml::Value::String(existing.clone()),
            )])
        }
        _ => serde_yaml::Mapping::new(),
    };
    model.insert(
        serde_yaml::Value::String("default".to_string()),
        serde_yaml::Value::String(model_id.to_string()),
    );
    document.insert(
        serde_yaml::Value::String("model".to_string()),
        serde_yaml::Value::Mapping(model),
    );
    let mut providers = match document.get("providers") {
        Some(serde_yaml::Value::Mapping(existing)) => existing.clone(),
        _ => serde_yaml::Mapping::new(),
    };
    let mut provider = serde_yaml::Mapping::new();
    provider.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(HERMES_PROVIDER_NAME.to_string()),
    );
    provider.insert(
        serde_yaml::Value::String("api".to_string()),
        serde_yaml::Value::String(request.base_url.trim().to_string()),
    );
    provider.insert(
        serde_yaml::Value::String("transport".to_string()),
        serde_yaml::Value::String("openai_chat".to_string()),
    );
    if api_key.is_some() {
        // The raw key goes into ~/.hermes/.env; config.yaml references it.
        provider.insert(
            serde_yaml::Value::String("key_env".to_string()),
            serde_yaml::Value::String(HERMES_API_KEY_ENV.to_string()),
        );
    }
    providers.insert(
        serde_yaml::Value::String(HERMES_PROVIDER_ID.to_string()),
        serde_yaml::Value::Mapping(provider),
    );
    document.insert(
        serde_yaml::Value::String("providers".to_string()),
        serde_yaml::Value::Mapping(providers),
    );
    serde_yaml::to_string(&serde_yaml::Value::Mapping(document)).map_err(|error| {
        KernelError::provider_error(
            "hermes_config_serialize",
            format!("~/.hermes/config.yaml could not be serialized: {error}"),
        )
    })
}

fn update_selected_model(current: Option<&str>, model_id: &str) -> KernelResult<String> {
    let mut document: serde_yaml::Mapping = match current {
        Some(content) => serde_yaml::from_str(content).map_err(|error| {
            KernelError::provider_error(
                "hermes_config_parse",
                format!("~/.hermes/config.yaml could not be parsed: {error}"),
            )
        })?,
        None => serde_yaml::Mapping::new(),
    };
    let mut model = match document.get("model") {
        Some(serde_yaml::Value::Mapping(existing)) => existing.clone(),
        _ => serde_yaml::Mapping::new(),
    };
    model.insert(
        serde_yaml::Value::String("default".to_string()),
        serde_yaml::Value::String(model_id.to_string()),
    );
    document.insert(
        serde_yaml::Value::String("model".to_string()),
        serde_yaml::Value::Mapping(model),
    );
    serde_yaml::to_string(&serde_yaml::Value::Mapping(document)).map_err(|error| {
        KernelError::provider_error(
            "hermes_config_serialize",
            format!("~/.hermes/config.yaml could not be serialized: {error}"),
        )
    })
}

fn merge_env_key(current: Option<&str>, api_key: &str) -> KernelResult<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut replaced = false;
    if let Some(content) = current {
        for line in content.lines() {
            if let Some((key, _)) = line.split_once('=') {
                if key.trim() == HERMES_API_KEY_ENV {
                    lines.push(format!("{HERMES_API_KEY_ENV}={api_key}"));
                    replaced = true;
                    continue;
                }
            }
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("{HERMES_API_KEY_ENV}={api_key}"));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

/// Materializes a Hermes model configuration into `~/.hermes/config.yaml`.
pub fn materialize_hermes_model_configuration(
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = hermes_config_path() else {
        return Ok(());
    };
    materialize_hermes_model_configuration_at(&path, request)
}

/// Materializes a Hermes model configuration into explicit files.
pub(crate) fn materialize_hermes_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_provider_config_file(path, |current| {
        build_materialized_config(current, request, api_key.as_deref())
    })?;
    if let (Some(api_key), Some(env_path)) = (api_key, hermes_env_path()) {
        if let Err(error) =
            update_provider_config_file(&env_path, |current| merge_env_key(current, &api_key))
        {
            // The config.yaml was already materialized; restore its backup so
            // a partial two-file materialization never persists.
            let _ = dematerialize_provider_config(path);
            return Err(error);
        }
    }
    Ok(())
}

/// Materializes a Hermes model selection (updates the `model` key).
pub fn materialize_hermes_model_selection(
    request: &AgentModelSelectionRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = hermes_config_path() else {
        return Ok(());
    };
    update_provider_config_file(&path, |current| {
        update_selected_model(current, request.model_id.trim())
    })
}

/// Restores the pre-materialization Hermes config backup (config + env file).
pub fn dematerialize_hermes_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = hermes_config_path() else {
        return Ok(());
    };
    dematerialize_provider_config(&path)?;
    if let Some(env_path) = hermes_env_path() {
        dematerialize_provider_config(&env_path)?;
    }
    Ok(())
}

/// Restores the pre-materialization backup for an explicit config file.
pub(crate) fn dematerialize_hermes_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config(path)
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
            "agent.code-engine.hermes",
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
        let content = build_materialized_config(None, &request, Some("token-abc")).expect("build");
        let document: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse");
        assert_eq!(document["model"]["default"].as_str(), Some("gpt-5.4-mini"));
        assert_eq!(
            document["providers"]["sdkwork"]["api"].as_str(),
            Some("https://api.birdcoder.com/v1")
        );
        assert_eq!(
            document["providers"]["sdkwork"]["transport"].as_str(),
            Some("openai_chat")
        );
        assert_eq!(
            document["providers"]["sdkwork"]["key_env"].as_str(),
            Some("SDKWORK_BIRDOODER_RELAY_API_KEY")
        );
    }

    #[test]
    fn build_config_merges_existing_providers() {
        let existing = "model: old\nproviders:\n  deepseek:\n    api: https://x\n";
        let request = request_with(None, "https://api.birdcoder.com/v1", "gpt-5");
        let content = build_materialized_config(Some(existing), &request, None).expect("merge");
        let document: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse");
        assert!(document["providers"]["deepseek"].is_mapping());
        assert!(document["providers"]["sdkwork"].is_mapping());
        assert_eq!(document["model"]["default"].as_str(), Some("gpt-5"));
    }

    #[test]
    fn merge_env_key_replaces_and_appends() {
        let merged = merge_env_key(
            Some("OTHER=1\nSDKWORK_BIRDOODER_RELAY_API_KEY=old\n"),
            "new",
        )
        .expect("merge");
        assert!(merged.contains("SDKWORK_BIRDOODER_RELAY_API_KEY=new"));
        assert!(merged.contains("OTHER=1"));
        assert!(!merged.contains("=old"));
        let appended = merge_env_key(None, "new").expect("append");
        assert!(appended.contains("SDKWORK_BIRDOODER_RELAY_API_KEY=new"));
    }

    #[test]
    fn update_selected_model_keeps_providers() {
        let existing = "model: a\nproviders:\n  sdkwork:\n    api: https://x\n";
        let content = update_selected_model(Some(existing), "gpt-5.5").expect("update");
        let document: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse");
        assert_eq!(document["model"]["default"].as_str(), Some("gpt-5.5"));
        assert!(document["providers"]["sdkwork"].is_mapping());
    }
}
