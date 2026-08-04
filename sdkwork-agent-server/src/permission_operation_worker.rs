//! Durable worker for one-shot approval-gated tool execution.

use crate::api::internal_runtime::InternalRuntimeApiState;
use crate::approval_payload_vault::ApprovalPayloadContext;
use crate::config::ServerConfig;
use crate::metrics::DurableWorkerKind;
use chrono::{Duration as ChronoDuration, Utc};
use sdkwork_agent_database::{format_runtime_timestamp, ClaimedPermissionOperation, EventRow};
use sdkwork_agent_kernel::{ApprovedToolExecution, ToolCall, ToolCallStatus};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing::{info, warn};

const MAX_TOOL_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_ERROR_DETAIL_CHARS: usize = 1024;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct PermissionOperationWorker {
    task: Option<JoinHandle<()>>,
}

impl PermissionOperationWorker {
    pub fn spawn(
        state: Arc<InternalRuntimeApiState>,
        config: Arc<ServerConfig>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        let task = tokio::spawn(async move {
            let mut workers = JoinSet::new();
            for worker_index in 0..config.task_worker_max_concurrency {
                workers.spawn(run_worker_loop(
                    format!("permission-worker-{}-{worker_index}", std::process::id()),
                    state.clone(),
                    config.clone(),
                    shutdown.clone(),
                ));
            }
            while let Some(result) = workers.join_next().await {
                if let Err(error) = result {
                    // Restart a panicked worker so approval capacity
                    // self-heals; permission claims are lease-fenced, so a
                    // restart cannot resume a stale lease.
                    warn!(error = %error, "permission operation worker panicked; restarting");
                    workers.spawn(run_worker_loop(
                        format!(
                            "permission-worker-{}-{}",
                            std::process::id(),
                            sdkwork_utils_rust::uuid()
                        ),
                        state.clone(),
                        config.clone(),
                        shutdown.clone(),
                    ));
                }
            }
        });
        Self { task: Some(task) }
    }

    pub async fn join(mut self) {
        if let Some(mut task) = self.task.take() {
            if time::timeout(SHUTDOWN_TIMEOUT, &mut task).await.is_err() {
                warn!("permission workers did not stop before the shutdown deadline");
                task.abort();
                let _ = task.await;
            }
        }
    }
}

impl Drop for PermissionOperationWorker {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn run_worker_loop(
    worker_id: String,
    state: Arc<InternalRuntimeApiState>,
    config: Arc<ServerConfig>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut ticker = time::interval(Duration::from_millis(config.task_worker_poll_interval_ms));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            _ = ticker.tick() => {
                match expire_operations(&state).await {
                    Ok(expired) => state.runtime.record_durable_worker_outcome(
                        DurableWorkerKind::Permission,
                        "expired",
                        expired as u64,
                    ),
                    Err(error) => {
                        warn!(worker_id, error = %error, "permission operation expiry sweep failed");
                        continue;
                    }
                }
                match claim_operation(&state, &worker_id, config.task_worker_lease_secs).await {
                    Ok(Some(claim)) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Permission,
                            "claimed",
                            1,
                        );
                        execute_claim(state.clone(), config.clone(), claim).await;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Permission,
                            "claim_error",
                            1,
                        );
                        warn!(worker_id, error = %error, "permission operation claim failed");
                    }
                }
            }
        }
    }
}

async fn expire_operations(state: &InternalRuntimeApiState) -> Result<usize, String> {
    let now = sdkwork_agent_database::runtime_now_timestamp();
    state
        .persist(move |persistence| persistence.expire_permission_operations(&now, 200))
        .await
}

async fn claim_operation(
    state: &InternalRuntimeApiState,
    worker_id: &str,
    lease_secs: u64,
) -> Result<Option<ClaimedPermissionOperation>, String> {
    let now = Utc::now();
    let lease_seconds = i64::try_from(lease_secs)
        .map_err(|_| "permission worker lease duration exceeds i64".to_string())?;
    let now_value = format_runtime_timestamp(now);
    let lease_expires_at = format_runtime_timestamp(now + ChronoDuration::seconds(lease_seconds));
    let worker_id = worker_id.to_string();
    state
        .persist(move |persistence| {
            persistence.claim_permission_operation(&worker_id, &now_value, &lease_expires_at)
        })
        .await
}

async fn execute_claim(
    state: Arc<InternalRuntimeApiState>,
    config: Arc<ServerConfig>,
    claim: ClaimedPermissionOperation,
) {
    let _active_guard = state
        .runtime
        .begin_durable_worker_operation(DurableWorkerKind::Permission);
    let permission_id = claim.operation.permission_request_id.clone();
    let session_id = claim.run.session_id.clone();
    let loaded = state
        .persist(move |persistence| {
            let permission = persistence
                .load_permission(&permission_id)?
                .ok_or_else(|| "permission request not found".to_string())?;
            let session = persistence.get_session(&session_id)?;
            Ok((permission, session))
        })
        .await;
    let (permission, session) = match loaded {
        Ok(rows) => rows,
        Err(error) => {
            fail_claim(&state, &claim, "persistence_error", Some("50001"), &error).await;
            return;
        }
    };
    if permission.status != "allow" {
        fail_claim(
            &state,
            &claim,
            "permission_not_allowed",
            Some("40301"),
            "permission decision is not allow",
        )
        .await;
        return;
    }
    let Some(vault) = state.approval_payload_vault.as_ref() else {
        fail_claim(
            &state,
            &claim,
            "encryption_unavailable",
            Some("50001"),
            "approval payload encryption is not configured",
        )
        .await;
        return;
    };
    let aad = match (ApprovalPayloadContext {
        permission_request_id: &claim.operation.permission_request_id,
        session_id: &claim.run.session_id,
        task_id: &claim.run.task_id,
        run_id: &claim.run.run_id,
        step_id: &claim.step.step_id,
        tool_call_id: &claim.operation.tool_call_id,
        provider_id: &claim.operation.provider_id,
        descriptor_revision: &claim.operation.descriptor_revision,
        policy_revision: &claim.operation.policy_revision,
    })
    .to_aad()
    {
        Ok(aad) => aad,
        Err(error) => {
            fail_claim(
                &state,
                &claim,
                "payload_context_invalid",
                Some("50001"),
                &error,
            )
            .await;
            return;
        }
    };
    let arguments = match vault.open(&claim.operation.payload_ref, &aad) {
        Ok(arguments)
            if sdkwork_utils_rust::secure_compare(
                &sdkwork_utils_rust::sha256_hash(arguments.as_bytes()),
                &claim.operation.payload_digest,
            ) =>
        {
            arguments
        }
        Ok(_) => {
            fail_claim(
                &state,
                &claim,
                "payload_digest_mismatch",
                Some("50001"),
                "approval payload digest verification failed",
            )
            .await;
            return;
        }
        Err(error) => {
            fail_claim(
                &state,
                &claim,
                "payload_authentication_failed",
                Some("50001"),
                &error,
            )
            .await;
            return;
        }
    };

    let admission = match state.runtime.acquire_provider_admission().await {
        Ok(admission) => admission,
        Err(error) => {
            fail_claim(
                &state,
                &claim,
                "provider_unavailable",
                Some("50301"),
                error.safe_message(),
            )
            .await;
            return;
        }
    };
    let trace_id = sdkwork_utils_rust::uuid();
    if let Err(error) = state
        .register_persisted_session(&admission, &session, &trace_id)
        .await
    {
        fail_claim(
            &state,
            &claim,
            "session_restore_failed",
            Some(&error.code.as_i32().to_string()),
            &error.detail,
        )
        .await;
        return;
    }

    let call = ToolCall::new(
        claim.operation.tool_call_id.clone(),
        permission.resource.clone(),
        arguments,
    )
    .for_session(claim.run.session_id.clone())
    .for_task(claim.run.task_id.clone())
    .for_run(claim.run.run_id.clone())
    .for_step(claim.step.step_id.clone())
    .with_provider(claim.operation.provider_id.clone());
    let approval = ApprovedToolExecution::new(
        claim.operation.permission_request_id.clone(),
        claim.operation.provider_id.clone(),
        claim.operation.descriptor_revision.clone(),
        claim.operation.policy_revision.clone(),
    );
    let operation = state
        .runtime
        .run_provider_admitted(admission, move |runtime| {
            runtime.execute_approved_tool(call, approval)
        });
    tokio::pin!(operation);
    let mut renewals = time::interval(Duration::from_secs(
        (config.task_worker_lease_secs / 3).max(1),
    ));
    renewals.set_missed_tick_behavior(MissedTickBehavior::Skip);
    renewals.tick().await;
    let result = loop {
        tokio::select! {
            result = &mut operation => break result,
            _ = renewals.tick() => {
                match renew_claim(&state, &claim, config.task_worker_lease_secs).await {
                    Ok(true) => {}
                    Ok(false) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Permission,
                            "lease_lost",
                            1,
                        );
                        warn!(permission_request_id = %claim.operation.permission_request_id,
                            "permission operation lease ownership was lost");
                        return;
                    }
                    Err(error) => warn!(permission_request_id = %claim.operation.permission_request_id,
                        error = %error, "permission operation lease renewal failed"),
                }
            }
        }
    };
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            fail_claim(
                &state,
                &claim,
                &format!("{:?}", error.kind()).to_lowercase(),
                None,
                error.safe_message(),
            )
            .await;
            return;
        }
    };
    if result.normalized_status != ToolCallStatus::Succeeded {
        fail_claim(
            &state,
            &claim,
            "tool_execution_failed",
            None,
            result.error.as_deref().unwrap_or("tool execution failed"),
        )
        .await;
        return;
    }
    if result.output.len() > MAX_TOOL_OUTPUT_BYTES {
        fail_claim(
            &state,
            &claim,
            "resource_exhausted",
            Some("50301"),
            "tool output exceeds the durable permission output limit",
        )
        .await;
        return;
    }
    let result_json = serde_json::json!({
        "toolCallId": result.tool_call_id,
        "status": result.status,
        "output": result.output,
    })
    .to_string();
    let finished_at = sdkwork_agent_database::runtime_now_timestamp();
    let event = operation_event(
        &claim,
        "permission.operation.completed",
        "info",
        &finished_at,
    );
    let permission_request_id = claim.operation.permission_request_id.clone();
    let persisted_claim = claim.clone();
    if let Err(error) = state
        .persist(move |persistence| {
            persistence.complete_permission_operation(
                &persisted_claim,
                &result_json,
                &finished_at,
                &event,
            )
        })
        .await
    {
        warn!(permission_request_id, error = %error, "permission operation completion was rejected");
    } else {
        state
            .runtime
            .record_durable_worker_outcome(DurableWorkerKind::Permission, "completed", 1);
        info!(permission_request_id, "permission operation completed");
    }
}

async fn renew_claim(
    state: &InternalRuntimeApiState,
    claim: &ClaimedPermissionOperation,
    lease_secs: u64,
) -> Result<bool, String> {
    let now = Utc::now();
    let lease_seconds = i64::try_from(lease_secs)
        .map_err(|_| "permission worker lease duration exceeds i64".to_string())?;
    let now_value = format_runtime_timestamp(now);
    let lease_expires_at = format_runtime_timestamp(now + ChronoDuration::seconds(lease_seconds));
    let permission_request_id = claim.operation.permission_request_id.clone();
    let worker_id = claim.operation.lease_owner.clone().unwrap_or_default();
    let fencing_token = claim.operation.fencing_token;
    state
        .persist(move |persistence| {
            persistence.renew_permission_operation_lease(
                &permission_request_id,
                &worker_id,
                fencing_token,
                &now_value,
                &lease_expires_at,
            )
        })
        .await
}

async fn fail_claim(
    state: &InternalRuntimeApiState,
    claim: &ClaimedPermissionOperation,
    error_kind: &str,
    error_code: Option<&str>,
    error_detail: &str,
) {
    let finished_at = sdkwork_agent_database::runtime_now_timestamp();
    let event = operation_event(claim, "permission.operation.failed", "error", &finished_at);
    let permission_request_id = claim.operation.permission_request_id.clone();
    let claim = claim.clone();
    let error_kind = error_kind.to_string();
    let error_code = error_code.map(str::to_string);
    let error_detail = error_detail
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect::<String>();
    if let Err(error) = state
        .persist(move |persistence| {
            persistence.fail_permission_operation(
                &claim,
                &error_kind,
                error_code.as_deref(),
                &error_detail,
                &finished_at,
                &event,
            )
        })
        .await
    {
        warn!(permission_request_id, error = %error, "permission operation failure could not be persisted");
    } else {
        state
            .runtime
            .record_durable_worker_outcome(DurableWorkerKind::Permission, "failed", 1);
    }
}

fn operation_event(
    claim: &ClaimedPermissionOperation,
    event_type: &str,
    severity: &str,
    created_at: &str,
) -> EventRow {
    EventRow {
        event_id: format!("event.{}", sdkwork_utils_rust::uuid()),
        session_id: Some(claim.run.session_id.clone()),
        event_type: event_type.to_string(),
        severity: severity.to_string(),
        payload: Some(
            serde_json::json!({
                "permissionRequestId": claim.operation.permission_request_id,
                "taskId": claim.run.task_id,
                "runId": claim.run.run_id,
                "stepId": claim.step.step_id,
                "toolCallId": claim.operation.tool_call_id,
            })
            .to_string(),
        ),
        created_at: created_at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::PersistenceState;
    use sdkwork_agent_database::{
        ActionKind, PermissionOperationRow, PermissionOperationState, PermissionPayloadKind,
        PermissionRow, RunRow, RunState, StepRow, StepState, TaskRow,
    };
    use sdkwork_agent_kernel::{
        AgentManifest, KernelResult, PolicyDecision, PolicyProvider, PolicyRequest, ProviderHealth,
        ProviderManifest, RuntimeBuilder, SideEffectLevel, ToolDescriptor, ToolProvider,
        ToolResult,
    };
    use sdkwork_agent_session::SessionConfig;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct RecordingToolProvider {
        calls: Arc<Mutex<Vec<ToolCall>>>,
    }

    impl ToolProvider for RecordingToolProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.tool.permission-test",
                "tool",
                "permission-test-tool",
                "0.1.0",
                vec!["tool.invoke".into(), "tool.discovery".into()],
            )
        }

        fn list_tools(&self) -> Vec<ToolDescriptor> {
            vec![ToolDescriptor::new(
                "tool.permission-test",
                "provider.tool.permission-test",
                "Permission Test Tool",
                SideEffectLevel::SideEffectful,
            )
            .with_version("0.1.0")
            .with_policy_categories(vec!["tool.invoke".into()])]
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }

        fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
            self.calls.lock().expect("calls").push(call.clone());
            Ok(ToolResult::succeeded(call.tool_call_id, "approved output"))
        }
    }

    #[derive(Clone)]
    struct AllowPolicyProvider;

    impl PolicyProvider for AllowPolicyProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.policy.permission-test",
                "policy",
                "permission-test-policy",
                "0.1.0",
                vec!["policy.evaluate".into()],
            )
        }

        fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
            Ok(PolicyDecision::allow(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                "provider.policy.permission-test",
            ))
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }
    }

    fn agent_manifest() -> AgentManifest {
        AgentManifest::from_json(
            r#"{
              "schema_version": "1",
              "manifest_type": "agent",
              "agent_id": "agent.permission-worker-test",
              "name": "permission-worker-test",
              "display_name": "Permission Worker Test",
              "description": "Agent used to verify durable permission execution.",
              "version": "0.1.0",
              "domain": "intelligence",
              "required_capabilities": [
                { "capability_id": "tool.invoke", "min_version": "0.1.0" },
                { "capability_id": "policy.evaluate", "min_version": "0.1.0" }
              ],
              "optional_capabilities": [],
              "event_families": ["agent.tool.*", "agent.policy.*"],
              "owner": { "name": "sdkwork-platform" },
              "status": "candidate"
            }"#,
        )
        .expect("manifest")
    }

    #[tokio::test]
    async fn worker_decrypts_invokes_and_crypto_erases_operation() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runtime = RuntimeBuilder::new("runtime.permission-worker-test", agent_manifest())
            .register_tool_provider(
                "provider.tool.permission-test",
                "0.1.0",
                RecordingToolProvider {
                    calls: calls.clone(),
                },
            )
            .register_policy_provider(
                "provider.policy.permission-test",
                "0.1.0",
                AllowPolicyProvider,
            )
            .bootstrap()
            .expect("runtime")
            .runtime;
        let key = sdkwork_utils_rust::base64url_encode(&[11_u8; 32]);
        let config = Arc::new(ServerConfig {
            approval_payload_encryption_key: Some(key),
            task_worker_max_concurrency: 1,
            task_worker_poll_interval_ms: 50,
            task_worker_lease_secs: 10,
            ..ServerConfig::default()
        });
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let mut state = InternalRuntimeApiState::new(persistence.clone(), config.clone())
            .expect("runtime state");
        state.runtime = crate::runtime::RuntimeState::from_agent_runtime_for_test(runtime, &config);
        let state = Arc::new(state);
        let session = persistence
            .create_session(SessionConfig::new("agent.permission-worker-test"))
            .expect("session");
        let now = sdkwork_agent_database::runtime_now_timestamp();
        let permission_id = "policy-request.tool-call.permission-test".to_string();
        let task_id = "task.permission-worker-test".to_string();
        let run_id = "run.permission-worker-test".to_string();
        let step_id = "step.permission-worker-test".to_string();
        let tool_call_id = "tool-call.permission-test".to_string();
        let aad = ApprovalPayloadContext {
            permission_request_id: &permission_id,
            session_id: &session.session_id,
            task_id: &task_id,
            run_id: &run_id,
            step_id: &step_id,
            tool_call_id: &tool_call_id,
            provider_id: "provider.tool.permission-test",
            descriptor_revision: "0.1.0",
            policy_revision: "0.1.0",
        }
        .to_aad()
        .expect("aad");
        let sealed = state
            .approval_payload_vault
            .as_ref()
            .expect("vault")
            .seal(r#"{"approved":true}"#, &aad)
            .expect("sealed");
        let task = TaskRow {
            task_id: task_id.clone(),
            session_id: session.session_id.clone(),
            instruction: "Execute approved tool".into(),
            state: "accepted".into(),
            created_at: now.clone(),
            updated_at: Some(now.clone()),
        };
        let run = RunRow {
            run_id: run_id.clone(),
            task_id: task_id.clone(),
            session_id: session.session_id.clone(),
            attempt: 1,
            state: RunState::AwaitingPermission,
            next_attempt_at: None,
            lease_owner: None,
            lease_expires_at: None,
            fencing_token: 0,
            cancel_requested_at: None,
            started_at: None,
            finished_at: None,
            error_kind: None,
            error_code: None,
            error_detail: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let step = StepRow {
            step_id: step_id.clone(),
            run_id: run_id.clone(),
            sequence_no: 0,
            action_kind: ActionKind::ToolCall,
            state: StepState::AwaitingPermission,
            provider_id: Some("provider.tool.permission-test".into()),
            descriptor_revision: Some("0.1.0".into()),
            policy_revision: Some("0.1.0".into()),
            causation_step_id: None,
            idempotency_key_hash: None,
            result_json: None,
            error_kind: None,
            error_code: None,
            error_detail: None,
            started_at: None,
            finished_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let permission = PermissionRow {
            permission_request_id: permission_id.clone(),
            session_id: Some(session.session_id.clone()),
            category: "tool.invoke".into(),
            resource: "tool.permission-test".into(),
            side_effect_level: "side_effectful".into(),
            reason: "approval required".into(),
            status: "pending".into(),
            owner_tenant_id: session.owner_tenant_id.clone(),
            owner_user_ref: session.owner_user_ref.clone(),
            created_at: now.clone(),
            updated_at: None,
        };
        let operation = PermissionOperationRow {
            permission_request_id: permission_id.clone(),
            run_id,
            step_id,
            tool_call_id: tool_call_id.clone(),
            provider_id: "provider.tool.permission-test".into(),
            descriptor_revision: "0.1.0".into(),
            policy_revision: "0.1.0".into(),
            payload_kind: PermissionPayloadKind::Ciphertext,
            payload_ref: sealed.payload_ref,
            payload_digest: sealed.payload_digest,
            encryption_key_id: Some(sealed.encryption_key_id),
            state: PermissionOperationState::Pending,
            expires_at: format_runtime_timestamp(Utc::now() + ChronoDuration::minutes(5)),
            lease_owner: None,
            lease_expires_at: None,
            fencing_token: 0,
            result_json: None,
            error_kind: None,
            error_code: None,
            error_detail: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let requested = EventRow {
            event_id: "event.permission-worker-test.requested".into(),
            session_id: Some(session.session_id),
            event_type: "permission.requested".into(),
            severity: "warn".into(),
            payload: None,
            created_at: now.clone(),
        };
        persistence
            .create_permission_execution(&permission, &task, &run, &step, &operation, &requested)
            .expect("permission execution");
        persistence
            .decide_permission_operation(
                &permission_id,
                "allow",
                &now,
                &EventRow {
                    event_id: "event.permission-worker-test.allowed".into(),
                    event_type: "permission.allowed".into(),
                    ..requested
                },
            )
            .expect("allow");

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let worker = PermissionOperationWorker::spawn(state, config, shutdown_rx);
        let deadline = time::Instant::now() + Duration::from_secs(3);
        loop {
            let stored = persistence
                .load_permission_operation(&permission_id)
                .expect("load")
                .expect("operation");
            if stored.state == PermissionOperationState::Completed {
                assert!(stored.payload_ref.is_empty());
                break;
            }
            assert!(
                time::Instant::now() < deadline,
                "worker completion timed out"
            );
            time::sleep(Duration::from_millis(20)).await;
        }
        shutdown_tx.send(true).expect("shutdown");
        worker.join().await;
        let calls = calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_call_id, tool_call_id);
        assert_eq!(calls[0].arguments, r#"{"approved":true}"#);
    }
}
