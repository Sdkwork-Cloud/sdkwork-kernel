use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelStreamChunk, ProviderHealth, ProviderManifest, SessionKind, SessionSource,
    SideEffectLevel, ToolCall, ToolDescriptor, ToolProvider, ToolResult, ToolSchema,
};
use sdkwork_agent_provider_core::{
    create_session_from_config, uuid_simple, MessageAdapter, SessionAdapter, SessionConfig,
};

#[cfg(test)]
use sdkwork_agent_provider_core::{
    ConversationManager, InMemoryConversationManager, SessionLifecycleProvider,
};

// ============================================================================
// OpenCode Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenCodeMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<OpenCodeToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenCodeToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

// ============================================================================
// OpenCode Session Adapter (existing, preserved)
// ============================================================================

#[derive(Debug, Clone)]
pub struct OpenCodeSession {
    pub id: String,
    pub parent_session_id: Option<String>,
    pub title: Option<String>,
    pub message_count: u32,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_cents: Option<u64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
}

pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for OpenCodeAdapter {
    type ExternalSession = OpenCodeSession;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        let kind = if external.parent_session_id.is_some() {
            SessionKind::Subagent
        } else {
            SessionKind::Main
        };

        let mut config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(kind);

        if let Some(ref model) = external.model {
            config = config.with_model(model);
        }
        if let Some(ref title) = external.title {
            config = config.with_title(title);
        }
        if let Some(ref cwd) = external.cwd {
            config = config.with_cwd(cwd);
        }

        let mut session = create_session_from_config(
            &external.id,
            None,
            None,
            None,
            config,
            external.created_at.as_deref().unwrap_or(""),
        );

        session.parent_session_id = external.parent_session_id.clone();
        session.updated_at = external.updated_at.clone();
        session.message_count = external.message_count;
        session.token_usage.input_tokens = external.prompt_tokens;
        session.token_usage.output_tokens = external.completion_tokens;
        session.token_usage.total_tokens = external.prompt_tokens + external.completion_tokens;
        session.cost_cents = external.cost_cents;

        Ok(session)
    }
}

// ============================================================================
// OpenCode Message Adapter
// ============================================================================

pub struct OpenCodeMessageAdapter;

impl OpenCodeMessageAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenCodeMessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAdapter for OpenCodeMessageAdapter {
    type ExternalMessage = OpenCodeMessage;

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
            parts.push(AgentPart::text("opencode.content", &external.content));
        }

        if let Some(tool_calls) = &external.tool_calls {
            for tc in tool_calls {
                let mut part =
                    AgentPart::tool_call_ref(format!("opencode.tool_call.{}", tc.id), &tc.id);
                part.name = Some(tc.function_name.clone());
                part.json = Some(tc.arguments.clone());
                parts.push(part);
            }
        }

        if parts.is_empty() {
            parts.push(AgentPart::text("opencode.empty", ""));
        }

        let mut message = AgentMessage::new(format!("opencode.msg.{}", uuid_simple()), role, parts);

        if let Some(tool_call_id) = &external.tool_call_id {
            message = message.with_metadata("opencode.tool_call_id", tool_call_id);
        }

        Ok(message)
    }
}

// ============================================================================
// OpenCode Model Provider
// ============================================================================

pub struct OpenCodeModelProvider {
    default_model: String,
}

impl OpenCodeModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "opencode-default".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for OpenCodeModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for OpenCodeModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.opencode",
            "model",
            "OpenCode Model Provider",
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
            "opencode-default",
            "provider.model.opencode",
            "OpenCode Default",
            "opencode",
        )
        .with_version("1.0")
        .with_capability("chat")
        .with_capability("tool_call")
        .with_context_window_tokens(128000)
        .with_max_output_tokens(8192)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_response_format(ModelResponseFormat::Text)
        .with_tool_capability("function_calling")]
    }

    fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        sdkwork_agent_provider_core::reject_in_process_model_invoke("provider.model.opencode")
    }

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        sdkwork_agent_provider_core::reject_in_process_model_stream("provider.model.opencode")
    }
}

// ============================================================================
// OpenCode Tool Provider
// ============================================================================

pub struct OpenCodeToolProvider;

impl OpenCodeToolProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenCodeToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for OpenCodeToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.opencode",
            "tool",
            "OpenCode Tool Provider",
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
                "opencode.code_edit",
                "provider.tool.opencode",
                "Code Edit",
                SideEffectLevel::SideEffectful,
            )
            .with_name("code_edit")
            .with_description("Edit code files")
            .with_input_schema(ToolSchema::json_schema("opencode.code_edit.input"))
            .with_output_schema(ToolSchema::json_schema("opencode.code_edit.output")),
            ToolDescriptor::new(
                "opencode.terminal",
                "provider.tool.opencode",
                "Terminal",
                SideEffectLevel::SideEffectful,
            )
            .with_name("terminal")
            .with_description("Execute terminal commands")
            .with_input_schema(ToolSchema::json_schema("opencode.terminal.input"))
            .with_output_schema(ToolSchema::json_schema("opencode.terminal.output")),
            ToolDescriptor::new(
                "opencode.search",
                "provider.tool.opencode",
                "Search",
                SideEffectLevel::ReadOnly,
            )
            .with_name("search")
            .with_description("Search code")
            .with_input_schema(ToolSchema::json_schema("opencode.search.input"))
            .with_output_schema(ToolSchema::json_schema("opencode.search.output")),
        ]
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        match call.tool_id.as_str() {
            "opencode.code_edit" | "opencode.terminal" | "opencode.search" => {
                sdkwork_agent_provider_core::reject_in_process_tool_invoke(
                    "provider.tool.opencode",
                )
            }
            _ => Err(KernelError::CapabilityMissing {
                capability_id: call.tool_id.clone(),
            }),
        }
    }
}

// ============================================================================
// OpenCode Lifecycle Provider (existing, preserved)
// ============================================================================

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(
    OpenCodeLifecycleProvider,
    "opencode"
);

// ============================================================================
// SDK Integration
// ============================================================================

pub mod sdk_integration;
pub use sdk_integration::{opencode_binding_manifest, OpenCodeSdkIntegration};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_opencode_session() -> OpenCodeSession {
        OpenCodeSession {
            id: "oc.session.1".to_string(),
            parent_session_id: None,
            title: Some("Debug Session".to_string()),
            message_count: 15,
            prompt_tokens: 3000,
            completion_tokens: 1200,
            cost_cents: Some(42),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T03:00:00Z".to_string()),
            model: Some("deepseek-v3".to_string()),
            cwd: Some("/home/user/code".to_string()),
        }
    }

    // --- Session Adapter Tests ---

    #[test]
    fn maps_basic_fields() {
        let adapter = OpenCodeAdapter::new();
        let ext = sample_opencode_session();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.session_id, "oc.session.1");
        assert_eq!(session.source, SessionSource::Cli);
        assert_eq!(session.kind, SessionKind::Main);
        assert_eq!(session.title, Some("Debug Session".to_string()));
        assert_eq!(session.message_count, 15);
        assert_eq!(session.token_usage.input_tokens, 3000);
        assert_eq!(session.token_usage.output_tokens, 1200);
        assert_eq!(session.token_usage.total_tokens, 4200);
        assert_eq!(session.cost_cents, Some(42));
        assert_eq!(session.created_at, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(session.updated_at, Some("2026-01-01T03:00:00Z".to_string()));
        assert_eq!(session.model, Some("deepseek-v3".to_string()));
        assert_eq!(session.cwd, Some("/home/user/code".to_string()));
    }

    #[test]
    fn maps_subagent_when_parent_exists() {
        let adapter = OpenCodeAdapter::new();
        let mut ext = sample_opencode_session();
        ext.parent_session_id = Some("parent.1".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.kind, SessionKind::Subagent);
        assert_eq!(session.parent_session_id, Some("parent.1".to_string()));
    }

    #[test]
    fn lifecycle_provider_create_and_list() {
        let provider = OpenCodeLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);
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
        let adapter = OpenCodeMessageAdapter::new();
        let msg = OpenCodeMessage {
            role: "user".to_string(),
            content: "Debug this function".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::User);
        assert_eq!(result.parts.len(), 1);
        assert_eq!(
            result.parts[0].text,
            Some("Debug this function".to_string())
        );
    }

    #[test]
    fn converts_assistant_message() {
        let adapter = OpenCodeMessageAdapter::new();
        let msg = OpenCodeMessage {
            role: "assistant".to_string(),
            content: "Found the issue".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
    }

    #[test]
    fn converts_tool_calls() {
        let adapter = OpenCodeMessageAdapter::new();
        let msg = OpenCodeMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![OpenCodeToolCall {
                id: "tc.1".to_string(),
                function_name: "code_edit".to_string(),
                arguments: r#"{"file":"src/main.rs"}"#.to_string(),
            }]),
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.parts.len(), 1);
        assert_eq!(
            result.parts[0].kind,
            sdkwork_agent_kernel::AgentPartKind::ToolCallRef
        );
        assert_eq!(result.parts[0].name, Some("code_edit".to_string()));
    }

    #[test]
    fn converts_batch_messages() {
        let adapter = OpenCodeMessageAdapter::new();
        let messages = vec![
            OpenCodeMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            OpenCodeMessage {
                role: "assistant".to_string(),
                content: "Hi".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let result = adapter.to_agent_messages(&messages).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn converts_system_message() {
        let adapter = OpenCodeMessageAdapter::new();
        let msg = OpenCodeMessage {
            role: "system".to_string(),
            content: "You are a coding assistant".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::System);
    }

    // --- Model Provider Tests ---

    #[test]
    fn model_provider_manifest() {
        let provider = OpenCodeModelProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.model.opencode");
        assert_eq!(manifest.provider_family, "model");
    }

    #[test]
    fn model_provider_health() {
        let provider = OpenCodeModelProvider::new();
        assert_eq!(provider.health().status, "available");
    }

    #[test]
    fn model_provider_list_models() {
        let provider = OpenCodeModelProvider::new();
        let models = provider.list_models();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "opencode-default");
    }

    #[test]
    fn model_provider_invoke_requires_transport_worker() {
        let provider = OpenCodeModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["Explain async".to_string()])
            .with_model_id("opencode-default");
        let error = provider
            .invoke(request)
            .expect_err("in-process model invocation is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    #[test]
    fn model_provider_stream_requires_transport_worker() {
        let provider = OpenCodeModelProvider::new();
        let request = ModelRequest::new("req.2", vec!["Hello".to_string()]);
        let error = provider
            .stream(request)
            .expect_err("in-process model streaming is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    // --- Tool Provider Tests ---

    #[test]
    fn tool_provider_manifest() {
        let provider = OpenCodeToolProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.tool.opencode");
        assert_eq!(manifest.provider_family, "tool");
    }

    #[test]
    fn tool_provider_list_tools() {
        let provider = OpenCodeToolProvider::new();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.tool_id == "opencode.code_edit"));
        assert!(tools.iter().any(|t| t.tool_id == "opencode.terminal"));
        assert!(tools.iter().any(|t| t.tool_id == "opencode.search"));

        let search = tools
            .iter()
            .find(|t| t.tool_id == "opencode.search")
            .unwrap();
        assert_eq!(search.side_effect_level, SideEffectLevel::ReadOnly);
    }

    #[test]
    fn tool_provider_invoke_requires_transport_worker() {
        let provider = OpenCodeToolProvider::new();
        let call = ToolCall::new("c.1", "opencode.terminal", r#"{"cmd":"echo hello"}"#);
        let error = provider
            .invoke_tool(call)
            .expect_err("in-process tool invocation is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    #[test]
    fn tool_provider_invoke_unknown() {
        let provider = OpenCodeToolProvider::new();
        let call = ToolCall::new("c.2", "opencode.nonexistent", "{}");
        assert!(provider.invoke_tool(call).is_err());
    }

    #[test]
    fn tool_provider_describe_tool() {
        let provider = OpenCodeToolProvider::new();
        let desc = provider.describe_tool("opencode.code_edit").unwrap();
        assert_eq!(desc.display_name, "Code Edit");
        assert_eq!(desc.side_effect_level, SideEffectLevel::SideEffectful);
    }

    // --- Conversation Manager Tests ---

    #[test]
    fn conversation_manager_append_and_get() {
        let mut manager = InMemoryConversationManager::new();
        let msg = AgentMessage::new(
            "msg.1",
            AgentMessageRole::User,
            vec![AgentPart::text("p1", "Hello")],
        );
        manager.append_message("session.1", msg).unwrap();
        let history = manager.get_history("session.1").unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn conversation_manager_turn_tracking() {
        let mut manager = InMemoryConversationManager::new();
        assert_eq!(manager.current_turn("s1").unwrap(), 0);
        manager.begin_turn("s1").unwrap();
        assert_eq!(manager.current_turn("s1").unwrap(), 1);
        manager.begin_turn("s1").unwrap();
        assert_eq!(manager.current_turn("s1").unwrap(), 2);
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
