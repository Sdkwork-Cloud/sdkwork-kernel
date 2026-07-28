//! Adapter from Agents-owned identities to the SDKWork Sandbox lifecycle port.

use std::collections::BTreeSet;
use std::sync::Arc;

use sdkwork_intelligence_sandbox_service::{
    CreateSandboxSessionCommand, SandboxLifecycleError, SandboxSession,
    SandboxSessionLifecycleCommand, SandboxSessionLifecyclePort, SandboxSessionRepositoryError,
    SandboxSessionState,
};
use sdkwork_sandbox_provider_spi::{
    IsolationAssurance, OperationId, RuntimeCapability, SandboxSessionId, SandboxWorkspaceId,
    TenantId,
};

use crate::{KernelError, KernelErrorSource, KernelResult};

/// Kernel-owned input for creating a Sandbox runtime projection of an Agent Session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxSessionCreateRequest {
    pub tenant_id: String,
    pub agent_workspace_id: String,
    pub agent_session_id: String,
    pub sandbox_operation_id: String,
    pub sandbox_required_capabilities: BTreeSet<RuntimeCapability>,
    pub sandbox_minimum_assurance: IsolationAssurance,
}

/// Kernel-owned input for an existing Sandbox Session lifecycle command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxSessionCommandRequest {
    pub tenant_id: String,
    pub agent_session_id: String,
    pub sandbox_operation_id: String,
}

/// Safe runtime projection returned to Kernel and Agents integration code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxSessionRuntimeProjection {
    sandbox_workspace_id: String,
    sandbox_session_id: String,
    sandbox_session_state: SandboxSessionState,
    sandbox_id: Option<String>,
    sandbox_runtime_binding_id: Option<String>,
    sandbox_provider_id: Option<String>,
    agent_runtime_location_id: Option<String>,
}

impl SandboxSessionRuntimeProjection {
    pub fn sandbox_workspace_id(&self) -> &str {
        self.sandbox_workspace_id.as_str()
    }

    pub fn sandbox_session_id(&self) -> &str {
        self.sandbox_session_id.as_str()
    }

    pub fn sandbox_session_state(&self) -> SandboxSessionState {
        self.sandbox_session_state
    }

    pub fn sandbox_id(&self) -> Option<&str> {
        self.sandbox_id.as_deref()
    }

    pub fn sandbox_runtime_binding_id(&self) -> Option<&str> {
        self.sandbox_runtime_binding_id.as_deref()
    }

    pub fn sandbox_provider_id(&self) -> Option<&str> {
        self.sandbox_provider_id.as_deref()
    }

    /// Agents persists this opaque value as `runtimeLocationId`.
    pub fn agent_runtime_location_id(&self) -> Option<&str> {
        self.agent_runtime_location_id.as_deref()
    }
}

impl From<SandboxSession> for SandboxSessionRuntimeProjection {
    fn from(sandbox_session: SandboxSession) -> Self {
        let sandbox_runtime_binding = sandbox_session.sandbox_runtime_binding();
        let sandbox_runtime_binding_id = sandbox_runtime_binding.map(|sandbox_runtime_binding| {
            sandbox_runtime_binding
                .sandbox_runtime_binding_id()
                .as_str()
                .to_string()
        });

        Self {
            sandbox_workspace_id: sandbox_session.sandbox_workspace_id().as_str().to_string(),
            sandbox_session_id: sandbox_session.sandbox_session_id().as_str().to_string(),
            sandbox_session_state: sandbox_session.sandbox_session_state(),
            sandbox_id: sandbox_runtime_binding.map(|sandbox_runtime_binding| {
                sandbox_runtime_binding.sandbox_id().as_str().to_string()
            }),
            sandbox_provider_id: sandbox_runtime_binding.map(|sandbox_runtime_binding| {
                sandbox_runtime_binding
                    .sandbox_provider_id()
                    .as_str()
                    .to_string()
            }),
            agent_runtime_location_id: sandbox_runtime_binding_id.clone(),
            sandbox_runtime_binding_id,
        }
    }
}

/// Namespaced Kernel adapter that consumes the Sandbox-owned lifecycle port.
pub struct SandboxSessionLifecycleAdapter {
    sandbox_session_lifecycle_port: Arc<dyn SandboxSessionLifecyclePort>,
}

impl SandboxSessionLifecycleAdapter {
    pub fn new(sandbox_session_lifecycle_port: Arc<dyn SandboxSessionLifecyclePort>) -> Self {
        Self {
            sandbox_session_lifecycle_port,
        }
    }

    pub async fn create_sandbox_session(
        &self,
        sandbox_request: SandboxSessionCreateRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        let sandbox_command = CreateSandboxSessionCommand {
            tenant_id: parse_tenant_id(sandbox_request.tenant_id)?,
            sandbox_workspace_id: parse_sandbox_workspace_id(sandbox_request.agent_workspace_id)?,
            sandbox_session_id: parse_sandbox_session_id(sandbox_request.agent_session_id)?,
            sandbox_operation_id: parse_sandbox_operation_id(sandbox_request.sandbox_operation_id)?,
            sandbox_required_capabilities: sandbox_request.sandbox_required_capabilities,
            sandbox_minimum_assurance: sandbox_request.sandbox_minimum_assurance,
        };

        self.sandbox_session_lifecycle_port
            .create_sandbox_session(sandbox_command)
            .await
            .map(SandboxSessionRuntimeProjection::from)
            .map_err(map_sandbox_lifecycle_error)
    }

    pub async fn get_sandbox_session(
        &self,
        tenant_id: impl Into<String>,
        agent_session_id: impl Into<String>,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        let tenant_id = parse_tenant_id(tenant_id.into())?;
        let sandbox_session_id = parse_sandbox_session_id(agent_session_id.into())?;
        self.sandbox_session_lifecycle_port
            .get_sandbox_session(&tenant_id, &sandbox_session_id)
            .await
            .map(SandboxSessionRuntimeProjection::from)
            .map_err(map_sandbox_lifecycle_error)
    }

    pub async fn start_sandbox_session(
        &self,
        sandbox_request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.execute_sandbox_session_command(sandbox_request, SandboxLifecycleAction::Start)
            .await
    }

    pub async fn stop_sandbox_session(
        &self,
        sandbox_request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.execute_sandbox_session_command(sandbox_request, SandboxLifecycleAction::Stop)
            .await
    }

    pub async fn destroy_sandbox_session(
        &self,
        sandbox_request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.execute_sandbox_session_command(sandbox_request, SandboxLifecycleAction::Destroy)
            .await
    }

    async fn execute_sandbox_session_command(
        &self,
        sandbox_request: SandboxSessionCommandRequest,
        sandbox_lifecycle_action: SandboxLifecycleAction,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        let sandbox_command = SandboxSessionLifecycleCommand {
            tenant_id: parse_tenant_id(sandbox_request.tenant_id)?,
            sandbox_session_id: parse_sandbox_session_id(sandbox_request.agent_session_id)?,
            sandbox_operation_id: parse_sandbox_operation_id(sandbox_request.sandbox_operation_id)?,
        };

        let sandbox_result = match sandbox_lifecycle_action {
            SandboxLifecycleAction::Start => {
                self.sandbox_session_lifecycle_port
                    .start_sandbox_session(sandbox_command)
                    .await
            }
            SandboxLifecycleAction::Stop => {
                self.sandbox_session_lifecycle_port
                    .stop_sandbox_session(sandbox_command)
                    .await
            }
            SandboxLifecycleAction::Destroy => {
                self.sandbox_session_lifecycle_port
                    .destroy_sandbox_session(sandbox_command)
                    .await
            }
        };

        sandbox_result
            .map(SandboxSessionRuntimeProjection::from)
            .map_err(map_sandbox_lifecycle_error)
    }
}

#[derive(Clone, Copy)]
enum SandboxLifecycleAction {
    Start,
    Stop,
    Destroy,
}

fn parse_tenant_id(tenant_id: String) -> KernelResult<TenantId> {
    TenantId::parse(tenant_id).map_err(map_sandbox_identifier_error)
}

fn parse_sandbox_workspace_id(agent_workspace_id: String) -> KernelResult<SandboxWorkspaceId> {
    SandboxWorkspaceId::parse(agent_workspace_id).map_err(map_sandbox_identifier_error)
}

fn parse_sandbox_session_id(agent_session_id: String) -> KernelResult<SandboxSessionId> {
    SandboxSessionId::parse(agent_session_id).map_err(map_sandbox_identifier_error)
}

fn parse_sandbox_operation_id(sandbox_operation_id: String) -> KernelResult<OperationId> {
    OperationId::parse(sandbox_operation_id).map_err(map_sandbox_identifier_error)
}

fn map_sandbox_identifier_error(
    sandbox_identifier_error: sdkwork_sandbox_provider_spi::SandboxIdentifierError,
) -> KernelError {
    KernelError::validation(format!(
        "sandbox identity mapping failed: {sandbox_identifier_error}"
    ))
}

fn map_sandbox_lifecycle_error(sandbox_lifecycle_error: SandboxLifecycleError) -> KernelError {
    match sandbox_lifecycle_error {
        SandboxLifecycleError::SandboxSessionNotFound { .. }
        | SandboxLifecycleError::Repository(SandboxSessionRepositoryError::NotFound) => {
            KernelError::validation("sandbox session was not found")
        }
        SandboxLifecycleError::InvalidTransition { .. }
        | SandboxLifecycleError::IdempotencyConflict { .. }
        | SandboxLifecycleError::OperationInProgress { .. }
        | SandboxLifecycleError::OperationPreviouslyFailed { .. }
        | SandboxLifecycleError::Repository(SandboxSessionRepositoryError::VersionConflict)
        | SandboxLifecycleError::Repository(SandboxSessionRepositoryError::DuplicateOperation) => {
            KernelError::conflict(sandbox_lifecycle_error.to_string())
                .from_source(KernelErrorSource::Runtime)
        }
        SandboxLifecycleError::NoEligibleProvider => KernelError::CapabilityMissing {
            capability_id: "sandbox.runtime".to_string(),
        },
        SandboxLifecycleError::NoHealthyProvider => KernelError::ProviderUnavailable {
            provider_id: "sandbox".to_string(),
        },
        SandboxLifecycleError::ProviderReadinessRejected {
            sandbox_provider_id,
        } => KernelError::ProviderUnavailable {
            provider_id: sandbox_provider_id.as_str().to_string(),
        },
        SandboxLifecycleError::Provider(sandbox_provider_error) => KernelError::provider_error(
            "sandbox_provider_error",
            sandbox_provider_error.to_string(),
        ),
        SandboxLifecycleError::DuplicateProvider { .. }
        | SandboxLifecycleError::InvariantViolation(_)
        | SandboxLifecycleError::Repository(SandboxSessionRepositoryError::Unavailable) => {
            KernelError::Internal {
                message: "sandbox lifecycle service is internally unavailable".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use sdkwork_intelligence_sandbox_service::{
        SandboxLifecycleResult, SandboxSessionLifecyclePort,
    };

    use super::*;

    #[derive(Default)]
    struct CapturingSandboxSessionLifecyclePort {
        sandbox_create_command: Mutex<Option<CreateSandboxSessionCommand>>,
    }

    #[async_trait]
    impl SandboxSessionLifecyclePort for CapturingSandboxSessionLifecyclePort {
        async fn create_sandbox_session(
            &self,
            sandbox_command: CreateSandboxSessionCommand,
        ) -> SandboxLifecycleResult<SandboxSession> {
            let mut sandbox_create_command = self
                .sandbox_create_command
                .lock()
                .unwrap_or_else(|poisoned_state| poisoned_state.into_inner());
            *sandbox_create_command = Some(sandbox_command);
            Err(SandboxLifecycleError::NoHealthyProvider)
        }

        async fn get_sandbox_session(
            &self,
            _tenant_id: &TenantId,
            _sandbox_session_id: &SandboxSessionId,
        ) -> SandboxLifecycleResult<SandboxSession> {
            Err(SandboxLifecycleError::NoHealthyProvider)
        }

        async fn start_sandbox_session(
            &self,
            _sandbox_command: SandboxSessionLifecycleCommand,
        ) -> SandboxLifecycleResult<SandboxSession> {
            Err(SandboxLifecycleError::NoHealthyProvider)
        }

        async fn stop_sandbox_session(
            &self,
            _sandbox_command: SandboxSessionLifecycleCommand,
        ) -> SandboxLifecycleResult<SandboxSession> {
            Err(SandboxLifecycleError::NoHealthyProvider)
        }

        async fn destroy_sandbox_session(
            &self,
            _sandbox_command: SandboxSessionLifecycleCommand,
        ) -> SandboxLifecycleResult<SandboxSession> {
            Err(SandboxLifecycleError::NoHealthyProvider)
        }
    }

    #[tokio::test]
    async fn maps_agents_owned_ids_into_sandbox_qualified_command_fields() {
        let sandbox_port = Arc::new(CapturingSandboxSessionLifecyclePort::default());
        let sandbox_port_for_assertion = Arc::clone(&sandbox_port);
        let sandbox_adapter = SandboxSessionLifecycleAdapter::new(sandbox_port);

        let sandbox_result = sandbox_adapter
            .create_sandbox_session(SandboxSessionCreateRequest {
                tenant_id: "tenant-1".to_string(),
                agent_workspace_id: "agent-workspace-1".to_string(),
                agent_session_id: "agent-session-1".to_string(),
                sandbox_operation_id: "sandbox-operation-1".to_string(),
                sandbox_required_capabilities: BTreeSet::from([RuntimeCapability::Filesystem]),
                sandbox_minimum_assurance: IsolationAssurance::HostUser,
            })
            .await;

        assert!(matches!(
            sandbox_result,
            Err(KernelError::ProviderUnavailable { ref provider_id }) if provider_id == "sandbox"
        ));
        let sandbox_create_command = sandbox_port_for_assertion
            .sandbox_create_command
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .clone()
            .unwrap_or_else(|| panic!("sandbox create command must be captured"));
        assert_eq!(
            sandbox_create_command.sandbox_workspace_id.as_str(),
            "agent-workspace-1"
        );
        assert_eq!(
            sandbox_create_command.sandbox_session_id.as_str(),
            "agent-session-1"
        );
        assert_eq!(
            sandbox_create_command.sandbox_operation_id.as_str(),
            "sandbox-operation-1"
        );
    }

    #[test]
    fn rejects_path_like_agents_ids_before_calling_sandbox() {
        let sandbox_mapping_result = parse_sandbox_workspace_id("../workspace".to_string());
        assert!(matches!(
            sandbox_mapping_result,
            Err(KernelError::Validation { .. })
        ));
    }
}
