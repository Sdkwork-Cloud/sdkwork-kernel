//! Host secret resolution via environment variables and file mounts.

use crate::host::{
    FilesystemRequest, FilesystemResult, HostProvider, NetworkRequest, NetworkResult,
    ProcessRequest, ProcessResult, SecretRef, SecretValue,
};
use crate::secret_env::lookup_env_file_secret;
use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};
use std::sync::Arc;

/// Wraps a host provider and resolves secrets from env/file before delegating.
#[derive(Clone)]
pub struct EnvFileSecretFallbackHostProvider {
    inner: Arc<dyn HostProvider + Send + Sync>,
}

impl EnvFileSecretFallbackHostProvider {
    pub fn new(inner: Arc<dyn HostProvider + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl HostProvider for EnvFileSecretFallbackHostProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        self.inner.provider_manifest()
    }

    fn health(&self) -> ProviderHealth {
        self.inner.health()
    }

    fn filesystem(&self, request: FilesystemRequest) -> KernelResult<FilesystemResult> {
        self.inner.filesystem(request)
    }

    fn process(&self, request: ProcessRequest) -> KernelResult<ProcessResult> {
        self.inner.process(request)
    }

    fn network(&self, request: NetworkRequest) -> KernelResult<NetworkResult> {
        self.inner.network(request)
    }

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue> {
        let secret_ref_id = secret_ref.secret_ref_id.clone();
        if let Some(value) = lookup_env_file_secret(&secret_ref_id) {
            return Ok(SecretValue::new(secret_ref_id, value));
        }
        self.inner.resolve_secret(secret_ref)
    }

    fn storage(
        &self,
        request: crate::host::StorageRequest,
    ) -> KernelResult<crate::host::StorageResult> {
        self.inner.storage(request)
    }

    fn time(&self, request: crate::host::TimeRequest) -> KernelResult<crate::host::TimeResult> {
        self.inner.time(request)
    }

    fn environment(
        &self,
        request: crate::host::EnvironmentRequest,
    ) -> KernelResult<crate::host::EnvironmentResult> {
        self.inner.environment(request)
    }

    fn executor(
        &self,
        request: crate::host::ExecutorRequest,
    ) -> KernelResult<crate::host::ExecutorResult> {
        self.inner.executor(request)
    }
}

/// Standalone host provider that only resolves secrets from env/file mounts.
#[derive(Debug, Clone, Default)]
pub struct EnvFileSecretHostProvider;

impl EnvFileSecretHostProvider {
    pub fn new() -> Self {
        Self
    }
}

impl HostProvider for EnvFileSecretHostProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.host.secret-env-file",
            "host",
            "Env/File Secret Host",
            "1.0.0",
            vec!["host.secrets".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn filesystem(&self, _request: FilesystemRequest) -> KernelResult<FilesystemResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.filesystem".to_string(),
        })
    }

    fn process(&self, _request: ProcessRequest) -> KernelResult<ProcessResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.process".to_string(),
        })
    }

    fn network(&self, _request: NetworkRequest) -> KernelResult<NetworkResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.network".to_string(),
        })
    }

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue> {
        let secret_ref_id = secret_ref.secret_ref_id.clone();
        match lookup_env_file_secret(&secret_ref_id) {
            Some(value) => Ok(SecretValue::new(secret_ref_id, value)),
            None => Err(KernelError::validation(format!(
                "secret not found in environment or SDKWORK_SECRETS_DIR: {secret_ref_id}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::ProcessResult;
    use std::sync::{Mutex, MutexGuard};

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn env_test_guard() -> MutexGuard<'static, ()> {
        ENV_TEST_LOCK.lock().expect("env test lock")
    }

    struct StubHost;

    impl HostProvider for StubHost {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.host.stub",
                "host",
                "stub",
                "0.1.0",
                vec!["host.secrets".to_string()],
            )
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }

        fn filesystem(&self, _request: FilesystemRequest) -> KernelResult<FilesystemResult> {
            Err(KernelError::CapabilityMissing {
                capability_id: "host.filesystem".to_string(),
            })
        }

        fn process(&self, request: ProcessRequest) -> KernelResult<ProcessResult> {
            Ok(ProcessResult::exited(request.operation_id, 0, "stub", ""))
        }

        fn network(&self, _request: NetworkRequest) -> KernelResult<NetworkResult> {
            Err(KernelError::CapabilityMissing {
                capability_id: "host.network".to_string(),
            })
        }

        fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue> {
            Ok(SecretValue::new(secret_ref.secret_ref_id, "inner-secret"))
        }
    }

    #[test]
    fn fallback_host_prefers_env_over_inner() {
        let _guard = env_test_guard();
        let key = "SDKWORK_SECRET_HOST_FALLBACK_DEMO";
        std::env::set_var(key, "env-secret");
        let host = EnvFileSecretFallbackHostProvider::new(Arc::new(StubHost));
        let resolved = host
            .resolve_secret(SecretRef::new("host.fallback.demo", "demo"))
            .expect("resolve");
        assert_eq!(resolved.expose_value(), "env-secret");
        std::env::remove_var(key);
    }

    #[test]
    fn fallback_host_delegates_when_env_missing() {
        let host = EnvFileSecretFallbackHostProvider::new(Arc::new(StubHost));
        let resolved = host
            .resolve_secret(SecretRef::new("missing.env.secret", "missing"))
            .expect("resolve");
        assert_eq!(resolved.expose_value(), "inner-secret");
    }
}
