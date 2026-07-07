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
        let mut locks = self.message_turn_locks()?;
        Ok(locks
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    fn message_turn_locks(
        &self,
    ) -> KernelResult<std::sync::MutexGuard<'_, HashMap<String, Arc<Mutex<()>>>>> {
        self.session_turn_locks.lock().map_err(|error| {
            sdkwork_agent_kernel::KernelError::Internal {
                message: format!("runtime session turn lock registry poisoned: {error}"),
            }
        })
    }

    fn remove_session_turn_lock(&self, session_id: &str) -> KernelResult<()> {
        self.message_turn_locks()?.remove(session_id);
        Ok(())
    }

    /// Close a registered session and release all server-owned transient state for it.
    pub fn close_session(
        &self,
        session_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        let turn_lock = self.session_turn_lock(session_id)?;
        let _turn_guard =
            turn_lock
                .lock()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime session turn lock poisoned: {error}"),
                })?;
        let close_result = self.with_bridge_write(|bridge| {
            let closed = bridge.close_session(session_id)?;
            bridge.remove_session(session_id);
            Ok(closed)
        });
        let cleanup_result = self.remove_session_turn_lock(session_id);
        let closed = close_result?;
        cleanup_result?;
        Ok(closed)
    }

    /// Release transient runtime state after a persisted session is deleted.
    pub fn release_session_state(&self, session_id: &str) -> KernelResult<()> {
        let turn_lock = self.session_turn_lock(session_id)?;
        let _turn_guard =
            turn_lock
                .lock()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime session turn lock poisoned: {error}"),
                })?;
        let release_result = self.with_bridge_write(|bridge| {
            bridge.remove_session(session_id);
            Ok(())
        });
        let cleanup_result = self.remove_session_turn_lock(session_id);
        release_result?;
        cleanup_result
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
        let turn_lock = self.session_turn_lock(session_id)?;
        let _turn_guard =
            turn_lock
                .lock()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime session turn lock poisoned: {error}"),
                })?;
        let (model_bridge, request, provider_id, user_payload_len) =
            self.with_bridge_write(|bridge| bridge.prepare_send_message_turn(session_id, content))?;
        let model_result = model_bridge.invoke(&request, provider_id.as_deref())?;
        self.with_bridge_write(|bridge| {
            bridge.complete_user_message_turn(session_id, user_payload_len, model_result)
        })
    }

    /// Stream a user message turn (write — uses exclusive lock).
    pub fn stream_message(
        &self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(String, Vec<sdkwork_agent_kernel::ModelStreamChunk>)> {
        let turn_lock = self.session_turn_lock(session_id)?;
        let _turn_guard =
            turn_lock
                .lock()
                .map_err(|error| sdkwork_agent_kernel::KernelError::Internal {
                    message: format!("runtime session turn lock poisoned: {error}"),
                })?;
        let (model_bridge, request, provider_id, user_payload_len) =
            self.with_bridge_write(|bridge| {
                bridge.prepare_stream_message_turn(session_id, content, model_override)
            })?;
        let chunks = model_bridge.stream(&request, provider_id.as_deref())?;
        self.with_bridge_write(|bridge| {
            bridge.complete_stream_message_turn(session_id, user_payload_len, chunks)
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
    use std::time::Duration;

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
        let _turn_lock = state
            .session_turn_lock("session.closed")
            .expect("turn lock created");
        assert_eq!(session_turn_lock_count(&state), 1);
        assert_eq!(runtime_bridge_session_count(&state), 1);

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
        let _turn_lock = state
            .session_turn_lock("session.deleted")
            .expect("turn lock created");
        assert_eq!(session_turn_lock_count(&state), 1);
        assert_eq!(runtime_bridge_session_count(&state), 1);

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
    }
}
