use crate::{
    OpenCodeActivityObservation, OpenCodeAdapter, OpenCodeLifecycleProvider,
    OpenCodeMessageAdapter, OpenCodeModelProvider,
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
    OPENCODE_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const OPENCODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/opencode/provider-binding.manifest.json");

pub fn opencode_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(OPENCODE_BINDING_MANIFEST_JSON)
        .expect("opencode provider binding manifest must parse")
}

pub struct OpenCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: OpenCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_control: SdkRuntimeBackedSessionControlProvider,
    pub session_adapter: OpenCodeAdapter,
    pub message_adapter: OpenCodeMessageAdapter,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl OpenCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = opencode_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let activity = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let runtime_activity_sink =
            Arc::new(ProviderSessionActivityRuntimeSink::new(activity.clone()));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@opencode-ai/sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(
            NodeSdkBackendRuntime::bootstrap("@opencode-ai/sdk")
                .with_activity_sink(runtime_activity_sink),
        ));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let inner_model = Arc::new(OpenCodeModelProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.opencode",
        );
        let session_control = SdkRuntimeBackedSessionControlProvider::new(
            runtime.clone(),
            crate::ids::SESSION_CONTROL_PROVIDER_ID,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: OpenCodeLifecycleProvider::new(),
            model,
            session_control,
            session_adapter: OpenCodeAdapter::new(),
            message_adapter: OpenCodeMessageAdapter::new(),
            activity,
        })
    }

    pub fn binding_id(&self) -> &str {
        OPENCODE_BINDING_ID
    }

    pub fn list_provider_sessions(&self) -> KernelResult<Vec<SdkRuntimeSessionRecord>> {
        self.list_provider_sessions_for_directory(None)
    }

    pub fn list_provider_sessions_for_directory(
        &self,
        working_directory: Option<&str>,
    ) -> KernelResult<Vec<SdkRuntimeSessionRecord>> {
        list_all_provider_sessions_from_runtime(&self.runtime, "opencode", working_directory)
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
            "opencode",
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

    /// Records one OpenCode `session.status` event from the live event stream.
    pub fn record_provider_session_activity(
        &self,
        observation: &OpenCodeActivityObservation,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .record(self.session_adapter.to_session_activity(observation)?)
    }

    pub fn provider_session_activity_provider(&self) -> Arc<dyn ProviderSessionActivityProvider> {
        self.activity.clone()
    }
}

impl ProviderSessionActivityProvider for OpenCodeSdkIntegration {
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
        ModelProvider, ModelRequest, SessionActivityFreshness, SessionActivityState,
    };
    use sdkwork_agent_provider_spi::SdkBackendKind;

    #[test]
    fn bootstrap_negotiates_opencode_capabilities() {
        let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), OPENCODE_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn runtime_ping_reaches_typescript_backend() {
        let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.model.chat"))
            .expect("runtime ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::TypeScriptNode);
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        // The sdk_probe backend is the local mock runtime for these tests;
        // it requires the explicit non-production mock override (fail-closed
        // by default).
        std::env::set_var("SDKWORK_KERNEL_ENVIRONMENT", "development");
        std::env::set_var("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS", "1");
        let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
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
    fn runtime_activity_capability_records_queries_and_expires_opencode_events() {
        let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        integration
            .record_provider_session_activity(&OpenCodeActivityObservation {
                provider_session_id: "opencode.provider.1".to_string(),
                status: crate::OpenCodeSessionStatus::Busy,
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("record activity");
        let working = integration
            .get_provider_session_activity("opencode.provider.1")
            .expect("query activity");
        integration
            .record_provider_session_activity(&OpenCodeActivityObservation {
                provider_session_id: "opencode.provider.stale".to_string(),
                status: crate::OpenCodeSessionStatus::Busy,
                observed_at: "2000-01-01T00:00:00Z".to_string(),
            })
            .expect("record stale activity");
        let stale = integration
            .get_provider_session_activity("opencode.provider.stale")
            .expect("query stale activity");
        let unknown = integration
            .get_provider_session_activity("opencode.provider.unknown")
            .expect("query unknown activity");

        assert_eq!(working.state, Some(SessionActivityState::Working));
        assert_eq!(stale.freshness, SessionActivityFreshness::Stale);
        assert_eq!(unknown.freshness, SessionActivityFreshness::Unsupported);
    }
}
