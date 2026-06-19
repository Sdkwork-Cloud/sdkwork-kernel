use crate::{
    OpenClawAdapter, OpenClawLifecycleProvider, OpenClawMessageAdapter, OpenClawModelProvider,
    OpenClawToolProvider,
};
use sdkwork_agent_sdk_backend_core::{
    BackendHostRegistry, HttpOpenApiBackendHost, TypeScriptNodeBackendHost,
};
use sdkwork_agent_sdk_backend_node::NodeSdkBackendRuntime;
use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedToolProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    OPENCLAW_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use std::sync::Arc;

const OPENCLAW_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../sdks/external-agent-sdks/openclaw/sdk-binding.manifest.json");

pub fn openclaw_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(OPENCLAW_BINDING_MANIFEST_JSON)
        .expect("openclaw sdk binding manifest must parse")
}

pub struct OpenClawSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub backends: BackendHostRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: OpenClawLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: OpenClawAdapter,
    pub message_adapter: OpenClawMessageAdapter,
}

impl OpenClawSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = openclaw_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut backends = BackendHostRegistry::new();
        backends.register(Arc::new(TypeScriptNodeBackendHost::new("openclaw")));
        backends.register(Arc::new(HttpOpenApiBackendHost::new(
            "openclaw-gateway-open-api",
        )));
        backends.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("backend.prepare: {error}")],
            )
        })?;

        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation.clone())
                .with_typescript_runtime(Arc::new(NodeSdkBackendRuntime::bootstrap("openclaw"))),
        );
        let inner_model = Arc::new(OpenClawModelProvider::new());
        let inner_tools = Arc::new(OpenClawToolProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.openclaw",
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
            lifecycle: OpenClawLifecycleProvider::new(),
            model,
            tools,
            session_adapter: OpenClawAdapter::new(),
            message_adapter: OpenClawMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        OPENCLAW_BINDING_ID
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
    fn bootstrap_prefers_typescript_backend_for_session_lifecycle() {
        let integration = OpenClawSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), OPENCLAW_BINDING_ID);
        assert_eq!(
            integration
                .sdk
                .selected_backend_kind("sdk.session.lifecycle"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn runtime_ping_reaches_typescript_backend() {
        let integration = OpenClawSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.session.lifecycle"))
            .expect("runtime ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::TypeScriptNode);
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        let integration = OpenClawSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .messages
            .iter()
            .any(|message| message.contains("openclaw")));
    }
}
