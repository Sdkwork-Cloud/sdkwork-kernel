//! Contract tests: sandboxed execution lifecycle coordination.
//!
//! The `SandboxedExecutionCoordinator` runs an action inside a bound
//! sandbox session lifecycle (get -> start when `auto_start` -> action ->
//! stop when `auto_stop`) through the kernel-side `SandboxedSessionPort`.
//! It is fail-closed: a missing session refuses execution before the
//! action runs, and lifecycle failures propagate as kernel errors instead
//! of being hidden.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use sdkwork_agent_kernel::{
    KernelError, KernelResult, SandboxExecutionBinding, SandboxSessionCommandRequest,
    SandboxSessionRuntimeProjection, SandboxSessionState, SandboxedExecutionCoordinator,
    SandboxedExecutionResult, SandboxedLifecycleStep, SandboxedSessionPort,
};

/// Recording double for the kernel-side sandbox lifecycle port.
struct RecordingSandboxedSessionPort {
    calls: Mutex<Vec<String>>,
    session_state: SandboxSessionState,
    session_found: bool,
    stop_succeeds: bool,
}

impl RecordingSandboxedSessionPort {
    fn running() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            session_state: SandboxSessionState::Running,
            session_found: true,
            stop_succeeds: true,
        }
    }

    fn created() -> Self {
        Self {
            session_state: SandboxSessionState::Created,
            ..Self::running()
        }
    }

    fn missing() -> Self {
        Self {
            session_found: false,
            ..Self::running()
        }
    }

    fn failing_stop() -> Self {
        Self {
            stop_succeeds: false,
            ..Self::running()
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .clone()
    }
}

fn projection(state: SandboxSessionState) -> SandboxSessionRuntimeProjection {
    SandboxSessionRuntimeProjection::new(
        "agent-workspace-1",
        "agent-session-1",
        state,
        Some("sandbox-1"),
        Some("binding-1"),
        Some("provider.sandbox.acme"),
        Some("location-1"),
    )
}

#[async_trait]
impl SandboxedSessionPort for RecordingSandboxedSessionPort {
    async fn get_sandbox_session(
        &self,
        tenant_id: String,
        agent_session_id: String,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!("get:{tenant_id}:{agent_session_id}"));
        if !self.session_found {
            return Err(KernelError::validation("sandbox session was not found"));
        }
        Ok(projection(self.session_state))
    }

    async fn start_sandbox_session(
        &self,
        request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!(
                "start:{}:{}",
                request.tenant_id, request.agent_session_id
            ));
        Ok(projection(SandboxSessionState::Running))
    }

    async fn stop_sandbox_session(
        &self,
        request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!(
                "stop:{}:{}",
                request.tenant_id, request.agent_session_id
            ));
        if !self.stop_succeeds {
            return Err(KernelError::conflict("sandbox stop failed"));
        }
        Ok(projection(SandboxSessionState::Stopped))
    }
}

async fn identity<T>(value: T) -> KernelResult<T> {
    Ok(value)
}

#[tokio::test]
async fn runs_action_inside_full_lifecycle_when_auto_start_and_auto_stop() {
    let port = Arc::new(RecordingSandboxedSessionPort::created());
    let coordinator = SandboxedExecutionCoordinator::new(port.clone());
    let binding = SandboxExecutionBinding::new("agent-session-1")
        .with_auto_start()
        .with_auto_stop();

    let outcome: SandboxedExecutionResult<i32> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { identity(42).await },
        )
        .await
        .expect("sandboxed execution succeeds");

    assert_eq!(outcome.result, 42);
    assert_eq!(
        outcome.lifecycle,
        vec![
            SandboxedLifecycleStep::SessionFound,
            SandboxedLifecycleStep::SessionStarted,
            SandboxedLifecycleStep::SessionStopped,
        ]
    );
    assert_eq!(
        port.calls(),
        vec![
            "get:tenant-1:agent-session-1".to_string(),
            "start:tenant-1:agent-session-1".to_string(),
            "stop:tenant-1:agent-session-1".to_string(),
        ]
    );
}

#[tokio::test]
async fn skips_start_when_session_already_running_and_skips_stop_without_auto_stop() {
    let port = Arc::new(RecordingSandboxedSessionPort::running());
    let coordinator = SandboxedExecutionCoordinator::new(port.clone());
    let binding = SandboxExecutionBinding::new("agent-session-1");

    let outcome: SandboxedExecutionResult<&str> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { identity("done").await },
        )
        .await
        .expect("sandboxed execution succeeds");

    assert_eq!(outcome.result, "done");
    assert_eq!(
        outcome.lifecycle,
        vec![
            SandboxedLifecycleStep::SessionFound,
            SandboxedLifecycleStep::SessionAlreadyRunning,
        ]
    );
    assert_eq!(
        port.calls(),
        vec!["get:tenant-1:agent-session-1".to_string()]
    );
}

#[tokio::test]
async fn refuses_execution_when_session_is_missing() {
    let port = Arc::new(RecordingSandboxedSessionPort::missing());
    let coordinator = SandboxedExecutionCoordinator::new(port.clone());
    let binding = SandboxExecutionBinding::new("agent-session-1").with_auto_start();

    let action_ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let action_ran_for_closure = Arc::clone(&action_ran);
    let result = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            move || {
                let action_ran_for_closure = Arc::clone(&action_ran_for_closure);
                async move {
                    action_ran_for_closure.store(true, std::sync::atomic::Ordering::SeqCst);
                    identity(1).await
                }
            },
        )
        .await;

    assert!(result.is_err(), "missing session must refuse execution");
    assert!(
        !action_ran.load(std::sync::atomic::Ordering::SeqCst),
        "action must not run when the session is missing"
    );
    assert_eq!(
        result.unwrap_err().kind(),
        sdkwork_agent_kernel::KernelErrorKind::ValidationError
    );
    assert_eq!(
        port.calls(),
        vec!["get:tenant-1:agent-session-1".to_string()]
    );
}

#[tokio::test]
async fn refuses_execution_when_session_not_running_and_auto_start_disabled() {
    let port = Arc::new(RecordingSandboxedSessionPort::created());
    let coordinator = SandboxedExecutionCoordinator::new(port);
    let binding = SandboxExecutionBinding::new("agent-session-1");

    let result: KernelResult<SandboxedExecutionResult<i32>> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { identity(1).await },
        )
        .await;

    assert!(matches!(result, Err(KernelError::Validation { .. })));
}

#[tokio::test]
async fn action_failure_propagates_but_stop_cleanup_still_attempted() {
    let port = Arc::new(RecordingSandboxedSessionPort::created());
    let coordinator = SandboxedExecutionCoordinator::new(port.clone());
    let binding = SandboxExecutionBinding::new("agent-session-1")
        .with_auto_start()
        .with_auto_stop();

    let result: KernelResult<SandboxedExecutionResult<i32>> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { Err(KernelError::provider_error("model", "model exploded")) },
        )
        .await;

    assert!(result.is_err(), "action error must propagate");
    assert_eq!(
        port.calls(),
        vec![
            "get:tenant-1:agent-session-1".to_string(),
            "start:tenant-1:agent-session-1".to_string(),
            "stop:tenant-1:agent-session-1".to_string(),
        ],
        "stop cleanup must run even when the action failed"
    );
}

#[tokio::test]
async fn failed_stop_on_successful_action_is_fail_closed() {
    let port = Arc::new(RecordingSandboxedSessionPort::failing_stop());
    let coordinator = SandboxedExecutionCoordinator::new(port);
    let binding = SandboxExecutionBinding::new("agent-session-1").with_auto_stop();

    let result: KernelResult<SandboxedExecutionResult<i32>> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { identity(7).await },
        )
        .await;

    assert!(
        result.is_err(),
        "failed stop must surface instead of being hidden"
    );
    assert_eq!(
        result.unwrap_err().kind(),
        sdkwork_agent_kernel::KernelErrorKind::Conflict
    );
}

#[tokio::test]
async fn invalid_binding_is_rejected_before_any_port_call() {
    let port = Arc::new(RecordingSandboxedSessionPort::running());
    let coordinator = SandboxedExecutionCoordinator::new(port.clone());
    let binding = SandboxExecutionBinding::new("  ").with_auto_start();

    let result: KernelResult<SandboxedExecutionResult<i32>> = coordinator
        .run_sandboxed(
            "tenant-1".to_string(),
            &binding,
            "op-1".to_string(),
            || async { identity(1).await },
        )
        .await;

    assert!(result.is_err());
    assert!(
        port.calls().is_empty(),
        "no sandbox call for invalid binding"
    );
}

#[test]
fn binding_builders_configure_lifecycle_flags() {
    let binding = SandboxExecutionBinding::new("session-1")
        .with_workspace_id("workspace-1")
        .with_auto_start()
        .with_auto_stop();

    assert_eq!(binding.sandbox_session_id, "session-1");
    assert_eq!(binding.sandbox_workspace_id.as_deref(), Some("workspace-1"));
    assert!(binding.auto_start);
    assert!(binding.auto_stop);

    let minimal = SandboxExecutionBinding::new("session-2");
    assert!(!minimal.auto_start);
    assert!(!minimal.auto_stop);
    assert!(minimal.sandbox_workspace_id.is_none());
}

#[test]
fn projection_exposes_all_identity_fields() {
    let projection = projection(SandboxSessionState::Running);
    assert_eq!(projection.sandbox_workspace_id(), "agent-workspace-1");
    assert_eq!(projection.sandbox_session_id(), "agent-session-1");
    assert_eq!(
        projection.sandbox_session_state(),
        SandboxSessionState::Running
    );
    assert_eq!(projection.sandbox_id(), Some("sandbox-1"));
    assert_eq!(projection.sandbox_runtime_binding_id(), Some("binding-1"));
    assert_eq!(
        projection.sandbox_provider_id(),
        Some("provider.sandbox.acme")
    );
    assert_eq!(projection.agent_runtime_location_id(), Some("location-1"));
}
