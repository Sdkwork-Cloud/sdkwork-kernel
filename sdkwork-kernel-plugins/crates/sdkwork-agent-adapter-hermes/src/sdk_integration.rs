use crate::{
    HermesAdapter, HermesLifecycleProvider, HermesMessageAdapter, HermesModelProvider,
    HermesToolProvider,
};
use sdkwork_agent_sdk_backend_core::{
    BackendHostRegistry, IpcProtocolBackendHost, PythonProcessBackendHost,
};
use sdkwork_agent_sdk_backend_python::PythonSdkBackendRuntime;
use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedToolProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    HERMES_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use std::sync::Arc;

const HERMES_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../sdks/external-agent-sdks/hermes/sdk-binding.manifest.json");

pub fn hermes_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(HERMES_BINDING_MANIFEST_JSON)
        .expect("hermes sdk binding manifest must parse")
}

pub struct HermesSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub backends: BackendHostRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: HermesLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: HermesAdapter,
    pub message_adapter: HermesMessageAdapter,
}

impl HermesSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = hermes_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut backends = BackendHostRegistry::new();
        backends.register(Arc::new(PythonProcessBackendHost::new("hermes_agent")));
        backends.register(Arc::new(IpcProtocolBackendHost::new("jsonrpc_stdio")));
        backends.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("backend.prepare: {error}")],
            )
        })?;

        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation.clone())
                .with_python_runtime(Arc::new(PythonSdkBackendRuntime::bootstrap("hermes_agent"))),
        );
        let inner_model = Arc::new(HermesModelProvider::new());
        let inner_tools = Arc::new(HermesToolProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.hermes",
        );
        let tools = SdkRuntimeBackedToolProvider::new(
            runtime.clone(),
            inner_tools,
            SDK_CAPABILITY_TOOL_INVOKE,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            backends,
            runtime,
            lifecycle: HermesLifecycleProvider::new(),
            model,
            tools,
            session_adapter: HermesAdapter::new(),
            message_adapter: HermesMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        HERMES_BINDING_ID
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_sdk_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{ModelProvider, ModelRequest};
    use sdkwork_agent_sdk_spi::SdkBackendKind;

    #[test]
    fn bootstrap_prefers_python_backend_for_model_chat() {
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), HERMES_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::PythonProcess)
        );
    }

    #[test]
    fn runtime_ping_reaches_python_backend() {
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.model.chat"))
            .expect("runtime ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::PythonProcess);
    }

    #[test]
    fn model_provider_routes_invoke_through_python_runtime() {
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .messages
            .iter()
            .any(|message| message.contains("hermes_agent")));
    }
}
