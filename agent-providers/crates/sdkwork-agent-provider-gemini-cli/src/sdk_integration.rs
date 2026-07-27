use crate::{
    ids, GeminiActivityObservation, GeminiCliAdapter, GeminiCliLifecycleProvider,
    GeminiMessageAdapter, GeminiModelProvider, GeminiToolProvider,
};
use sdkwork_agent_kernel::{
    KernelResult, ProviderSessionActivityProvider, SessionActivitySnapshot,
};
use sdkwork_agent_provider_core::{
    InMemoryProviderSessionActivityProvider, ProviderSessionActivityAdapter,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, ProviderSessionActivityRuntimeSink, SdkNegotiationError,
    SdkRuntimeBackedModelProvider, SdkRuntimeBackedToolProvider, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, GEMINI_CLI_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
    SDK_CAPABILITY_TOOL_INVOKE,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const GEMINI_CLI_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/gemini-cli/provider-binding.manifest.json");

pub fn gemini_cli_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(GEMINI_CLI_BINDING_MANIFEST_JSON)
        .expect("gemini-cli provider binding manifest must parse")
}

pub struct GeminiCliSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: GeminiCliLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: GeminiCliAdapter,
    pub message_adapter: GeminiMessageAdapter,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl GeminiCliSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = gemini_cli_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let activity = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let runtime_activity_sink =
            Arc::new(ProviderSessionActivityRuntimeSink::new(activity.clone()));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@google/gemini-cli-sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(
            NodeSdkBackendRuntime::bootstrap("@google/gemini-cli-sdk")
                .with_activity_sink(runtime_activity_sink),
        ));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let inner_model = Arc::new(GeminiModelProvider::new());
        let inner_tools = Arc::new(GeminiToolProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            ids::MODEL_PROVIDER_ID,
        );
        let tools = SdkRuntimeBackedToolProvider::new(
            runtime.clone(),
            inner_tools,
            SDK_CAPABILITY_TOOL_INVOKE,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: GeminiCliLifecycleProvider::new(),
            model,
            tools,
            session_adapter: GeminiCliAdapter::new(),
            message_adapter: GeminiMessageAdapter::new(),
            activity,
        })
    }

    pub fn binding_id(&self) -> &str {
        GEMINI_CLI_BINDING_ID
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }

    /// Records one Gemini CLI `AgentEvent` emitted by the live runtime.
    pub fn record_provider_session_activity(
        &self,
        observation: &GeminiActivityObservation,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .record(self.session_adapter.to_session_activity(observation)?)
    }

    pub fn provider_session_activity_provider(&self) -> Arc<dyn ProviderSessionActivityProvider> {
        self.activity.clone()
    }
}

impl ProviderSessionActivityProvider for GeminiCliSdkIntegration {
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
    fn bootstrap_negotiates_gemini_cli_capabilities() {
        let integration = GeminiCliSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), GEMINI_CLI_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        let integration = GeminiCliSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-gemini-1", vec!["hello".to_string()]))
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
    fn runtime_activity_capability_records_queries_and_expires_gemini_events() {
        let integration = GeminiCliSdkIntegration::bootstrap().expect("bootstrap should succeed");
        integration
            .record_provider_session_activity(&GeminiActivityObservation {
                provider_session_id: "gemini.provider.1".to_string(),
                event: crate::GeminiAgentEventKind::ElicitationRequest,
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("record activity");
        let waiting = integration
            .get_provider_session_activity("gemini.provider.1")
            .expect("query activity");
        integration
            .record_provider_session_activity(&GeminiActivityObservation {
                provider_session_id: "gemini.provider.stale".to_string(),
                event: crate::GeminiAgentEventKind::AgentStart,
                observed_at: "2000-01-01T00:00:00Z".to_string(),
            })
            .expect("record stale activity");
        let stale = integration
            .get_provider_session_activity("gemini.provider.stale")
            .expect("query stale activity");
        let unknown = integration
            .get_provider_session_activity("gemini.provider.unknown")
            .expect("query unknown activity");

        assert_eq!(waiting.state, Some(SessionActivityState::Waiting));
        assert_eq!(
            waiting.interaction_hint,
            Some(SessionActivityInteractionHint::UserInputRequired)
        );
        assert_eq!(stale.freshness, SessionActivityFreshness::Stale);
        assert_eq!(unknown.freshness, SessionActivityFreshness::Unsupported);
    }
}
