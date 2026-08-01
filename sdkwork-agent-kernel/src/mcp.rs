use crate::{
    AgentRuntime, ContextFrame, KernelError, KernelResult, PolicyDecision, PolicyDecisionValue,
    ProviderHealth, ProviderManifest, RedactionClassification, ToolCall, ToolDescriptor,
    ToolResult, TrustLevel,
};

/// MCP transport kinds, aligned with the MCP ecosystem (stdio, SSE, HTTP,
/// streamable HTTP, WebSocket) and the sdkwork-mcp connector record format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransportKind {
    Stdio,
    Sse,
    Http,
    StreamableHttp,
    WebSocket,
}

impl McpTransportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse => "sse",
            Self::Http => "http",
            Self::StreamableHttp => "streamable-http",
            Self::WebSocket => "ws",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "stdio" => Some(Self::Stdio),
            "sse" => Some(Self::Sse),
            "http" => Some(Self::Http),
            "streamable-http" => Some(Self::StreamableHttp),
            "ws" | "websocket" => Some(Self::WebSocket),
            _ => None,
        }
    }
}

/// MCP server authentication kinds, aligned with the connector records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpAuthKind {
    None,
    Bearer,
    ApiKey,
    OAuth,
}

impl McpAuthKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bearer => "bearer",
            Self::ApiKey => "api_key",
            Self::OAuth => "oauth",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "bearer" => Some(Self::Bearer),
            "api_key" | "apikey" => Some(Self::ApiKey),
            "oauth" => Some(Self::OAuth),
            _ => None,
        }
    }
}

/// MCP server connection lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpConnectionState {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

impl McpConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Failed => "failed",
        }
    }
}

/// Observable MCP server connection snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConnection {
    pub state: McpConnectionState,
    pub endpoint: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub headers: Vec<(String, String)>,
    pub started_at: Option<String>,
    pub last_error: Option<String>,
}

impl McpServerConnection {
    pub fn new(state: McpConnectionState) -> Self {
        Self {
            state,
            endpoint: None,
            command: None,
            args: Vec::new(),
            headers: Vec::new(),
            started_at: None,
            last_error: None,
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<String>, args: Vec<String>) -> Self {
        self.command = Some(command.into());
        self.args = args;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.last_error = Some(error.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerDescriptor {
    pub server_id: String,
    pub provider_id: String,
    pub transport: McpTransportKind,
    pub capabilities: Vec<String>,
    /// Authentication kind when the server requires credentials.
    pub auth: Option<McpAuthKind>,
    /// Connection lifecycle snapshot when the runtime manages the server.
    pub connection: Option<McpServerConnection>,
    pub startup_timeout_ms: Option<u64>,
    pub tool_timeout_ms: Option<u64>,
    /// Per-server tool allow/deny lists (empty = no restriction).
    pub enabled_tools: Vec<String>,
    pub disabled_tools: Vec<String>,
}

impl McpServerDescriptor {
    pub fn new(
        server_id: impl Into<String>,
        provider_id: impl Into<String>,
        transport: McpTransportKind,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            provider_id: provider_id.into(),
            transport,
            capabilities: Vec::new(),
            auth: None,
            connection: None,
            startup_timeout_ms: None,
            tool_timeout_ms: None,
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    pub fn with_auth(mut self, auth: McpAuthKind) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_connection(mut self, connection: McpServerConnection) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn with_startup_timeout_ms(mut self, startup_timeout_ms: u64) -> Self {
        self.startup_timeout_ms = Some(startup_timeout_ms);
        self
    }

    pub fn with_tool_timeout_ms(mut self, tool_timeout_ms: u64) -> Self {
        self.tool_timeout_ms = Some(tool_timeout_ms);
        self
    }

    pub fn with_enabled_tool(mut self, tool: impl Into<String>) -> Self {
        self.enabled_tools.push(tool.into());
        self
    }

    pub fn with_disabled_tool(mut self, tool: impl Into<String>) -> Self {
        self.disabled_tools.push(tool.into());
        self
    }

    /// Whether a tool is permitted by this server's allow/deny lists.
    pub fn permits_tool(&self, tool_name: &str) -> bool {
        if self.disabled_tools.iter().any(|tool| tool == tool_name) {
            return false;
        }
        if self.enabled_tools.is_empty() {
            true
        } else {
            self.enabled_tools.iter().any(|tool| tool == tool_name)
        }
    }
}

/// Namespaced MCP tool name: `mcp__<server>__<tool>` (the agent SDK
/// convention).
pub fn mcp_tool_name(server_id: &str, tool_name: &str) -> String {
    format!("mcp__{server_id}__{tool_name}")
}

/// Parsed components of a namespaced MCP tool name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMcpToolName {
    pub server_id: String,
    pub tool_name: String,
}

/// Split a namespaced MCP tool name into server and tool components.
pub fn parse_mcp_tool_name(name: &str) -> Option<ParsedMcpToolName> {
    let rest = name.strip_prefix("mcp__")?;
    let (server_id, tool_name) = rest.split_once("__")?;
    if server_id.is_empty() || tool_name.is_empty() {
        return None;
    }
    Some(ParsedMcpToolName {
        server_id: server_id.to_string(),
        tool_name: tool_name.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResourceDescriptor {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
    pub description: Option<String>,
}

impl McpResourceDescriptor {
    pub fn new(
        uri: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            mime_type: mime_type.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub content: String,
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl McpResourceContent {
    pub fn new(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: mime_type.into(),
            content: content.into(),
            trust_level: TrustLevel::RetrievedExternal,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn to_context_frame(&self, session_id: impl Into<String>) -> ContextFrame {
        let mut frame = ContextFrame::new(
            format!("context.mcp.resource.{}", self.uri),
            session_id,
            "mcp.resource",
            self.content.clone(),
            self.trust_level,
            self.redaction_classification,
        )
        .with_content_type(self.mime_type.clone())
        .with_provenance(self.uri.clone());

        for (key, value) in &self.metadata {
            frame = frame.with_metadata(key.clone(), value.clone());
        }

        frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPromptDescriptor {
    pub prompt_id: String,
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<String>,
}

impl McpPromptDescriptor {
    pub fn new(prompt_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            prompt_id: prompt_id.into(),
            name: name.into(),
            description: None,
            arguments: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPromptMessage {
    pub prompt_id: String,
    pub messages: Vec<String>,
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl McpPromptMessage {
    pub fn new(prompt_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            prompt_id: prompt_id.into(),
            messages,
            trust_level: TrustLevel::TrustedHost,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn to_context_frames(&self, session_id: impl Into<String>) -> Vec<ContextFrame> {
        let session_id = session_id.into();
        let provenance = format!("mcp.prompt:{}", self.prompt_id);

        self.messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                let mut frame = ContextFrame::new(
                    format!("context.mcp.prompt.{}.{}", self.prompt_id, index),
                    session_id.clone(),
                    "mcp.prompt",
                    message.clone(),
                    self.trust_level,
                    self.redaction_classification,
                )
                .with_content_type("text/plain")
                .with_provenance(provenance.clone());

                for (key, value) in &self.metadata {
                    frame = frame.with_metadata(key.clone(), value.clone());
                }

                frame
            })
            .collect()
    }
}

pub trait McpProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn list_servers(&self) -> KernelResult<Vec<McpServerDescriptor>>;

    fn list_tools(&self, _server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.tools".to_string(),
        })
    }

    fn invoke_tool(&self, _server_id: &str, _call: ToolCall) -> KernelResult<ToolResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.tools".to_string(),
        })
    }

    fn list_resources(&self, _server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.resources".to_string(),
        })
    }

    fn read_resource(&self, _server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        Err(KernelError::CapabilityMissing {
            capability_id: uri.to_string(),
        })
    }

    fn list_prompts(&self, _server_id: &str) -> KernelResult<Vec<McpPromptDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.prompts".to_string(),
        })
    }

    fn get_prompt(
        &self,
        _server_id: &str,
        prompt_id: &str,
        _arguments: Vec<(String, String)>,
    ) -> KernelResult<McpPromptMessage> {
        Err(KernelError::CapabilityMissing {
            capability_id: prompt_id.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolExecutionRequest {
    pub mcp_execution_id: String,
    pub server_id: String,
    pub provider_id: Option<String>,
    pub tool_call: ToolCall,
}

impl McpToolExecutionRequest {
    pub fn new(
        mcp_execution_id: impl Into<String>,
        server_id: impl Into<String>,
        tool_call: ToolCall,
    ) -> Self {
        Self {
            mcp_execution_id: mcp_execution_id.into(),
            server_id: server_id.into(),
            provider_id: None,
            tool_call,
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolExecutionResponse {
    pub mcp_execution_id: String,
    pub server_id: String,
    pub provider_id: String,
    pub descriptor: ToolDescriptor,
    pub policy_decision: PolicyDecision,
    pub result: ToolResult,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct McpToolExecutionService;

impl McpToolExecutionService {
    pub fn new() -> Self {
        Self
    }

    pub fn invoke(
        &self,
        runtime: &AgentRuntime,
        request: McpToolExecutionRequest,
    ) -> KernelResult<McpToolExecutionResponse> {
        let provider_id = request
            .provider_id
            .as_deref()
            .or(request.tool_call.provider_id.as_deref());
        let provider = match provider_id {
            Some(provider_id) => runtime.mcp_provider_by_id(provider_id)?,
            None => runtime.mcp_provider()?,
        };

        let descriptor = provider
            .list_tools(&request.server_id)?
            .into_iter()
            .find(|descriptor| descriptor.tool_id == request.tool_call.tool_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: request.tool_call.tool_id.clone(),
            })?;
        let policy_request = descriptor.policy_request(
            format!("policy-request.{}", request.tool_call.tool_call_id),
            &request.tool_call,
        );
        let policy_decision = runtime.policy_provider()?.evaluate(policy_request)?;
        self.ensure_allowed(&policy_decision)?;

        let mut tool_call = request.tool_call;
        tool_call.policy_decision_id = Some(policy_decision.decision_id.clone());
        if tool_call.provider_id.is_none() {
            tool_call.provider_id = Some(descriptor.provider_id.clone());
        }

        let result = provider.invoke_tool(&request.server_id, tool_call)?;

        Ok(McpToolExecutionResponse {
            mcp_execution_id: request.mcp_execution_id,
            server_id: request.server_id,
            provider_id: descriptor.provider_id.clone(),
            descriptor,
            policy_decision,
            result,
        })
    }

    fn ensure_allowed(&self, policy_decision: &PolicyDecision) -> KernelResult<()> {
        match policy_decision.decision {
            PolicyDecisionValue::Allow => Ok(()),
            PolicyDecisionValue::Deny => Err(KernelError::PolicyDenied {
                reason_code: policy_decision.reason_code.clone(),
            }),
            PolicyDecisionValue::NeedsApproval => Err(KernelError::permission_required(
                policy_decision
                    .safe_reason
                    .clone()
                    .unwrap_or_else(|| policy_decision.reason_code.clone()),
            )),
            PolicyDecisionValue::Defer => Err(KernelError::provider_error(
                "policy.deferred",
                policy_decision.reason_code.clone(),
            )),
        }
    }
}
