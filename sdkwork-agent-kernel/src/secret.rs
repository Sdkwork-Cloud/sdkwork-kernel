//! Secret Provider for secure secret management.
//!
//! This module provides secure secret storage and access:
//! - Encrypted storage for sensitive data
//! - Policy-based access control
//! - Lifecycle management (create, rotate, revoke)
//! - Audit trail for all secret access

use std::collections::HashMap;

/// Secret metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMetadata {
    /// Secret identifier.
    pub secret_id: String,
    /// Secret name (human-readable).
    pub name: String,
    /// Secret description.
    pub description: String,
    /// Secret type.
    pub secret_type: SecretType,
    /// Creation time (ms since epoch).
    pub created_at: u64,
    /// Last access time (ms since epoch).
    pub last_accessed_at: Option<u64>,
    /// Last rotation time (ms since epoch).
    pub last_rotated_at: Option<u64>,
    /// Expiration time (ms since epoch).
    pub expires_at: Option<u64>,
    /// Access count.
    pub access_count: u64,
    /// Tags.
    pub tags: HashMap<String, String>,
}

impl SecretMetadata {
    pub fn new(
        secret_id: impl Into<String>,
        name: impl Into<String>,
        secret_type: SecretType,
    ) -> Self {
        Self {
            secret_id: secret_id.into(),
            name: name.into(),
            description: String::new(),
            secret_type,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            last_accessed_at: None,
            last_rotated_at: None,
            expires_at: None,
            access_count: 0,
            tags: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_expiration(mut self, expires_at: u64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            now >= expires_at
        } else {
            false
        }
    }
}

/// Secret type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretType {
    /// API key.
    ApiKey,
    /// OAuth token.
    OAuthToken,
    /// Password.
    Password,
    /// Certificate.
    Certificate,
    /// Encryption key.
    EncryptionKey,
    /// Generic secret.
    Generic,
}

impl SecretType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::OAuthToken => "oauth_token",
            Self::Password => "password",
            Self::Certificate => "certificate",
            Self::EncryptionKey => "encryption_key",
            Self::Generic => "generic",
        }
    }
}

/// Secret value (stored encrypted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretValue {
    /// Secret ID.
    pub secret_id: String,
    /// Encrypted value (base64 encoded).
    pub encrypted_value: String,
    /// Encryption algorithm.
    pub encryption_algorithm: EncryptionAlgorithm,
    /// Value version (for rotation tracking).
    pub version: u32,
}

impl SecretValue {
    pub fn new(
        secret_id: impl Into<String>,
        encrypted_value: impl Into<String>,
        encryption_algorithm: EncryptionAlgorithm,
    ) -> Self {
        Self {
            secret_id: secret_id.into(),
            encrypted_value: encrypted_value.into(),
            encryption_algorithm,
            version: 1,
        }
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
}

/// Encryption algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM.
    Aes256Gcm,
    /// ChaCha20-Poly1305.
    ChaCha20Poly1305,
    /// RSA-4096.
    Rsa4096,
    /// No encryption (for testing only).
    None,
}

impl EncryptionAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aes256Gcm => "aes256_gcm",
            Self::ChaCha20Poly1305 => "chacha20_poly1305",
            Self::Rsa4096 => "rsa4096",
            Self::None => "none",
        }
    }
}

/// Secret access request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretAccessRequest {
    /// Secret ID to access.
    pub secret_id: String,
    /// Requester identity (agent, tool, provider).
    pub requester: String,
    /// Access purpose.
    pub purpose: SecretAccessPurpose,
    /// Request timestamp (ms since epoch).
    pub timestamp: u64,
    /// Request context.
    pub context: HashMap<String, String>,
}

impl SecretAccessRequest {
    pub fn new(secret_id: impl Into<String>, requester: impl Into<String>) -> Self {
        Self {
            secret_id: secret_id.into(),
            requester: requester.into(),
            purpose: SecretAccessPurpose::Read,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            context: HashMap::new(),
        }
    }

    pub fn with_purpose(mut self, purpose: SecretAccessPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Secret access purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretAccessPurpose {
    /// Read secret value.
    Read,
    /// Write secret value.
    Write,
    /// Rotate secret value.
    Rotate,
    /// Delete secret.
    Delete,
    /// Admin operation.
    Admin,
}

impl SecretAccessPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Rotate => "rotate",
            Self::Delete => "delete",
            Self::Admin => "admin",
        }
    }
}

/// Secret access result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretAccessResult {
    /// Whether access was granted.
    pub granted: bool,
    /// Secret value (if granted and read purpose).
    pub value: Option<String>,
    /// Denial reason (if not granted).
    pub denial_reason: Option<String>,
    /// Audit record ID.
    pub audit_record_id: String,
}

impl SecretAccessResult {
    pub fn granted(value: impl Into<String>, audit_record_id: impl Into<String>) -> Self {
        Self {
            granted: true,
            value: Some(value.into()),
            denial_reason: None,
            audit_record_id: audit_record_id.into(),
        }
    }

    pub fn denied(reason: impl Into<String>, audit_record_id: impl Into<String>) -> Self {
        Self {
            granted: false,
            value: None,
            denial_reason: Some(reason.into()),
            audit_record_id: audit_record_id.into(),
        }
    }
}

/// Secret creation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCreateRequest {
    /// Secret name.
    pub name: String,
    /// Secret description.
    pub description: String,
    /// Secret type.
    pub secret_type: SecretType,
    /// Secret value (plaintext, will be encrypted).
    pub value: String,
    /// Expiration time (ms since epoch, optional).
    pub expires_at: Option<u64>,
    /// Tags.
    pub tags: HashMap<String, String>,
}

impl SecretCreateRequest {
    pub fn new(name: impl Into<String>, secret_type: SecretType, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            secret_type,
            value: value.into(),
            expires_at: None,
            tags: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_expiration(mut self, expires_at: u64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Secret rotation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRotateRequest {
    /// Secret ID to rotate.
    pub secret_id: String,
    /// New value (plaintext, will be encrypted).
    pub new_value: String,
    /// Rotation reason.
    pub reason: String,
    /// Requester.
    pub requester: String,
}

impl SecretRotateRequest {
    pub fn new(
        secret_id: impl Into<String>,
        new_value: impl Into<String>,
        requester: impl Into<String>,
    ) -> Self {
        Self {
            secret_id: secret_id.into(),
            new_value: new_value.into(),
            reason: String::new(),
            requester: requester.into(),
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

/// Secret Provider for secure secret management.
pub trait SecretProvider: Send + Sync {
    /// Create a new secret.
    fn create_secret(
        &mut self,
        request: SecretCreateRequest,
    ) -> Result<SecretMetadata, SecretError>;

    /// Access a secret (read value).
    fn access_secret(
        &self,
        request: SecretAccessRequest,
    ) -> Result<SecretAccessResult, SecretError>;

    /// Rotate a secret (update value).
    fn rotate_secret(
        &mut self,
        request: SecretRotateRequest,
    ) -> Result<SecretMetadata, SecretError>;

    /// Delete a secret.
    fn delete_secret(&mut self, secret_id: &str) -> Result<(), SecretError>;

    /// List secrets (metadata only, no values).
    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError>;

    /// Get secret metadata.
    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError>;

    /// Check secret health.
    fn health_check(&self) -> Result<SecretProviderHealth, SecretError>;

    /// Get provider manifest.
    fn provider_manifest(&self) -> SecretProviderManifest;
}

/// Secret Provider health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretProviderHealth {
    /// Health status.
    pub status: SecretProviderStatus,
    /// Number of secrets stored.
    pub secrets_count: usize,
    /// Number of expired secrets.
    pub expired_count: usize,
    /// Last health check time (ms since epoch).
    pub last_check_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretProviderStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl SecretProviderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

/// Secret Provider manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretProviderManifest {
    /// Provider ID.
    pub provider_id: String,
    /// Provider name.
    pub name: String,
    /// Provider version.
    pub version: String,
    /// Supported encryption algorithms.
    pub supported_algorithms: Vec<EncryptionAlgorithm>,
    /// Max secrets capacity.
    pub max_secrets: usize,
    /// Supports rotation.
    pub supports_rotation: bool,
    /// Supports expiration.
    pub supports_expiration: bool,
}

/// Secret error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// Secret not found.
    NotFound(String),
    /// Access denied.
    AccessDenied(String),
    /// Encryption failed.
    EncryptionFailed(String),
    /// Decryption failed.
    DecryptionFailed(String),
    /// Secret expired.
    Expired(String),
    /// Invalid request.
    InvalidRequest(String),
    /// Storage error.
    StorageError(String),
    /// Provider unavailable.
    ProviderUnavailable,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Secret not found: {}", id),
            Self::AccessDenied(reason) => write!(f, "Access denied: {}", reason),
            Self::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            Self::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            Self::Expired(id) => write!(f, "Secret expired: {}", id),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
            Self::ProviderUnavailable => write!(f, "Secret provider unavailable"),
        }
    }
}

impl std::error::Error for SecretError {}

/// In-memory secret provider (for testing).
#[derive(Debug, Clone)]
pub struct InMemorySecretProvider {
    secrets: HashMap<String, (SecretMetadata, String)>,
    encryption_algorithm: EncryptionAlgorithm,
}

impl InMemorySecretProvider {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            encryption_algorithm: EncryptionAlgorithm::None, // For testing
        }
    }

    pub fn with_algorithm(mut self, algorithm: EncryptionAlgorithm) -> Self {
        self.encryption_algorithm = algorithm;
        self
    }
}

impl Default for InMemorySecretProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretProvider for InMemorySecretProvider {
    fn create_secret(
        &mut self,
        request: SecretCreateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        let secret_id = format!("secret-{}", self.secrets.len() + 1);
        let metadata = SecretMetadata::new(&secret_id, &request.name, request.secret_type)
            .with_description(&request.description);

        // In testing, store plaintext
        self.secrets
            .insert(secret_id.clone(), (metadata.clone(), request.value));

        Ok(metadata)
    }

    fn access_secret(
        &self,
        request: SecretAccessRequest,
    ) -> Result<SecretAccessResult, SecretError> {
        let (metadata, value) = self
            .secrets
            .get(&request.secret_id)
            .ok_or_else(|| SecretError::NotFound(request.secret_id.clone()))?;

        if metadata.is_expired() {
            return Err(SecretError::Expired(request.secret_id));
        }

        let audit_record_id = format!("audit-{}", request.timestamp);
        Ok(SecretAccessResult::granted(value.clone(), audit_record_id))
    }

    fn rotate_secret(
        &mut self,
        request: SecretRotateRequest,
    ) -> Result<SecretMetadata, SecretError> {
        let (metadata, _) = self
            .secrets
            .get(&request.secret_id)
            .ok_or_else(|| SecretError::NotFound(request.secret_id.clone()))?;

        let mut updated_metadata = metadata.clone();
        updated_metadata.last_rotated_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );

        // Update value
        self.secrets.get_mut(&request.secret_id).unwrap().1 = request.new_value;
        self.secrets.get_mut(&request.secret_id).unwrap().0 = updated_metadata.clone();

        Ok(updated_metadata)
    }

    fn delete_secret(&mut self, secret_id: &str) -> Result<(), SecretError> {
        self.secrets
            .remove(secret_id)
            .map(|_| ())
            .ok_or_else(|| SecretError::NotFound(secret_id.to_string()))
    }

    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError> {
        Ok(self
            .secrets
            .values()
            .map(|(meta, _)| meta.clone())
            .collect())
    }

    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError> {
        self.secrets
            .get(secret_id)
            .map(|(meta, _)| meta.clone())
            .ok_or_else(|| SecretError::NotFound(secret_id.to_string()))
    }

    fn health_check(&self) -> Result<SecretProviderHealth, SecretError> {
        let expired_count = self
            .secrets
            .values()
            .filter(|(meta, _)| meta.is_expired())
            .count();

        Ok(SecretProviderHealth {
            status: if expired_count > 0 {
                SecretProviderStatus::Degraded
            } else {
                SecretProviderStatus::Healthy
            },
            secrets_count: self.secrets.len(),
            expired_count,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> SecretProviderManifest {
        SecretProviderManifest {
            provider_id: "in-memory-secret-provider".to_string(),
            name: "In-Memory Secret Provider".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![EncryptionAlgorithm::None],
            max_secrets: 1000,
            supports_rotation: true,
            supports_expiration: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_metadata_new() {
        let metadata = SecretMetadata::new("secret-1", "API Key", SecretType::ApiKey);
        assert_eq!(metadata.secret_id, "secret-1");
        assert_eq!(metadata.name, "API Key");
        assert_eq!(metadata.secret_type, SecretType::ApiKey);
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_secret_metadata_expiration() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let metadata = SecretMetadata::new("secret-1", "Test", SecretType::Generic)
            .with_expiration(now - 1000); // Expired

        assert!(metadata.is_expired());
    }

    #[test]
    fn test_secret_type_as_str() {
        assert_eq!(SecretType::ApiKey.as_str(), "api_key");
        assert_eq!(SecretType::OAuthToken.as_str(), "oauth_token");
    }

    #[test]
    fn test_encryption_algorithm_as_str() {
        assert_eq!(EncryptionAlgorithm::Aes256Gcm.as_str(), "aes256_gcm");
        assert_eq!(
            EncryptionAlgorithm::ChaCha20Poly1305.as_str(),
            "chacha20_poly1305"
        );
    }

    #[test]
    fn test_secret_access_request_new() {
        let request = SecretAccessRequest::new("secret-1", "agent-1");
        assert_eq!(request.secret_id, "secret-1");
        assert_eq!(request.requester, "agent-1");
        assert_eq!(request.purpose, SecretAccessPurpose::Read);
    }

    #[test]
    fn test_secret_access_result_granted() {
        let result = SecretAccessResult::granted("value", "audit-1");
        assert!(result.granted);
        assert_eq!(result.value, Some("value".to_string()));
        assert!(result.denial_reason.is_none());
    }

    #[test]
    fn test_secret_access_result_denied() {
        let result = SecretAccessResult::denied("Access denied", "audit-1");
        assert!(!result.granted);
        assert!(result.value.is_none());
        assert_eq!(result.denial_reason, Some("Access denied".to_string()));
    }

    #[test]
    fn test_secret_create_request_new() {
        let request = SecretCreateRequest::new("API Key", SecretType::ApiKey, "secret-value");
        assert_eq!(request.name, "API Key");
        assert_eq!(request.secret_type, SecretType::ApiKey);
        assert_eq!(request.value, "secret-value");
    }

    #[test]
    fn test_secret_rotate_request_new() {
        let request = SecretRotateRequest::new("secret-1", "new-value", "admin")
            .with_reason("Monthly rotation");

        assert_eq!(request.secret_id, "secret-1");
        assert_eq!(request.new_value, "new-value");
        assert_eq!(request.reason, "Monthly rotation");
    }

    #[test]
    fn test_in_memory_secret_provider_create() {
        let mut provider = InMemorySecretProvider::new();
        let request = SecretCreateRequest::new("Test Secret", SecretType::ApiKey, "test-value");

        let metadata = provider.create_secret(request).unwrap();
        assert_eq!(metadata.name, "Test Secret");
        assert_eq!(metadata.secret_type, SecretType::ApiKey);
    }

    #[test]
    fn test_in_memory_secret_provider_access() {
        let mut provider = InMemorySecretProvider::new();

        // Create secret
        let create_request = SecretCreateRequest::new("Test", SecretType::Generic, "value");
        let metadata = provider.create_secret(create_request).unwrap();

        // Access secret
        let access_request = SecretAccessRequest::new(&metadata.secret_id, "agent-1");
        let result = provider.access_secret(access_request).unwrap();

        assert!(result.granted);
        assert_eq!(result.value, Some("value".to_string()));
    }

    #[test]
    fn test_in_memory_secret_provider_rotate() {
        let mut provider = InMemorySecretProvider::new();

        // Create secret
        let create_request = SecretCreateRequest::new("Test", SecretType::ApiKey, "old-value");
        let metadata = provider.create_secret(create_request).unwrap();

        // Rotate secret
        let rotate_request = SecretRotateRequest::new(&metadata.secret_id, "new-value", "admin");
        let updated_metadata = provider.rotate_secret(rotate_request).unwrap();

        assert!(updated_metadata.last_rotated_at.is_some());

        // Verify new value
        let access_request = SecretAccessRequest::new(&metadata.secret_id, "agent-1");
        let result = provider.access_secret(access_request).unwrap();
        assert_eq!(result.value, Some("new-value".to_string()));
    }

    #[test]
    fn test_in_memory_secret_provider_delete() {
        let mut provider = InMemorySecretProvider::new();

        // Create secret
        let create_request = SecretCreateRequest::new("Test", SecretType::ApiKey, "value");
        let metadata = provider.create_secret(create_request).unwrap();

        // Delete secret
        provider.delete_secret(&metadata.secret_id).unwrap();

        // Verify deleted
        assert!(provider.get_metadata(&metadata.secret_id).is_err());
    }

    #[test]
    fn test_in_memory_secret_provider_list() {
        let mut provider = InMemorySecretProvider::new();

        // Create 2 secrets
        provider
            .create_secret(SecretCreateRequest::new("S1", SecretType::ApiKey, "v1"))
            .unwrap();
        provider
            .create_secret(SecretCreateRequest::new("S2", SecretType::Password, "v2"))
            .unwrap();

        // List secrets
        let secrets = provider.list_secrets().unwrap();
        assert_eq!(secrets.len(), 2);
    }

    #[test]
    fn test_in_memory_secret_provider_health() {
        let mut provider = InMemorySecretProvider::new();

        // Create secret
        provider
            .create_secret(SecretCreateRequest::new(
                "Test",
                SecretType::ApiKey,
                "value",
            ))
            .unwrap();

        // Health check
        let health = provider.health_check().unwrap();
        assert_eq!(health.status, SecretProviderStatus::Healthy);
        assert_eq!(health.secrets_count, 1);
        assert_eq!(health.expired_count, 0);
    }

    #[test]
    fn test_secret_error_display() {
        assert_eq!(
            SecretError::NotFound("secret-1".to_string()).to_string(),
            "Secret not found: secret-1"
        );
        assert_eq!(
            SecretError::AccessDenied("No permission".to_string()).to_string(),
            "Access denied: No permission"
        );
    }
}
