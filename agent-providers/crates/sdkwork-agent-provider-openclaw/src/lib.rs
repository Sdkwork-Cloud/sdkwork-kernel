use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelStreamChunk, ProviderHealth, ProviderManifest, SessionKind, SessionSource, SessionState,
    SideEffectLevel, ToolCall, ToolDescriptor, ToolProvider, ToolResult, ToolSchema,
};
use sdkwork_agent_provider_core::{
    create_session_from_config, uuid_simple, MessageAdapter, SessionAdapter, SessionConfig,
};

#[cfg(test)]
use sdkwork_agent_kernel::{ModelStatus, ToolCallStatus};
#[cfg(test)]
use sdkwork_agent_provider_core::{
    ConversationManager, InMemoryConversationManager, SessionLifecycleProvider,
};

// ============================================================================
// OpenClaw Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenClawMessage {
    pub role: String,
    pub content: String,
    pub task_id: Option<String>,
    pub goal: Option<String>,
    pub tool_calls: Option<Vec<OpenClawToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenClawToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

// ============================================================================
// OpenClaw Session Adapter (existing, preserved)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenClawSessionKind {
    Cron,
    Direct,
    Group,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenClawSessionStatus {
    Running,
    Done,
    Failed,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct OpenClawGatewaySessionRow {
    pub key: String,
    pub session_id: Option<String>,
    pub kind: String,
    pub label: Option<String>,
    pub display_name: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub child_sessions: Vec<String>,
    pub goal: Option<String>,
}

pub struct OpenClawAdapter;

impl OpenClawAdapter {
    pub fn new() -> Self {
        Self
    }

    fn map_kind(kind: &str) -> SessionKind {
        match kind.to_lowercase().as_str() {
            "cron" => SessionKind::Background,
            "direct" => SessionKind::Direct,
            "group" => SessionKind::Group,
            "global" => SessionKind::Main,
            _ => SessionKind::Main,
        }
    }

    fn map_status(status: &str) -> SessionState {
        match status.to_lowercase().as_str() {
            "running" => SessionState::Working,
            "done" => SessionState::Closed,
            "failed" | "killed" | "timeout" => SessionState::Failed,
            _ => SessionState::Created,
        }
    }
}

impl Default for OpenClawAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for OpenClawAdapter {
    type ExternalSession = OpenClawGatewaySessionRow;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        let session_id = external
            .session_id
            .as_deref()
            .unwrap_or(&external.key)
            .to_string();
        let kind = Self::map_kind(&external.kind);
        let state = Self::map_status(&external.status);
        let title = external
            .title
            .clone()
            .or_else(|| external.display_name.clone())
            .or_else(|| external.label.clone());

        let mut config = SessionConfig::new()
            .with_source(SessionSource::Api)
            .with_kind(kind);

        if let Some(ref model) = external.model {
            config = config.with_model(model);
        }
        if let Some(ref title) = title {
            config = config.with_title(title);
        }

        let mut session = create_session_from_config(
            &session_id,
            None,
            None,
            None,
            config,
            external.started_at.as_deref().unwrap_or(""),
        );

        session.state = state;
        session.updated_at = external.updated_at.clone();
        session.token_usage.input_tokens = external.input_tokens;
        session.token_usage.output_tokens = external.output_tokens;
        session.token_usage.total_tokens = external.input_tokens + external.output_tokens;
        session.child_session_ids = external.child_sessions.clone();
        session.goal = external.goal.clone();

        Ok(session)
    }
}

// ============================================================================
// OpenClaw Message Adapter
// ============================================================================

pub struct OpenClawMessageAdapter;

impl OpenClawMessageAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenClawMessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAdapter for OpenClawMessageAdapter {
    type ExternalMessage = OpenClawMessage;

    fn to_agent_message(&self, external: &Self::ExternalMessage) -> KernelResult<AgentMessage> {
        let role = match external.role.as_str() {
            "user" => AgentMessageRole::User,
            "assistant" => AgentMessageRole::Agent,
            "system" => AgentMessageRole::System,
            "tool" => AgentMessageRole::Tool,
            _ => AgentMessageRole::Adapter,
        };

        let mut parts = Vec::new();

        if !external.content.is_empty() {
            parts.push(AgentPart::text("openclaw.content", &external.content));
        }

        if let Some(ref goal) = external.goal {
            parts.push(
                AgentPart::text("openclaw.goal", goal)
                    .with_metadata("openclaw.content_type", "goal"),
            );
        }

        if let Some(tool_calls) = &external.tool_calls {
            for tc in tool_calls {
                let mut part =
                    AgentPart::tool_call_ref(format!("openclaw.tool_call.{}", tc.id), &tc.id);
                part.name = Some(tc.function_name.clone());
                part.json = Some(tc.arguments.clone());
                parts.push(part);
            }
        }

        if parts.is_empty() {
            parts.push(AgentPart::text("openclaw.empty", ""));
        }

        let mut message = AgentMessage::new(format!("openclaw.msg.{}", uuid_simple()), role, parts);

        if let Some(task_id) = &external.task_id {
            message = message.with_metadata("openclaw.task_id", task_id);
        }
        if let Some(tool_call_id) = &external.tool_call_id {
            message = message.with_metadata("openclaw.tool_call_id", tool_call_id);
        }

        Ok(message)
    }
}

// ============================================================================
// OpenClaw Model Provider
// ============================================================================

pub struct OpenClawModelProvider {
    default_model: String,
}

impl OpenClawModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "openclaw-default".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for OpenClawModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for OpenClawModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.openclaw",
            "model",
            "OpenClaw Model Provider",
            "0.1.0",
            vec![
                "model.chat".to_string(),
                "model.stream".to_string(),
                "model.tool_call".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            "openclaw-default",
            "provider.model.openclaw",
            "OpenClaw Default",
            "openclaw",
        )
        .with_version("1.0")
        .with_capability("chat")
        .with_capability("tool_call")
        .with_context_window_tokens(128000)
        .with_max_output_tokens(8192)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_response_format(ModelResponseFormat::Text)
        .with_response_format(ModelResponseFormat::Json)
        .with_tool_capability("function_calling")]
    }

    fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        sdkwork_agent_provider_core::reject_in_process_model_invoke("provider.model.openclaw")
    }

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        sdkwork_agent_provider_core::reject_in_process_model_stream("provider.model.openclaw")
    }
}

// ============================================================================
// OpenClaw Tool Provider
// ============================================================================

pub struct OpenClawToolProvider;

impl OpenClawToolProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenClawToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for OpenClawToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.openclaw",
            "tool",
            "OpenClaw Tool Provider",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "openclaw.message",
                "provider.tool.openclaw",
                "Message",
                SideEffectLevel::ExternalSend,
            )
            .with_name("message")
            .with_description("Send a message to the current channel or thread")
            .with_input_schema(ToolSchema::json_schema("openclaw.message.input"))
            .with_output_schema(ToolSchema::json_schema("openclaw.message.output")),
            ToolDescriptor::new(
                "openclaw.sessions_spawn",
                "provider.tool.openclaw",
                "Sessions Spawn",
                SideEffectLevel::SideEffectful,
            )
            .with_name("sessions_spawn")
            .with_description("Spawn a subagent session")
            .with_input_schema(ToolSchema::json_schema("openclaw.sessions_spawn.input"))
            .with_output_schema(ToolSchema::json_schema("openclaw.sessions_spawn.output")),
            ToolDescriptor::new(
                "openclaw.web_search",
                "provider.tool.openclaw",
                "Web Search",
                SideEffectLevel::ExternalSend,
            )
            .with_name("web_search")
            .with_description("Search the web")
            .with_input_schema(ToolSchema::json_schema("openclaw.web_search.input"))
            .with_output_schema(ToolSchema::json_schema("openclaw.web_search.output")),
            ToolDescriptor::new(
                "openclaw.cron",
                "provider.tool.openclaw",
                "Cron",
                SideEffectLevel::SideEffectful,
            )
            .with_name("cron")
            .with_description("Manage scheduled cron jobs")
            .with_input_schema(ToolSchema::json_schema("openclaw.cron.input"))
            .with_output_schema(ToolSchema::json_schema("openclaw.cron.output")),
        ]
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        let output = match call.tool_id.as_str() {
            "openclaw.message" => {
                format!("[OpenClaw Message] Mock send: {}", call.arguments)
            }
            "openclaw.sessions_spawn" => {
                format!(
                    "[OpenClaw SessionsSpawn] Mock subagent spawn: {}",
                    call.arguments
                )
            }
            "openclaw.web_search" => {
                format!(
                    "[OpenClaw WebSearch] Mock search results for: {}",
                    call.arguments
                )
            }
            "openclaw.cron" => {
                format!("[OpenClaw Cron] Mock cron operation: {}", call.arguments)
            }
            _ => {
                return Err(KernelError::CapabilityMissing {
                    capability_id: call.tool_id.clone(),
                });
            }
        };

        Ok(ToolResult::succeeded(&call.tool_call_id, output))
    }
}

// ============================================================================
// OpenClaw Lifecycle Provider (existing, preserved)
// ============================================================================

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(
    OpenClawLifecycleProvider,
    "openclaw"
);

pub mod sdk_integration;
pub use sdk_integration::{
    openclaw_binding_manifest, OpenClawSdkIntegration, OPENCLAW_NPM_PACKAGE,
};

mod agent_definition;
mod conformance;
pub mod ids;
mod manifest;
mod package;

pub use manifest::{openclaw_kernel_plugin_manifest, OpenClawKernelPlugin};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_openclaw_session() -> OpenClawGatewaySessionRow {
        OpenClawGatewaySessionRow {
            key: "gw.row.1".to_string(),
            session_id: Some("oc.session.1".to_string()),
            kind: "direct".to_string(),
            label: Some("Direct Chat".to_string()),
            display_name: None,
            title: None,
            model: Some("claude-3".to_string()),
            status: "running".to_string(),
            input_tokens: 2000,
            output_tokens: 800,
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T02:00:00Z".to_string()),
            child_sessions: vec!["child.1".to_string()],
            goal: Some("Fix the bug".to_string()),
        }
    }

    // --- Session Adapter Tests ---

    #[test]
    fn maps_basic_fields() {
        let adapter = OpenClawAdapter::new();
        let ext = sample_openclaw_session();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.session_id, "oc.session.1");
        assert_eq!(session.kind, SessionKind::Direct);
        assert_eq!(session.state, SessionState::Working);
        assert_eq!(session.model, Some("claude-3".to_string()));
        assert_eq!(session.title, Some("Direct Chat".to_string()));
        assert_eq!(session.token_usage.input_tokens, 2000);
        assert_eq!(session.token_usage.output_tokens, 800);
        assert_eq!(session.token_usage.total_tokens, 2800);
        assert_eq!(session.child_session_ids, vec!["child.1".to_string()]);
        assert_eq!(session.goal, Some("Fix the bug".to_string()));
    }

    #[test]
    fn falls_back_to_key_when_no_session_id() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.session_id = None;
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.session_id, "gw.row.1");
    }

    #[test]
    fn maps_cron_to_background() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.kind = "cron".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.kind, SessionKind::Background);
    }

    #[test]
    fn maps_group_to_group() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.kind = "group".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.kind, SessionKind::Group);
    }

    #[test]
    fn maps_global_to_main() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.kind = "global".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.kind, SessionKind::Main);
    }

    #[test]
    fn maps_done_to_closed() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.status = "done".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.state, SessionState::Closed);
    }

    #[test]
    fn maps_failed_to_failed() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.status = "failed".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.state, SessionState::Failed);
    }

    #[test]
    fn maps_timeout_to_failed() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.status = "timeout".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.state, SessionState::Failed);
    }

    #[test]
    fn prefers_display_name_for_title() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.display_name = Some("Display".to_string());
        ext.title = None;
        ext.label = Some("Label".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.title, Some("Display".to_string()));
    }

    #[test]
    fn prefers_title_over_label() {
        let adapter = OpenClawAdapter::new();
        let mut ext = sample_openclaw_session();
        ext.title = Some("Custom Title".to_string());
        ext.label = Some("Label".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.title, Some("Custom Title".to_string()));
    }

    #[test]
    fn lifecycle_provider_create_and_list() {
        let provider = OpenClawLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_source(SessionSource::Api)
            .with_kind(SessionKind::Direct);
        let session = provider
            .create_session("agent.1", Some("user.1"), config)
            .unwrap();
        assert_eq!(session.agent_id, Some("agent.1".to_string()));

        provider.resume_session(&session.session_id).unwrap();
        let active = provider.list_active_sessions().unwrap();
        assert_eq!(active.len(), 1);
    }

    // --- Message Adapter Tests ---

    #[test]
    fn converts_user_message() {
        let adapter = OpenClawMessageAdapter::new();
        let msg = OpenClawMessage {
            role: "user".to_string(),
            content: "Fix the bug".to_string(),
            task_id: None,
            goal: None,
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::User);
        assert_eq!(result.parts.len(), 1);
        assert_eq!(result.parts[0].text, Some("Fix the bug".to_string()));
    }

    #[test]
    fn converts_assistant_with_goal() {
        let adapter = OpenClawMessageAdapter::new();
        let msg = OpenClawMessage {
            role: "assistant".to_string(),
            content: "Working on it".to_string(),
            task_id: None,
            goal: Some("Fix authentication".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
        assert_eq!(result.parts.len(), 2);
        assert_eq!(result.parts[0].text, Some("Working on it".to_string()));
        assert_eq!(
            result.parts[1].metadata_value("openclaw.content_type"),
            Some("goal")
        );
    }

    #[test]
    fn converts_message_with_task_id() {
        let adapter = OpenClawMessageAdapter::new();
        let msg = OpenClawMessage {
            role: "assistant".to_string(),
            content: "Done".to_string(),
            task_id: Some("task.42".to_string()),
            goal: None,
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.metadata_value("openclaw.task_id"), Some("task.42"));
    }

    #[test]
    fn converts_tool_calls() {
        let adapter = OpenClawMessageAdapter::new();
        let msg = OpenClawMessage {
            role: "assistant".to_string(),
            content: String::new(),
            task_id: None,
            goal: None,
            tool_calls: Some(vec![OpenClawToolCall {
                id: "tc.1".to_string(),
                function_name: "sessions_spawn".to_string(),
                arguments: "{}".to_string(),
            }]),
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.parts.len(), 1);
        assert_eq!(
            result.parts[0].kind,
            sdkwork_agent_kernel::AgentPartKind::ToolCallRef
        );
    }

    // --- Model Provider Tests ---

    #[test]
    fn model_provider_manifest() {
        let provider = OpenClawModelProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.model.openclaw");
        assert_eq!(manifest.provider_family, "model");
    }

    #[test]
    fn model_provider_list_models() {
        let provider = OpenClawModelProvider::new();
        let models = provider.list_models();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "openclaw-default");
    }

    #[test]
    fn model_provider_invoke() {
        let provider = OpenClawModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["Hello".to_string()]);
        let response = provider.invoke(request).unwrap();
        assert_eq!(response.status, ModelStatus::Succeeded);
        assert_eq!(response.provider_id, "provider.model.openclaw");
    }

    #[test]
    fn model_provider_stream() {
        let provider = OpenClawModelProvider::new();
        let request = ModelRequest::new("req.2", vec!["Hello".to_string()]);
        let chunks = provider.stream(request).unwrap();
        assert!(!chunks.is_empty());
    }

    // --- Tool Provider Tests ---

    #[test]
    fn tool_provider_list_tools() {
        let provider = OpenClawToolProvider::new();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.tool_id == "openclaw.message"));
        assert!(tools.iter().any(|t| t.tool_id == "openclaw.sessions_spawn"));
        assert!(tools.iter().any(|t| t.tool_id == "openclaw.web_search"));
        assert!(tools.iter().any(|t| t.tool_id == "openclaw.cron"));
    }

    #[test]
    fn tool_provider_invoke_spawn() {
        let provider = OpenClawToolProvider::new();
        let call = ToolCall::new("c.1", "openclaw.sessions_spawn", r#"{"task":"fix auth"}"#);
        let result = provider.invoke_tool(call).unwrap();
        assert_eq!(result.normalized_status, ToolCallStatus::Succeeded);
        assert!(result.output.contains("Mock subagent spawn"));
    }

    #[test]
    fn tool_provider_invoke_unknown() {
        let provider = OpenClawToolProvider::new();
        let call = ToolCall::new("c.2", "openclaw.nonexistent", "{}");
        assert!(provider.invoke_tool(call).is_err());
    }

    // --- Conversation Manager Tests ---

    #[test]
    fn conversation_manager_full_cycle() {
        let mut manager = InMemoryConversationManager::new();
        manager.begin_turn("s1").unwrap();
        manager
            .append_message(
                "s1",
                AgentMessage::new(
                    "m1",
                    AgentMessageRole::User,
                    vec![AgentPart::text("p1", "Hello")],
                ),
            )
            .unwrap();
        assert_eq!(manager.get_history("s1").unwrap().len(), 1);
        assert_eq!(manager.current_turn("s1").unwrap(), 1);
        manager.clear_history("s1").unwrap();
        assert!(manager.get_history("s1").unwrap().is_empty());
    }

    #[test]
    fn conversation_manager_compress() {
        let mut manager = InMemoryConversationManager::new();
        for i in 0..10 {
            manager
                .append_message(
                    "s1",
                    AgentMessage::new(
                        format!("m.{}", i),
                        AgentMessageRole::User,
                        vec![AgentPart::text(
                            format!("p.{}", i),
                            format!("Message {}", i),
                        )],
                    ),
                )
                .unwrap();
        }
        let compressed = manager.compress_history("s1", 100).unwrap();
        assert_eq!(compressed.role, AgentMessageRole::System);
        assert!(compressed.parts[0]
            .text
            .as_ref()
            .unwrap()
            .contains("Compressed"));
        let remaining = manager.get_history("s1").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].role, AgentMessageRole::System);
    }
}
