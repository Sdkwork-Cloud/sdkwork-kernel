//! HashiCorp Vault KV v2 read backend (feature `secret-vault`).

use crate::secret::{
    EncryptionAlgorithm, SecretAccessPurpose, SecretAccessRequest, SecretAccessResult,
    SecretCreateRequest, SecretError, SecretMetadata, SecretProvider, SecretProviderHealth,
    SecretProviderManifest, SecretProviderStatus, SecretRotateRequest, SecretType,
};
use sdkwork_utils_rust::string::{is_blank, trim};

pub const SDKWORK_VAULT_ADDR_ENV: &str = "SDKWORK_VAULT_ADDR";
pub const SDKWORK_VAULT_TOKEN_ENV: &str = "SDKWORK_VAULT_TOKEN";
pub const SDKWORK_VAULT_MOUNT_ENV: &str = "SDKWORK_VAULT_MOUNT";
pub const SDKWORK_VAULT_NAMESPACE_ENV: &str = "SDKWORK_VAULT_NAMESPACE";

/// Read-only Vault KV v2 secret provider configured from process environment.
#[derive(Debug, Clone)]
pub struct VaultSecretProvider {
    addr: String,
    token: String,
    mount: String,
    namespace: Option<String>,
}

impl VaultSecretProvider {
    pub fn from_process_environment() -> Option<Self> {
        let addr = std::env::var(SDKWORK_VAULT_ADDR_ENV)
            .ok()
            .map(|value| trim(&value));
        let token = std::env::var(SDKWORK_VAULT_TOKEN_ENV)
            .ok()
            .map(|value| trim(&value));
        let addr = addr.filter(|value| !is_blank(Some(value)))?;
        let token = token.filter(|value| !is_blank(Some(value)))?;
        let mount = std::env::var(SDKWORK_VAULT_MOUNT_ENV)
            .ok()
            .map(|value| trim(&value))
            .filter(|value| !is_blank(Some(value)))
            .unwrap_or_else(|| "secret".to_string());
        let namespace = std::env::var(SDKWORK_VAULT_NAMESPACE_ENV)
            .ok()
            .map(|value| trim(&value))
            .filter(|value| !is_blank(Some(value)));
        Some(Self {
            addr: addr.trim_end_matches('/').to_string(),
            token,
            mount,
            namespace,
        })
    }

    fn secret_path(&self, secret_id: &str) -> String {
        secret_id.replace('.', "/").replace('\\', "/")
    }

    fn read_plaintext(&self, secret_id: &str) -> Result<String, SecretError> {
        let path = self.secret_path(secret_id);
        let url = format!("{}/v1/{}/data/{}", self.addr, self.mount, path);
        let mut request = ureq::get(&url).set("X-Vault-Token", &self.token);
        if let Some(namespace) = &self.namespace {
            request = request.set("X-Vault-Namespace", namespace);
        }
        let response = request
            .call()
            .map_err(|error| SecretError::StorageError(format!("vault request failed: {error}")))?;
        if response.status() >= 400 {
            if response.status() == 404 {
                return Err(SecretError::NotFound(secret_id.to_string()));
            }
            return Err(SecretError::StorageError(format!(
                "vault returned HTTP {}",
                response.status()
            )));
        }
        let body: serde_json::Value = response.into_json().map_err(|error| {
            SecretError::StorageError(format!("vault response decode failed: {error}"))
        })?;
        extract_vault_secret_value(&body).ok_or_else(|| {
            SecretError::DecryptionFailed(format!(
                "vault secret payload missing value for {secret_id}"
            ))
        })
    }
}

fn extract_vault_secret_value(body: &serde_json::Value) -> Option<String> {
    let data = body.get("data")?.get("data")?;
    if let Some(value) = data.get("value").and_then(serde_json::Value::as_str) {
        return Some(value.to_string());
    }
    if let Some(value) = data.as_str() {
        return Some(value.to_string());
    }
    if data.is_object() && data.as_object().is_some_and(|map| map.len() == 1) {
        return data
            .as_object()
            .and_then(|map| map.values().next())
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
    }
    None
}

impl SecretProvider for VaultSecretProvider {
    fn create_secret(
        &mut self,
        _request: SecretCreateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        Err(SecretError::InvalidRequest(
            "vault secret backend is read-only in kernel runtime".to_string(),
        ))
    }

    fn access_secret(
        &self,
        request: SecretAccessRequest,
    ) -> Result<SecretAccessResult, SecretError> {
        if request.purpose != SecretAccessPurpose::Read {
            return Err(SecretError::InvalidRequest(
                "vault secret backend supports read access only".to_string(),
            ));
        }
        let value = self.read_plaintext(&request.secret_id)?;
        Ok(SecretAccessResult::granted(
            value,
            format!("vault-audit-{}", request.timestamp),
        ))
    }

    fn rotate_secret(
        &mut self,
        _request: SecretRotateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        Err(SecretError::InvalidRequest(
            "vault secret backend is read-only in kernel runtime".to_string(),
        ))
    }

    fn delete_secret(&mut self, _secret_id: &str) -> Result<(), SecretError> {
        Err(SecretError::InvalidRequest(
            "vault secret backend is read-only in kernel runtime".to_string(),
        ))
    }

    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError> {
        Err(SecretError::InvalidRequest(
            "vault list_secrets is not enabled; use vault-native inventory".to_string(),
        ))
    }

    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError> {
        self.read_plaintext(secret_id)?;
        Ok(
            SecretMetadata::new(secret_id, secret_id, SecretType::Generic)
                .with_description("Resolved from HashiCorp Vault KV v2")
                .with_tag("backend", "vault")
                .with_tag("mount", &self.mount),
        )
    }

    fn health_check(&self) -> Result<SecretProviderHealth, SecretError> {
        let url = format!("{}/v1/sys/health", self.addr);
        let mut request = ureq::get(&url).set("X-Vault-Token", &self.token);
        if let Some(namespace) = &self.namespace {
            request = request.set("X-Vault-Namespace", namespace);
        }
        let status = match request.call() {
            Ok(response) if response.status() < 500 => SecretProviderStatus::Healthy,
            Ok(_) => SecretProviderStatus::Degraded,
            Err(_) => return Err(SecretError::ProviderUnavailable),
        };
        Ok(SecretProviderHealth {
            status,
            secrets_count: 0,
            expired_count: 0,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> SecretProviderManifest {
        SecretProviderManifest {
            provider_id: "sdkwork.secret.vault".to_string(),
            name: "SDKWork Vault Secret Provider".to_string(),
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

    #[test]
    fn extract_vault_secret_value_reads_value_field() {
        let body = serde_json::json!({
            "data": {
                "data": { "value": "sk-test" }
            }
        });
        assert_eq!(
            extract_vault_secret_value(&body),
            Some("sk-test".to_string())
        );
    }

    #[test]
    fn secret_path_normalizes_dots() {
        let provider = VaultSecretProvider {
            addr: "https://vault.example".to_string(),
            token: "token".to_string(),
            mount: "secret".to_string(),
            namespace: None,
        };
        assert_eq!(provider.secret_path("provider.openai"), "provider/openai");
    }
}
