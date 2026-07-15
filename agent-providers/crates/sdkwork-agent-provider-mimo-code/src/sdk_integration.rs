use crate::{
    MiMoCodeAdapter, MiMoCodeLifecycleProvider, MiMoCodeMessageAdapter, MiMoCodeModelProvider,
    MiMoCodeToolProvider,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedToolProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    MIMO_CODE_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const MIMO_CODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/mimo-code/provider-binding.manifest.json");

pub fn mimo_code_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(MIMO_CODE_BINDING_MANIFEST_JSON)
        .expect("mimo-code provider binding manifest must parse")
}

pub struct MiMoCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: MiMoCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: MiMoCodeAdapter,
    pub message_adapter: MiMoCodeMessageAdapter,
}

impl MiMoCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = mimo_code_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new("@mimo-ai/sdk")));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap
            .with_typescript_runtime(Arc::new(NodeSdkBackendRuntime::bootstrap("@mimo-ai/sdk")));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            Arc::new(MiMoCodeModelProvider::new()),
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.mimo",
        );
        let tools = SdkRuntimeBackedToolProvider::new(
            runtime.clone(),
            Arc::new(MiMoCodeToolProvider::new()),
            SDK_CAPABILITY_TOOL_INVOKE,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: MiMoCodeLifecycleProvider::new(),
            model,
            tools,
            session_adapter: MiMoCodeAdapter::new(),
            message_adapter: MiMoCodeMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        MIMO_CODE_BINDING_ID
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
    use sdkwork_agent_provider_core::{SessionConfig, SessionLifecycleProvider};

    #[test]
    fn bootstrap_exposes_session_lifecycle_and_transport_runtime() {
        let integration = MiMoCodeSdkIntegration::bootstrap().expect("bootstrap");
        assert_eq!(integration.binding_id(), MIMO_CODE_BINDING_ID);
        assert!(integration
            .sdk
            .selected_driver_id("sdk.session.lifecycle")
            .is_some());
        let session = integration
            .lifecycle
            .create_session("agent.intelligence.mimo-code", None, SessionConfig::new())
            .expect("session");
        assert_eq!(
            integration
                .lifecycle
                .get_session(&session.session_id)
                .expect("loaded")
                .session_id,
            session.session_id
        );
    }
}
