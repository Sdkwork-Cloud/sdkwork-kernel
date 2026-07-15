use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelResult,
    ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelStreamChunk, ProviderHealth, ProviderManifest, SessionKind, SessionSource, SessionState,
};
use sdkwork_agent_provider_core::{
    create_session_from_config, uuid_simple, MessageAdapter, SessionAdapter, SessionConfig,
};

#[cfg(test)]
use sdkwork_agent_kernel::KernelError;
#[cfg(test)]
use sdkwork_agent_provider_core::{
    ConversationManager, InMemoryConversationManager, SessionLifecycleProvider,
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
    pub ended_at: Option<String>,
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
        if external.ended_at.is_some() {
            session.state = SessionState::Closed;
        }
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
            default_model: "hermes-runtime-default".to_string(),
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
        vec![ModelDescriptor::new(
            "hermes-runtime-default",
            "provider.model.hermes",
            "Hermes Agent Runtime (configured model)",
            "hermes",
        )
        .with_version("0.17.0")
        .with_capability("chat")
        .with_capability("tool_call")
        .with_context_window_tokens(200000)
        .with_max_output_tokens(16384)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_response_format(ModelResponseFormat::Text)
        .with_tool_capability("function_calling")]
    }

    fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        sdkwork_agent_provider_core::reject_in_process_model_invoke("provider.model.hermes")
    }

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        sdkwork_agent_provider_core::reject_in_process_model_stream("provider.model.hermes")
    }
}

// ============================================================================
// Hermes Lifecycle Provider (existing, preserved)
// ============================================================================

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(HermesLifecycleProvider, "hermes");

pub mod sdk_integration;
pub use sdk_integration::{
    hermes_binding_manifest, HermesSdkIntegration, HERMES_PYTHON_PROBE_MODULE,
    HERMES_TUI_GATEWAY_MODULE,
};

mod agent_definition;
mod conformance;
pub mod ids;
mod manifest;
mod package;

pub use agent_definition::{hermes_agent_definition, hermes_agent_manifest};
pub use conformance::hermes_conformance_profile;
pub use manifest::{hermes_kernel_plugin_manifest, hermes_provider_manifests, HermesKernelPlugin};
pub use package::hermes_package_manifest;

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
            ended_at: None,
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
    fn maps_closed_when_ended_at_present() {
        let adapter = HermesAdapter::new();
        let mut ext = sample_hermes_session();
        ext.ended_at = Some("2026-01-02T00:00:00Z".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.state, SessionState::Closed);
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
                    function_name: "terminal".to_string(),
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
        assert_eq!(result.parts[0].name, Some("terminal".to_string()));
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
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "hermes-runtime-default");
    }

    #[test]
    fn model_provider_describe_model() {
        let provider = HermesModelProvider::new();
        let model = provider.describe_model("hermes-runtime-default").unwrap();
        assert_eq!(
            model.display_name,
            "Hermes Agent Runtime (configured model)"
        );
        assert_eq!(model.family, "hermes");
        assert!(model.supports_capability("chat"));
        assert!(model.supports_capability("tool_call"));
    }

    #[test]
    fn model_provider_invoke_requires_transport_worker() {
        let provider = HermesModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["What is Rust?".to_string()])
            .with_model_id("hermes-runtime-default");
        let error = provider
            .invoke(request)
            .expect_err("in-process invoke is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    #[test]
    fn model_provider_stream_requires_transport_worker() {
        let provider = HermesModelProvider::new();
        let request = ModelRequest::new("req.2", vec!["Hello".to_string()]);
        let error = provider
            .stream(request)
            .expect_err("in-process stream is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
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
