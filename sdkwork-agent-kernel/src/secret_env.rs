//! Environment and file-backed secret resolution for pre-production deployments.
//!
//! Production enterprise vault backends (HashiCorp Vault, cloud SM) remain behind
//! future feature flags. Until then, operators mount secrets via process environment
//! or a read-only directory referenced by `SDKWORK_SECRETS_DIR`.

use crate::secret::{
    EncryptionAlgorithm, SecretAccessPurpose, SecretAccessRequest, SecretAccessResult,
    SecretCreateRequest, SecretError, SecretMetadata, SecretProvider, SecretProviderHealth,
    SecretProviderManifest, SecretProviderStatus, SecretRotateRequest, SecretType,
};
use sdkwork_utils_rust::string::{is_blank, trim};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Default prefix for discoverable secret environment variables.
pub const SDKWORK_SECRET_ENV_PREFIX: &str = "SDKWORK_SECRET_";

/// Environment variable pointing at a directory of `{secret_id}` files.
pub const SDKWORK_SECRETS_DIR_ENV: &str = "SDKWORK_SECRETS_DIR";

/// Read-only secret provider backed by process environment and optional file mounts.
#[derive(Debug, Clone)]
pub struct EnvFileSecretProvider {
    env_prefix: String,
    secrets_dir: Option<PathBuf>,
}

impl EnvFileSecretProvider {
    pub fn from_process_environment() -> Self {
        Self {
            env_prefix: SDKWORK_SECRET_ENV_PREFIX.to_string(),
            secrets_dir: std::env::var(SDKWORK_SECRETS_DIR_ENV)
                .ok()
                .map(|value| PathBuf::from(trim(&value)))
                .filter(|path| !path.as_os_str().is_empty()),
        }
    }

    pub fn with_secrets_dir(mut self, secrets_dir: impl Into<PathBuf>) -> Self {
        self.secrets_dir = Some(secrets_dir.into());
        self
    }

    pub fn with_env_prefix(mut self, env_prefix: impl Into<String>) -> Self {
        self.env_prefix = env_prefix.into();
        self
    }
}

impl Default for EnvFileSecretProvider {
    fn default() -> Self {
        Self::from_process_environment()
    }
}

/// Resolve a plaintext secret value for the given secret identifier.
pub fn lookup_env_file_secret(secret_id: &str) -> Option<String> {
    EnvFileSecretProvider::from_process_environment().lookup_plaintext(secret_id)
}

/// Normalize a secret identifier into an uppercase env-var suffix (`secret.openai` → `SECRET_OPENAI`).
pub fn secret_id_to_env_suffix(secret_id: &str) -> String {
    secret_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn prefixed_env_var(env_prefix: &str, secret_id: &str) -> String {
    format!("{env_prefix}{}", secret_id_to_env_suffix(secret_id))
}

fn secret_file_path(secrets_dir: &Path, secret_id: &str) -> Option<PathBuf> {
    if secret_id.is_empty() || secret_id.contains("..") {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in Path::new(secret_id).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            _ => return None,
        }
    }
    if relative.as_os_str().is_empty() {
        return None;
    }
    Some(secrets_dir.join(relative))
}

impl EnvFileSecretProvider {
    fn lookup_plaintext(&self, secret_id: &str) -> Option<String> {
        if is_blank(Some(secret_id)) {
            return None;
        }

        if let Ok(value) = std::env::var(secret_id) {
            if !is_blank(Some(&value)) {
                return Some(trim(&value));
            }
        }

        let prefixed = prefixed_env_var(&self.env_prefix, secret_id);
        if let Ok(value) = std::env::var(&prefixed) {
            if !is_blank(Some(&value)) {
                return Some(trim(&value));
            }
        }

        let secrets_dir = self.secrets_dir.as_ref()?;
        let file_path = secret_file_path(secrets_dir, secret_id)?;
        let contents = std::fs::read_to_string(&file_path).ok()?;
        if is_blank(Some(&contents)) {
            return None;
        }
        Some(trim(&contents))
    }

    fn metadata_for(&self, secret_id: &str, source: &str) -> SecretMetadata {
        SecretMetadata::new(secret_id, secret_id, SecretType::Generic)
            .with_description(format!("Resolved from {source}"))
            .with_tag("backend", "env-file")
            .with_tag("source", source)
    }

    fn discover_secret_ids(&self) -> Vec<String> {
        let mut ids = HashMap::<String, ()>::new();

        for (key, value) in std::env::vars() {
            if is_blank(Some(&value)) {
                continue;
            }
            if let Some(suffix) = key.strip_prefix(&self.env_prefix) {
                if !suffix.is_empty() {
                    ids.insert(suffix.to_ascii_lowercase().replace('_', "."), ());
                }
            }
        }

        if let Some(secrets_dir) = &self.secrets_dir {
            if let Ok(entries) = std::fs::read_dir(secrets_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        if !file_name.is_empty() && !file_name.contains("..") {
                            ids.insert(file_name.to_string(), ());
                        }
                    }
                }
            }
        }

        let mut secret_ids: Vec<_> = ids.into_keys().collect();
        secret_ids.sort();
        secret_ids
    }
}

impl SecretProvider for EnvFileSecretProvider {
    fn create_secret(
        &mut self,
        _request: SecretCreateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        Err(SecretError::InvalidRequest(
            "env/file secret backend is read-only; use environment variables or SDKWORK_SECRETS_DIR files"
                .to_string(),
        ))
    }

    fn access_secret(
        &self,
        request: SecretAccessRequest,
    ) -> Result<SecretAccessResult, SecretError> {
        if request.purpose != SecretAccessPurpose::Read {
            return Err(SecretError::InvalidRequest(
                "env/file secret backend supports read access only".to_string(),
            ));
        }

        let value = self
            .lookup_plaintext(&request.secret_id)
            .ok_or_else(|| SecretError::NotFound(request.secret_id.clone()))?;

        Ok(SecretAccessResult::granted(
            value,
            format!("env-file-audit-{}", request.timestamp),
        ))
    }

    fn rotate_secret(
        &mut self,
        _request: SecretRotateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        Err(SecretError::InvalidRequest(
            "env/file secret backend is read-only".to_string(),
        ))
    }

    fn delete_secret(&mut self, _secret_id: &str) -> Result<(), SecretError> {
        Err(SecretError::InvalidRequest(
            "env/file secret backend is read-only".to_string(),
        ))
    }

    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError> {
        Ok(self
            .discover_secret_ids()
            .into_iter()
            .map(|secret_id| self.metadata_for(&secret_id, "discovery"))
            .collect())
    }

    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError> {
        if self.lookup_plaintext(secret_id).is_some() {
            Ok(self.metadata_for(secret_id, "env-or-file"))
        } else {
            Err(SecretError::NotFound(secret_id.to_string()))
        }
    }

    fn health_check(&self) -> Result<SecretProviderHealth, SecretError> {
        let secrets_count = self.discover_secret_ids().len();
        Ok(SecretProviderHealth {
            status: if secrets_count > 0 {
                SecretProviderStatus::Healthy
            } else {
                SecretProviderStatus::Degraded
            },
            secrets_count,
            expired_count: 0,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> SecretProviderManifest {
        SecretProviderManifest {
            provider_id: "sdkwork.secret.env-file".to_string(),
            name: "SDKWork Env/File Secret Provider".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![EncryptionAlgorithm::None],
            max_secrets: usize::MAX,
            supports_rotation: false,
            supports_expiration: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn env_test_guard() -> MutexGuard<'static, ()> {
        ENV_TEST_LOCK.lock().expect("env test lock")
    }

    #[test]
    fn secret_id_to_env_suffix_normalizes_delimiters() {
        assert_eq!(secret_id_to_env_suffix("secret.openai"), "SECRET_OPENAI");
        assert_eq!(
            secret_id_to_env_suffix("provider/api-key"),
            "PROVIDER_API_KEY"
        );
    }

    #[test]
    fn lookup_reads_exact_env_var() {
        let _guard = env_test_guard();
        let key = "SDKWORK_TEST_SECRET_EXACT_001";
        std::env::set_var(key, "exact-value");
        assert_eq!(lookup_env_file_secret(key), Some("exact-value".to_string()));
        std::env::remove_var(key);
    }

    #[test]
    fn lookup_reads_prefixed_env_var() {
        let _guard = env_test_guard();
        let key = "SDKWORK_SECRET_PROVIDER_DEMO_KEY";
        std::env::set_var(key, "prefixed-value");
        assert_eq!(
            lookup_env_file_secret("provider.demo.key"),
            Some("prefixed-value".to_string())
        );
        std::env::remove_var(key);
    }

    #[test]
    fn provider_access_denies_non_read_purpose() {
        let provider = EnvFileSecretProvider::default();
        let request =
            SecretAccessRequest::new("missing", "agent").with_purpose(SecretAccessPurpose::Write);
        assert!(matches!(
            provider.access_secret(request),
            Err(SecretError::InvalidRequest(_))
        ));
    }

    #[test]
    fn provider_rejects_mutating_operations() {
        let mut provider = EnvFileSecretProvider::default();
        assert!(provider
            .create_secret(SecretCreateRequest::new("x", SecretType::ApiKey, "v"))
            .is_err());
        assert!(provider
            .rotate_secret(SecretRotateRequest::new("x", "v", "agent"))
            .is_err());
        assert!(provider.delete_secret("x").is_err());
    }
}
