use crate::{
    OpenClawAdapter, OpenClawLifecycleProvider, OpenClawMessageAdapter, OpenClawModelProvider,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, OPENCLAW_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    HttpOpenApiTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const OPENCLAW_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/openclaw/provider-binding.manifest.json");

/// Official OpenAI SDK used for the OpenClaw OpenAI-compatible gateway protocol.
pub const OPENCLAW_NPM_PACKAGE: &str = "openai";

pub fn openclaw_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(OPENCLAW_BINDING_MANIFEST_JSON)
        .expect("openclaw provider binding manifest must parse")
}

pub struct OpenClawSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: OpenClawLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_adapter: OpenClawAdapter,
    pub message_adapter: OpenClawMessageAdapter,
}

impl OpenClawSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = openclaw_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            OPENCLAW_NPM_PACKAGE,
        )));
        bootstrap.register_host(Arc::new(HttpOpenApiTransportHost::new(
            "openclaw-gateway-open-api",
        )));
        bootstrap.with_typescript_runtime(Arc::new(NodeSdkBackendRuntime::bootstrap(
            OPENCLAW_NPM_PACKAGE,
        )));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            Arc::new(OpenClawModelProvider::new()),
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.openclaw",
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: OpenClawLifecycleProvider::new(),
            model,
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
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{ModelProvider, ModelRequest};
    use sdkwork_agent_provider_spi::SdkBackendKind;

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
        assert!(!response.messages.is_empty());
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("sdk_runtime_mode=")));
    }
}
