use crate::{
    OpenCodeAdapter, OpenCodeLifecycleProvider, OpenCodeMessageAdapter, OpenCodeModelProvider,
    OpenCodeToolProvider,
};
use sdkwork_agent_sdk_backend_core::{
    BackendHostRegistry, HttpOpenApiBackendHost, IpcProtocolBackendHost, TypeScriptNodeBackendHost,
};
use sdkwork_agent_sdk_backend_node::NodeSdkBackendRuntime;
use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedToolProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    OPENCODE_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use std::sync::Arc;

const OPENCODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../sdks/external-agent-sdks/opencode/sdk-binding.manifest.json");

pub fn opencode_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(OPENCODE_BINDING_MANIFEST_JSON)
        .expect("opencode sdk binding manifest must parse")
}

pub struct OpenCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub backends: BackendHostRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: OpenCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: OpenCodeAdapter,
    pub message_adapter: OpenCodeMessageAdapter,
}

impl OpenCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = opencode_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut backends = BackendHostRegistry::new();
        backends.register(Arc::new(TypeScriptNodeBackendHost::new("@opencode-ai/sdk")));
        backends.register(Arc::new(HttpOpenApiBackendHost::new("opencode-open-api")));
        backends.register(Arc::new(IpcProtocolBackendHost::new("jsonrpc_stdio")));
        backends.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("backend.prepare: {error}")],
            )
        })?;

        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation.clone()).with_typescript_runtime(Arc::new(
                NodeSdkBackendRuntime::bootstrap("@opencode-ai/sdk"),
            )),
        );
        let inner_model = Arc::new(OpenCodeModelProvider::new());
        let inner_tools = Arc::new(OpenCodeToolProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.opencode",
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
            lifecycle: OpenCodeLifecycleProvider::new(),
            model,
            tools,
            session_adapter: OpenCodeAdapter::new(),
            message_adapter: OpenCodeMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        OPENCODE_BINDING_ID
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
        let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .messages
            .iter()
            .any(|message| message.contains("@opencode-ai/sdk")));
    }
}
