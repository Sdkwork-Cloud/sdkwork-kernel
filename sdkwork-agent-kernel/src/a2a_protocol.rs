//! A2A (Agent-to-Agent) Protocol Adapter for external agent communication.
//!
//! This module implements the Google A2A Protocol standard:
//! - Agent discovery via AgentCard
//! - Agent endpoint resolution
//! - Task delegation and handoff
//! - Result aggregation
//!
//! Reference: https://github.com/google/A2A-Protocol

use std::collections::HashMap;

use crate::{agent_messages_to_text_lines, AgentMessage};

/// A2A Agent Card describing agent capabilities and endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2AAgentCard {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Human-readable name.
    pub name: String,
    /// Agent description.
    pub description: String,
    /// Agent version.
    pub version: String,
    /// Supported capabilities.
    pub capabilities: Vec<A2ACapability>,
    /// Agent endpoints.
    pub endpoints: Vec<A2AEndpoint>,
    /// Authentication requirements.
    pub authentication: A2AAuthentication,
    /// Metadata (key-value pairs).
    pub metadata: HashMap<String, String>,
}

impl A2AAgentCard {
    pub fn new(
        agent_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            name: name.into(),
            description: description.into(),
            version: version.into(),
            capabilities: Vec::new(),
            endpoints: Vec::new(),
            authentication: A2AAuthentication::None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_capability(mut self, capability: A2ACapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_endpoint(mut self, endpoint: A2AEndpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    pub fn with_authentication(mut self, authentication: A2AAuthentication) -> Self {
        self.authentication = authentication;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn get_endpoint(&self, endpoint_type: &str) -> Option<&A2AEndpoint> {
        self.endpoints
            .iter()
            .find(|ep| ep.endpoint_type == endpoint_type)
    }

    pub fn has_capability(&self, capability_id: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.capability_id == capability_id)
    }
}

/// A2A Capability definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2ACapability {
    /// Capability identifier.
    pub capability_id: String,
    /// Capability name.
    pub name: String,
    /// Capability description.
    pub description: String,
    /// Input schema (JSON schema).
    pub input_schema: Option<String>,
    /// Output schema (JSON schema).
    pub output_schema: Option<String>,
    /// Required parameters.
    pub required_params: Vec<String>,
}

impl A2ACapability {
    pub fn new(
        capability_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            name: name.into(),
            description: description.into(),
            input_schema: None,
            output_schema: None,
            required_params: Vec::new(),
        }
    }

    pub fn with_input_schema(mut self, schema: impl Into<String>) -> Self {
        self.input_schema = Some(schema.into());
        self
    }

    pub fn with_output_schema(mut self, schema: impl Into<String>) -> Self {
        self.output_schema = Some(schema.into());
        self
    }

    pub fn with_required_param(mut self, param: impl Into<String>) -> Self {
        self.required_params.push(param.into());
        self
    }
}

/// A2A Endpoint for agent communication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2AEndpoint {
    /// Endpoint type (e.g., "http", "grpc", "websocket").
    pub endpoint_type: String,
    /// Endpoint URL/address.
    pub url: String,
    /// Supported protocols.
    pub protocols: Vec<String>,
    /// Endpoint metadata.
    pub metadata: HashMap<String, String>,
}

impl A2AEndpoint {
    pub fn new(endpoint_type: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            endpoint_type: endpoint_type.into(),
            url: url.into(),
            protocols: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocols.push(protocol.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A2A Authentication requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2AAuthentication {
    /// No authentication required.
    None,
    /// API key authentication.
    ApiKey {
        key_name: String,
        key_location: String,
    },
    /// OAuth2 authentication.
    OAuth2 {
        auth_url: String,
        token_url: String,
        scopes: Vec<String>,
    },
    /// JWT authentication.
    Jwt { issuer: String, audience: String },
    /// Custom authentication.
    Custom {
        auth_type: String,
        params: HashMap<String, String>,
    },
}

/// A2A Task request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2ATaskRequest {
    /// Task identifier.
    pub task_id: String,
    /// Target agent ID.
    pub target_agent_id: String,
    /// Capability to invoke.
    pub capability_id: String,
    /// Structured multimodal conversation input (canonical).
    pub messages: Vec<AgentMessage>,
    /// Capability parameters (scalar key-value hints).
    pub parameters: HashMap<String, String>,
    /// Task context.
    pub context: A2ATaskContext,
    /// Timeout (milliseconds).
    pub timeout_ms: Option<u64>,
}

impl A2ATaskRequest {
    pub fn new(
        task_id: impl Into<String>,
        target_agent_id: impl Into<String>,
        capability_id: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            target_agent_id: target_agent_id.into(),
            capability_id: capability_id.into(),
            messages: Vec::new(),
            parameters: HashMap::new(),
            context: A2ATaskContext::default(),
            timeout_ms: None,
        }
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Legacy alias for scalar capability parameters.
    pub fn with_input(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_parameter(key, value)
    }

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_messages(mut self, messages: Vec<AgentMessage>) -> Self {
        self.messages = messages;
        self
    }

    pub fn validate(&self) -> Result<(), A2AError> {
        if self.task_id.trim().is_empty() {
            return Err(A2AError::InvalidRequest("task_id is required".to_string()));
        }
        if self.target_agent_id.trim().is_empty() {
            return Err(A2AError::InvalidRequest(
                "target_agent_id is required".to_string(),
            ));
        }
        if self.capability_id.trim().is_empty() {
            return Err(A2AError::InvalidRequest(
                "capability_id is required".to_string(),
            ));
        }
        if self.messages.is_empty() && self.parameters.is_empty() {
            return Err(A2AError::InvalidRequest(
                "task requires structured messages or scalar parameters".to_string(),
            ));
        }
        for message in &self.messages {
            message.validate().map_err(|error| {
                A2AError::InvalidRequest(format!("invalid task message: {error}"))
            })?;
        }
        Ok(())
    }

    /// Primary user-visible text extracted from structured messages.
    pub fn primary_user_text(&self) -> Option<String> {
        if self.messages.is_empty() {
            return None;
        }
        let lines = agent_messages_to_text_lines(&self.messages);
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    pub fn with_context(mut self, context: A2ATaskContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// A2A Task context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2ATaskContext {
    /// Session ID.
    pub session_id: Option<String>,
    /// Conversation ID.
    pub conversation_id: Option<String>,
    /// User ID.
    pub user_id: Option<String>,
    /// Trace context for distributed tracing.
    pub trace_id: Option<String>,
    /// Parent task ID (for nested tasks).
    pub parent_task_id: Option<String>,
}

impl Default for A2ATaskContext {
    fn default() -> Self {
        Self {
            session_id: None,
            conversation_id: None,
            user_id: None,
            trace_id: None,
            parent_task_id: None,
        }
    }
}

impl A2ATaskContext {
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_conversation(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_trace(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_parent_task(mut self, parent_task_id: impl Into<String>) -> Self {
        self.parent_task_id = Some(parent_task_id.into());
        self
    }
}

/// A2A Task response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2ATaskResponse {
    /// Task identifier.
    pub task_id: String,
    /// Response status.
    pub status: A2ATaskStatus,
    /// Output data.
    pub output: HashMap<String, String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Execution time (milliseconds).
    pub execution_time_ms: u64,
    /// Metadata.
    pub metadata: HashMap<String, String>,
}

impl A2ATaskResponse {
    pub fn success(
        task_id: impl Into<String>,
        output: HashMap<String, String>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            status: A2ATaskStatus::Completed,
            output,
            error: None,
            execution_time_ms,
            metadata: HashMap::new(),
        }
    }

    pub fn failure(
        task_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            status: A2ATaskStatus::Failed,
            output: HashMap::new(),
            error: Some(error.into()),
            execution_time_ms,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A2A Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2ATaskStatus {
    /// Task is pending.
    Pending,
    /// Task is in progress.
    InProgress,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task timed out.
    TimedOut,
}

impl A2ATaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
        }
    }
}

/// A2A Protocol Adapter for bridging external A2A agents.
pub trait A2AProtocolAdapter {
    /// Discover agents via A2A protocol.
    fn discover_agents(&self) -> Result<Vec<A2AAgentCard>, A2AError>;

    /// Get agent card by ID.
    fn get_agent_card(&self, agent_id: &str) -> Result<A2AAgentCard, A2AError>;

    /// Execute task on A2A agent.
    fn execute_task(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError>;

    /// Cancel task.
    fn cancel_task(&self, task_id: &str) -> Result<(), A2AError>;

    /// Check adapter health.
    fn health_check(&self) -> Result<A2AAdapterHealth, A2AError>;
}

/// A2A Adapter health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2AAdapterHealth {
    /// Adapter status.
    pub status: A2AAdapterStatus,
    /// Connected agents count.
    pub connected_agents: usize,
    /// Last health check time (ms since epoch).
    pub last_check_time_ms: u64,
    /// Adapter version.
    pub adapter_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2AAdapterStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// A2A Protocol error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2AError {
    /// Agent not found.
    AgentNotFound(String),
    /// Capability not supported.
    CapabilityNotSupported(String),
    /// Authentication failed.
    AuthenticationFailed(String),
    /// Task execution failed.
    TaskExecutionFailed(String),
    /// Timeout.
    Timeout(String),
    /// Network error.
    NetworkError(String),
    /// Invalid request.
    InvalidRequest(String),
    /// Adapter unavailable.
    AdapterUnavailable,
}

impl std::fmt::Display for A2AError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotFound(id) => write!(f, "Agent not found: {}", id),
            Self::CapabilityNotSupported(cap) => write!(f, "Capability not supported: {}", cap),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            Self::TaskExecutionFailed(msg) => write!(f, "Task execution failed: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::AdapterUnavailable => write!(f, "A2A adapter unavailable"),
        }
    }
}

impl std::error::Error for A2AError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a2a_agent_card_new() {
        let card = A2AAgentCard::new("agent-1", "Test Agent", "A test agent", "1.0.0");
        assert_eq!(card.agent_id, "agent-1");
        assert_eq!(card.name, "Test Agent");
        assert!(card.capabilities.is_empty());
    }

    #[test]
    fn test_a2a_agent_card_with_capability() {
        let card = A2AAgentCard::new("agent-1", "Test", "Test", "1.0").with_capability(
            A2ACapability::new("cap-1", "Capability 1", "Test capability"),
        );

        assert_eq!(card.capabilities.len(), 1);
        assert!(card.has_capability("cap-1"));
        assert!(!card.has_capability("cap-2"));
    }

    #[test]
    fn test_a2a_agent_card_get_endpoint() {
        let card = A2AAgentCard::new("agent-1", "Test", "Test", "1.0")
            .with_endpoint(A2AEndpoint::new("http", "https://api.example.com"))
            .with_endpoint(A2AEndpoint::new("grpc", "grpc://api.example.com"));

        let http_ep = card.get_endpoint("http");
        assert!(http_ep.is_some());
        assert_eq!(http_ep.unwrap().url, "https://api.example.com");

        let grpc_ep = card.get_endpoint("grpc");
        assert!(grpc_ep.is_some());
    }

    #[test]
    fn test_a2a_capability_new() {
        let cap = A2ACapability::new("cap-1", "Capability", "Test capability");
        assert_eq!(cap.capability_id, "cap-1");
        assert!(cap.input_schema.is_none());
    }

    #[test]
    fn test_a2a_capability_with_schemas() {
        let cap = A2ACapability::new("cap-1", "Cap", "Test")
            .with_input_schema("{\"type\": \"object\"}")
            .with_output_schema("{\"type\": \"string\"}");

        assert!(cap.input_schema.is_some());
        assert!(cap.output_schema.is_some());
    }

    #[test]
    fn test_a2a_endpoint_new() {
        let endpoint = A2AEndpoint::new("http", "https://api.example.com");
        assert_eq!(endpoint.endpoint_type, "http");
        assert_eq!(endpoint.url, "https://api.example.com");
    }

    #[test]
    fn test_a2a_endpoint_with_protocol() {
        let endpoint = A2AEndpoint::new("http", "https://api.example.com").with_protocol("https");

        assert_eq!(endpoint.protocols, vec!["https"]);
    }

    #[test]
    fn test_a2a_task_request_new() {
        let request =
            A2ATaskRequest::new("task-1", "agent-1", "cap-1").with_parameter("spec", "value");
        assert_eq!(request.task_id, "task-1");
        assert_eq!(request.target_agent_id, "agent-1");
        assert!(request.messages.is_empty());
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_a2a_task_request_with_input() {
        let request = A2ATaskRequest::new("task-1", "agent-1", "cap-1")
            .with_input("param1", "value1")
            .with_input("param2", "value2");

        assert_eq!(request.parameters.len(), 2);
        assert_eq!(
            request.parameters.get("param1"),
            Some(&"value1".to_string())
        );
    }

    #[test]
    fn test_a2a_task_context_default() {
        let context = A2ATaskContext::default();
        assert!(context.session_id.is_none());
        assert!(context.user_id.is_none());
    }

    #[test]
    fn test_a2a_task_context_with_fields() {
        let context = A2ATaskContext::default()
            .with_session("session-1")
            .with_user("user-1")
            .with_trace("trace-1");

        assert_eq!(context.session_id, Some("session-1".to_string()));
        assert_eq!(context.user_id, Some("user-1".to_string()));
    }

    #[test]
    fn test_a2a_task_response_success() {
        let output = HashMap::from([("result".to_string(), "success".to_string())]);
        let response = A2ATaskResponse::success("task-1", output, 100);

        assert_eq!(response.status, A2ATaskStatus::Completed);
        assert!(response.error.is_none());
        assert_eq!(response.execution_time_ms, 100);
    }

    #[test]
    fn test_a2a_task_response_failure() {
        let response = A2ATaskResponse::failure("task-1", "Error message", 50);

        assert_eq!(response.status, A2ATaskStatus::Failed);
        assert_eq!(response.error, Some("Error message".to_string()));
    }

    #[test]
    fn test_a2a_task_status_as_str() {
        assert_eq!(A2ATaskStatus::Completed.as_str(), "completed");
        assert_eq!(A2ATaskStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_a2a_error_display() {
        assert_eq!(
            A2AError::AgentNotFound("agent-1".to_string()).to_string(),
            "Agent not found: agent-1"
        );
        assert_eq!(
            A2AError::Timeout("task-1".to_string()).to_string(),
            "Timeout: task-1"
        );
    }
}
