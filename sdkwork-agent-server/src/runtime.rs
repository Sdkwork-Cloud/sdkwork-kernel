use sdkwork_agent_api_bridge::{AgentRuntimeBridge, ModelBridge};
use sdkwork_agent_kernel::{AgentRuntime, KernelResult, ModelRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::backend_health_worker::BackendHealthWorker;
use crate::config::ServerConfig;
use crate::runtime_bootstrap::bootstrap_agent_runtime;

/// Shared agent runtime bridge wired into HTTP handlers.
///
/// Uses `RwLock` instead of `Mutex` to allow concurrent local bridge reads
/// (list_models, list_tools, request preparation) while still serializing write
/// operations (register_session, send_message, execute_tool) that mutate
/// internal bridge state. Model provider calls clone the model bridge or build
/// the session request under the bridge lock, then execute outside the lock.
/// Message turns retain a per-session mutex so two turns in the same session do
/// not interleave user and assistant messages while other sessions can proceed.
#[derive(Clone)]
pub struct RuntimeState {
    bridge: Arc<RwLock<AgentRuntimeBridge>>,
    agent_runtime: Arc<AgentRuntime>,
    allow_mock_fallback: bool,
    backend_health: Arc<BackendHealthWorker>,
    session_turn_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl RuntimeState {
    pub fn try_new() -> KernelResult<Self> {
        Self::try_for_config(&ServerConfig::default())
    }

    pub fn try_for_config(config: &ServerConfig) -> KernelResult<Self> {
        let allow_mock_fallback = config.allow_mock_provider_fallback();
        let agent_runtime = Arc::new(bootstrap_agent_runtime()?);
        let backend_health = Arc::new(BackendHealthWorker::spawn_default(agent_runtime.clone()));
        let bridge = Arc::new(RwLock::new(AgentRuntimeBridge::with_agent_runtime(
            agent_runtime.clone(),
            allow_mock_fallback,
        )));

        Ok(Self {
            bridge,
            agent_runtime,
            allow_mock_fallback,
            backend_health,
            session_turn_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn agent_runtime(&self) -> &AgentRuntime {
        &self.agent_runtime
    }

    pub fn allow_mock_fallback(&self) -> bool {
        self.allow_mock_fallback
    }

    pub fn backend_health_monitor(
        &self,
    ) -> Arc<std::sync::RwLock<sdkwork_agent_kernel::BackendHealthMonitor>> {
        self.backend_health.monitor()
    }

    /// Execute a write operation on the bridge with an exclusive lock.
    fn with_bridge_write<T>(
        &self,
        operation: impl FnOnce(&mut AgentRuntimeBridge) -> KernelResult<T>,
    ) -> KernelResult<T> {
        let mut bridge =
            self.bridge
                .write()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime bridge lock poisoned: {error}"),
                })?;
        operation(&mut bridge)
    }

    /// Execute a read operation on the bridge with a shared lock.
    fn with_bridge_read<T>(
        &self,
        operation: impl FnOnce(&AgentRuntimeBridge) -> KernelResult<T>,
    ) -> KernelResult<T> {
        let bridge =
            self.bridge
                .read()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime bridge lock poisoned: {error}"),
                })?;
        operation(&bridge)
    }

    fn clone_model_bridge(&self) -> KernelResult<ModelBridge> {
        self.with_bridge_read(|bridge| Ok(bridge.model_bridge().clone()))
    }

    fn prepare_model_request_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
    ) -> KernelResult<(ModelBridge, ModelRequest, Option<String>)> {
        self.with_bridge_read(|bridge| {
            let model_bridge = bridge.model_bridge().clone();
            let (request, provider_id) = bridge.prepare_model_request_for_session(
                session_id,
                model_id,
                override_messages,
            )?;
            Ok((model_bridge, request, provider_id))
        })
    }

    fn session_turn_lock(&self, session_id: &str) -> KernelResult<Arc<Mutex<()>>> {
        let mut locks = self.session_turn_lock_registry()?;
        Ok(locks
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    fn session_turn_lock_registry(
        &self,
    ) -> KernelResult<std::sync::MutexGuard<'_, HashMap<String, Arc<Mutex<()>>>>> {
        self.session_turn_locks.lock().map_err(|error| {
            sdkwork_agent_kernel::KernelError::Internal {
                message: format!("runtime session turn lock registry poisoned: {error}"),
            }
        })
    }

    fn cleanup_session_turn_lock(
        &self,
        session_id: &str,
        turn_lock: &Arc<Mutex<()>>,
    ) -> KernelResult<()> {
        let mut locks = self.session_turn_lock_registry()?;
        // The registry lock prevents new acquisitions while this count is checked.
        // Two references means only the registry and this completed operation remain.
        let can_remove = locks.get(session_id).is_some_and(|registered_lock| {
            Arc::ptr_eq(registered_lock, turn_lock) && Arc::strong_count(turn_lock) == 2
        });
        if can_remove {
            locks.remove(session_id);
        }
        Ok(())
    }

    fn with_session_turn_lock<T>(
        &self,
        session_id: &str,
        operation: impl FnOnce() -> KernelResult<T>,
    ) -> KernelResult<T> {
        let turn_lock = self.session_turn_lock(session_id)?;
        let operation_result = match turn_lock.lock() {
            Ok(turn_guard) => {
                let result = operation();
                drop(turn_guard);
                result
            }
            Err(error) => Err(sdkwork_agent_kernel::KernelError::Internal {
                message: format!("runtime session turn lock poisoned: {error}"),
            }),
        };
        // Drop the per-session guard before taking the registry mutex. A waiter may
        // now proceed, but its Arc reference prevents premature registry removal.
        let cleanup_result = self.cleanup_session_turn_lock(session_id, &turn_lock);

        match operation_result {
            Err(error) => Err(error),
            Ok(value) => {
                cleanup_result?;
                Ok(value)
            }
        }
    }

    /// Close a registered session and release all server-owned transient state for it.
    pub fn close_session(
        &self,
        session_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        self.with_session_turn_lock(session_id, || {
            self.with_bridge_write(|bridge| {
                let closed = bridge.close_session(session_id)?;
                bridge.remove_session(session_id);
                Ok(closed)
            })
        })
    }

    /// Release transient runtime state after a persisted session is deleted.
    pub fn release_session_state(&self, session_id: &str) -> KernelResult<()> {
        self.with_session_turn_lock(session_id, || {
            self.with_bridge_write(|bridge| {
                bridge.remove_session(session_id);
                Ok(())
            })
        })
    }

    /// Invoke the model directly without holding the bridge lock during provider execution.
    pub fn invoke_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeModelResult> {
        let model_bridge = self.clone_model_bridge()?;
        model_bridge.invoke(&request, None)
    }

    /// Invoke the model for a session using structured history (read-only).
    pub fn invoke_model_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeModelResult> {
        let (model_bridge, request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, None)?;
        model_bridge.invoke(&request, provider_id.as_deref())
    }

    /// Send a user message (write — uses exclusive lock).
    pub fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeMessageResponse> {
        self.with_session_turn_lock(session_id, || {
            let (model_bridge, request, provider_id, user_message) =
                self.with_bridge_write(|bridge| {
                    bridge.prepare_send_message_turn(session_id, content)
                })?;
            let model_result = model_bridge.invoke(&request, provider_id.as_deref())?;
            self.with_bridge_write(|bridge| {
                bridge.complete_user_message_turn(session_id, user_message, model_result)
            })
        })
    }

    /// Stream a user message turn (write — uses exclusive lock).
    pub fn stream_message(
        &self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(String, Vec<sdkwork_agent_kernel::ModelStreamChunk>)> {
        self.with_session_turn_lock(session_id, || {
            let (model_bridge, request, provider_id, user_message) =
                self.with_bridge_write(|bridge| {
                    bridge.prepare_stream_message_turn(session_id, content, model_override)
                })?;
            let chunks = model_bridge.stream(&request, provider_id.as_deref())?;
            self.with_bridge_write(|bridge| {
                bridge.complete_stream_message_turn(session_id, user_message, chunks)
            })
        })
    }

    /// Stream a model response directly without holding the bridge lock during provider execution.
    pub fn stream_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        let model_bridge = self.clone_model_bridge()?;
        model_bridge.stream(&request, None)
    }

    /// Stream model output for a session (read-only).
    pub fn stream_model_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        let (model_bridge, request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, override_messages)?;
        model_bridge.stream(&request, provider_id.as_deref())
    }

    /// Stream model output incrementally for a session (read-only).
    pub fn stream_model_for_session_into(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
        sink: &mut dyn sdkwork_agent_kernel::ModelStreamSink,
    ) -> KernelResult<()> {
        let (model_bridge, request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, override_messages)?;
        model_bridge.stream_into(&request, provider_id.as_deref(), sink)
    }

    /// Cancel an in-flight model invocation without holding the bridge lock.
    pub fn cancel_model(
        &self,
        model_request_id: &str,
        model_provider_id: Option<&str>,
    ) -> KernelResult<sdkwork_agent_kernel::ModelResponse> {
        let model_bridge = self.clone_model_bridge()?;
        model_bridge.cancel(model_request_id, model_provider_id)
    }

    /// List available tools (read-only — uses shared lock).
    pub fn list_tools(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ToolDescriptor>> {
        self.with_bridge_read(|bridge| bridge.list_tools())
    }

    /// Execute a tool call (write — uses exclusive lock).
    pub fn execute_tool(
        &self,
        session_id: &str,
        tool_name: &str,
        arguments: &str,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeToolResult> {
        self.with_bridge_write(|bridge| bridge.execute_tool(session_id, tool_name, arguments))
    }

    /// Register a session (write — uses exclusive lock).
    pub fn register_session(
        &self,
        session_id: &str,
        config: sdkwork_agent_api_bridge::BridgeSessionConfig,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        self.with_bridge_write(|bridge| bridge.register_session(session_id, config))
    }

    /// Register a persisted session and refresh its bounded message history.
    pub fn register_session_with_history(
        &self,
        session_id: &str,
        config: sdkwork_agent_api_bridge::BridgeSessionConfig,
        history: Vec<sdkwork_agent_kernel::AgentMessage>,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        self.with_session_turn_lock(session_id, || {
            self.with_bridge_write(|bridge| {
                bridge.register_session_with_history(session_id, config, history)
            })
        })
    }

    pub fn register_session_with_history_revision(
        &self,
        session_id: &str,
        config: sdkwork_agent_api_bridge::BridgeSessionConfig,
        history_revision: u64,
        history: Vec<sdkwork_agent_kernel::AgentMessage>,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        self.with_session_turn_lock(session_id, || {
            self.with_bridge_write(|bridge| {
                bridge.register_session_with_history_revision(
                    session_id,
                    config,
                    history_revision,
                    history,
                )
            })
        })
    }

    /// List registered model descriptors (read-only — uses shared lock).
    pub fn list_models(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ModelDescriptor>> {
        self.with_bridge_read(|bridge| bridge.list_models())
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::try_for_config(&ServerConfig::default())
            .expect("default runtime bootstrap should succeed in tests")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_api_bridge::BridgeSessionConfig;
    use sdkwork_agent_kernel::{
        AgentManifest, HealthMonitorConfig, ModelDescriptor, ModelProvider, ModelRequest,
        ModelResponse, ModelStreamChunk, PolicyDecision, PolicyProvider, PolicyRequest,
        ProviderHealth, ProviderManifest, RuntimeBuilder,
    };
    use std::sync::mpsc;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    struct BlockingModelProvider {
        started_tx: Mutex<Option<mpsc::Sender<()>>>,
        release_rx: Mutex<mpsc::Receiver<()>>,
    }

    impl BlockingModelProvider {
        fn new(started_tx: mpsc::Sender<()>, release_rx: mpsc::Receiver<()>) -> Self {
            Self {
                started_tx: Mutex::new(Some(started_tx)),
                release_rx: Mutex::new(release_rx),
            }
        }
    }

    impl ModelProvider for BlockingModelProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.model.blocking",
                "model",
                "blocking-model",
                "0.1.0",
                vec!["model.chat".to_string(), "model.streaming".to_string()],
            )
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }

        fn list_models(&self) -> Vec<ModelDescriptor> {
            vec![ModelDescriptor::new(
                "model.blocking",
                "provider.model.blocking",
                "Blocking Model",
                "test",
            )
            .with_input_mode("text")]
        }

        fn invoke(
            &self,
            request: ModelRequest,
        ) -> sdkwork_agent_kernel::KernelResult<ModelResponse> {
            if let Some(sender) = self.started_tx.lock().expect("started lock").take() {
                let _ = sender.send(());
            }
            self.release_rx
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(2))
                .expect("test released provider");
            Ok(ModelResponse::text(
                &request.model_request_id,
                "provider.model.blocking",
                "blocked response",
            ))
        }

        fn stream(
            &self,
            request: ModelRequest,
        ) -> sdkwork_agent_kernel::KernelResult<Vec<ModelStreamChunk>> {
            if let Some(sender) = self.started_tx.lock().expect("started lock").take() {
                let _ = sender.send(());
            }
            self.release_rx
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(2))
                .expect("test released provider");
            Ok(vec![ModelStreamChunk::output(
                &request.model_request_id,
                0,
                "blocked stream",
            )])
        }
    }

    struct AllowPolicyProvider;

    impl PolicyProvider for AllowPolicyProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new(
                "provider.policy.allow",
                "policy",
                "allow-policy",
                "0.1.0",
                vec!["policy.evaluate".to_string()],
            )
        }

        fn evaluate(
            &self,
            request: PolicyRequest,
        ) -> sdkwork_agent_kernel::KernelResult<PolicyDecision> {
            Ok(PolicyDecision::allow(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                "provider.policy.allow",
            ))
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }
    }

    fn test_session_config(model: Option<&str>) -> BridgeSessionConfig {
        BridgeSessionConfig {
            agent_id: "agent.runtime-lock-test".to_string(),
            tenant_id: 100_001,
            user_ref: Some("user.lock-test".to_string()),
            model: model.map(str::to_string),
            instructions: None,
            cwd: None,
            metadata: vec![(
                "modelProvider".to_string(),
                "provider.model.blocking".to_string(),
            )],
        }
    }

    fn test_agent_manifest() -> AgentManifest {
        AgentManifest::from_json(
            r#"{
              "schema_version": "1",
              "manifest_type": "agent",
              "agent_id": "agent.runtime-lock-test",
              "name": "runtime-lock-test",
              "display_name": "Runtime Lock Test",
              "description": "Agent used to verify runtime lock behavior.",
              "version": "0.1.0",
              "domain": "intelligence",
              "required_capabilities": [
                { "capability_id": "model.chat", "min_version": "0.1.0" }
              ],
              "optional_capabilities": [],
              "event_families": ["agent.model.*"],
              "owner": { "name": "sdkwork-platform" },
              "status": "candidate"
            }"#,
        )
        .expect("agent manifest parses")
    }

    fn runtime_state_with_blocking_model(
        started_tx: mpsc::Sender<()>,
        release_rx: mpsc::Receiver<()>,
    ) -> RuntimeState {
        let provider = BlockingModelProvider::new(started_tx, release_rx);
        let agent_runtime = Arc::new(
            RuntimeBuilder::new("runtime.lock-test", test_agent_manifest())
                .register_model_provider("provider.model.blocking", "0.1.0", provider)
                .register_policy_provider("provider.policy.allow", "0.1.0", AllowPolicyProvider)
                .bootstrap()
                .expect("runtime bootstraps")
                .runtime,
        );
        let backend_health = Arc::new(BackendHealthWorker::spawn(
            agent_runtime.clone(),
            HealthMonitorConfig::default().with_check_interval(Duration::from_millis(10)),
        ));
        let bridge = Arc::new(RwLock::new(AgentRuntimeBridge::with_agent_runtime(
            agent_runtime.clone(),
            false,
        )));

        RuntimeState {
            bridge,
            agent_runtime,
            allow_mock_fallback: false,
            backend_health,
            session_turn_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn session_turn_lock_count(state: &RuntimeState) -> usize {
        state
            .session_turn_locks
            .lock()
            .expect("turn lock registry")
            .len()
    }

    fn wait_for_session_turn_lock_references(
        state: &RuntimeState,
        session_id: &str,
        minimum_references: usize,
    ) {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let reference_count = state
                .session_turn_locks
                .lock()
                .expect("turn lock registry")
                .get(session_id)
                .map(Arc::strong_count)
                .unwrap_or_default();
            if reference_count >= minimum_references {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "session turn lock did not acquire the expected waiter"
            );
            std::thread::yield_now();
        }
    }

    fn runtime_bridge_session_count(state: &RuntimeState) -> usize {
        state
            .with_bridge_read(|bridge| bridge.list_sessions().map(|sessions| sessions.len()))
            .expect("bridge sessions counted")
    }

    #[test]
    fn close_session_releases_session_turn_lock() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        state
            .register_session(
                "session.closed",
                test_session_config(Some("model.blocking")),
            )
            .expect("session registered");
        let turn_lock = state
            .session_turn_lock("session.closed")
            .expect("turn lock created");
        assert_eq!(session_turn_lock_count(&state), 1);
        assert_eq!(runtime_bridge_session_count(&state), 1);
        drop(turn_lock);

        state
            .close_session("session.closed")
            .expect("session closed");

        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "closing a session must release its per-session turn lock"
        );
        assert_eq!(
            runtime_bridge_session_count(&state),
            0,
            "closing a session must release bridge-owned session state"
        );
    }

    #[test]
    fn failed_close_session_does_not_leave_session_turn_lock() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);

        assert!(state.close_session("session.missing").is_err());

        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "a failed close must not leave an orphaned per-session turn lock"
        );
    }

    #[test]
    fn invalid_session_flood_does_not_grow_session_turn_lock_registry() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);

        for index in 0..1_024 {
            let session_id = format!("session.missing.{index}");
            state
                .send_message(&session_id, "rejected message")
                .expect_err("unknown session must be rejected");
        }

        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "rejected session identifiers must not accumulate lock registry entries"
        );
    }

    #[test]
    fn cleanup_session_turn_lock_does_not_remove_a_replacement_lock() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        let stale_lock = state
            .session_turn_lock("session.replaced")
            .expect("stale lock created");
        let replacement_lock = Arc::new(Mutex::new(()));

        state
            .session_turn_lock_registry()
            .expect("turn lock registry")
            .insert("session.replaced".to_string(), replacement_lock.clone());
        state
            .cleanup_session_turn_lock("session.replaced", &stale_lock)
            .expect("stale lock cleanup succeeds");

        let registered_lock = state
            .session_turn_lock_registry()
            .expect("turn lock registry")
            .get("session.replaced")
            .cloned()
            .expect("replacement lock remains registered");
        assert!(Arc::ptr_eq(&registered_lock, &replacement_lock));
        drop(registered_lock);
        state
            .cleanup_session_turn_lock("session.replaced", &replacement_lock)
            .expect("replacement lock cleanup succeeds");
        assert_eq!(session_turn_lock_count(&state), 0);
    }

    #[test]
    fn session_turn_lock_serializes_waiters_and_drains_registry() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);

        let (first_entered_tx, first_entered_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();
        let first_state = state.clone();
        let first_thread = std::thread::spawn(move || {
            first_state
                .with_session_turn_lock("session.serialized", || {
                    first_entered_tx.send(()).expect("first turn entered");
                    release_first_rx
                        .recv_timeout(Duration::from_secs(2))
                        .expect("first turn released");
                    Ok(())
                })
                .expect("first turn succeeds");
        });
        first_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("first turn started");

        let (second_ready_tx, second_ready_rx) = mpsc::channel();
        let (second_entered_tx, second_entered_rx) = mpsc::channel();
        let (release_second_tx, release_second_rx) = mpsc::channel();
        let second_state = state.clone();
        let second_thread = std::thread::spawn(move || {
            second_ready_tx.send(()).expect("second turn ready");
            second_state
                .with_session_turn_lock("session.serialized", || {
                    second_entered_tx.send(()).expect("second turn entered");
                    release_second_rx
                        .recv_timeout(Duration::from_secs(2))
                        .expect("second turn released");
                    Ok(())
                })
                .expect("second turn succeeds");
        });
        second_ready_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second turn scheduled");
        wait_for_session_turn_lock_references(&state, "session.serialized", 3);
        assert!(
            second_entered_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "a waiter must not overlap the active turn"
        );

        release_first_tx.send(()).expect("first turn released");
        second_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second turn entered after first turn");
        first_thread.join().expect("first turn joins");
        assert_eq!(
            session_turn_lock_count(&state),
            1,
            "the registry must retain the shared lock while a waiter is active"
        );

        let (third_ready_tx, third_ready_rx) = mpsc::channel();
        let (third_entered_tx, third_entered_rx) = mpsc::channel();
        let third_state = state.clone();
        let third_thread = std::thread::spawn(move || {
            third_ready_tx.send(()).expect("third turn ready");
            third_state
                .with_session_turn_lock("session.serialized", || {
                    third_entered_tx.send(()).expect("third turn entered");
                    Ok(())
                })
                .expect("third turn succeeds");
        });
        third_ready_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("third turn scheduled");
        wait_for_session_turn_lock_references(&state, "session.serialized", 3);
        assert!(
            third_entered_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "a later turn must use the registered lock instead of a parallel replacement"
        );

        release_second_tx.send(()).expect("second turn released");
        third_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("third turn entered after second turn");
        second_thread.join().expect("second turn joins");
        third_thread.join().expect("third turn joins");
        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "the final turn must drain the unused lock registry entry"
        );
    }

    #[test]
    fn release_session_state_releases_session_turn_lock() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        state
            .register_session(
                "session.deleted",
                test_session_config(Some("model.blocking")),
            )
            .expect("session registered");
        let turn_lock = state
            .session_turn_lock("session.deleted")
            .expect("turn lock created");
        assert_eq!(session_turn_lock_count(&state), 1);
        assert_eq!(runtime_bridge_session_count(&state), 1);
        drop(turn_lock);

        state
            .release_session_state("session.deleted")
            .expect("session runtime state released");

        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "deleting a persisted session must release server-owned runtime state"
        );
        assert_eq!(
            runtime_bridge_session_count(&state),
            0,
            "deleting a persisted session must release bridge-owned session state"
        );
    }

    #[test]
    fn register_session_with_history_releases_session_turn_lock() {
        let (started_tx, _started_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);

        state
            .register_session_with_history(
                "session.restored",
                test_session_config(Some("model.blocking")),
                Vec::new(),
            )
            .expect("session history restored");

        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "successful history restoration must release its turn lock"
        );
    }

    #[test]
    fn model_invocation_does_not_hold_bridge_lock_while_provider_runs() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        state
            .register_session(
                "session.blocked",
                test_session_config(Some("model.blocking")),
            )
            .expect("initial session registered");

        let invoke_state = state.clone();
        let invoke_thread = std::thread::spawn(move || {
            invoke_state
                .invoke_model_for_session("session.blocked", Some("model.blocking".to_string()))
                .expect("model invocation succeeds");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("provider invocation started");

        let (registered_tx, registered_rx) = mpsc::channel();
        let register_state = state.clone();
        let register_thread = std::thread::spawn(move || {
            let result = register_state.register_session(
                "session.concurrent",
                test_session_config(Some("model.blocking")),
            );
            let _ = registered_tx.send(result.is_ok());
        });

        let registered_before_provider_release = registered_rx
            .recv_timeout(Duration::from_millis(150))
            .is_ok();
        release_tx.send(()).expect("provider released");
        invoke_thread.join().expect("invoke thread joins");
        register_thread.join().expect("register thread joins");

        assert!(
            registered_before_provider_release,
            "runtime bridge lock must not be held while a model provider invocation is running"
        );
    }

    #[test]
    fn send_message_does_not_hold_bridge_lock_while_provider_runs() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        state
            .register_session(
                "session.blocked",
                test_session_config(Some("model.blocking")),
            )
            .expect("initial session registered");

        let send_state = state.clone();
        let send_thread = std::thread::spawn(move || {
            send_state
                .send_message("session.blocked", "hello from user")
                .expect("message turn succeeds");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("provider invocation started");

        let (registered_tx, registered_rx) = mpsc::channel();
        let register_state = state.clone();
        let register_thread = std::thread::spawn(move || {
            let result = register_state.register_session(
                "session.concurrent",
                test_session_config(Some("model.blocking")),
            );
            let _ = registered_tx.send(result.is_ok());
        });

        let registered_before_provider_release = registered_rx
            .recv_timeout(Duration::from_millis(150))
            .is_ok();
        release_tx.send(()).expect("provider released");
        send_thread.join().expect("send thread joins");
        register_thread.join().expect("register thread joins");

        assert!(
            registered_before_provider_release,
            "send_message must not hold the runtime bridge lock while the model provider runs"
        );
        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "a successful message turn must release its session lock"
        );
    }

    #[test]
    fn stream_message_does_not_hold_bridge_lock_while_provider_runs() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let state = runtime_state_with_blocking_model(started_tx, release_rx);
        state
            .register_session(
                "session.blocked",
                test_session_config(Some("model.blocking")),
            )
            .expect("initial session registered");

        let stream_state = state.clone();
        let stream_thread = std::thread::spawn(move || {
            stream_state
                .stream_message("session.blocked", "hello from user", None)
                .expect("stream turn succeeds");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("provider stream started");

        let (registered_tx, registered_rx) = mpsc::channel();
        let register_state = state.clone();
        let register_thread = std::thread::spawn(move || {
            let result = register_state.register_session(
                "session.concurrent",
                test_session_config(Some("model.blocking")),
            );
            let _ = registered_tx.send(result.is_ok());
        });

        let registered_before_provider_release = registered_rx
            .recv_timeout(Duration::from_millis(150))
            .is_ok();
        release_tx.send(()).expect("provider released");
        stream_thread.join().expect("stream thread joins");
        register_thread.join().expect("register thread joins");

        assert!(
            registered_before_provider_release,
            "stream_message must not hold the runtime bridge lock while the model provider runs"
        );
        assert_eq!(
            session_turn_lock_count(&state),
            0,
            "a successful stream turn must release its session lock"
        );
    }
}
