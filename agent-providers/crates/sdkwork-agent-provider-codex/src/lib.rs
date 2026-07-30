use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelResult, ModelDescriptor,
    ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, ModelStreamChunk,
    ProviderHealth, ProviderManifest, SessionActivityEvidenceKind, SessionActivityInteractionHint,
    SessionActivitySnapshot, SessionActivityState, SessionKind, SessionSource,
};
use sdkwork_agent_provider_core::{
    create_session_from_config, finalize_provider_session_snapshot,
    session_activity_from_provider_observation, uuid_simple, MessageAdapter,
    ProviderSessionActivityAdapter, SessionAdapter, SessionConfig,
    DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
};

mod local_plugins;
mod provider_sessions;

pub use local_plugins::CodexLocalPluginProvider;
pub use provider_sessions::{
    discover_codex_provider_session_messages, discover_codex_provider_sessions,
    read_codex_provider_session_messages, read_codex_provider_sessions,
};

#[cfg(test)]
use sdkwork_agent_kernel::KernelError;
#[cfg(test)]
use sdkwork_agent_kernel::SessionState;
#[cfg(test)]
use sdkwork_agent_provider_core::{
    ConversationManager, InMemoryConversationManager, SessionLifecycleProvider,
};

// ============================================================================
// Codex Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CodexMessage {
    pub role: String,
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<CodexToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CodexToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

// ============================================================================
// Codex Session Adapter (existing, preserved)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CodexSessionMeta {
    pub id: String,
    pub forked_from_id: Option<String>,
    pub parent_thread_id: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub agent_nickname: Option<String>,
    pub role: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodexThreadSessionState {
    pub id: String,
    pub forked_from_id: Option<String>,
    pub parent_thread_id: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub agent_nickname: Option<String>,
    pub role: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
    pub active: bool,
    pub observed_at: Option<String>,
}

impl CodexThreadSessionState {
    pub fn from_meta(meta: &CodexSessionMeta, active: bool) -> Self {
        Self {
            id: meta.id.clone(),
            forked_from_id: meta.forked_from_id.clone(),
            parent_thread_id: meta.parent_thread_id.clone(),
            timestamp: meta.timestamp.clone(),
            cwd: meta.cwd.clone(),
            originator: meta.originator.clone(),
            model: meta.model.clone(),
            model_provider: meta.model_provider.clone(),
            agent_nickname: meta.agent_nickname.clone(),
            role: meta.role.clone(),
            reasoning_effort: meta.reasoning_effort.clone(),
            approval_policy: meta.approval_policy.clone(),
            active,
            observed_at: None,
        }
    }

    pub fn with_runtime_observed_at(mut self, observed_at: impl Into<String>) -> Self {
        self.observed_at = Some(observed_at.into());
        self
    }

    fn to_meta(&self) -> CodexSessionMeta {
        CodexSessionMeta {
            id: self.id.clone(),
            forked_from_id: self.forked_from_id.clone(),
            parent_thread_id: self.parent_thread_id.clone(),
            timestamp: self.timestamp.clone(),
            cwd: self.cwd.clone(),
            originator: self.originator.clone(),
            model: self.model.clone(),
            model_provider: self.model_provider.clone(),
            agent_nickname: self.agent_nickname.clone(),
            role: self.role.clone(),
            reasoning_effort: self.reasoning_effort.clone(),
            approval_policy: self.approval_policy.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexThreadActiveFlag {
    WaitingOnApproval,
    WaitingOnUserInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexThreadRuntimeStatus {
    NotLoaded,
    Idle,
    SystemError,
    Active {
        active_flags: Vec<CodexThreadActiveFlag>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexThreadActivityObservation {
    pub provider_session_id: String,
    pub status: CodexThreadRuntimeStatus,
    pub observed_at: String,
}

pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    fn convert_meta(meta: &CodexSessionMeta) -> KernelResult<AgentSession> {
        let kind = if meta.parent_thread_id.is_some() {
            SessionKind::Subagent
        } else {
            SessionKind::Main
        };

        let mut config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(kind);

        if let Some(ref model) = meta.model {
            config = config.with_model(model);
        }
        if let Some(ref model_provider) = meta.model_provider {
            config = config.with_model_provider(model_provider);
        }
        if let Some(ref cwd) = meta.cwd {
            config = config.with_cwd(cwd);
        }
        if let Some(ref reasoning_effort) = meta.reasoning_effort {
            config = config.with_metadata("reasoning_effort", reasoning_effort);
        }
        if let Some(ref approval_policy) = meta.approval_policy {
            config = config.with_metadata("approval_policy", approval_policy);
        }

        let mut session = create_session_from_config(
            &meta.id,
            None,
            meta.originator.clone(),
            None,
            config,
            meta.timestamp.as_deref().unwrap_or(""),
        );

        session.forked_from_id = meta.forked_from_id.clone();
        session.parent_session_id = meta.parent_thread_id.clone();
        session.agent_nickname = meta.agent_nickname.clone();
        session.agent_role = meta.role.clone();

        finalize_provider_session_snapshot("codex", session)
    }

    /// Converts the compatibility active flag only when its runtime source also
    /// supplies an observation timestamp. Persisted metadata remains unsupported.
    pub fn to_agent_session_state(
        &self,
        external: &CodexThreadSessionState,
    ) -> KernelResult<AgentSession> {
        let mut session = Self::convert_meta(&external.to_meta())?;
        let Some(observed_at) = external.observed_at.as_ref() else {
            return Ok(session);
        };
        let status = if external.active {
            CodexThreadRuntimeStatus::Active {
                active_flags: Vec::new(),
            }
        } else {
            CodexThreadRuntimeStatus::Idle
        };
        session.apply_activity(self.to_session_activity(&CodexThreadActivityObservation {
            provider_session_id: external.id.clone(),
            status,
            observed_at: observed_at.clone(),
        })?)?;
        Ok(session)
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for CodexAdapter {
    type ExternalSession = CodexSessionMeta;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        Self::convert_meta(external)
    }
}

impl ProviderSessionActivityAdapter for CodexAdapter {
    type ExternalActivity = CodexThreadActivityObservation;

    fn to_session_activity(
        &self,
        external: &Self::ExternalActivity,
    ) -> KernelResult<SessionActivitySnapshot> {
        let (state, interaction_hint) = match &external.status {
            CodexThreadRuntimeStatus::NotLoaded => {
                return Ok(SessionActivitySnapshot::unsupported_with_evidence(
                    &external.provider_session_id,
                    SessionActivityEvidenceKind::ProviderStatus,
                    external.observed_at.clone(),
                ));
            }
            CodexThreadRuntimeStatus::Idle => (SessionActivityState::Idle, None),
            CodexThreadRuntimeStatus::SystemError => (SessionActivityState::Failed, None),
            CodexThreadRuntimeStatus::Active { active_flags } => {
                if active_flags.contains(&CodexThreadActiveFlag::WaitingOnApproval) {
                    (
                        SessionActivityState::Waiting,
                        Some(SessionActivityInteractionHint::ApprovalRequired),
                    )
                } else if active_flags.contains(&CodexThreadActiveFlag::WaitingOnUserInput) {
                    (
                        SessionActivityState::Waiting,
                        Some(SessionActivityInteractionHint::UserInputRequired),
                    )
                } else {
                    (SessionActivityState::Working, None)
                }
            }
        };
        session_activity_from_provider_observation(
            &external.provider_session_id,
            state,
            SessionActivityEvidenceKind::ProviderStatus,
            interaction_hint,
            &external.observed_at,
            DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
        )
    }
}

// ============================================================================
// Codex Message Adapter
// ============================================================================

pub struct CodexMessageAdapter;

impl CodexMessageAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexMessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAdapter for CodexMessageAdapter {
    type ExternalMessage = CodexMessage;

    fn to_agent_message(&self, external: &Self::ExternalMessage) -> KernelResult<AgentMessage> {
        let role = match external.role.as_str() {
            "user" => AgentMessageRole::User,
            "assistant" => AgentMessageRole::Agent,
            "system" => AgentMessageRole::System,
            "tool" => AgentMessageRole::Tool,
            _ => AgentMessageRole::Adapter,
        };

        let mut parts = Vec::new();

        if let Some(ref reasoning) = external.reasoning_content {
            if !reasoning.is_empty() {
                parts.push(
                    AgentPart::text("codex.reasoning", reasoning)
                        .with_metadata("codex.content_type", "reasoning"),
                );
            }
        }

        if !external.content.is_empty() {
            parts.push(AgentPart::text("codex.content", &external.content));
        }

        if let Some(tool_calls) = &external.tool_calls {
            for tc in tool_calls {
                let mut part =
                    AgentPart::tool_call_ref(format!("codex.tool_call.{}", tc.id), &tc.id);
                part.name = Some(tc.function_name.clone());
                part.json = Some(tc.arguments.clone());
                parts.push(part);
            }
        }

        if parts.is_empty() {
            parts.push(AgentPart::text("codex.empty", ""));
        }

        let mut message = AgentMessage::new(format!("codex.msg.{}", uuid_simple()), role, parts);

        if external.reasoning_content.is_some() {
            message = message.with_metadata("codex.has_reasoning", "true");
        }
        if let Some(tool_call_id) = &external.tool_call_id {
            message = message.with_metadata("codex.tool_call_id", tool_call_id);
        }

        Ok(message)
    }
}

// ============================================================================
// Codex Model Provider
// ============================================================================

#[derive(Clone)]
pub struct CodexModelProvider {
    default_model: String,
}

impl CodexModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "codex-1".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for CodexModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for CodexModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.codex",
            "model",
            "Codex Model Provider",
            "0.1.0",
            vec![
                "model.chat".to_string(),
                "model.stream".to_string(),
                "model.tool_call".to_string(),
                "model.reasoning".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new("codex-mini", "provider.model.codex", "Codex Mini", "codex")
                .with_version("mini")
                .with_capability("chat")
                .with_capability("tool_call")
                .with_context_window_tokens(128000)
                .with_max_output_tokens(16000)
                .with_input_mode("text")
                .with_output_mode("text")
                .with_response_format(ModelResponseFormat::Text)
                .with_tool_capability("function_calling"),
            ModelDescriptor::new("codex-1", "provider.model.codex", "Codex 1", "codex")
                .with_version("1.0")
                .with_capability("chat")
                .with_capability("tool_call")
                .with_capability("reasoning")
                .with_context_window_tokens(200000)
                .with_max_output_tokens(32000)
                .with_input_mode("text")
                .with_output_mode("text")
                .with_response_format(ModelResponseFormat::Text)
                .with_response_format(ModelResponseFormat::Json)
                .with_tool_capability("function_calling"),
            ModelDescriptor::new(
                "codex-1-pro",
                "provider.model.codex",
                "Codex 1 Pro",
                "codex",
            )
            .with_version("1.0")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_capability("reasoning")
            .with_context_window_tokens(200000)
            .with_max_output_tokens(64000)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_response_format(ModelResponseFormat::Json)
            .with_tool_capability("function_calling"),
        ]
    }

    fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        sdkwork_agent_provider_core::reject_in_process_model_invoke("provider.model.codex")
    }

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        sdkwork_agent_provider_core::reject_in_process_model_stream("provider.model.codex")
    }
}

// ============================================================================
// Codex Lifecycle Provider (existing, preserved)
// ============================================================================

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(CodexLifecycleProvider, "codex");

mod agent_definition;
mod configuration;
mod conformance;
pub mod ids;
mod installer;
mod manifest;
mod package;
pub mod sdk_integration;

pub use agent_definition::{codex_agent_definition, codex_agent_manifest};
pub use configuration::CodexConfigurationProvider;
pub use conformance::codex_conformance_profile;
pub use installer::{codex_agent_installer, CODEX_SDK_PACKAGE, CODEX_SDK_VERSION};
pub use manifest::{codex_kernel_plugin_manifest, codex_provider_manifests, CodexKernelPlugin};
pub use package::codex_package_manifest;
pub use sdk_integration::{codex_binding_manifest, CodexSdkIntegration};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_codex_meta() -> CodexSessionMeta {
        CodexSessionMeta {
            id: "codex.session.1".to_string(),
            forked_from_id: None,
            parent_thread_id: None,
            timestamp: Some("2026-01-01T00:00:00Z".to_string()),
            cwd: Some("/home/user/project".to_string()),
            originator: Some("user.1".to_string()),
            model: Some("o3".to_string()),
            model_provider: Some("openai".to_string()),
            agent_nickname: Some("Codex".to_string()),
            role: Some("assistant".to_string()),
            reasoning_effort: Some("high".to_string()),
            approval_policy: Some("suggest".to_string()),
        }
    }

    // --- Session Adapter Tests ---

    #[test]
    fn maps_basic_fields() {
        let adapter = CodexAdapter::new();
        let ext = sample_codex_meta();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.session_id, "codex.session.1");
        assert_eq!(session.source, SessionSource::Cli);
        assert_eq!(session.kind, SessionKind::Main);
        assert_eq!(session.model, Some("o3".to_string()));
        assert_eq!(session.model_provider, Some("openai".to_string()));
        assert_eq!(session.cwd, Some("/home/user/project".to_string()));
        assert_eq!(session.user_ref, Some("user.1".to_string()));
        assert_eq!(session.created_at, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(session.agent_nickname, Some("Codex".to_string()));
        assert_eq!(session.agent_role, Some("assistant".to_string()));
    }

    #[test]
    fn maps_subagent_when_parent_thread_exists() {
        let adapter = CodexAdapter::new();
        let mut ext = sample_codex_meta();
        ext.parent_thread_id = Some("thread.parent.1".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();

        assert_eq!(session.kind, SessionKind::Subagent);
        assert_eq!(
            session.parent_session_id,
            Some("thread.parent.1".to_string())
        );
    }

    #[test]
    fn maps_forked_from_id() {
        let adapter = CodexAdapter::new();
        let mut ext = sample_codex_meta();
        ext.forked_from_id = Some("original.session".to_string());
        let session = adapter.to_agent_session(&ext).unwrap();
        assert_eq!(session.forked_from_id, Some("original.session".to_string()));
    }

    #[test]
    fn maps_reasoning_and_approval_to_metadata() {
        let adapter = CodexAdapter::new();
        let ext = sample_codex_meta();
        let session = adapter.to_agent_session(&ext).unwrap();

        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| k == "reasoning_effort" && v == "high"));
        assert!(session
            .metadata
            .iter()
            .any(|(k, v)| k == "approval_policy" && v == "suggest"));
    }

    #[test]
    fn thread_session_state_from_meta() {
        let meta = sample_codex_meta();
        let state = CodexThreadSessionState::from_meta(&meta, true);
        assert!(state.active);
        assert_eq!(state.id, "codex.session.1");
        assert_eq!(state.model, Some("o3".to_string()));
    }

    #[test]
    fn historical_thread_session_state_does_not_treat_active_flag_as_live_evidence() {
        let adapter = CodexAdapter::new();
        let state = CodexThreadSessionState::from_meta(&sample_codex_meta(), false);
        let session = adapter
            .to_agent_session_state(&state)
            .expect("thread state");
        assert_eq!(session.state, SessionState::Created);
        assert_eq!(
            session.activity.freshness,
            sdkwork_agent_kernel::SessionActivityFreshness::Unsupported
        );
        assert_eq!(session.metadata_value("provider_id"), Some("codex"));
    }

    #[test]
    fn live_thread_session_state_maps_explicit_inactive_status_to_idle() {
        let adapter = CodexAdapter::new();
        let state = CodexThreadSessionState::from_meta(&sample_codex_meta(), false)
            .with_runtime_observed_at(sdkwork_agent_provider_core::now_iso());
        let session = adapter
            .to_agent_session_state(&state)
            .expect("thread state");
        assert_eq!(session.state, SessionState::Active);
        assert_eq!(session.activity.state, Some(SessionActivityState::Idle));
    }

    #[test]
    fn thread_status_maps_waiting_flags_and_rejects_not_loaded_as_ready() {
        let adapter = CodexAdapter::new();
        let waiting = adapter
            .to_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.session.1".to_string(),
                status: CodexThreadRuntimeStatus::Active {
                    active_flags: vec![CodexThreadActiveFlag::WaitingOnApproval],
                },
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("waiting activity");
        let not_loaded = adapter
            .to_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.session.1".to_string(),
                status: CodexThreadRuntimeStatus::NotLoaded,
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("not-loaded activity");

        assert_eq!(waiting.state, Some(SessionActivityState::Waiting));
        assert_eq!(
            waiting.interaction_hint,
            Some(SessionActivityInteractionHint::ApprovalRequired)
        );
        assert!(waiting.is_authoritative());
        assert_eq!(
            not_loaded.freshness,
            sdkwork_agent_kernel::SessionActivityFreshness::Unsupported
        );
        assert_eq!(not_loaded.state, None);
    }

    #[test]
    fn lifecycle_provider_create_and_resume() {
        let provider = CodexLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);
        let created = provider
            .create_session("agent.1", Some("user.1"), config)
            .unwrap();

        let resumed = provider.resume_session(&created.session_id).unwrap();
        assert_eq!(resumed.state, SessionState::Active);
    }

    // --- Message Adapter Tests ---

    #[test]
    fn converts_user_message() {
        let adapter = CodexMessageAdapter::new();
        let msg = CodexMessage {
            role: "user".to_string(),
            content: "Explain this code".to_string(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::User);
        assert_eq!(result.parts.len(), 1);
        assert_eq!(result.parts[0].text, Some("Explain this code".to_string()));
    }

    #[test]
    fn converts_assistant_with_reasoning() {
        let adapter = CodexMessageAdapter::new();
        let msg = CodexMessage {
            role: "assistant".to_string(),
            content: "Here is the explanation".to_string(),
            reasoning_content: Some("Let me analyze the code...".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::Agent);
        assert_eq!(result.parts.len(), 2);
        assert_eq!(
            result.parts[0].text,
            Some("Let me analyze the code...".to_string())
        );
        assert_eq!(
            result.parts[0].metadata_value("codex.content_type"),
            Some("reasoning")
        );
        assert_eq!(
            result.parts[1].text,
            Some("Here is the explanation".to_string())
        );
        assert_eq!(result.metadata_value("codex.has_reasoning"), Some("true"));
    }

    #[test]
    fn converts_tool_calls() {
        let adapter = CodexMessageAdapter::new();
        let msg = CodexMessage {
            role: "assistant".to_string(),
            content: String::new(),
            reasoning_content: None,
            tool_calls: Some(vec![CodexToolCall {
                id: "call.1".to_string(),
                function_name: "execute_command".to_string(),
                arguments: r#"{"cmd":"ls"}"#.to_string(),
            }]),
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.parts.len(), 1);
        assert_eq!(
            result.parts[0].kind,
            sdkwork_agent_kernel::AgentPartKind::ToolCallRef
        );
        assert_eq!(result.parts[0].tool_call_id, Some("call.1".to_string()));
        assert_eq!(result.parts[0].name, Some("execute_command".to_string()));
    }

    #[test]
    fn converts_batch_messages() {
        let adapter = CodexMessageAdapter::new();
        let messages = vec![
            CodexMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            },
            CodexMessage {
                role: "assistant".to_string(),
                content: "Hi".to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let result = adapter.to_agent_messages(&messages).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, AgentMessageRole::User);
        assert_eq!(result[1].role, AgentMessageRole::Agent);
    }

    #[test]
    fn converts_system_message() {
        let adapter = CodexMessageAdapter::new();
        let msg = CodexMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant".to_string(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        };
        let result = adapter.to_agent_message(&msg).unwrap();
        assert_eq!(result.role, AgentMessageRole::System);
    }

    // --- Model Provider Tests ---

    #[test]
    fn model_provider_manifest() {
        let provider = CodexModelProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "provider.model.codex");
        assert_eq!(manifest.provider_family, "model");
        assert!(manifest
            .capabilities
            .contains(&"model.reasoning".to_string()));
    }

    #[test]
    fn model_provider_health() {
        let provider = CodexModelProvider::new();
        assert_eq!(provider.health().status, "available");
    }

    #[test]
    fn model_provider_list_models() {
        let provider = CodexModelProvider::new();
        let models = provider.list_models();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].model_id, "codex-mini");
        assert_eq!(models[1].model_id, "codex-1");
        assert_eq!(models[2].model_id, "codex-1-pro");
    }

    #[test]
    fn model_provider_describe_model() {
        let provider = CodexModelProvider::new();
        let model = provider.describe_model("codex-1").unwrap();
        assert_eq!(model.display_name, "Codex 1");
        assert_eq!(model.family, "codex");
        assert!(model.supports_capability("chat"));
        assert!(model.supports_capability("reasoning"));
    }

    #[test]
    fn model_provider_invoke_requires_transport_worker() {
        let provider = CodexModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["Explain lifetimes".to_string()])
            .with_model_id("codex-1");
        let error = provider
            .invoke(request)
            .expect_err("in-process invoke is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    #[test]
    fn model_provider_stream_requires_transport_worker() {
        let provider = CodexModelProvider::new();
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
}
