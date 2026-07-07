//! Host provider wrapper that routes process execution through [`SandboxProvider`].
//!
//! Filesystem, network, and secret operations delegate to the inner host; only
//! `process` is sandboxed so tool and subprocess paths get policy validation
//! before execution.

use crate::host::SecretValue;
use crate::{
    FilesystemRequest, FilesystemResult, HostProvider, KernelError, KernelResult, NetworkRequest,
    NetworkResult, ProcessRequest, ProcessResult, ProviderHealth, ProviderManifest, SandboxCommand,
    SandboxPolicy, SandboxProvider, SecretRef,
};
use crate::{StorageRequest, StorageResult};
use std::sync::Arc;

/// Wraps a [`HostProvider`] and executes `process` calls inside the configured sandbox.
#[derive(Clone)]
pub struct SandboxingHostProvider {
    inner: Arc<dyn HostProvider + Send + Sync>,
    sandbox: Arc<dyn SandboxProvider>,
    default_policy: SandboxPolicy,
}

impl SandboxingHostProvider {
    pub fn new(
        inner: Arc<dyn HostProvider + Send + Sync>,
        sandbox: Arc<dyn SandboxProvider>,
    ) -> Self {
        let sandbox_type = sandbox.sandbox_type();
        let default_policy = SandboxPolicy::new(sandbox_type)
            .with_file_system(crate::FileSystemSandboxPolicy::restrictive(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ))
            .with_network(crate::NetworkSandboxPolicy::no_network());
        Self {
            inner,
            sandbox,
            default_policy,
        }
    }

    pub fn with_default_policy(mut self, policy: SandboxPolicy) -> Self {
        self.default_policy = policy;
        self
    }

    fn sandbox_process(&self, request: ProcessRequest) -> KernelResult<ProcessResult> {
        self.sandbox
            .validate_policy(&self.default_policy)
            .map_err(|error| KernelError::validation(error.to_string()))?;

        let mut command = SandboxCommand::new(&request.command)
            .with_args(request.args)
            .with_cwd(&request.working_directory);

        match &request.env_policy {
            crate::HostEnvPolicy::Explicit(entries) => {
                for (key, value) in entries {
                    command = command.with_env(key, value);
                }
            }
            crate::HostEnvPolicy::AllowList(keys) => {
                for key in keys {
                    if let Ok(value) = std::env::var(key) {
                        command = command.with_env(key, value);
                    }
                }
            }
            crate::HostEnvPolicy::Inherit | crate::HostEnvPolicy::None => {}
        }

        let execution = self
            .sandbox
            .execute(command, self.default_policy.clone())
            .map_err(|error| KernelError::Internal {
                message: format!("sandbox process execution failed: {error}"),
            })?;

        Ok(ProcessResult::exited(
            request.operation_id,
            execution.exit_code,
            execution.stdout,
            execution.stderr,
        ))
    }
}

impl HostProvider for SandboxingHostProvider {
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
        self.sandbox_process(request)
    }

    fn network(&self, request: NetworkRequest) -> KernelResult<NetworkResult> {
        self.inner.network(request)
    }

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue> {
        self.inner.resolve_secret(secret_ref)
    }

    fn storage(&self, request: StorageRequest) -> KernelResult<StorageResult> {
        self.inner.storage(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostEnvPolicy, NoOpSandboxProvider, ProviderManifest};

    struct EchoHostProvider;

    impl HostProvider for EchoHostProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.host.echo",
                "host",
                "echo-host",
                "0.1.0",
                vec!["host.filesystem".to_string(), "host.process".to_string()],
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
            Ok(ProcessResult::exited(request.operation_id, 0, "inner", ""))
        }

        fn network(&self, _request: NetworkRequest) -> KernelResult<NetworkResult> {
            Err(KernelError::CapabilityMissing {
                capability_id: "host.network".to_string(),
            })
        }

        fn resolve_secret(&self, _secret_ref: SecretRef) -> KernelResult<SecretValue> {
            Err(KernelError::CapabilityMissing {
                capability_id: "host.secrets".to_string(),
            })
        }
    }

    #[test]
    fn sandboxing_host_routes_process_through_sandbox_provider() {
        let inner = Arc::new(EchoHostProvider);
        let sandbox = Arc::new(NoOpSandboxProvider);
        let host = SandboxingHostProvider::new(inner, sandbox);

        let request = ProcessRequest::spawn(
            "op.1",
            if cfg!(windows) { "cmd" } else { "echo" },
            if cfg!(windows) {
                vec![
                    "/C".to_string(),
                    "echo".to_string(),
                    "sandboxed".to_string(),
                ]
            } else {
                vec!["sandboxed".to_string()]
            },
            ".",
        )
        .with_env_policy(HostEnvPolicy::None);

        let result = host.process(request).expect("sandboxed process should run");
        assert!(result.exit_code.unwrap_or(-1) == 0);
        assert!(result.stdout.contains("sandboxed"));
    }
}
