//! Gemini CLI provider config materialization.
//!
//! The Gemini CLI loads `~/.gemini/.env` into its process environment before
//! reading settings. Applied model configurations are materialized there
//! (`GEMINI_API_KEY` + `GOOGLE_GEMINI_BASE_URL`, which switches the CLI into
//! gateway/relay mode) so the CLI actually routes through the configured relay
//! endpoint with the configured credential. The model id is passed per turn,
//! so model selections do not touch the file.

use sdkwork_agent_kernel::{
    AgentModelConfigurationApplication, AgentModelConfigurationRequest, KernelError, KernelResult,
    ProviderModelConfigurationStatus, ProviderModelMaterializationState,
};
use sdkwork_agent_provider_core::{
    dematerialize_provider_config_named, provider_user_home, read_provider_config,
    update_provider_config_file_named,
};

const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";
const GEMINI_BASE_URL_ENV: &str = "GOOGLE_GEMINI_BASE_URL";
/// Marker written next to the materialized values so read-back can tell
/// "SDKWork materialized this env file" apart from a user-configured relay.
/// The CLI ignores unknown environment names, so the marker is inert at
/// request time.
const SDKWORK_MANAGED_MARKER_ENV: &str = "SDKWORK_MANAGED";

/// Resolves `~/.gemini/.env` (no override when the home is unknown).
pub fn gemini_env_path() -> Option<std::path::PathBuf> {
    provider_user_home().map(|home| home.join(".gemini").join(".env"))
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

fn escape_env_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

/// Auth-mode selectors that would override the gateway (relay) mode forced by
/// `GOOGLE_GEMINI_BASE_URL`; they are dropped so the applied relay config wins.
const OVERRIDDEN_AUTH_MODE_KEYS: &[&str] = &[
    "GOOGLE_GENAI_USE_VERTEXAI",
    "GOOGLE_GENAI_USE_GCA",
    "GEMINI_CLI_USE_COMPUTE_ADC",
];

fn merge_env_lines(current: Option<&str>, entries: &[(String, String)]) -> KernelResult<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(content) = current {
        for raw_line in content.lines() {
            let line = raw_line.trim_end();
            let Some((key, _)) = line.split_once('=') else {
                lines.push(line.to_string());
                continue;
            };
            let key = key.trim();
            if OVERRIDDEN_AUTH_MODE_KEYS.contains(&key) {
                continue;
            }
            if entries.iter().any(|(entry_key, _)| entry_key == key) {
                if seen.insert(key.to_string()) {
                    let value = entries
                        .iter()
                        .find(|(entry_key, _)| entry_key == key)
                        .map(|(_, value)| value.as_str())
                        .unwrap_or("");
                    lines.push(format!("{key}={}", escape_env_value(value)));
                }
            } else {
                lines.push(line.to_string());
            }
        }
    }
    for (key, value) in entries {
        if seen.insert(key.clone()) {
            lines.push(format!("{key}={}", escape_env_value(value)));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

/// Materializes a Gemini CLI model configuration into `~/.gemini/.env`.
pub fn materialize_gemini_cli_model_configuration(
    request: &AgentModelConfigurationRequest,
    _application: &AgentModelConfigurationApplication,
) -> KernelResult<()> {
    let Some(path) = gemini_env_path() else {
        return Err(KernelError::provider_error(
            "provider_config_path",
            "could not resolve the ~/.gemini/.env path: user home is unavailable",
        ));
    };
    materialize_gemini_cli_model_configuration_at(&path, request)
}

/// Materializes a Gemini CLI model configuration into an explicit `.env` file.
pub(crate) fn materialize_gemini_cli_model_configuration_at(
    path: &std::path::Path,
    request: &AgentModelConfigurationRequest,
) -> KernelResult<()> {
    let api_key = resolve_materialization_api_key(request);
    update_provider_config_file_named(path, "gemini-cli", |current| {
        let mut entries = vec![
            (
                GEMINI_BASE_URL_ENV.to_string(),
                request.base_url.trim().to_string(),
            ),
            (SDKWORK_MANAGED_MARKER_ENV.to_string(), "true".to_string()),
        ];
        if let Some(api_key) = api_key {
            entries.push((GEMINI_API_KEY_ENV.to_string(), api_key));
        }
        merge_env_lines(current, &entries)
    })
}

/// Restores the pre-materialization `~/.gemini/.env` backup.
pub fn dematerialize_gemini_cli_model_configuration(
    _agent_id: &str,
    _profile_id: &str,
) -> KernelResult<()> {
    let Some(path) = gemini_env_path() else {
        return Err(KernelError::provider_error(
            "provider_config_path",
            "could not resolve the ~/.gemini/.env path: user home is unavailable",
        ));
    };
    dematerialize_provider_config_named(&path, "gemini-cli")
}

/// Restores the pre-materialization backup for an explicit `.env` file.
pub(crate) fn dematerialize_gemini_cli_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<()> {
    dematerialize_provider_config_named(path, "gemini-cli")
}

/// Parses `KEY=VALUE` pairs from an env file, dropping comments and blank
/// lines, and unescaping values escaped by [`escape_env_value`] (symmetric
/// with the write path). Returns an empty vec when the file is absent.
fn parse_env_lines(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), unescape_env_value(value.trim())))
        })
        .collect()
}

/// Reverses [`escape_env_value`]: `\\` becomes `\` and `\n` becomes a newline.
fn unescape_env_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some(next) => {
                    result.push('\\');
                    result.push(next);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Reads the currently effective Gemini CLI model configuration back from
/// `~/.gemini/.env` and reports the materialization state. The model id is
/// passed per turn, so no model entry is expected in the env file.
pub fn read_gemini_cli_model_configuration() -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(path) = gemini_env_path() else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "gemini-cli",
        ));
    };
    read_gemini_cli_model_configuration_at(&path)
}

/// Reads the effective Gemini CLI model configuration back from an explicit
/// `.env` file (used by tests and by config surfaces with a known path).
pub(crate) fn read_gemini_cli_model_configuration_at(
    path: &std::path::Path,
) -> KernelResult<ProviderModelConfigurationStatus> {
    let Some(content) = read_provider_config(path)? else {
        return Ok(ProviderModelConfigurationStatus::not_materialized(
            "gemini-cli",
        ));
    };
    let entries = parse_env_lines(&content);
    let value_of = |key: &str| {
        entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value.clone())
            .filter(|value| !value.is_empty())
    };
    let sdkwork_managed = value_of(SDKWORK_MANAGED_MARKER_ENV).is_some();
    let effective_base_url = value_of(GEMINI_BASE_URL_ENV);
    let credential_configured = value_of(GEMINI_API_KEY_ENV).is_some();
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
                "SDKWork-managed marker exists but {GEMINI_BASE_URL_ENV} is missing"
            )],
        )
    } else {
        (ProviderModelMaterializationState::Materialized, Vec::new())
    };
    Ok(ProviderModelConfigurationStatus {
        provider_scope: "gemini-cli".to_string(),
        materialization,
        effective_base_url,
        effective_default_model: None,
        credential_configured,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_with(api_key: Option<&str>, base_url: &str) -> AgentModelConfigurationRequest {
        let mut request = AgentModelConfigurationRequest::new(
            "request-1",
            "agent.code-engine.gemini",
            "profile.test",
            "google",
            base_url,
            "secret.ref",
            "gemini-2.5-pro",
        );
        if let Some(api_key) = api_key {
            request = request.with_api_key_materialization(api_key);
        }
        request
    }

    #[test]
    fn merge_env_replaces_matching_keys_and_keeps_others() {
        let current = "GOOGLE_GENAI_USE_VERTEXAI=1\nGEMINI_API_KEY=old\nCUSTOM=keep\n";
        let merged = merge_env_lines(
            Some(current),
            &[
                (
                    GEMINI_BASE_URL_ENV.to_string(),
                    "https://api.birdcoder.com".to_string(),
                ),
                (GEMINI_API_KEY_ENV.to_string(), "token-new".to_string()),
            ],
        )
        .expect("merge");
        assert!(merged.contains("GEMINI_API_KEY=token-new"));
        assert!(merged.contains("GOOGLE_GEMINI_BASE_URL=https://api.birdcoder.com"));
        assert!(merged.contains("CUSTOM=keep"));
        assert!(!merged.contains("GEMINI_API_KEY=old"));
        assert!(
            !merged.contains("GOOGLE_GENAI_USE_VERTEXAI=1"),
            "gateway mode must override vertex"
        );
    }

    #[test]
    fn merge_env_appends_new_entries() {
        let merged =
            merge_env_lines(None, &[("KEY_A".to_string(), "1".to_string())]).expect("merge");
        assert!(merged.contains("KEY_A=1"));
    }

    #[test]
    fn materialize_round_trip_writes_and_dematerializes() {
        let dir =
            std::env::temp_dir().join(format!("sdkwork-gemini-materialize-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(&path, "GEMINI_API_KEY=original\n").expect("seed");
        let request = request_with(Some("token-xyz"), "https://api.birdcoder.com");
        materialize_gemini_cli_model_configuration_at(&path, &request).expect("materialize");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("GEMINI_API_KEY=token-xyz"));
        assert!(content.contains("GOOGLE_GEMINI_BASE_URL=https://api.birdcoder.com"));
        dematerialize_gemini_cli_model_configuration_at(&path).expect("dematerialize");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read restored"),
            "GEMINI_API_KEY=original\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_materialized_env_and_credential() {
        let dir =
            std::env::temp_dir().join(format!("sdkwork-gemini-readback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(
            &path,
            "GEMINI_API_KEY=token-abc\nGOOGLE_GEMINI_BASE_URL=https://api.birdcoder.com\nSDKWORK_MANAGED=true\nCUSTOM=keep\n",
        )
        .expect("seed");

        let status = read_gemini_cli_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Materialized
        );
        assert_eq!(
            status.effective_base_url.as_deref(),
            Some("https://api.birdcoder.com")
        );
        assert!(status.credential_configured);
        // The model id is passed per turn, so no model is expected in the file.
        assert_eq!(status.effective_default_model, None);
        assert!(status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_reports_not_materialized_without_relay_base_url() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-gemini-readback-native-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(&path, "GEMINI_API_KEY=original\n").expect("seed");

        let status = read_gemini_cli_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::NotMaterialized
        );
        assert!(status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_back_never_reports_user_relay_without_marker_as_materialized() {
        let dir = std::env::temp_dir().join(format!(
            "sdkwork-gemini-readback-user-relay-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(
            &path,
            "GOOGLE_GEMINI_BASE_URL=https://user-relay.example.com\n",
        )
        .expect("seed");

        let status = read_gemini_cli_model_configuration_at(&path).expect("read back");
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
            "sdkwork-gemini-readback-diverged-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(&path, "SDKWORK_MANAGED=true\n").expect("seed");

        let status = read_gemini_cli_model_configuration_at(&path).expect("read back");
        assert_eq!(
            status.materialization,
            sdkwork_agent_kernel::ProviderModelMaterializationState::Diverged,
            "a marker without its values is drift"
        );
        assert!(!status.issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
