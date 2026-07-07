//! Chained secret resolution across multiple read-only backends.

use crate::secret::{
    SecretAccessRequest, SecretAccessResult, SecretCreateRequest, SecretError, SecretMetadata,
    SecretProvider, SecretProviderHealth, SecretProviderManifest, SecretRotateRequest,
};

/// Resolves secrets by trying providers in order until one grants access.
pub struct ChainedSecretProvider {
    providers: Vec<Box<dyn SecretProvider + Send + Sync>>,
}

impl ChainedSecretProvider {
    pub fn new(providers: Vec<Box<dyn SecretProvider + Send + Sync>>) -> Self {
        Self { providers }
    }

    pub fn from_process_environment() -> Self {
        let providers: Vec<Box<dyn SecretProvider + Send + Sync>> = vec![Box::new(
            crate::secret_env::EnvFileSecretProvider::from_process_environment(),
        )];

        #[cfg(feature = "secret-vault")]
        {
            let mut providers = providers;
            if let Some(vault) =
                crate::secret_vault::VaultSecretProvider::from_process_environment()
            {
                providers.push(Box::new(vault));
            }
            return Self { providers };
        }

        Self { providers }
    }
}

impl SecretProvider for ChainedSecretProvider {
    fn create_secret(
        &mut self,
        request: SecretCreateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        for provider in self.providers.iter_mut() {
            match provider.create_secret(request.clone()) {
                Ok(metadata) => return Ok(metadata),
                Err(SecretError::InvalidRequest(_)) => continue,
                Err(other) => return Err(other),
            }
        }
        Err(SecretError::InvalidRequest(
            "no chained secret backend supports create_secret".to_string(),
        ))
    }

    fn access_secret(
        &self,
        request: SecretAccessRequest,
    ) -> Result<SecretAccessResult, SecretError> {
        let mut last_error = SecretError::NotFound(request.secret_id.clone());
        for provider in &self.providers {
            match provider.access_secret(request.clone()) {
                Ok(result) if result.granted => return Ok(result),
                Ok(_) => continue,
                Err(SecretError::NotFound(_)) => {
                    last_error = SecretError::NotFound(request.secret_id.clone());
                }
                Err(other) => return Err(other),
            }
        }
        Err(last_error)
    }

    fn rotate_secret(
        &mut self,
        request: SecretRotateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        for provider in self.providers.iter_mut() {
            match provider.rotate_secret(request.clone()) {
                Ok(metadata) => return Ok(metadata),
                Err(SecretError::InvalidRequest(_)) => continue,
                Err(other) => return Err(other),
            }
        }
        Err(SecretError::InvalidRequest(
            "no chained secret backend supports rotate_secret".to_string(),
        ))
    }

    fn delete_secret(&mut self, secret_id: &str) -> Result<(), SecretError> {
        for provider in self.providers.iter_mut() {
            match provider.delete_secret(secret_id) {
                Ok(()) => return Ok(()),
                Err(SecretError::InvalidRequest(_)) => continue,
                Err(SecretError::NotFound(_)) => continue,
                Err(other) => return Err(other),
            }
        }
        Err(SecretError::InvalidRequest(
            "no chained secret backend supports delete_secret".to_string(),
        ))
    }

    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError> {
        let mut merged = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for provider in &self.providers {
            for metadata in provider.list_secrets()? {
                if seen.insert(metadata.secret_id.clone()) {
                    merged.push(metadata);
                }
            }
        }
        merged.sort_by(|left, right| left.secret_id.cmp(&right.secret_id));
        Ok(merged)
    }

    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError> {
        let mut last_error = SecretError::NotFound(secret_id.to_string());
        for provider in &self.providers {
            match provider.get_metadata(secret_id) {
                Ok(metadata) => return Ok(metadata),
                Err(SecretError::NotFound(_)) => {
                    last_error = SecretError::NotFound(secret_id.to_string());
                }
                Err(other) => return Err(other),
            }
        }
        Err(last_error)
    }

    fn health_check(&self) -> Result<SecretProviderHealth, SecretError> {
        let mut secrets_count = 0usize;
        let mut expired_count = 0usize;
        let mut healthy = 0usize;
        for provider in &self.providers {
            let health = provider.health_check()?;
            secrets_count += health.secrets_count;
            expired_count += health.expired_count;
            if health.status == crate::secret::SecretProviderStatus::Healthy {
                healthy += 1;
            }
        }
        Ok(SecretProviderHealth {
            status: if healthy > 0 || secrets_count == 0 {
                crate::secret::SecretProviderStatus::Healthy
            } else {
                crate::secret::SecretProviderStatus::Degraded
            },
            secrets_count,
            expired_count,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> SecretProviderManifest {
        SecretProviderManifest {
            provider_id: "sdkwork.secret.chained".to_string(),
            name: "SDKWork Chained Secret Provider".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![crate::secret::EncryptionAlgorithm::None],
            max_secrets: usize::MAX,
            supports_rotation: false,
            supports_expiration: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::{InMemorySecretProvider, SecretAccessPurpose, SecretType};

    #[test]
    fn chained_provider_falls_through_to_secondary_backend() {
        let mut secondary = InMemorySecretProvider::new();
        let metadata = secondary
            .create_secret(SecretCreateRequest::new(
                "secondary",
                SecretType::ApiKey,
                "vault-value",
            ))
            .expect("create");

        let chained = ChainedSecretProvider::new(vec![
            Box::new(crate::secret_env::EnvFileSecretProvider::default()),
            Box::new(secondary),
        ]);

        let result = chained
            .access_secret(SecretAccessRequest::new(&metadata.secret_id, "agent"))
            .expect("access");
        assert!(result.granted);
        assert_eq!(result.value, Some("vault-value".to_string()));
    }

    #[test]
    fn chained_provider_denies_non_read_on_read_only_backends() {
        let chained = ChainedSecretProvider::new(vec![Box::new(
            crate::secret_env::EnvFileSecretProvider::default(),
        )]);
        let request =
            SecretAccessRequest::new("missing", "agent").with_purpose(SecretAccessPurpose::Write);
        assert!(matches!(
            chained.access_secret(request),
            Err(SecretError::InvalidRequest(_))
        ));
    }
}
