//! Bounded durable task executor.

use crate::api::internal_runtime::InternalRuntimeApiState;
use crate::config::ServerConfig;
use crate::metrics::DurableWorkerKind;
use chrono::{Duration as ChronoDuration, Utc};
use sdkwork_agent_database::{format_runtime_timestamp, ClaimedRun, EventRow, MessageRow, TaskRow};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing::{info, warn};

const MAX_TASK_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_ERROR_DETAIL_CHARS: usize = 1024;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct TaskExecutionWorker {
    task: Option<JoinHandle<()>>,
}

impl TaskExecutionWorker {
    pub fn spawn(
        state: Arc<InternalRuntimeApiState>,
        config: Arc<ServerConfig>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        let task = tokio::spawn(async move {
            let mut workers = JoinSet::new();
            for worker_index in 0..config.task_worker_max_concurrency {
                workers.spawn(run_worker_loop(
                    format!("task-worker-{}-{worker_index}", std::process::id()),
                    state.clone(),
                    config.clone(),
                    shutdown.clone(),
                ));
            }
            while let Some(result) = workers.join_next().await {
                if let Err(error) = result {
                    warn!(error = %error, "durable task worker terminated unexpectedly");
                }
            }
        });
        Self { task: Some(task) }
    }

    pub async fn join(mut self) {
        if let Some(mut task) = self.task.take() {
            if time::timeout(SHUTDOWN_TIMEOUT, &mut task).await.is_err() {
                warn!("durable task workers did not stop before the shutdown deadline");
                task.abort();
                let _ = task.await;
            }
        }
    }
}

impl Drop for TaskExecutionWorker {
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
                match claim_run(&state, &worker_id, config.task_worker_lease_secs).await {
                    Ok(Some(claim)) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Task,
                            "claimed",
                            1,
                        );
                        execute_claim(
                            state.clone(),
                            config.clone(),
                            shutdown.clone(),
                            claim,
                        ).await;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Task,
                            "claim_error",
                            1,
                        );
                        warn!(worker_id, error = %error, "durable task claim failed");
                    }
                }
            }
        }
    }
}

async fn claim_run(
    state: &InternalRuntimeApiState,
    worker_id: &str,
    lease_secs: u64,
) -> Result<Option<ClaimedRun>, String> {
    let now = Utc::now();
    let lease_seconds = i64::try_from(lease_secs)
        .map_err(|_| "task worker lease duration exceeds i64".to_string())?;
    let now = format_runtime_timestamp(now);
    let lease_expires_at =
        format_runtime_timestamp(Utc::now() + ChronoDuration::seconds(lease_seconds));
    let worker_id = worker_id.to_string();
    state
        .persist(move |persistence| {
            persistence.claim_ready_run(&worker_id, &now, &lease_expires_at)
        })
        .await
}

async fn execute_claim(
    state: Arc<InternalRuntimeApiState>,
    config: Arc<ServerConfig>,
    mut shutdown: watch::Receiver<bool>,
    claim: ClaimedRun,
) {
    let _active_guard = state
        .runtime
        .begin_durable_worker_operation(DurableWorkerKind::Task);
    let trace_id = sdkwork_utils_rust::uuid();
    let task_id = claim.run.task_id.clone();
    let session_id = claim.run.session_id.clone();
    let loaded = state
        .persist(move |persistence| {
            let task = persistence.get_task(&task_id)?;
            let session = persistence.get_session(&session_id)?;
            Ok((task, session))
        })
        .await;
    let (task, session) = match loaded {
        Ok(rows) => rows,
        Err(error) => {
            fail_claim(&state, &claim, "persistence_error", Some("50001"), &error).await;
            return;
        }
    };

    let started_at = sdkwork_agent_database::runtime_now_timestamp();
    let started_event = run_event(&claim, "task.started", "info", &started_at);
    let start_claim = claim.clone();
    let start_time = started_at.clone();
    if let Err(error) = state
        .persist(move |persistence| {
            persistence.start_claimed_run(&start_claim, &start_time, &started_event)
        })
        .await
    {
        warn!(run_id = %claim.run.run_id, error = %error, "claimed run could not enter executing state");
        return;
    }

    let admission = match state.runtime.acquire_provider_admission().await {
        Ok(lease) => lease,
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

    let runtime = state.runtime.clone();
    let execution_session_id = claim.run.session_id.clone();
    let instruction = task.instruction.clone();
    let operation = runtime.run_provider_admitted(admission, move |runtime| {
        runtime.invoke_task_instruction_for_session(&execution_session_id, instruction)
    });
    tokio::pin!(operation);
    let renewal_interval = Duration::from_secs((config.task_worker_lease_secs / 3).max(1));
    let mut renewals = time::interval(renewal_interval);
    renewals.set_missed_tick_behavior(MissedTickBehavior::Skip);
    renewals.tick().await;

    let result = loop {
        tokio::select! {
            result = &mut operation => break Some(result),
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            _ = renewals.tick() => {
                match renew_claim(&state, &claim, config.task_worker_lease_secs).await {
                    Ok(true) => {}
                    Ok(false) => {
                        state.runtime.record_durable_worker_outcome(
                            DurableWorkerKind::Task,
                            "lease_lost",
                            1,
                        );
                        warn!(run_id = %claim.run.run_id, "durable task lease ownership was lost");
                        return;
                    }
                    Err(error) => warn!(run_id = %claim.run.run_id, error = %error, "durable task lease renewal failed"),
                }
            }
        }
    };
    let Some(result) = result else {
        return;
    };
    let model_result = match result {
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
    if !model_result.tool_calls.is_empty() {
        fail_claim(
            &state,
            &claim,
            "execution_requires_tool_steps",
            Some("60005"),
            "model requested tool execution that requires planned durable steps",
        )
        .await;
        return;
    }
    let assistant_content = model_result.response.messages.concat();
    if assistant_content.len() > MAX_TASK_OUTPUT_BYTES {
        fail_claim(
            &state,
            &claim,
            "resource_exhausted",
            Some("50301"),
            "model output exceeds the durable task output limit",
        )
        .await;
        return;
    }
    complete_claim(&state, &claim, &task, assistant_content).await;
}

async fn renew_claim(
    state: &InternalRuntimeApiState,
    claim: &ClaimedRun,
    lease_secs: u64,
) -> Result<bool, String> {
    let now_value = Utc::now();
    let lease_seconds = i64::try_from(lease_secs)
        .map_err(|_| "task worker lease duration exceeds i64".to_string())?;
    let now = format_runtime_timestamp(now_value);
    let lease_expires_at =
        format_runtime_timestamp(now_value + ChronoDuration::seconds(lease_seconds));
    let run_id = claim.run.run_id.clone();
    let worker_id = claim.run.lease_owner.clone().unwrap_or_default();
    let fencing_token = claim.run.fencing_token;
    state
        .persist(move |persistence| {
            persistence.renew_run_lease(&run_id, &worker_id, fencing_token, &now, &lease_expires_at)
        })
        .await
}

async fn complete_claim(
    state: &InternalRuntimeApiState,
    claim: &ClaimedRun,
    task: &TaskRow,
    assistant_content: String,
) {
    let user_created_at = Utc::now();
    let assistant_created_at = user_created_at + ChronoDuration::nanoseconds(1);
    let user_created_at = format_runtime_timestamp(user_created_at);
    let finished_at = format_runtime_timestamp(assistant_created_at);
    let user_message_id = format!("msg.{}", sdkwork_utils_rust::uuid());
    let assistant_message_id = format!("msg.{}", sdkwork_utils_rust::uuid());
    let messages = vec![
        MessageRow {
            message_id: user_message_id.clone(),
            session_id: claim.run.session_id.clone(),
            role: "user".into(),
            content: task.instruction.clone(),
            created_at: user_created_at,
            metadata_json: Some(format!(r#"{{"runId":"{}"}}"#, claim.run.run_id)),
        },
        MessageRow {
            message_id: assistant_message_id.clone(),
            session_id: claim.run.session_id.clone(),
            role: "assistant".into(),
            content: assistant_content,
            created_at: finished_at.clone(),
            metadata_json: Some(format!(r#"{{"runId":"{}"}}"#, claim.run.run_id)),
        },
    ];
    let result_json = serde_json::json!({
        "userMessageId": user_message_id,
        "assistantMessageId": assistant_message_id,
    })
    .to_string();
    let event = run_event(claim, "task.completed", "info", &finished_at);
    let completed_run_id = claim.run.run_id.clone();
    let completed_task_id = claim.run.task_id.clone();
    let claim = claim.clone();
    if let Err(error) = state
        .persist(move |persistence| {
            persistence.complete_claimed_run_with_messages(
                &claim,
                &messages,
                Some(&result_json),
                &finished_at,
                &event,
            )
        })
        .await
    {
        warn!(run_id = %completed_run_id, error = %error, "durable task completion was rejected");
    } else {
        state
            .runtime
            .record_durable_worker_outcome(DurableWorkerKind::Task, "completed", 1);
        info!(run_id = %completed_run_id, task_id = %completed_task_id, "durable task completed");
    }
}

async fn fail_claim(
    state: &InternalRuntimeApiState,
    claim: &ClaimedRun,
    error_kind: &str,
    error_code: Option<&str>,
    error_detail: &str,
) {
    let finished_at = sdkwork_agent_database::runtime_now_timestamp();
    let event = run_event(claim, "task.failed", "error", &finished_at);
    let failed_run_id = claim.run.run_id.clone();
    let claim = claim.clone();
    let error_kind = error_kind.to_string();
    let error_code = error_code.map(str::to_string);
    let error_detail = error_detail
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect::<String>();
    if let Err(error) = state
        .persist(move |persistence| {
            persistence.fail_claimed_run(
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
        warn!(run_id = %failed_run_id, error = %error, "durable task failure could not be persisted");
    } else {
        state
            .runtime
            .record_durable_worker_outcome(DurableWorkerKind::Task, "failed", 1);
    }
}

fn run_event(claim: &ClaimedRun, event_type: &str, severity: &str, created_at: &str) -> EventRow {
    EventRow {
        event_id: format!("event.{}", sdkwork_utils_rust::uuid()),
        session_id: Some(claim.run.session_id.clone()),
        event_type: event_type.to_string(),
        severity: severity.to_string(),
        payload: Some(
            serde_json::json!({
                "taskId": claim.run.task_id,
                "runId": claim.run.run_id,
                "stepId": claim.step.step_id,
                "attempt": claim.run.attempt,
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
    use sdkwork_agent_database::{ActionKind, RunRow, RunState, StepRow, StepState};
    use sdkwork_agent_session::SessionConfig;

    #[tokio::test]
    async fn worker_executes_and_atomically_persists_a_task() {
        let (persistence, state, config) = {
            let _lock = crate::testing::env::lock();
            let _plugin = crate::testing::env::VarGuard::set(
                crate::runtime_bootstrap::KERNEL_AGENT_PLUGIN_ENV,
                None,
            );
            let config = ServerConfig {
                task_worker_max_concurrency: 1,
                task_worker_poll_interval_ms: 50,
                task_worker_lease_secs: 10,
                ..ServerConfig::default()
            };
            let config = Arc::new(config);
            let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
            let state = Arc::new(
                InternalRuntimeApiState::new(persistence.clone(), config.clone())
                    .expect("runtime state"),
            );
            (persistence, state, config)
        };
        let session = persistence
            .create_session(SessionConfig::new("agent.1"))
            .expect("session");
        let now = sdkwork_agent_database::runtime_now_timestamp();
        let task = TaskRow {
            task_id: "task.worker.test".into(),
            session_id: session.session_id.clone(),
            instruction: "Return a bounded test response".into(),
            state: "accepted".into(),
            created_at: now.clone(),
            updated_at: Some(now.clone()),
        };
        let run = RunRow {
            run_id: "run.worker.test".into(),
            task_id: task.task_id.clone(),
            session_id: session.session_id.clone(),
            attempt: 1,
            state: RunState::Created,
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
            step_id: "step.worker.test".into(),
            run_id: run.run_id.clone(),
            sequence_no: 0,
            action_kind: ActionKind::ModelCall,
            state: StepState::Ready,
            provider_id: None,
            descriptor_revision: None,
            policy_revision: None,
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
        let accepted = EventRow {
            event_id: "event.worker.accepted".into(),
            session_id: Some(session.session_id.clone()),
            event_type: "task.accepted".into(),
            severity: "info".into(),
            payload: None,
            created_at: now,
        };
        persistence
            .create_task_execution(&task, &run, &step, &accepted)
            .expect("execution created");

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let worker = TaskExecutionWorker::spawn(state, config, shutdown_rx);
        time::timeout(Duration::from_secs(5), async {
            loop {
                let state = persistence
                    .load_run(&run.run_id)
                    .expect("run")
                    .expect("present")
                    .state;
                if state == RunState::Completed {
                    break;
                }
                assert_ne!(state, RunState::Failed, "worker task unexpectedly failed");
                time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("worker completion deadline");

        let messages = persistence
            .get_messages(&session.session_id, Some(10), Some(0))
            .expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        let _ = shutdown_tx.send(true);
        worker.join().await;
    }
}
