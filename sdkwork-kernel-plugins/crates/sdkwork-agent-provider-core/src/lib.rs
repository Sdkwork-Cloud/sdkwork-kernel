use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    ModelRequest, ModelResponse, ModelStreamChunk, PolicyDecision, PolicyRequest, SessionKind,
    SessionSource, ToolCall, ToolDescriptor, ToolResult,
};
use std::collections::HashMap;

mod mock_policy;
mod model_wire;
mod provider_session_store;

pub use mock_policy::{
    is_mock_response_text, is_production_kernel_profile, kernel_profile_id,
    mock_provider_invocation_allowed, reject_direct_mock_provider_invocation,
    reject_in_process_model_invoke, reject_in_process_model_stream, reject_in_process_tool_invoke,
    validate_runtime_model_payload,
};
pub use model_wire::{
    build_model_chat_operation, model_request_has_structured_input, resolve_model_wire_messages,
    wire_messages_summary, wire_messages_to_anthropic_json, wire_messages_to_openai_json,
    wire_system_text, ModelWireMessage,
};
pub use provider_session_store::{
    sort_sessions_by_updated_at, InMemoryProviderSessionStore, ProviderSessionChange,
    ProviderSessionChangeBatch, ProviderSessionChangeKind, SessionListQuery,
};

/// Joins model request messages for legacy text-only provider adapters.
///
/// When structured `input_messages` are present, returns a wire summary that
/// preserves multimodal part markers instead of lossy flattening.
pub fn model_request_prompt(request: &ModelRequest) -> String {
    if model_request_has_structured_input(request) {
        if let Ok(wire) = resolve_model_wire_messages(request) {
            return wire_messages_summary(&wire);
        }
    }
    request.effective_prompt_text()
}

/// Streaming-friendly token split for legacy text-only provider adapters.
pub fn model_request_prompt_tokens(request: &ModelRequest) -> Vec<String> {
    request
        .effective_prompt_text()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

// ============================================================================
// Session Adapter
// ============================================================================

/// Trait for converting external agent session models to unified AgentSession
pub trait SessionAdapter {
    type ExternalSession;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession>;

    // Keep the public bidirectional SPI name; reverse conversion may depend on adapter state.
    #[allow(clippy::wrong_self_convention)]
    fn from_agent_session(&self, _session: &AgentSession) -> KernelResult<Self::ExternalSession> {
        Err(KernelError::validation("reverse conversion not supported"))
    }
}

/// Trait for providing session lifecycle operations
pub trait SessionLifecycleProvider {
    fn create_session(
        &self,
        agent_id: &str,
        user_ref: Option<&str>,
        config: SessionConfig,
    ) -> KernelResult<AgentSession>;

    fn resume_session(&self, session_id: &str) -> KernelResult<AgentSession>;

    fn close_session(&self, session_id: &str) -> KernelResult<AgentSession>;

    fn list_active_sessions(&self) -> KernelResult<Vec<AgentSession>>;

    fn get_session(&self, _session_id: &str) -> KernelResult<AgentSession> {
        Err(KernelError::validation(
            "get_session requires a provider session store implementation",
        ))
    }

    /// Read a session without conflating an absent provider-native snapshot
    /// with a provider transport or storage failure.
    fn find_session(&self, session_id: &str) -> KernelResult<Option<AgentSession>> {
        self.get_session(session_id).map(Some)
    }

    fn update_session(&self, _session: AgentSession) -> KernelResult<AgentSession> {
        Err(KernelError::validation(
            "update_session requires a provider session store implementation",
        ))
    }

    fn delete_session(&self, _session_id: &str) -> KernelResult<AgentSession> {
        Err(KernelError::validation(
            "delete_session requires a provider session store implementation",
        ))
    }

    /// Upsert a snapshot discovered from a provider-native session API.
    fn synchronize_session(&self, _session: AgentSession) -> KernelResult<AgentSession> {
        Err(KernelError::validation(
            "synchronize_session requires a provider session store implementation",
        ))
    }

    /// Read ordered lifecycle changes after a provider-local sequence cursor.
    fn session_changes(
        &self,
        _after_sequence: u64,
        _limit: Option<usize>,
    ) -> KernelResult<ProviderSessionChangeBatch> {
        Err(KernelError::validation(
            "session_changes requires a provider session store implementation",
        ))
    }

    /// List persisted sessions for this provider, sorted by `updated_at` descending.
    ///
    /// Implementations must override this method; the default rejects in-process listing.
    fn list_sessions(&self, _query: &SessionListQuery) -> KernelResult<Vec<AgentSession>> {
        Err(KernelError::validation(
            "list_sessions requires a provider session store implementation",
        ))
    }

    /// Load conversation history for a persisted session.
    fn get_conversation_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        let _ = session_id;
        Err(KernelError::validation(
            "conversation history not supported for this provider",
        ))
    }

    /// Append a message to a persisted session conversation.
    fn append_conversation_message(
        &self,
        session_id: &str,
        message: AgentMessage,
    ) -> KernelResult<()> {
        let _ = (session_id, message);
        Err(KernelError::validation(
            "conversation append not supported for this provider",
        ))
    }
}

// ============================================================================
// Message Adapter - bidirectional message conversion
// ============================================================================

/// Trait for converting external agent messages to kernel AgentMessage
pub trait MessageAdapter {
    type ExternalMessage;

    fn to_agent_message(&self, external: &Self::ExternalMessage) -> KernelResult<AgentMessage>;

    // Keep the public bidirectional SPI name; reverse conversion may depend on adapter state.
    #[allow(clippy::wrong_self_convention)]
    fn from_agent_message(&self, _message: &AgentMessage) -> KernelResult<Self::ExternalMessage> {
        Err(KernelError::validation(
            "reverse message conversion not supported",
        ))
    }

    fn to_agent_messages(
        &self,
        externals: &[Self::ExternalMessage],
    ) -> KernelResult<Vec<AgentMessage>> {
        externals.iter().map(|m| self.to_agent_message(m)).collect()
    }
}

// ============================================================================
// Model Adapter - model request/response conversion
// ============================================================================

/// Trait for converting external model requests to kernel ModelRequest
pub trait ModelAdapter {
    type ExternalRequest;
    type ExternalResponse;

    fn to_model_request(&self, external: &Self::ExternalRequest) -> KernelResult<ModelRequest>;

    fn to_model_response(
        &self,
        request_id: &str,
        external: &Self::ExternalResponse,
    ) -> KernelResult<ModelResponse>;
}

/// Trait for handling streaming model responses
pub trait StreamAdapter {
    type ExternalChunk;

    fn to_stream_chunk(
        &self,
        request_id: &str,
        sequence: u32,
        external: &Self::ExternalChunk,
    ) -> KernelResult<ModelStreamChunk>;
}

// ============================================================================
// Tool Adapter - tool definition and execution conversion
// ============================================================================

/// Trait for converting external tool definitions to kernel ToolDescriptor
pub trait ToolAdapter {
    type ExternalToolDef;
    type ExternalToolCall;
    type ExternalToolResult;

    fn to_tool_descriptor(&self, external: &Self::ExternalToolDef) -> KernelResult<ToolDescriptor>;

    fn to_tool_call(&self, external: &Self::ExternalToolCall) -> KernelResult<ToolCall>;

    fn to_tool_result(
        &self,
        call_id: &str,
        external: &Self::ExternalToolResult,
    ) -> KernelResult<ToolResult>;

    fn to_tool_descriptors(
        &self,
        externals: &[Self::ExternalToolDef],
    ) -> KernelResult<Vec<ToolDescriptor>> {
        externals
            .iter()
            .map(|t| self.to_tool_descriptor(t))
            .collect()
    }
}

// ============================================================================
// Policy Adapter - permission/approval model conversion
// ============================================================================

/// Trait for converting external permission models to kernel policy
pub trait PolicyAdapter {
    type ExternalPermission;

    fn to_policy_request(&self, external: &Self::ExternalPermission)
        -> KernelResult<PolicyRequest>;

    fn map_policy_decision(
        &self,
        _decision: &PolicyDecision,
    ) -> KernelResult<Self::ExternalPermission> {
        Err(KernelError::validation(
            "reverse policy conversion not supported",
        ))
    }
}

// ============================================================================
// Agent Runtime - complete agent runtime abstraction
// ============================================================================

/// Complete agent runtime that combines all adapter traits
pub trait AgentRuntimeAdapter {
    /// Get the agent ID
    fn agent_id(&self) -> &str;

    /// Get the agent name
    fn agent_name(&self) -> &str;

    /// Get supported model IDs
    fn supported_models(&self) -> Vec<String>;

    /// Get supported tool names
    fn supported_tools(&self) -> Vec<String>;

    /// Check if the agent supports streaming
    fn supports_streaming(&self) -> bool;

    /// Check if the agent supports tool calls
    fn supports_tool_calls(&self) -> bool;

    /// Check if the agent supports multi-turn conversation
    fn supports_multi_turn(&self) -> bool;

    /// Check if the agent supports sub-agents
    fn supports_sub_agents(&self) -> bool;
}

// ============================================================================
// Conversation Manager - multi-turn conversation abstraction
// ============================================================================

/// Trait for managing multi-turn conversations
pub trait ConversationManager {
    /// Start a new conversation turn
    fn begin_turn(&mut self, session_id: &str) -> KernelResult<()>;

    /// End the current conversation turn
    fn end_turn(&mut self, session_id: &str) -> KernelResult<()>;

    /// Get conversation history for a session
    fn get_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>>;

    /// Append a message to the conversation
    fn append_message(&mut self, session_id: &str, message: AgentMessage) -> KernelResult<()>;

    /// Get the current turn number
    fn current_turn(&self, session_id: &str) -> KernelResult<u32>;

    /// Clear conversation history
    fn clear_history(&mut self, session_id: &str) -> KernelResult<()>;

    /// Compress/summarize conversation history
    fn compress_history(
        &mut self,
        session_id: &str,
        max_tokens: usize,
    ) -> KernelResult<AgentMessage>;
}

// ============================================================================
// Session Config
// ============================================================================

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub title: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Vec<String>,
    pub instructions: Option<String>,
    pub personality: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
    pub permission_profile: Option<String>,
    pub timeout_ms: Option<u64>,
    pub source: SessionSource,
    pub kind: SessionKind,
    pub metadata: Vec<(String, String)>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            title: None,
            model: None,
            model_provider: None,
            cwd: None,
            workspace_roots: Vec::new(),
            instructions: None,
            personality: None,
            reasoning_effort: None,
            approval_policy: None,
            permission_profile: None,
            timeout_ms: None,
            source: SessionSource::Unknown,
            kind: SessionKind::Main,
            metadata: Vec::new(),
        }
    }
}

impl SessionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_model_provider(mut self, provider: impl Into<String>) -> Self {
        self.model_provider = Some(provider.into());
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_roots.push(root.into());
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_source(mut self, source: SessionSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_kind(mut self, kind: SessionKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

// ============================================================================
// Helpers
// ============================================================================

pub fn create_session_from_config(
    session_id: impl Into<String>,
    agent_id: Option<String>,
    user_ref: Option<String>,
    tenant_id: Option<String>,
    config: SessionConfig,
    created_at: impl Into<String>,
) -> AgentSession {
    let mut session = AgentSession::new(session_id)
        .with_source(config.source)
        .with_kind(config.kind)
        .created_at(created_at);

    if let Some(id) = agent_id {
        session = session.with_agent_id(id);
    }
    if let Some(u) = user_ref {
        session = session.with_user_ref(u);
    }
    if let Some(t) = tenant_id {
        session = session.with_tenant_id(t);
    }
    if let Some(title) = config.title {
        session = session.with_title(title);
    }
    if let Some(model) = config.model {
        session = session.with_model(model);
    }
    if let Some(mp) = config.model_provider {
        session = session.with_model_provider(mp);
    }
    if let Some(cwd) = config.cwd {
        session = session.with_cwd(cwd);
    }
    for root in config.workspace_roots {
        session = session.with_workspace_root(root);
    }
    if let Some(inst) = config.instructions {
        session = session.with_instructions(inst);
    }
    if let Some(p) = config.personality {
        session = session.with_personality(p);
    }
    if let Some(re) = config.reasoning_effort {
        session = session.with_reasoning_effort(re);
    }
    if let Some(ap) = config.approval_policy {
        session = session.with_approval_policy(ap);
    }
    if let Some(pp) = config.permission_profile {
        session = session.with_permission_profile(pp);
    }
    if let Some(to) = config.timeout_ms {
        session = session.with_timeout_ms(to);
    }
    for (k, v) in config.metadata {
        session = session.with_metadata(k, v);
    }

    session
}

// ============================================================================
// InMemoryConversationManager
// ============================================================================

pub struct InMemoryConversationManager {
    messages: HashMap<String, Vec<AgentMessage>>,
    turn_counts: HashMap<String, u32>,
}

impl InMemoryConversationManager {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            turn_counts: HashMap::new(),
        }
    }
}

impl Default for InMemoryConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationManager for InMemoryConversationManager {
    fn begin_turn(&mut self, session_id: &str) -> KernelResult<()> {
        let count = self.turn_counts.entry(session_id.to_string()).or_insert(0);
        *count += 1;
        Ok(())
    }

    fn end_turn(&mut self, _session_id: &str) -> KernelResult<()> {
        Ok(())
    }

    fn get_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        Ok(self.messages.get(session_id).cloned().unwrap_or_default())
    }

    fn append_message(&mut self, session_id: &str, message: AgentMessage) -> KernelResult<()> {
        self.messages
            .entry(session_id.to_string())
            .or_default()
            .push(message);
        Ok(())
    }

    fn current_turn(&self, session_id: &str) -> KernelResult<u32> {
        Ok(self.turn_counts.get(session_id).copied().unwrap_or(0))
    }

    fn clear_history(&mut self, session_id: &str) -> KernelResult<()> {
        self.messages.remove(session_id);
        self.turn_counts.remove(session_id);
        Ok(())
    }

    fn compress_history(
        &mut self,
        session_id: &str,
        max_tokens: usize,
    ) -> KernelResult<AgentMessage> {
        let messages = self.messages.get(session_id).cloned().unwrap_or_default();

        let mut system_prefix: Option<AgentMessage> = None;
        let compressible: Vec<AgentMessage> = messages
            .iter()
            .filter(|msg| {
                if system_prefix.is_none() && msg.role == AgentMessageRole::System {
                    system_prefix = Some((*msg).clone());
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let mut total_chars = 0;
        let mut kept = Vec::new();
        for msg in compressible.iter().rev() {
            let msg_chars: usize = msg
                .parts
                .iter()
                .map(|p| p.text.as_ref().map_or(0, |t| t.len()))
                .sum();
            if total_chars + msg_chars > max_tokens * 4 {
                break;
            }
            total_chars += msg_chars;
            kept.push((*msg).clone());
        }
        kept.reverse();

        let summary_text = format!(
            "Compressed {} messages ({} chars) into summary",
            kept.len(),
            total_chars
        );

        let summary_msg = AgentMessage::new(
            format!("compressed.{}", session_id),
            AgentMessageRole::System,
            vec![AgentPart::text("compressed", summary_text)],
        );

        let mut new_messages = Vec::new();
        if let Some(sys) = system_prefix {
            new_messages.push(sys);
        }
        new_messages.push(summary_msg);

        self.messages.insert(session_id.to_string(), new_messages);

        Ok(AgentMessage::new(
            format!("compressed.summary.{}", session_id),
            AgentMessageRole::System,
            vec![AgentPart::text(
                "compressed.result",
                format!(
                    "Compressed {} messages ({} chars) into summary",
                    kept.len(),
                    total_chars
                ),
            )],
        ))
    }
}

// ============================================================================
// Helpers
// ============================================================================

pub fn uuid_simple() -> String {
    sdkwork_utils_rust::uuid()
}

pub fn now_iso() -> String {
    sdkwork_utils_rust::format_datetime(sdkwork_utils_rust::now(), Some("%Y-%m-%dT%H:%M:%S%.9fZ"))
}

/// Defines a provider lifecycle wrapper backed by [`InMemoryProviderSessionStore`].
#[macro_export]
macro_rules! define_provider_lifecycle_provider {
    ($wrapper:ident, $provider_id:expr) => {
        pub struct $wrapper {
            store: $crate::InMemoryProviderSessionStore,
        }

        impl $wrapper {
            pub fn new() -> Self {
                Self {
                    store: $crate::InMemoryProviderSessionStore::new($provider_id),
                }
            }

            pub fn session_store(&self) -> &$crate::InMemoryProviderSessionStore {
                &self.store
            }
        }

        impl Default for $wrapper {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $crate::SessionLifecycleProvider for $wrapper {
            fn create_session(
                &self,
                agent_id: &str,
                user_ref: Option<&str>,
                config: $crate::SessionConfig,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.create_session(agent_id, user_ref, config)
            }

            fn resume_session(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.resume_session(session_id)
            }

            fn get_session(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.get_session(session_id)
            }

            fn find_session(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<Option<sdkwork_agent_kernel::AgentSession>>
            {
                self.store.find_session(session_id)
            }

            fn update_session(
                &self,
                session: sdkwork_agent_kernel::AgentSession,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.update_session(session)
            }

            fn delete_session(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.delete_session(session_id)
            }

            fn synchronize_session(
                &self,
                session: sdkwork_agent_kernel::AgentSession,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.synchronize_session(session)
            }

            fn session_changes(
                &self,
                after_sequence: u64,
                limit: Option<usize>,
            ) -> sdkwork_agent_kernel::KernelResult<$crate::ProviderSessionChangeBatch> {
                self.store.changes_since(after_sequence, limit)
            }

            fn close_session(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
                self.store.close_session(session_id)
            }

            fn list_active_sessions(
                &self,
            ) -> sdkwork_agent_kernel::KernelResult<Vec<sdkwork_agent_kernel::AgentSession>> {
                self.store.list_active_sessions()
            }

            fn list_sessions(
                &self,
                query: &$crate::SessionListQuery,
            ) -> sdkwork_agent_kernel::KernelResult<Vec<sdkwork_agent_kernel::AgentSession>> {
                self.store.list_sessions(query)
            }

            fn get_conversation_history(
                &self,
                session_id: &str,
            ) -> sdkwork_agent_kernel::KernelResult<Vec<sdkwork_agent_kernel::AgentMessage>> {
                self.store.get_conversation_history(session_id)
            }

            fn append_conversation_message(
                &self,
                session_id: &str,
                message: sdkwork_agent_kernel::AgentMessage,
            ) -> sdkwork_agent_kernel::KernelResult<()> {
                self.store.append_conversation_message(session_id, message)
            }
        }
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::SessionState;

    #[test]
    fn session_config_builder() {
        let config = SessionConfig::new()
            .with_title("Test Session")
            .with_model("gpt-4")
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);

        assert_eq!(config.title, Some("Test Session".to_string()));
        assert_eq!(config.model, Some("gpt-4".to_string()));
        assert_eq!(config.source, SessionSource::Cli);
        assert_eq!(config.kind, SessionKind::Main);
    }

    #[test]
    fn create_session_from_config_works() {
        let config = SessionConfig::new()
            .with_title("Test")
            .with_model("claude-3");

        let session = create_session_from_config(
            "session.1",
            Some("agent.1".to_string()),
            Some("user.1".to_string()),
            Some("tenant.1".to_string()),
            config,
            "2026-01-01T00:00:00Z",
        );

        assert_eq!(session.session_id, "session.1");
        assert_eq!(session.agent_id, Some("agent.1".to_string()));
        assert_eq!(session.user_ref, Some("user.1".to_string()));
        assert_eq!(session.title, Some("Test".to_string()));
        assert_eq!(session.model, Some("claude-3".to_string()));
        assert_eq!(session.state, SessionState::Created);
    }

    #[test]
    fn in_memory_conversation_manager_append_and_get() {
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
    fn in_memory_conversation_manager_turn_tracking() {
        let mut manager = InMemoryConversationManager::new();

        assert_eq!(manager.current_turn("session.1").unwrap(), 0);

        manager.begin_turn("session.1").unwrap();
        assert_eq!(manager.current_turn("session.1").unwrap(), 1);

        manager.begin_turn("session.1").unwrap();
        assert_eq!(manager.current_turn("session.1").unwrap(), 2);
    }

    #[test]
    fn in_memory_conversation_manager_clear_history() {
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
    fn in_memory_conversation_manager_compress_preserves_summary() {
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

        let _compressed = manager.compress_history("session.1", 100).unwrap();

        let remaining = manager.get_history("session.1").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].role, AgentMessageRole::System);
        assert!(remaining[0].parts[0]
            .text
            .as_ref()
            .unwrap()
            .contains("Compressed"));
    }

    #[test]
    fn in_memory_conversation_manager_compress_preserves_system_prefix() {
        let mut manager = InMemoryConversationManager::new();

        let sys_msg = AgentMessage::new(
            "sys.1",
            AgentMessageRole::System,
            vec![AgentPart::text("p.sys", "You are a helpful assistant")],
        );
        manager.append_message("session.1", sys_msg).unwrap();

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

        let _compressed = manager.compress_history("session.1", 100).unwrap();

        let remaining = manager.get_history("session.1").unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].role, AgentMessageRole::System);
        assert_eq!(remaining[0].message_id, "sys.1");
        assert_eq!(remaining[1].role, AgentMessageRole::System);
        assert!(remaining[1].parts[0]
            .text
            .as_ref()
            .unwrap()
            .contains("Compressed"));
    }

    #[test]
    fn uuid_simple_returns_unique_uuids() {
        let ids: std::collections::HashSet<_> = (0..1_024).map(|_| uuid_simple()).collect();
        assert_eq!(ids.len(), 1_024);
        assert!(ids.iter().all(|id| sdkwork_utils_rust::is_uuid(id)));
    }

    #[test]
    fn now_iso_returns_valid_format() {
        let ts = now_iso();
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert!(sdkwork_utils_rust::parse_datetime(&ts, None).is_some());
    }
}
