use sdkwork_agent_kernel::{AgentMessage, AgentSession, ModelResponse, ToolDescriptor, ToolResult};

/// Configuration for creating a new agent session through the bridge
#[derive(Debug, Clone)]
pub struct BridgeSessionConfig {
    pub agent_id: String,
    pub tenant_id: u64,
    pub user_ref: Option<String>,
    pub model: Option<String>,
    pub instructions: Option<String>,
    pub cwd: Option<String>,
    pub metadata: Vec<(String, String)>,
}

/// Response from sending a message through the bridge
#[derive(Debug, Clone)]
pub struct BridgeMessageResponse {
    pub session_id: String,
    pub message: AgentMessage,
    pub model_response: Option<ModelResponse>,
    pub tool_results: Vec<ToolResult>,
    pub events: Vec<BridgeEvent>,
}

/// Event emitted during bridge operations
#[derive(Debug, Clone)]
pub struct BridgeEvent {
    pub event_type: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub payload: String,
    pub severity: BridgeEventSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeEventSeverity {
    Debug,
    Info,
    Warn,
    Error,
}

/// Result of a model invocation through the bridge
#[derive(Debug, Clone)]
pub struct BridgeModelResult {
    pub response: ModelResponse,
    pub tool_calls: Vec<BridgeToolCall>,
    pub events: Vec<BridgeEvent>,
}

/// A tool call extracted from model response
#[derive(Debug, Clone)]
pub struct BridgeToolCall {
    pub call_id: String,
    pub tool_id: String,
    pub tool_name: String,
    pub arguments: String,
}

/// Result of executing a tool through the bridge
#[derive(Debug, Clone)]
pub struct BridgeToolResult {
    pub call_id: String,
    pub result: ToolResult,
    pub events: Vec<BridgeEvent>,
}

/// Snapshot of bridge state for UI consumption
#[derive(Debug, Clone)]
pub struct BridgeSnapshot {
    pub session_id: String,
    pub session: AgentSession,
    pub messages: Vec<AgentMessage>,
    pub available_tools: Vec<ToolDescriptor>,
    pub pending_tool_calls: Vec<BridgeToolCall>,
    pub events: Vec<BridgeEvent>,
}

/// Generate a simple unique ID (nanos-based hex)
pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}
