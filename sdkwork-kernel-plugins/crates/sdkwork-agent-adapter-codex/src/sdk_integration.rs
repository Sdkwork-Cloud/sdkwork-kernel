use crate::{
    CodexAdapter, CodexLifecycleProvider, CodexMessageAdapter, CodexModelProvider,
    CodexToolProvider,
};
use sdkwork_agent_sdk_backend_core::{
    BackendHostRegistry, IpcProtocolBackendHost, RustNativeBackendHost,
};
use sdkwork_agent_sdk_backend_rust::{InProcessRustSdkRuntime, ProviderBackedRustHandler};
use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedToolProvider, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    CODEX_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use std::sync::Arc;

const CODEX_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");

pub fn codex_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CODEX_BINDING_MANIFEST_JSON)
        .expect("codex sdk binding manifest must parse")
}

pub struct CodexSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub backends: BackendHostRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: CodexLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: CodexAdapter,
    pub message_adapter: CodexMessageAdapter,
}

impl CodexSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = codex_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut backends = BackendHostRegistry::new();
        backends.register(Arc::new(RustNativeBackendHost::new("codex-core")));
        backends.register(Arc::new(IpcProtocolBackendHost::new("jsonrpc_stdio")));
        backends.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("backend.prepare: {error}")],
            )
        })?;

        let inner_model = Arc::new(CodexModelProvider::new());
        let inner_tools = Arc::new(CodexToolProvider::new());
        let rust_handler = Arc::new(ProviderBackedRustHandler::new(
            inner_model.clone(),
            inner_tools.clone(),
            "codex-1",
        ));
        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation.clone())
                .with_rust_runtime(Arc::new(InProcessRustSdkRuntime::new(rust_handler))),
        );
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.codex",
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
            lifecycle: CodexLifecycleProvider::new(),
            model,
            tools,
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
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_sdk_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{ModelProvider, ModelRequest, ToolProvider};
    use sdkwork_agent_sdk_spi::SdkBackendKind;

    #[test]
    fn bootstrap_negotiates_required_codex_capabilities() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CODEX_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::RustNative)
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
        assert!(!integration.tools.list_tools().is_empty());
    }

    #[test]
    fn runtime_model_chat_uses_in_process_rust_backend() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req-runtime-1",
                vec!["hello codex".to_string()],
            ))
            .expect("runtime invoke should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::RustNative);
    }

    #[test]
    fn model_provider_routes_invoke_through_runtime() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .messages
            .iter()
            .any(|message| message.contains("Codex")));
    }
}
