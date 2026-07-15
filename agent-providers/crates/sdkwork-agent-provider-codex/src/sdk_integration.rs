use crate::{CodexAdapter, CodexLifecycleProvider, CodexMessageAdapter, CodexModelProvider};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, CODEX_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
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

        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(RustNativeTransportHost::new("codex-core")));
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@openai/codex-sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(NodeSdkBackendRuntime::bootstrap(
            "@openai/codex-sdk",
        )));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{ModelProvider, ModelRequest};
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
}
