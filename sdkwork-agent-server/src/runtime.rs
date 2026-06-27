use sdkwork_agent_api_bridge::AgentRuntimeBridge;
use sdkwork_agent_kernel::{AgentRuntime, KernelResult, ModelRequest};
use std::sync::{Arc, RwLock};

use crate::config::ServerConfig;
use crate::runtime_bootstrap::bootstrap_agent_runtime;

/// Shared agent runtime bridge wired into HTTP handlers.
///
/// Uses `RwLock` instead of `Mutex` to allow concurrent read operations
/// (list_models, list_tools, invoke_model) while still serializing write
/// operations (register_session, send_message, execute_tool) that mutate
/// internal bridge state.
#[derive(Clone)]
pub struct RuntimeState {
    bridge: Arc<RwLock<AgentRuntimeBridge>>,
    agent_runtime: Arc<AgentRuntime>,
    allow_mock_fallback: bool,
}

impl RuntimeState {
    pub fn try_new() -> KernelResult<Self> {
        Self::try_for_config(&ServerConfig::default())
    }

    pub fn try_for_config(config: &ServerConfig) -> KernelResult<Self> {
        let allow_mock_fallback = config.allow_mock_provider_fallback();
        let agent_runtime = Arc::new(bootstrap_agent_runtime()?);
        let bridge = Arc::new(RwLock::new(AgentRuntimeBridge::with_agent_runtime(
            agent_runtime.clone(),
            allow_mock_fallback,
        )));

        Ok(Self {
            bridge,
            agent_runtime,
            allow_mock_fallback,
        })
    }

    pub fn agent_runtime(&self) -> &AgentRuntime {
        &self.agent_runtime
    }

    pub fn allow_mock_fallback(&self) -> bool {
        self.allow_mock_fallback
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

    /// Invoke the model directly (read-only — uses shared lock).
    pub fn invoke_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeModelResult> {
        self.with_bridge_read(|bridge| bridge.invoke_model(request))
    }

    /// Send a user message (write — uses exclusive lock).
    pub fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeMessageResponse> {
        self.with_bridge_write(|bridge| bridge.send_message(session_id, content))
    }

    /// Stream a user message turn (write — uses exclusive lock).
    pub fn stream_message(
        &self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(String, Vec<sdkwork_agent_kernel::ModelStreamChunk>)> {
        self.with_bridge_write(|bridge| bridge.stream_message(session_id, content, model_override))
    }

    /// Stream a model response directly without conversation context (read-only — uses shared lock).
    pub fn stream_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        self.with_bridge_read(|bridge| bridge.stream_model(request))
    }

    /// Cancel an in-flight model invocation by its model request id (read-only — uses shared lock).
    pub fn cancel_model(
        &self,
        model_request_id: &str,
        model_provider_id: Option<&str>,
    ) -> KernelResult<sdkwork_agent_kernel::ModelResponse> {
        self.with_bridge_read(|bridge| bridge.cancel_model(model_request_id, model_provider_id))
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
