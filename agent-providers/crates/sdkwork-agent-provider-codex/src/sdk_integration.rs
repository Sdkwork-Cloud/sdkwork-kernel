use crate::{
    CodexAdapter, CodexLifecycleProvider, CodexMessageAdapter, CodexModelProvider,
    CodexThreadActivityObservation,
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
    SdkRuntimeBackedModelProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    CODEX_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    RustNativeTransportHost, TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use sdkwork_agent_provider_transport_rust::{InProcessRustSdkRuntime, ProviderBackedRustHandler};
use std::sync::Arc;

const CODEX_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/codex/provider-binding.manifest.json");

pub fn codex_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CODEX_BINDING_MANIFEST_JSON)
        .expect("codex provider binding manifest must parse")
}

pub struct CodexSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: CodexLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_adapter: CodexAdapter,
    pub message_adapter: CodexMessageAdapter,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl CodexSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = codex_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let inner_model = Arc::new(CodexModelProvider::new());
        let rust_handler = Arc::new(ProviderBackedRustHandler::model_only(
            inner_model.clone(),
            "codex-1",
        ));

        let activity = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let runtime_activity_sink =
            Arc::new(ProviderSessionActivityRuntimeSink::new(activity.clone()));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(RustNativeTransportHost::new("codex-core")));
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@openai/codex-sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(
            NodeSdkBackendRuntime::bootstrap("@openai/codex-sdk")
                .with_activity_sink(runtime_activity_sink),
        ));
        bootstrap.with_rust_runtime(Arc::new(InProcessRustSdkRuntime::new(rust_handler)));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.codex",
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: CodexLifecycleProvider::new(),
            model,
            session_adapter: CodexAdapter::new(),
            message_adapter: CodexMessageAdapter::new(),
            activity,
        })
    }

    pub fn binding_id(&self) -> &str {
        CODEX_BINDING_ID
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }

    /// Records one status returned by the live Codex app-server thread API.
    pub fn record_provider_session_activity(
        &self,
        observation: &CodexThreadActivityObservation,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .record(self.session_adapter.to_session_activity(observation)?)
    }

    pub fn provider_session_activity_provider(&self) -> Arc<dyn ProviderSessionActivityProvider> {
        self.activity.clone()
    }
}

impl ProviderSessionActivityProvider for CodexSdkIntegration {
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
    fn bootstrap_negotiates_required_codex_capabilities() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CODEX_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
        assert_eq!(
            integration.sdk.selected_driver_id("sdk.session.lifecycle"),
            Some("driver.codex.session.lifecycle.rust")
        );
    }

    #[test]
    fn exposes_kernel_providers_after_bootstrap() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert!(!integration.model.list_models().is_empty());
    }

    #[test]
    fn runtime_model_chat_uses_typescript_backend() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req-runtime-1",
                vec!["hello codex".to_string()],
            ))
            .expect("runtime invoke should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::TypeScriptNode);
    }

    #[test]
    fn model_provider_routes_invoke_through_runtime() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
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
    fn runtime_activity_capability_records_queries_and_expires_codex_status() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        integration
            .record_provider_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.provider.1".to_string(),
                status: crate::CodexThreadRuntimeStatus::Active {
                    active_flags: vec![crate::CodexThreadActiveFlag::WaitingOnApproval],
                },
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("record activity");
        let waiting = integration
            .get_provider_session_activity("codex.provider.1")
            .expect("query activity");
        integration
            .record_provider_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.provider.stale".to_string(),
                status: crate::CodexThreadRuntimeStatus::Active {
                    active_flags: Vec::new(),
                },
                observed_at: "2000-01-01T00:00:00Z".to_string(),
            })
            .expect("record stale activity");
        let stale = integration
            .get_provider_session_activity("codex.provider.stale")
            .expect("query stale activity");
        let unknown = integration
            .get_provider_session_activity("codex.provider.unknown")
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
