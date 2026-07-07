# SDKWORK Secret Provider Specification

- **Version**: 0.1.0
- **Status**: Core Primitives Implemented
- **Date**: 2025-06-28
- **Scope**: Secure secret management and storage
- **Domain**: `security`
- **Capability**: `agent-kernel.secret-provider`
- **Implementation**: `sdkwork-agent-kernel/src/secret.rs`, `sdkwork-agent-kernel/src/secret_env.rs`
- **Test Coverage**: 16/16 tests passing (100%)

## 1. Overview

Secret Provider provides secure storage and management for sensitive data:

- **Encrypted Storage**: AES-256-GCM, ChaCha20-Poly1305, RSA-4096
- **Access Control**: Policy-based secret access
- **Lifecycle Management**: Create, rotate, delete secrets
- **Audit Trail**: All access recorded to TelemetryProvider
- **Expiration Support**: Time-based secret expiration

### Key Features

1. **SecretMetadata**: Secret metadata without value
2. **SecretValue**: Encrypted secret value
3. **SecretAccessRequest**: Access request with audit trail
4. **SecretProvider**: Provider trait for implementation

## 2. Architecture

### Component Structure

```text
SecretMetadata
  ├── secret_id: String
  ├── name: String
  ├── secret_type: SecretType
  ├── created_at: u64
  ├── last_accessed_at: Option<u64>
  ├── last_rotated_at: Option<u64>
  ├── expires_at: Option<u64>
  ├── access_count: u64
  └── tags: HashMap<String, String>

SecretValue
  ├── secret_id: String
  ├── encrypted_value: String
  ├── encryption_algorithm: EncryptionAlgorithm
  └── version: u32

SecretAccessRequest
  ├── secret_id: String
  ├── requester: String
  ├── purpose: SecretAccessPurpose
  ├── timestamp: u64
  └── context: HashMap<String, String>

SecretProvider Trait
  ├── create_secret(request) -> SecretMetadata
  ├── access_secret(request) -> SecretAccessResult
  ├── rotate_secret(request) -> SecretMetadata
  ├── delete_secret(secret_id)
  ├── list_secrets() -> Vec<SecretMetadata>
  ├── get_metadata(secret_id) -> SecretMetadata
  ├── health_check() -> SecretProviderHealth
  └── provider_manifest() -> SecretProviderManifest
```

## 3. Secret Types

### Supported Secret Types

| Type | Description | Use Case |
|------|-------------|----------|
| `ApiKey` | API key | External API authentication |
| `OAuthToken` | OAuth token | OAuth2 authentication |
| `Password` | Password | User authentication |
| `Certificate` | Certificate | TLS certificates |
| `EncryptionKey` | Encryption key | Data encryption |
| `Generic` | Generic secret | Any sensitive data |

### Example

```rust
let api_key_secret = SecretMetadata::new("secret-1", "OpenAI API Key", SecretType::ApiKey);
let oauth_secret = SecretMetadata::new("secret-2", "GitHub OAuth", SecretType::OAuthToken);
let cert_secret = SecretMetadata::new("secret-3", "TLS Certificate", SecretType::Certificate);
```

## 4. Encryption Algorithms

### Supported Algorithms

| Algorithm | Key Size | Use Case | Security Level |
|-----------|----------|----------|----------------|
| `Aes256Gcm` | 256-bit | General encryption | High |
| `ChaCha20Poly1305` | 256-bit | Mobile/embedded | High |
| `Rsa4096` | 4096-bit | Key exchange | Very High |
| `None` | - | Testing only | None |

### Recommendation

- **Production**: Use `Aes256Gcm` or `ChaCha20Poly1305`
- **Testing**: Use `None` for unit tests
- **Key Exchange**: Use `Rsa4096` for asymmetric encryption

## 5. Secret Metadata

### Definition

```rust
pub struct SecretMetadata {
    pub secret_id: String,
    pub name: String,
    pub description: String,
    pub secret_type: SecretType,
    pub created_at: u64,
    pub last_accessed_at: Option<u64>,
    pub last_rotated_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub access_count: u64,
    pub tags: HashMap<String, String>,
}
```

### Creation

```rust
let metadata = SecretMetadata::new("secret-1", "API Key", SecretType::ApiKey)
    .with_description("OpenAI API key for GPT-4")
    .with_expiration(1735689600000) // Expires Jan 1, 2025
    .with_tag("environment", "production")
    .with_tag("service", "openai");
```

### Expiration Check

```rust
if metadata.is_expired() {
    // Handle expired secret
}
```

## 6. Secret Access

### Access Request

```rust
let request = SecretAccessRequest::new("secret-1", "agent-1")
    .with_purpose(SecretAccessPurpose::Read)
    .with_context("session_id", "session-123")
    .with_context("operation", "generate_code");
```

### Access Purpose

| Purpose | Description |
|---------|-------------|
| `Read` | Read secret value |
| `Write` | Write new value |
| `Rotate` | Rotate secret |
| `Delete` | Delete secret |
| `Admin` | Administrative operation |

### Access Result

```rust
// Granted
let result = SecretAccessResult::granted("secret-value", "audit-record-1");

// Denied
let result = SecretAccessResult::denied("Access denied", "audit-record-1");
```

## 7. Secret Creation

### Creation Request

```rust
let request = SecretCreateRequest::new("API Key", SecretType::ApiKey, "sk-...")
    .with_description("OpenAI API key")
    .with_expiration(1735689600000)
    .with_tag("environment", "production");
```

### Provider Creation

```rust
let mut provider = InMemorySecretProvider::new();
let metadata = provider.create_secret(request)?;

println!("Created secret: {}", metadata.secret_id);
```

## 8. Secret Rotation

### Rotation Request

```rust
let request = SecretRotateRequest::new("secret-1", "new-api-key", "admin")
    .with_reason("Monthly rotation policy");
```

### Provider Rotation

```rust
let updated_metadata = provider.rotate_secret(request)?;
println!("Rotated at: {:?}", updated_metadata.last_rotated_at);
```

### Rotation Best Practices

- Rotate secrets regularly (monthly recommended)
- Use automated rotation for API keys
- Monitor rotation timestamps
- Audit rotation events

## 9. Secret Provider

### Provider Trait

```rust
pub trait SecretProvider: Send + Sync {
    fn create_secret(&mut self, request: SecretCreateRequest) -> Result<SecretMetadata, SecretError>;
    fn access_secret(&self, request: SecretAccessRequest) -> Result<SecretAccessResult, SecretError>;
    fn rotate_secret(&mut self, request: SecretRotateRequest) -> Result<SecretMetadata, SecretError>;
    fn delete_secret(&mut self, secret_id: &str) -> Result<(), SecretError>;
    fn list_secrets(&self) -> Result<Vec<SecretMetadata>, SecretError>;
    fn get_metadata(&self, secret_id: &str) -> Result<SecretMetadata, SecretError>;
    fn health_check(&self) -> Result<SecretProviderHealth, SecretError>;
    fn provider_manifest(&self) -> SecretProviderManifest;
}
```

### In-Memory Provider (Testing)

```rust
let provider = InMemorySecretProvider::new()
    .with_algorithm(EncryptionAlgorithm::None); // Testing mode
```

## 10. Health Monitoring

### Provider Health

```rust
pub struct SecretProviderHealth {
    pub status: SecretProviderStatus,
    pub secrets_count: usize,
    pub expired_count: usize,
    pub last_check_time: u64,
}
```

### Health Status

| Status | Description |
|--------|-------------|
| `Healthy` | All secrets valid |
| `Degraded` | Some secrets expired |
| `Unhealthy` | Critical issues |

### Health Check

```rust
let health = provider.health_check()?;
if health.status == SecretProviderStatus::Degraded {
    println!("{} secrets expired", health.expired_count);
}
```

## 11. Provider Manifest

### Manifest Definition

```rust
pub struct SecretProviderManifest {
    pub provider_id: String,
    pub name: String,
    pub version: String,
    pub supported_algorithms: Vec<EncryptionAlgorithm>,
    pub max_secrets: usize,
    pub supports_rotation: bool,
    pub supports_expiration: bool,
}
```

### Example Manifest

```rust
SecretProviderManifest {
    provider_id: "vault-secret-provider",
    name: "HashiCorp Vault Provider",
    version: "1.0.0",
    supported_algorithms: vec![EncryptionAlgorithm::Aes256Gcm],
    max_secrets: 10000,
    supports_rotation: true,
    supports_expiration: true,
}
```

## 12. Error Handling

### Secret Error

| Error | Description |
|-------|-------------|
| `NotFound(id)` | Secret not found |
| `AccessDenied(reason)` | Access denied by policy |
| `EncryptionFailed(msg)` | Encryption operation failed |
| `DecryptionFailed(msg)` | Decryption operation failed |
| `Expired(id)` | Secret has expired |
| `InvalidRequest(msg)` | Invalid request parameters |
| `StorageError(msg)` | Storage backend error |
| `ProviderUnavailable` | Provider unavailable |

### Error Handling

```rust
match provider.access_secret(request) {
    Ok(result) => {
        if result.granted {
            let value = result.value.unwrap();
        } else {
            println!("Denied: {}", result.denial_reason.unwrap());
        }
    }
    Err(SecretError::NotFound(id)) => {
        println!("Secret not found: {}", id);
    }
    Err(SecretError::Expired(id)) => {
        println!("Secret expired: {}", id);
    }
    Err(e) => {
        println!("Error: {}", e);
    }
}
```

## 13. Conformance Tests

### Test Coverage (16 tests)

| Test Name | Coverage |
|-----------|----------|
| `test_secret_metadata_new` | Metadata creation |
| `test_secret_metadata_expiration` | Expiration check |
| `test_secret_type_as_str` | Type strings |
| `test_encryption_algorithm_as_str` | Algorithm strings |
| `test_secret_access_request_new` | Access request |
| `test_secret_access_result_granted` | Granted result |
| `test_secret_access_result_denied` | Denied result |
| `test_secret_create_request_new` | Creation request |
| `test_secret_rotate_request_new` | Rotation request |
| `test_in_memory_secret_provider_create` | Provider creation |
| `test_in_memory_secret_provider_access` | Provider access |
| `test_in_memory_secret_provider_rotate` | Provider rotation |
| `test_in_memory_secret_provider_delete` | Provider deletion |
| `test_in_memory_secret_provider_list` | Provider listing |
| `test_in_memory_secret_provider_health` | Provider health |
| `test_secret_error_display` | Error formatting |

### Test Execution

```bash
cargo test --package sdkwork-agent-kernel --lib secret::tests
```

### Expected Result

```
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured
```

## 14. Integration Points

### HostProvider Integration

```rust
pub trait HostProvider {
    // Existing method (placeholder)
    fn resolve_secret(&self, secret_ref: &str) -> KernelResult<String>;

    // New integration with SecretProvider
    fn resolve_secret_with_provider(
        &self,
        secret_ref: &str,
        secret_provider: &SecretProvider,
    ) -> KernelResult<String> {
        let request = SecretAccessRequest::new(secret_ref, "host-provider");
        let result = secret_provider.access_secret(request)?;

        if result.granted {
            Ok(result.value.unwrap())
        } else {
            Err(KernelError::AccessDenied)
        }
    }
}
```

### PolicyProvider Integration

```rust
pub trait PolicyProvider {
    // Approve secret access
    fn approve_secret_access(
        &self,
        request: &SecretAccessRequest,
    ) -> KernelResult<bool>;
}
```

### TelemetryProvider Integration

```rust
// Record secret access
telemetry.counter("secret.access", 1, &[
    ("secret_id", request.secret_id),
    ("requester", request.requester),
    ("purpose", request.purpose.as_str()),
    ("granted", result.granted.to_string()),
]);

telemetry.histogram("secret.access_count", metadata.access_count);
```

## 15. Security Considerations

### Encryption Requirements

- **Production**: Must use AES-256-GCM or ChaCha20-Poly1305
- **Testing**: Can use None for unit tests
- **Key Storage**: Encryption keys must be stored separately
- **Key Rotation**: Rotate encryption keys annually

### Access Control

- **Principle of Least Privilege**: Grant minimum necessary access
- **Audit All Access**: Record all secret access attempts
- **Deny by Default**: Deny access unless explicitly approved
- **Context Validation**: Validate access context (session, operation)

### Lifecycle Management

- **Regular Rotation**: Rotate secrets monthly
- **Expiration Monitoring**: Monitor expired secrets
- **Secure Deletion**: Securely delete old secret values
- **Version Tracking**: Track secret version history

### Audit Trail

- **Access Audit**: Record all access attempts (granted/denied)
- **Rotation Audit**: Record all rotation events
- **Deletion Audit**: Record all deletion events
- **Integration**: Send audit events to TelemetryProvider

## 16. Performance Characteristics

### Operation Latency

| Operation | Latency | Notes |
|-----------|---------|-------|
| Create secret | ~10ms | Encryption overhead |
| Access secret | ~5ms | Decryption overhead |
| Rotate secret | ~10ms | Re-encryption |
| List secrets | ~1ms | Metadata only |
| Health check | ~1ms | Quick scan |

### Storage Capacity

- **In-Memory**: 1000 secrets max
- **Vault Backend**: 10,000+ secrets
- **Memory per secret**: ~1KB metadata + ~500 bytes value

### Recommendations

- Cache frequently accessed secrets (with TTL)
- Use batch operations for bulk access
- Monitor health status regularly
- Rotate secrets during low-traffic periods

## 17. Usage Patterns

### Pattern 1: API Key Storage

```rust
let mut provider = InMemorySecretProvider::new();

// Create API key secret
let request = SecretCreateRequest::new("OpenAI Key", SecretType::ApiKey, "sk-...")
    .with_description("OpenAI API key")
    .with_tag("service", "openai");

let metadata = provider.create_secret(request)?;

// Access for model invocation
let access_request = SecretAccessRequest::new(&metadata.secret_id, "model-provider");
let result = provider.access_secret(access_request)?;

if result.granted {
    let api_key = result.value.unwrap();
    // Use API key
}
```

### Pattern 2: OAuth Token Management

```rust
// Create OAuth token
let request = SecretCreateRequest::new("GitHub OAuth", SecretType::OAuthToken, "gho_...")
    .with_expiration(timestamp + 3600000); // 1 hour

let metadata = provider.create_secret(request)?;

// Rotate after expiration
if metadata.is_expired() {
    let rotate_request = SecretRotateRequest::new(&metadata.secret_id, "new-token", "oauth-service");
    provider.rotate_secret(rotate_request)?;
}
```

### Pattern 3: Certificate Storage

```rust
// Store TLS certificate
let request = SecretCreateRequest::new("TLS Cert", SecretType::Certificate, cert_pem)
    .with_tag("domain", "api.example.com");

provider.create_secret(request)?;
```

### Pattern 4: Health Monitoring

```rust
// Periodic health check
let health = provider.health_check()?;

if health.expired_count > 0 {
    // Alert: secrets expired
    telemetry.counter("secret.expired_alert", health.expired_count);
}
```

### Env/File Provider (Pre-Production)

Read-only backend for operators who inject secrets via environment variables or
mounted files before enterprise vault integration lands.

Resolution order for `secret_id`:

1. Exact environment variable name matching `secret_id`
2. `SDKWORK_SECRET_<NORMALIZED>` where normalized uppercases and replaces
   non-alphanumeric characters with `_`
3. File `{SDKWORK_SECRETS_DIR}/{secret_id}` when `SDKWORK_SECRETS_DIR` is set

```rust
use sdkwork_agent_kernel::{EnvFileSecretProvider, SecretAccessRequest, SecretProvider};

let provider = EnvFileSecretProvider::from_process_environment();
let result = provider.access_secret(SecretAccessRequest::new("secret.openai", "model-provider"))?;
```

Host integration wraps an existing `HostProvider` and resolves env/file secrets
before delegating:

```rust
use sdkwork_agent_kernel::EnvFileSecretFallbackHostProvider;
```

Mutating operations (`create_secret`, `rotate_secret`, `delete_secret`) fail
closed with `SecretError::InvalidRequest` on this backend.

### Chained Provider (Production Default)

`ChainedSecretProvider::from_process_environment()` resolves secrets in order:

1. `EnvFileSecretProvider`
2. `VaultSecretProvider` when compiled with `--features secret-vault` and
   `SDKWORK_VAULT_ADDR` + `SDKWORK_VAULT_TOKEN` are configured

```rust
use sdkwork_agent_kernel::ChainedSecretProvider;

let provider = ChainedSecretProvider::from_process_environment();
```

### Vault Provider (Feature `secret-vault`)

Enable with `cargo build -p sdkwork-agent-kernel --features secret-vault`.

| Environment variable | Purpose |
| --- | --- |
| `SDKWORK_VAULT_ADDR` | Vault base URL |
| `SDKWORK_VAULT_TOKEN` | Vault token |
| `SDKWORK_VAULT_MOUNT` | KV mount (default `secret`) |
| `SDKWORK_VAULT_NAMESPACE` | Optional enterprise namespace |

Secret paths map `provider.openai` → `provider/openai` under the configured mount.
KV v2 payloads must expose a `value` field (or a single string/object entry).

## 18. Future Extensions

### Planned Extensions (Enterprise Phase)

1. **AWS Secrets Manager / Azure Key Vault / GCP Secret Manager** adapters
2. **Vault write/rotate** operations with policy integration
2. **AWS Secrets Manager**: AWS integration
3. **Azure Key Vault**: Azure integration
4. **GCP Secret Manager**: GCP integration
5. **Secret Sharing**: Secure secret sharing between agents

### Extension Points

```rust
// Future: Vault backend
pub struct VaultSecretProvider {
    vault_client: VaultClient,
    mount_point: String,
}

impl SecretProvider for VaultSecretProvider {
    fn create_secret(&mut self, request: SecretCreateRequest) -> Result<SecretMetadata, SecretError> {
        // Vault API integration
    }
}

// Future: Secret sharing
pub trait SecretSharingProvider {
    fn share_secret(&self, secret_id: &str, recipient: &str) -> Result<SharedSecret, SecretError>;
    fn receive_shared_secret(&self, share_id: &str) -> Result<String, SecretError>;
}
```

## 19. References

- `sdkwork-agent-kernel/src/secret.rs` - Implementation
- `sdkwork-agent-kernel/src/host.rs` - HostProvider
- `sdkwork-agent-kernel/src/policy.rs` - PolicyProvider
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- HashiCorp Vault - Production secret management

## 20. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-06-28 | Core primitives, 16/16 tests passing |

---

**Status**: ✅ Core Primitives Implemented
**Next Steps**: Vault/AWS/Azure/GCP backend integration (Phase 7)