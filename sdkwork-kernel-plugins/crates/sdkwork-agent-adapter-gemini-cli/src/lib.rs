use sdkwork_agent_adapter_core::{
    create_session_from_config, now_iso, uuid_simple, ConversationManager,
    InMemoryConversationManager, MessageAdapter, SessionAdapter, SessionConfig,
    SessionLifecycleProvider,
};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, ModelStatus,
    ModelStreamChunk, ModelUsage, ProviderHealth, ProviderManifest, SessionKind, SessionSource,
    SessionState, SideEffectLevel, ToolCall, ToolCallStatus, ToolDescriptor, ToolProvider,
    ToolResult, ToolSchema,
};
use std::collections::HashMap;
use std::sync::Mutex;

// ============================================================================
// Gemini CLI Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeminiMessage {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum GeminiPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "function_call")]
    FunctionCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "function_response")]
    FunctionResponse {
        id: String,
        name: String,
        response: String,
    },
}

// ============================================================================
// Gemini CLI Session Adapter (existing, preserved)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeminiConversationKind {
    Main,
    Subagent,
}

#[derive(Debug, Clone)]
pub struct GeminiConversationRecord {
    pub session_id: String,
    pub start_time: Option<String>,
    pub last_updated: Option<String>,
    pub messages: Vec<String>,
    pub summary: Option<String>,
    pub kind: String,
    pub memory_scratchpad: Option<String>,
    pub model: Option<String>,
    pub title: Option<String>,
}

pub struct GeminiCliAdapter;

impl GeminiCliAdapter {
    pub fn new() -> Self {
        Self
    }

    fn map_kind(kind: &str) -> SessionKind {
        match kind.to_lowercase().as_str() {
            "main" => SessionKind::Main,
            "subagent" => SessionKind::Subagent,
            _ => SessionKind::Main,
        }
    }
}

impl Default for GeminiCliAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for GeminiCliAdapter {
    type ExternalSession = GeminiConversationRecord;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        let kind = Self::map_kind(&external.kind);

        let mut config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(kind);

        if let Some(ref model) = external.model {
            config = config.with_model(model);
        }
        if let Some(ref title) = external.title {
            config = config.with_title(title);
        }
        if let Some(ref scratchpad) = external.memory_scratchpad {
            config = config.with_metadata("memory_scratchpad", scratchpad);
        }

        let mut session = create_session_from_config(
            &external.session_id,
            None,
            None,
            None,
            config,
            external.start_time.as_deref().unwrap_or(""),
        );

        session.updated_at = external.last_updated.clone();
        session.message_count = external.messages.len() as u32;
        session.summary = external.summary.clone();

        Ok(session)
    }
}

// ============================================================================
// Gemini CLI Message Adapter
// ============================================================================

pub struct GeminiMessageAdapter;

impl GeminiMessageAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiMessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAdapter for GeminiMessageAdapter {
    type ExternalMessage = GeminiMessage;

    fn to_agent_message(&self, external: &Self::ExternalMessage) -> KernelResult<AgentMessage> {
        let role = match external.role.as_str() {
            "user" => AgentMessageRole::User,
            "model" => AgentMessageRole::Agent,
            "system" => AgentMessageRole::System,
            "function" => AgentMessageRole::Tool,
            _ => AgentMessageRole::Adapter,
        };

        let mut parts = Vec::new();

        for (i, part) in external.parts.iter().enumerate() {
            match part {
                GeminiPart::Text { text } => {
                    if !text.is_empty() {
                        parts.push(AgentPart::text(format!("gemini.text.{}", i), text));
                    }
                }
                GeminiPart::FunctionCall { id, name, args } => {
                    let mut p = AgentPart::tool_call_ref(format!("gemini.fc.{}", i), id);
                    p.name = Some(name.clone());
                    p.json = Some(args.to_string());
                    parts.push(p);
                }
                GeminiPart::FunctionResponse {
                    id,
                    name: _,
                    response,
                } => {
                    let mut p = AgentPart::text(format!("gemini.fr.{}", i), response);
                    p.tool_call_id = Some(id.clone());
                    p.kind = sdkwork_agent_kernel::AgentPartKind::ToolCallRef;
                    parts.push(p);
                }
            }
        }

        if parts.is_empty() {
            parts.push(AgentPart::text("gemini.empty", ""));
        }

        Ok(AgentMessage::new(
            format!("gemini.msg.{}", uuid_simple()),
            role,
            parts,
        ))
    }
}

// ============================================================================
// Gemini CLI Model Provider
// ============================================================================

pub struct GeminiModelProvider {
    default_model: String,
}

impl GeminiModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "gemini-2.5-pro".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for GeminiModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for GeminiModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.gemini",
            "model",
            "Gemini Model Provider",
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
                "gemini-2.5-pro",
                "provider.model.gemini",
                "Gemini 2.5 Pro",
                "gemini",
            )
            .with_version("2.5")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_context_window_tokens(1000000)
            .with_max_output_tokens(65536)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_response_format(ModelResponseFormat::Json)
            .with_tool_capability("function_calling"),
            ModelDescriptor::new(
                "gemini-2.5-flash",
                "provider.model.gemini",
                "Gemini 2.5 Flash",
                "gemini",
            )
            .with_version("2.5")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_context_window_tokens(1000000)
            .with_max_output_tokens(65536)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_tool_capability("function_calling"),
        ]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let model_id = request.model_id.as_deref().unwrap_or(&self.default_model);
        let prompt = request.messages.join("\n");

        Ok(ModelResponse::text(
            &request.model_request_id,
            "provider.model.gemini",
            format!("[Gemini {}] Mock response to: {}", model_id, prompt),
        )
        .with_usage(ModelUsage::new(prompt.len() as u32 / 4, 256))
        .with_finish_reason("stop"))
    }

    fn stream(&self, request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        let response_text = format!(
            "[Gemini] Streaming mock response to: {}",
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
// Gemini CLI Tool Provider
// ============================================================================

pub struct GeminiToolProvider;

impl GeminiToolProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for GeminiToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.gemini",
            "tool",
            "Gemini Tool Provider",
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
                "gemini.run_shell",
                "provider.tool.gemini",
                "Run Shell",
                SideEffectLevel::SideEffectful,
            )
            .with_name("run_shell")
            .with_description("Execute a shell command")
            .with_input_schema(ToolSchema::json_schema("gemini.run_shell.input"))
            .with_output_schema(ToolSchema::json_schema("gemini.run_shell.output")),
            ToolDescriptor::new(
                "gemini.read_file",
                "provider.tool.gemini",
                "Read File",
                SideEffectLevel::ReadOnly,
            )
            .with_name("read_file")
            .with_description("Read a file")
            .with_input_schema(ToolSchema::json_schema("gemini.read_file.input"))
            .with_output_schema(ToolSchema::json_schema("gemini.read_file.output")),
            ToolDescriptor::new(
                "gemini.write_file",
                "provider.tool.gemini",
                "Write File",
                SideEffectLevel::SideEffectful,
            )
            .with_name("write_file")
            .with_description("Write to a file")
            .with_input_schema(ToolSchema::json_schema("gemini.write_file.input"))
            .with_output_schema(ToolSchema::json_schema("gemini.write_file.output")),
            ToolDescriptor::new(
                "gemini.search_web",
                "provider.tool.gemini",
                "Search Web",
                SideEffectLevel::ExternalSend,
            )
            .with_name("search_web")
            .with_description("Search the web")
            .with_input_schema(ToolSchema::json_schema("gemini.search_web.input"))
            .with_output_schema(ToolSchema::json_schema("gemini.search_web.output")),
        ]
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        let output = match call.tool_id.as_str() {
            "gemini.run_shell" => {
                format!("[Gemini RunShell] Mock execution: {}", call.arguments)
            }
            "gemini.read_file" => {
                format!("[Gemini ReadFile] Mock read: {}", call.arguments)
            }
            "gemini.write_file" => {
                format!("[Gemini WriteFile] Mock write: {}", call.arguments)
            }
            "gemini.search_web" => {
                format!("[Gemini SearchWeb] Mock search: {}", call.arguments)
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
// Gemini CLI Lifecycle Provider (existing, preserved)
// ============================================================================

pub struct GeminiCliLifecycleProvider {
    sessions: Mutex<HashMap<String, AgentSession>>,
}

impl GeminiCliLifecycleProvider {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for GeminiCliLifecycleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionLifecycleProvider for GeminiCliLifecycleProvider {
    fn create_session(
        &self,
        agent_id: &str,
        user_ref: Option<&str>,
        config: SessionConfig,
    ) -> KernelResult<AgentSession> {
        let session_id = format!("gemini.{}", uuid_simple());
        let session = create_session_from_config(
            &session_id,
            Some(agent_id.to_string()),
            user_ref.map(String::from),
            None,
            config,
            now_iso(),
        );
        let mut sessions = self.sessions.lock().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })?;
        sessions.insert(session_id.clone(), session.clone());
        Ok(session)
    }

    fn resume_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        let mut sessions = self.sessions.lock().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))?;
        session.state = SessionState::Active;
        Ok(session.clone())
    }

    fn close_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        let mut sessions = self.sessions.lock().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {session_id}")))?;
        session.state = SessionState::Closed;
        Ok(session.clone())
    }

    fn list_active_sessions(&self) -> KernelResult<Vec<AgentSession>> {
        let sessions = self.sessions.lock().map_err(|e| KernelError::Internal {
            message: format!("lock poisoned: {e}"),
        })?;
        Ok(sessions
            .values()
            .filter(|s| s.state.is_active())
            .cloned()
            .collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gemini_record() -> GeminiConversationRecord {
        GeminiConversationRecord {
            session_id: "gemini.conv.1".to_string(),
            start_time: Some("2026-01-01T00:00:00Z".to_string()),
            last_updated: Some("2026-01-01T04:00:00Z".to_string()),
            messages: vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()],
            summary: Some("Discussed Rust patterns".to_string()),
            kind: "main".to_string(),
            memory_scratchpad: Some("remember: use traits".to_string()),
            model: Some("gemini-2.5-pro".to_string()),
            title: Some("Rust Help".to_string()),
        }
    }

    // --- Session Adapter Tests ---

    #[test]
    fn maps_basic_fields() {
        let adapter = GeminiCliAdapter::new();
        let ext = sample_gemini_record();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.session_id, "gemini.conv.1");
        assert_eq!(session.source, SessionSource::Cli);
        assert_eq!(session.kind, SessionKind::Main);
        assert_eq!(session.model, Some("gemini-2.5-pro".to_string()));
        assert_eq!(session.title, Some("Rust Help".to_string()));
        assert_eq!(session.created_at, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(session.updated_at, Some("2026-01-01T04:00:00Z".to_string()));
        assert_eq!(session.message_count, 3);
        assert_eq!(session.summary, Some("Discussed Rust patterns".to_string()));
    }

    #[test]
    fn maps_subagent_kind() {
        let adapter = GeminiCliAdapter::new();
        let mut ext = sample_gemini_record();
        ext.kind = "subagent".to_string();
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.kind, SessionKind::Subagent);
    }

    #[test]
    fn maps_memory_scratchpad_to_metadata() {
        let adapter = GeminiCliAdapter::new();
        let ext = sample_gemini_record();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| { k == "memory_scratchpad" && v == "remember: use traits" }));
    }

    #[test]
    fn derives_message_count_from_messages_len() {
        let adapter = GeminiCliAdapter::new();
        let mut ext = sample_gemini_record();
        ext.messages = vec!["a".to_string(), "b".to_string()];
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.message_count, 2);
    }

    #[test]
    fn lifecycle_provider_create_and_resume() {
        let provider = GeminiCliLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);
        let created = provider.create_session("agent.1", None, config).unwrap();

        let resumed = provider.resume_session(&created.session_id).unwrap();
        assert_eq!(resumed.state, SessionState::Active);

        let active = provider.list_active_sessions().unwrap();
        assert_eq!(active.len(), 1);
    }

    // --- Message Adapter Tests ---

    #[test]
    fn converts_user_text_message() {
        let adapter = GeminiMessageAdapter::new();
        let msg = GeminiMessage {
            role: "user".to_string(),
            parts: vec![GeminiPart::Text {
                text: "Hello Gemini".to_string(),
            }],
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::User);
        assert_eq!(result.parts.len(), 1);
        assert_eq!(result.parts[0].text, Some("Hello Gemini".to_string()));
    }

    #[test]
    fn converts_model_message() {
        let adapter = GeminiMessageAdapter::new();
        let msg = GeminiMessage {
            role: "model".to_string(),
            parts: vec![GeminiPart::Text {
                text: "I can help".to_string(),
            }],
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
    }

    #[test]
    fn converts_function_call() {
        let adapter = GeminiMessageAdapter::new();
        let msg = GeminiMessage {
            role: "model".to_string(),
            parts: vec![GeminiPart::FunctionCall {
                id: "fc.1".to_string(),
                name: "run_shell".to_string(),
                args: serde_json::json!({"cmd": "ls"}),
            }],
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.parts.len(), 1);
        assert_eq!(
            result.parts[0].kind,
            sdkwork_agent_kernel::AgentPartKind::ToolCallRef
        );
        assert_eq!(result.parts[0].name, Some("run_shell".to_string()));
        assert_eq!(result.parts[0].tool_call_id, Some("fc.1".to_string()));
    }

    #[test]
    fn converts_function_response() {
        let adapter = GeminiMessageAdapter::new();
        let msg = GeminiMessage {
            role: "function".to_string(),
            parts: vec![GeminiPart::FunctionResponse {
                id: "fc.1".to_string(),
                name: "run_shell".to_string(),
                response: "file1.txt\nfile2.rs".to_string(),
            }],
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Tool);
        assert_eq!(result.parts[0].tool_call_id, Some("fc.1".to_string()));
        assert_eq!(
            result.parts[0].text,
            Some("file1.txt\nfile2.rs".to_string())
        );
    }

    #[test]
    fn converts_batch_messages() {
        let adapter = GeminiMessageAdapter::new();
        let messages = vec![
            GeminiMessage {
                role: "user".to_string(),
                parts: vec![GeminiPart::Text {
                    text: "Hello".to_string(),
                }],
            },
            GeminiMessage {
                role: "model".to_string(),
                parts: vec![GeminiPart::Text {
                    text: "Hi".to_string(),
                }],
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
        let provider = GeminiModelProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.model.gemini");
        assert_eq!(manifest.provider_family, "model");
    }

    #[test]
    fn model_provider_health() {
        let provider = GeminiModelProvider::new();
        assert_eq!(provider.health().status, "available");
    }

    #[test]
    fn model_provider_list_models() {
        let provider = GeminiModelProvider::new();
        let models = provider.list_models();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_id, "gemini-2.5-pro");
        assert_eq!(models[1].model_id, "gemini-2.5-flash");
    }

    #[test]
    fn model_provider_describe_model() {
        let provider = GeminiModelProvider::new();
        let model = provider.describe_model("gemini-2.5-pro").unwrap();
        assert_eq!(model.display_name, "Gemini 2.5 Pro");
        assert_eq!(model.family, "gemini");
        assert!(model.supports_capability("chat"));
        assert!(model.supports_capability("tool_call"));
    }

    #[test]
    fn model_provider_invoke() {
        let provider = GeminiModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["Explain Rust".to_string()])
            .with_model_id("gemini-2.5-pro");
        let response = provider.invoke(request).unwrap();
        assert_eq!(response.status, ModelStatus::Succeeded);
        assert_eq!(response.provider_id, "provider.model.gemini");
        assert!(response.messages[0].contains("Explain Rust"));
        assert!(response.usage.is_some());
    }

    #[test]
    fn model_provider_stream() {
        let provider = GeminiModelProvider::new();
        let request = ModelRequest::new("req.2", vec!["Hello".to_string()]);
        let chunks = provider.stream(request).unwrap();
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].sequence, 0);
        assert!(chunks.last().unwrap().sequence > 0);
    }

    // --- Tool Provider Tests ---

    #[test]
    fn tool_provider_manifest() {
        let provider = GeminiToolProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.tool.gemini");
        assert_eq!(manifest.provider_family, "tool");
    }

    #[test]
    fn tool_provider_list_tools() {
        let provider = GeminiToolProvider::new();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.tool_id == "gemini.run_shell"));
        assert!(tools.iter().any(|t| t.tool_id == "gemini.read_file"));
        assert!(tools.iter().any(|t| t.tool_id == "gemini.write_file"));
        assert!(tools.iter().any(|t| t.tool_id == "gemini.search_web"));

        let read = tools
            .iter()
            .find(|t| t.tool_id == "gemini.read_file")
            .unwrap();
        assert_eq!(read.side_effect_level, SideEffectLevel::ReadOnly);
        let search = tools
            .iter()
            .find(|t| t.tool_id == "gemini.search_web")
            .unwrap();
        assert_eq!(search.side_effect_level, SideEffectLevel::ExternalSend);
    }

    #[test]
    fn tool_provider_invoke_success() {
        let provider = GeminiToolProvider::new();
        let call = ToolCall::new("c.1", "gemini.run_shell", r#"{"cmd":"echo hello"}"#);
        let result = provider.invoke_tool(call).unwrap();
        assert_eq!(result.normalized_status, ToolCallStatus::Succeeded);
        assert!(result.output.contains("Mock execution"));
    }

    #[test]
    fn tool_provider_invoke_unknown() {
        let provider = GeminiToolProvider::new();
        let call = ToolCall::new("c.2", "gemini.nonexistent", "{}");
        assert!(provider.invoke_tool(call).is_err());
    }

    #[test]
    fn tool_provider_describe_tool() {
        let provider = GeminiToolProvider::new();
        let desc = provider.describe_tool("gemini.search_web").unwrap();
        assert_eq!(desc.display_name, "Search Web");
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
        assert_eq!(manager.current_turn("s1").unwrap(), 0);
        manager.begin_turn("s1").unwrap();
        assert_eq!(manager.current_turn("s1").unwrap(), 1);
        manager.begin_turn("s1").unwrap();
        assert_eq!(manager.current_turn("s1").unwrap(), 2);
    }

    #[test]
    fn conversation_manager_clear_history() {
        let mut manager = InMemoryConversationManager::new();
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
        manager.begin_turn("s1").unwrap();
        manager.clear_history("s1").unwrap();
        assert!(manager.get_history("s1").unwrap().is_empty());
        assert_eq!(manager.current_turn("s1").unwrap(), 0);
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
