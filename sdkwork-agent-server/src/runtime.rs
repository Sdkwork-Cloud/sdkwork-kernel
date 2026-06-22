use sdkwork_agent_api_bridge::AgentRuntimeBridge;
use sdkwork_agent_kernel::{AgentRuntime, KernelResult, ModelRequest};
use std::sync::{Arc, Mutex};

use crate::config::ServerConfig;
use crate::runtime_bootstrap::bootstrap_agent_runtime;

/// Shared agent runtime bridge wired into HTTP handlers.
#[derive(Clone)]
pub struct RuntimeState {
    bridge: Arc<Mutex<AgentRuntimeBridge>>,
    agent_runtime: Arc<AgentRuntime>,
    allow_mock_fallback: bool,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self::for_config(&ServerConfig::default())
    }

    pub fn for_config(config: &ServerConfig) -> Self {
        let allow_mock_fallback = config.allow_mock_provider_fallback();
        let agent_runtime = Arc::new(
            bootstrap_agent_runtime().unwrap_or_else(|error| {
                panic!("agent runtime bootstrap failed: {error}");
            }),
        );
        let bridge = Arc::new(Mutex::new(AgentRuntimeBridge::with_agent_runtime(
            agent_runtime.clone(),
            allow_mock_fallback,
        )));

        Self {
            bridge,
            agent_runtime,
            allow_mock_fallback,
        }
    }

    pub fn agent_runtime(&self) -> &AgentRuntime {
        &self.agent_runtime
    }

    pub fn allow_mock_fallback(&self) -> bool {
        self.allow_mock_fallback
    }

    pub fn with_bridge<T>(
        &self,
        operation: impl FnOnce(&mut AgentRuntimeBridge) -> KernelResult<T>,
    ) -> KernelResult<T> {
        let mut bridge = self.bridge.lock().map_err(|error| {
            sdkwork_agent_kernel::KernelError::Internal {
                message: format!("runtime bridge lock poisoned: {error}"),
            }
        })?;
        operation(&mut bridge)
    }

    pub fn invoke_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeModelResult> {
        self.with_bridge(|bridge| bridge.invoke_model(request))
    }

    pub fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeMessageResponse> {
        self.with_bridge(|bridge| bridge.send_message(session_id, content))
    }

    pub fn stream_message(
        &self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(String, Vec<sdkwork_agent_kernel::ModelStreamChunk>)> {
        self.with_bridge(|bridge| bridge.stream_message(session_id, content, model_override))
    }

    pub fn list_tools(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ToolDescriptor>> {
        self.with_bridge(|bridge| bridge.list_tools())
    }

    pub fn execute_tool(
        &self,
        session_id: &str,
        tool_name: &str,
        arguments: &str,
    ) -> KernelResult<sdkwork_agent_api_bridge::BridgeToolResult> {
        self.with_bridge(|bridge| bridge.execute_tool(session_id, tool_name, arguments))
    }

    pub fn register_session(
        &self,
        session_id: &str,
        config: sdkwork_agent_api_bridge::BridgeSessionConfig,
    ) -> KernelResult<sdkwork_agent_kernel::AgentSession> {
        self.with_bridge(|bridge| bridge.register_session(session_id, config))
    }

    pub fn list_models(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ModelDescriptor>> {
        self.with_bridge(|bridge| bridge.list_models())
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}
