use crate::{
    ClaudeCodeActivityObservation, ClaudeCodeAdapter, ClaudeCodeLifecycleProvider,
    ClaudeMessageAdapter, ClaudeModelProvider,
};
use sdkwork_agent_kernel::{
    KernelResult, ProviderSessionActivityProvider, SessionActivitySnapshot,
};
use sdkwork_agent_provider_core::{
    InMemoryProviderSessionActivityProvider, ProviderSessionActivityAdapter,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, list_all_provider_sessions_from_runtime,
    load_all_provider_messages_from_runtime, AgentSdkBindingManifest, AgentSdkIntegration,
    BindingRegistry, DriverRegistry, ProviderSessionActivityRuntimeSink, SdkNegotiationError,
    SdkRuntimeBackedModelProvider, SdkRuntimeBackedSessionControlProvider, SdkRuntimeMessageRecord,
    SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter, SdkRuntimeSessionRecord,
    CLAUDE_CODE_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const CLAUDE_CODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/claude-code/provider-binding.manifest.json");

pub fn claude_code_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CLAUDE_CODE_BINDING_MANIFEST_JSON)
        .expect("claude-code provider binding manifest must parse")
}

pub struct ClaudeCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: ClaudeCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_control: SdkRuntimeBackedSessionControlProvider,
    pub session_adapter: ClaudeCodeAdapter,
    pub message_adapter: ClaudeMessageAdapter,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl ClaudeCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = claude_code_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let activity = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let runtime_activity_sink =
            Arc::new(ProviderSessionActivityRuntimeSink::new(activity.clone()));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@anthropic-ai/claude-agent-sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(
            NodeSdkBackendRuntime::bootstrap("@anthropic-ai/claude-agent-sdk")
                .with_activity_sink(runtime_activity_sink),
        ));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let inner_model = Arc::new(ClaudeModelProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.claude-code",
        );
        let session_control = SdkRuntimeBackedSessionControlProvider::new(
            runtime.clone(),
            crate::ids::SESSION_CONTROL_PROVIDER_ID,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: ClaudeCodeLifecycleProvider::new(),
            model,
            session_control,
            session_adapter: ClaudeCodeAdapter::new(),
            message_adapter: ClaudeMessageAdapter::new(),
            activity,
        })
    }

    pub fn binding_id(&self) -> &str {
        CLAUDE_CODE_BINDING_ID
    }

    pub fn list_provider_sessions(&self) -> KernelResult<Vec<SdkRuntimeSessionRecord>> {
        self.list_provider_sessions_for_directory(None)
    }

    pub fn list_provider_sessions_for_directory(
        &self,
        working_directory: Option<&str>,
    ) -> KernelResult<Vec<SdkRuntimeSessionRecord>> {
        list_all_provider_sessions_from_runtime(&self.runtime, "claude-code", working_directory)
    }

    pub fn get_provider_session_history(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<Vec<SdkRuntimeMessageRecord>> {
        self.get_provider_session_history_for_directory(provider_session_id, None)
    }

    pub fn get_provider_session_history_for_directory(
        &self,
        provider_session_id: &str,
        working_directory: Option<&str>,
    ) -> KernelResult<Vec<SdkRuntimeMessageRecord>> {
        load_all_provider_messages_from_runtime(
            &self.runtime,
            "claude-code",
            provider_session_id,
            working_directory,
        )
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }

    /// Records one Claude Code hook event received by the runtime host.
    pub fn record_provider_session_activity(
        &self,
        observation: &ClaudeCodeActivityObservation,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .record(self.session_adapter.to_session_activity(observation)?)
    }

    pub fn provider_session_activity_provider(&self) -> Arc<dyn ProviderSessionActivityProvider> {
        self.activity.clone()
    }
}

impl ProviderSessionActivityProvider for ClaudeCodeSdkIntegration {
    fn get_provider_session_activity(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .get_provider_session_activity(provider_session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{
        ModelProvider, ModelRequest, SessionActivityFreshness, SessionActivityInteractionHint,
        SessionActivityState,
    };
    use sdkwork_agent_provider_spi::SdkBackendKind;

    #[test]
    fn bootstrap_negotiates_claude_code_capabilities() {
        let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CLAUDE_CODE_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        // The sdk_probe backend is the local mock runtime for these tests;
        // it requires the explicit non-production mock override (fail-closed
        // by default).
        std::env::set_var("SDKWORK_KERNEL_ENVIRONMENT", "development");
        std::env::set_var("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS", "1");
        let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-claude-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("sdk_runtime_mode=")));
        assert!(!response
            .messages
            .iter()
            .any(|message| message.contains("Mock response")));
    }

    #[test]
    fn runtime_activity_capability_records_queries_and_expires_claude_hooks() {
        let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        integration
            .record_provider_session_activity(&ClaudeCodeActivityObservation {
                provider_session_id: "claude.provider.1".to_string(),
                event: crate::ClaudeCodeHookEventKind::PermissionRequest,
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("record activity");
        let waiting = integration
            .get_provider_session_activity("claude.provider.1")
            .expect("query activity");
        integration
            .record_provider_session_activity(&ClaudeCodeActivityObservation {
                provider_session_id: "claude.provider.stale".to_string(),
                event: crate::ClaudeCodeHookEventKind::PreToolUse,
                observed_at: "2000-01-01T00:00:00Z".to_string(),
            })
            .expect("record stale activity");
        let stale = integration
            .get_provider_session_activity("claude.provider.stale")
            .expect("query stale activity");
        let unknown = integration
            .get_provider_session_activity("claude.provider.unknown")
            .expect("query unknown activity");

        assert_eq!(waiting.state, Some(SessionActivityState::Waiting));
        assert_eq!(
            waiting.interaction_hint,
            Some(SessionActivityInteractionHint::ApprovalRequired)
        );
        assert_eq!(stale.freshness, SessionActivityFreshness::Stale);
        assert_eq!(unknown.freshness, SessionActivityFreshness::Unsupported);
    }
}
