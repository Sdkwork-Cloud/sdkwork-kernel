use crate::{
    ClaudeCodeAdapter, ClaudeCodeLifecycleProvider, ClaudeMessageAdapter, ClaudeModelProvider,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, CLAUDE_CODE_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    TypeScriptNodeTransportHost,
};
use sdkwork_agent_provider_transport_node::NodeSdkBackendRuntime;
use std::sync::Arc;

const CLAUDE_CODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/claude-code/provider-binding.manifest.json");

pub fn claude_code_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CLAUDE_CODE_BINDING_MANIFEST_JSON)
        .expect("claude-code provider binding manifest must parse")
}

pub struct ClaudeCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: ClaudeCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_adapter: ClaudeCodeAdapter,
    pub message_adapter: ClaudeMessageAdapter,
}

impl ClaudeCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = claude_code_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(TypeScriptNodeTransportHost::new(
            "@anthropic-ai/claude-agent-sdk",
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_typescript_runtime(Arc::new(NodeSdkBackendRuntime::bootstrap(
            "@anthropic-ai/claude-agent-sdk",
        )));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let inner_model = Arc::new(ClaudeModelProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.claude-code",
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: ClaudeCodeLifecycleProvider::new(),
            model,
            session_adapter: ClaudeCodeAdapter::new(),
            message_adapter: ClaudeMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        CLAUDE_CODE_BINDING_ID
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
    fn bootstrap_negotiates_claude_code_capabilities() {
        let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CLAUDE_CODE_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-claude-1", vec!["hello".to_string()]))
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
