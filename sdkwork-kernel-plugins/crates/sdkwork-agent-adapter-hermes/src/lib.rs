use sdkwork_agent_adapter_core::{
    create_session_from_config, now_iso, reject_direct_mock_provider_invocation, uuid_simple,
    ConversationManager, InMemoryConversationManager, MessageAdapter, SessionAdapter,
    SessionConfig, SessionLifecycleProvider,
};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, ModelStatus,
    ModelStreamChunk, ModelUsage, ProviderHealth, ProviderManifest, SessionKind, SessionSource,
    SessionState, SideEffectLevel, ToolCall, ToolCallStatus, ToolDescriptor, ToolProvider,
    ToolResult, ToolSchema,
};

// ============================================================================
// Hermes Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HermesMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<HermesToolCall>>,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HermesToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

// ============================================================================
// Hermes Session Adapter (existing, preserved)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HermesSource {
    Cli,
    Telegram,
    Web,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HermesSessionInfo {
    pub id: String,
    pub source: String,
    pub model: Option<String>,
    pub title: Option<String>,
    pub started_at: Option<String>,
    pub last_active: Option<String>,
    pub message_count: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub preview: Option<String>,
    pub parent_session_id: Option<String>,
    pub skills: Vec<String>,
    pub tools: Vec<String>,
}

pub struct HermesAdapter;

impl HermesAdapter {
    pub fn new() -> Self {
        Self
    }

    fn map_source(source: &str) -> SessionSource {
        match source.to_lowercase().as_str() {
            "cli" => SessionSource::Cli,
            "telegram" => SessionSource::Telegram,
            "web" => SessionSource::Web,
            _ => SessionSource::Unknown,
        }
    }
}

impl Default for HermesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for HermesAdapter {
    type ExternalSession = HermesSessionInfo;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        let source = Self::map_source(&external.source);
        let kind = if external.parent_session_id.is_some() {
            SessionKind::Subagent
        } else {
            SessionKind::Main
        };

        let mut config = SessionConfig::new().with_source(source).with_kind(kind);

        if let Some(ref model) = external.model {
            config = config.with_model(model);
        }
        if let Some(ref title) = external.title {
            config = config.with_title(title);
        }

        for skill in &external.skills {
            config = config.with_metadata("skill", skill);
        }
        for tool in &external.tools {
            config = config.with_metadata("tool", tool);
        }

        let mut session = create_session_from_config(
            &external.id,
            None,
            None,
            None,
            config,
            external.started_at.as_deref().unwrap_or(""),
        );

        session.updated_at = external.last_active.clone();
        session.message_count = external.message_count;
        session.preview = external.preview.clone();
        session.parent_session_id = external.parent_session_id.clone();
        session.token_usage.input_tokens = external.input_tokens;
        session.token_usage.output_tokens = external.output_tokens;
        session.token_usage.total_tokens = external.input_tokens + external.output_tokens;

        Ok(session)
    }
}

// ============================================================================
// Hermes Message Adapter
// ============================================================================

pub struct HermesMessageAdapter;

impl HermesMessageAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HermesMessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAdapter for HermesMessageAdapter {
    type ExternalMessage = HermesMessage;

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
            parts.push(AgentPart::text("hermes.content", &external.content));
        }

        if let Some(tool_calls) = &external.tool_calls {
            for tc in tool_calls {
                let mut part =
                    AgentPart::tool_call_ref(format!("hermes.tool_call.{}", tc.id), &tc.id);
                part.name = Some(tc.function_name.clone());
                part.json = Some(tc.arguments.clone());
                parts.push(part);
            }
        }

        if parts.is_empty() {
            parts.push(AgentPart::text("hermes.empty", ""));
        }

        let mut message = AgentMessage::new(format!("hermes.msg.{}", uuid_simple()), role, parts);

        if let Some(tool_call_id) = &external.tool_call_id {
            message = message.with_metadata("hermes.tool_call_id", tool_call_id);
        }
        if let Some(tool_name) = &external.tool_name {
            message = message.with_metadata("hermes.tool_name", tool_name);
        }
        if let Some(timestamp) = &external.timestamp {
            message = message.created_at(timestamp);
        }

        Ok(message)
    }
}

// ============================================================================
// Hermes Model Provider
// ============================================================================

pub struct HermesModelProvider {
    default_model: String,
}

impl HermesModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "hermes-3-llama-3.1-70b".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for HermesModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for HermesModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.hermes",
            "model",
            "Hermes Model Provider",
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
        vec![
            ModelDescriptor::new(
                "hermes-3-llama-3.1-70b",
                "provider.model.hermes",
                "Hermes 3 Llama 3.1 70B",
                "hermes",
            )
            .with_version("3.0")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_context_window_tokens(128000)
            .with_max_output_tokens(4096)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_tool_capability("function_calling"),
            ModelDescriptor::new(
                "hermes-3-llama-3.1-8b",
                "provider.model.hermes",
                "Hermes 3 Llama 3.1 8B",
                "hermes",
            )
            .with_version("3.0")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_context_window_tokens(128000)
            .with_max_output_tokens(4096)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_tool_capability("function_calling"),
            ModelDescriptor::new(
                "hermes-3-mistral-7b",
                "provider.model.hermes",
                "Hermes 3 Mistral 7B",
                "hermes",
            )
            .with_version("3.0")
            .with_capability("chat")
            .with_context_window_tokens(32000)
            .with_max_output_tokens(4096)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text),
        ]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        reject_direct_mock_provider_invocation("provider.model.hermes.invoke")?;

        let model_id = request.model_id.as_deref().unwrap_or(&self.default_model);
        let prompt = request.messages.join("\n");

        Ok(ModelResponse::text(
            &request.model_request_id,
            "provider.model.hermes",
            format!("[Hermes {}] Mock response to: {}", model_id, prompt),
        )
        .with_usage(ModelUsage::new(prompt.len() as u32 / 4, 128))
        .with_finish_reason("stop"))
    }

    fn stream(&self, request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        reject_direct_mock_provider_invocation("provider.model.hermes.stream")?;

        let response_text = format!(
            "[Hermes] Streaming mock response to: {}",
            request.messages.join(" ")
        );
        let words: Vec<&str> = response_text.split_whitespace().collect();
        let chunks = words
            .into_iter()
            .enumerate()
            .map(|(i, word)| {
                ModelStreamChunk::output(&request.model_request_id, i as u64, format!("{} ", word))
            })
            .collect();

        Ok(chunks)
    }
}

// ============================================================================
// Hermes Tool Provider
// ============================================================================

pub struct HermesToolProvider;

impl HermesToolProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HermesToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for HermesToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.hermes",
            "tool",
            "Hermes Tool Provider",
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
                "hermes.bash",
                "provider.tool.hermes",
                "Bash",
                SideEffectLevel::SideEffectful,
            )
            .with_name("bash")
            .with_description("Execute a bash command")
            .with_input_schema(ToolSchema::json_schema("hermes.bash.input"))
            .with_output_schema(ToolSchema::json_schema("hermes.bash.output")),
            ToolDescriptor::new(
                "hermes.read_file",
                "provider.tool.hermes",
                "Read File",
                SideEffectLevel::ReadOnly,
            )
            .with_name("read_file")
            .with_description("Read a file from the filesystem")
            .with_input_schema(ToolSchema::json_schema("hermes.read_file.input"))
            .with_output_schema(ToolSchema::json_schema("hermes.read_file.output")),
            ToolDescriptor::new(
                "hermes.write_file",
                "provider.tool.hermes",
                "Write File",
                SideEffectLevel::SideEffectful,
            )
            .with_name("write_file")
            .with_description("Write content to a file")
            .with_input_schema(ToolSchema::json_schema("hermes.write_file.input"))
            .with_output_schema(ToolSchema::json_schema("hermes.write_file.output")),
            ToolDescriptor::new(
                "hermes.list_directory",
                "provider.tool.hermes",
                "List Directory",
                SideEffectLevel::ReadOnly,
            )
            .with_name("list_directory")
            .with_description("List files in a directory")
            .with_input_schema(ToolSchema::json_schema("hermes.list_directory.input"))
            .with_output_schema(ToolSchema::json_schema("hermes.list_directory.output")),
            ToolDescriptor::new(
                "hermes.web_search",
                "provider.tool.hermes",
                "Web Search",
                SideEffectLevel::ExternalSend,
            )
            .with_name("web_search")
            .with_description("Search the web")
            .with_input_schema(ToolSchema::json_schema("hermes.web_search.input"))
            .with_output_schema(ToolSchema::json_schema("hermes.web_search.output")),
        ]
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        let output = match call.tool_id.as_str() {
            "hermes.bash" => {
                format!("[Hermes Bash] Mock execution of: {}", call.arguments)
            }
            "hermes.read_file" => {
                format!("[Hermes ReadFile] Mock read of: {}", call.arguments)
            }
            "hermes.write_file" => {
                format!("[Hermes WriteFile] Mock write to: {}", call.arguments)
            }
            "hermes.list_directory" => {
                "[Hermes ListDirectory] file1.txt\nfile2.rs\ndir/".to_string()
            }
            "hermes.web_search" => {
                format!(
                    "[Hermes WebSearch] Mock search results for: {}",
                    call.arguments
                )
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
// Hermes Lifecycle Provider (existing, preserved)
// ============================================================================

sdkwork_agent_adapter_core::define_provider_lifecycle_provider!(HermesLifecycleProvider, "hermes");

pub mod sdk_integration;
pub use sdk_integration::{hermes_binding_manifest, HermesSdkIntegration};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hermes_session() -> HermesSessionInfo {
        HermesSessionInfo {
            id: "hermes.session.1".to_string(),
            source: "cli".to_string(),
            model: Some("gpt-4".to_string()),
            title: Some("Test Session".to_string()),
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            last_active: Some("2026-01-01T01:00:00Z".to_string()),
            message_count: 42,
            input_tokens: 1000,
            output_tokens: 500,
            preview: Some("Hello world".to_string()),
            parent_session_id: None,
            skills: vec!["coding".to_string()],
            tools: vec!["bash".to_string(), "read".to_string()],
        }
    }

    // --- Session Adapter Tests ---

    #[test]
    fn maps_basic_fields() {
        let adapter = HermesAdapter::new();
        let ext = sample_hermes_session();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.session_id, "hermes.session.1");
        assert_eq!(session.source, SessionSource::Cli);
        assert_eq!(session.kind, SessionKind::Main);
        assert_eq!(session.model, Some("gpt-4".to_string()));
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert_eq!(session.created_at, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(session.updated_at, Some("2026-01-01T01:00:00Z".to_string()));
        assert_eq!(session.message_count, 42);
        assert_eq!(session.token_usage.input_tokens, 1000);
        assert_eq!(session.token_usage.output_tokens, 500);
        assert_eq!(session.token_usage.total_tokens, 1500);
        assert_eq!(session.preview, Some("Hello world".to_string()));
    }

    #[test]
    fn maps_subagent_when_parent_exists() {
        let adapter = HermesAdapter::new();
        let mut ext = sample_hermes_session();
        ext.parent_session_id = Some("parent.1".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.kind, SessionKind::Subagent);
        assert_eq!(session.parent_session_id, Some("parent.1".to_string()));
    }

    #[test]
    fn maps_telegram_source() {
        let adapter = HermesAdapter::new();
        let mut ext = sample_hermes_session();
        ext.source = "telegram".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.source, SessionSource::Telegram);
    }

    #[test]
    fn maps_skills_and_tools_to_metadata() {
        let adapter = HermesAdapter::new();
        let ext = sample_hermes_session();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| k == "skill" && v == "coding"));
        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| k == "tool" && v == "bash"));
        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| k == "tool" && v == "read"));
    }

    // --- Message Adapter Tests ---

    #[test]
    fn converts_user_message() {
        let adapter = HermesMessageAdapter::new();
        let msg = HermesMessage {
            role: "user".to_string(),
            content: "Hello Hermes".to_string(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            timestamp: Some("2026-01-01T00:00:00Z".to_string()),
        };

        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::User);
        assert_eq!(result.parts.len(), 1);
        assert_eq!(result.parts[0].text, Some("Hello Hermes".to_string()));
        assert_eq!(result.created_at, Some("2026-01-01T00:00:00Z".to_string()));
    }

    #[test]
    fn converts_assistant_message() {
        let adapter = HermesMessageAdapter::new();
        let msg = HermesMessage {
            role: "assistant".to_string(),
            content: "I can help with that".to_string(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            timestamp: None,
        };

        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
    }

    #[test]
    fn converts_system_message() {
        let adapter = HermesMessageAdapter::new();
        let msg = HermesMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant".to_string(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            timestamp: None,
        };

        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::System);
    }

    #[test]
    fn converts_tool_message() {
        let adapter = HermesMessageAdapter::new();
        let msg = HermesMessage {
            role: "tool".to_string(),
            content: "file contents here".to_string(),
            tool_calls: None,
            tool_call_id: Some("call.123".to_string()),
            tool_name: Some("read_file".to_string()),
            timestamp: None,
        };

        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Tool);
        assert_eq!(
            result.metadata_value("hermes.tool_call_id"),
            Some("call.123")
        );
        assert_eq!(result.metadata_value("hermes.tool_name"), Some("read_file"));
    }

    #[test]
    fn converts_message_with_tool_calls() {
        let adapter = HermesMessageAdapter::new();
        let msg = HermesMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![
                HermesToolCall {
                    id: "call.1".to_string(),
                    function_name: "bash".to_string(),
                    arguments: r#"{"command":"ls"}"#.to_string(),
                },
                HermesToolCall {
                    id: "call.2".to_string(),
                    function_name: "read_file".to_string(),
                    arguments: r#"{"path":"/tmp/test.txt"}"#.to_string(),
                },
            ]),
            tool_call_id: None,
            tool_name: None,
            timestamp: None,
        };

        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
        assert_eq!(result.parts.len(), 2);
        assert_eq!(
            result.parts[0].kind,
            sdkwork_agent_kernel::AgentPartKind::ToolCallRef
        );
        assert_eq!(result.parts[0].tool_call_id, Some("call.1".to_string()));
        assert_eq!(result.parts[0].name, Some("bash".to_string()));
        assert_eq!(result.parts[1].tool_call_id, Some("call.2".to_string()));
        assert_eq!(result.parts[1].name, Some("read_file".to_string()));
    }

    #[test]
    fn converts_batch_messages() {
        let adapter = HermesMessageAdapter::new();
        let messages = vec![
            HermesMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
                tool_name: None,
                timestamp: None,
            },
            HermesMessage {
                role: "assistant".to_string(),
                content: "Hi there".to_string(),
                tool_calls: None,
                tool_call_id: None,
                tool_name: None,
                timestamp: None,
            },
        ];

        let result = adapter.to_agent_messages(&messages).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, AgentMessageRole::User);
        assert_eq!(result[1].role, AgentMessageRole::Agent);
    }

    // --- Model Provider Tests ---

    #[test]
    fn model_provider_manifest() {
        let provider = HermesModelProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.model.hermes");
        assert_eq!(manifest.provider_family, "model");
        assert_eq!(manifest.name, "Hermes Model Provider");
        assert!(manifest.capabilities.contains(&"model.chat".to_string()));
    }

    #[test]
    fn model_provider_health() {
        let provider = HermesModelProvider::new();
        assert_eq!(provider.health().status, "available");
    }

    #[test]
    fn model_provider_list_models() {
        let provider = HermesModelProvider::new();
        let models = provider.list_models();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].model_id, "hermes-3-llama-3.1-70b");
        assert_eq!(models[1].model_id, "hermes-3-llama-3.1-8b");
        assert_eq!(models[2].model_id, "hermes-3-mistral-7b");
    }

    #[test]
    fn model_provider_describe_model() {
        let provider = HermesModelProvider::new();
        let model = provider.describe_model("hermes-3-llama-3.1-70b").unwrap();
        assert_eq!(model.display_name, "Hermes 3 Llama 3.1 70B");
        assert_eq!(model.family, "hermes");
        assert!(model.supports_capability("chat"));
        assert!(model.supports_capability("tool_call"));
    }

    #[test]
    fn model_provider_invoke_mock() {
        let provider = HermesModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["What is Rust?".to_string()])
            .with_model_id("hermes-3-llama-3.1-70b");

        let response = provider.invoke(request).unwrap();
        assert_eq!(response.status, ModelStatus::Succeeded);
        assert_eq!(response.provider_id, "provider.model.hermes");
        assert!(!response.messages.is_empty());
        assert!(response.messages[0].contains("What is Rust?"));
        assert!(response.usage.is_some());
    }

    #[test]
    fn model_provider_stream_mock() {
        let provider = HermesModelProvider::new();
        let request = ModelRequest::new("req.2", vec!["Hello".to_string()]);

        let chunks = provider.stream(request).unwrap();
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].sequence, 0);
        assert!(chunks.last().unwrap().sequence > 0);
    }

    // --- Tool Provider Tests ---

    #[test]
    fn tool_provider_manifest() {
        let provider = HermesToolProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.tool.hermes");
        assert_eq!(manifest.provider_family, "tool");
    }

    #[test]
    fn tool_provider_list_tools() {
        let provider = HermesToolProvider::new();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 5);

        let bash = tools.iter().find(|t| t.tool_id == "hermes.bash").unwrap();
        assert_eq!(bash.display_name, "Bash");
        assert_eq!(bash.side_effect_level, SideEffectLevel::SideEffectful);

        let read = tools
            .iter()
            .find(|t| t.tool_id == "hermes.read_file")
            .unwrap();
        assert_eq!(read.side_effect_level, SideEffectLevel::ReadOnly);

        let write = tools
            .iter()
            .find(|t| t.tool_id == "hermes.write_file")
            .unwrap();
        assert_eq!(write.side_effect_level, SideEffectLevel::SideEffectful);
    }

    #[test]
    fn tool_provider_invoke_bash() {
        let provider = HermesToolProvider::new();
        let call = ToolCall::new("call.1", "hermes.bash", r#"{"command":"ls"}"#);
        let result = provider.invoke_tool(call).unwrap();
        assert_eq!(result.normalized_status, ToolCallStatus::Succeeded);
        assert!(result.output.contains("Mock execution"));
    }

    #[test]
    fn tool_provider_invoke_read_file() {
        let provider = HermesToolProvider::new();
        let call = ToolCall::new("call.2", "hermes.read_file", r#"{"path":"/tmp/test.txt"}"#);
        let result = provider.invoke_tool(call).unwrap();
        assert_eq!(result.normalized_status, ToolCallStatus::Succeeded);
    }

    #[test]
    fn tool_provider_invoke_unknown_tool() {
        let provider = HermesToolProvider::new();
        let call = ToolCall::new("call.3", "hermes.nonexistent", "{}");
        let result = provider.invoke_tool(call);
        assert!(result.is_err());
    }

    #[test]
    fn tool_provider_describe_tool() {
        let provider = HermesToolProvider::new();
        let desc = provider.describe_tool("hermes.web_search").unwrap();
        assert_eq!(desc.display_name, "Web Search");
        assert_eq!(desc.side_effect_level, SideEffectLevel::ExternalSend);
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
        assert_eq!(history[0].message_id, "msg.1");
    }

    #[test]
    fn conversation_manager_turn_tracking() {
        let mut manager = InMemoryConversationManager::new();

        assert_eq!(manager.current_turn("session.1").unwrap(), 0);

        manager.begin_turn("session.1").unwrap();
        assert_eq!(manager.current_turn("session.1").unwrap(), 1);

        manager.begin_turn("session.1").unwrap();
        assert_eq!(manager.current_turn("session.1").unwrap(), 2);
    }

    #[test]
    fn conversation_manager_clear_history() {
        let mut manager = InMemoryConversationManager::new();
        let msg = AgentMessage::new(
            "msg.1",
            AgentMessageRole::User,
            vec![AgentPart::text("p1", "Hello")],
        );

        manager.append_message("session.1", msg).unwrap();
        manager.begin_turn("session.1").unwrap();

        manager.clear_history("session.1").unwrap();
        assert!(manager.get_history("session.1").unwrap().is_empty());
        assert_eq!(manager.current_turn("session.1").unwrap(), 0);
    }

    #[test]
    fn conversation_manager_empty_history() {
        let manager = InMemoryConversationManager::new();
        let history = manager.get_history("nonexistent").unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn conversation_manager_compress_history() {
        let mut manager = InMemoryConversationManager::new();

        for i in 0..10 {
            let msg = AgentMessage::new(
                format!("msg.{}", i),
                AgentMessageRole::User,
                vec![AgentPart::text(
                    format!("p.{}", i),
                    format!("Message {}", i),
                )],
            );
            manager.append_message("session.1", msg).unwrap();
        }

        let compressed = manager.compress_history("session.1", 100).unwrap();
        assert_eq!(compressed.role, AgentMessageRole::System);
        assert!(compressed.parts[0]
            .text
            .as_ref()
            .unwrap()
            .contains("Compressed"));

        let remaining = manager.get_history("session.1").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].role, AgentMessageRole::System);
    }

    // --- Lifecycle Provider Tests ---

    #[test]
    fn lifecycle_provider_create_and_list() {
        let provider = HermesLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_title("Test")
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);

        let session = provider
            .create_session("agent.1", Some("user.1"), config)
            .unwrap();
        assert_eq!(session.agent_id, Some("agent.1".to_string()));
        assert_eq!(session.user_ref, Some("user.1".to_string()));

        provider.resume_session(&session.session_id).unwrap();
        let active = provider.list_active_sessions().unwrap();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn lifecycle_provider_resume_and_close() {
        let provider = HermesLifecycleProvider::new();
        let config = SessionConfig::new().with_source(SessionSource::Cli);
        let created = provider.create_session("agent.1", None, config).unwrap();

        let resumed = provider.resume_session(&created.session_id).unwrap();
        assert_eq!(resumed.state, SessionState::Active);

        let closed = provider.close_session(&created.session_id).unwrap();
        assert_eq!(closed.state, SessionState::Closed);
    }
}
