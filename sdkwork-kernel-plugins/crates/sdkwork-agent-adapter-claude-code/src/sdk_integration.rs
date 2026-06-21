use crate::{
    ClaudeCodeAdapter, ClaudeCodeLifecycleProvider, ClaudeMessageAdapter, ClaudeModelProvider,
    ClaudeToolProvider,
};
use sdkwork_agent_sdk_backend_core::{BackendHostRegistry, IpcProtocolBackendHost, TypeScriptNodeBackendHost};
use sdkwork_agent_sdk_backend_node::NodeSdkBackendRuntime;
use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider, SdkRuntimeBackedToolProvider,
    SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter, CLAUDE_CODE_BINDING_ID,
    SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_TOOL_INVOKE,
};
use std::sync::Arc;

const CLAUDE_CODE_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../sdks/external-agent-sdks/claude-code/sdk-binding.manifest.json");

pub fn claude_code_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CLAUDE_CODE_BINDING_MANIFEST_JSON)
        .expect("claude-code sdk binding manifest must parse")
}

pub struct ClaudeCodeSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub backends: BackendHostRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: ClaudeCodeLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
    pub session_adapter: ClaudeCodeAdapter,
    pub message_adapter: ClaudeMessageAdapter,
}

impl ClaudeCodeSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = claude_code_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let mut backends = BackendHostRegistry::new();
        backends.register(Arc::new(TypeScriptNodeBackendHost::new(
            "@anthropic-ai/claude-agent-sdk",
        )));
        backends.register(Arc::new(IpcProtocolBackendHost::new("jsonrpc_stdio")));
        backends.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("backend.prepare: {error}")],
            )
        })?;

        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation.clone()).with_typescript_runtime(Arc::new(
                NodeSdkBackendRuntime::bootstrap("@anthropic-ai/claude-agent-sdk"),
            )),
        );
        let inner_model = Arc::new(ClaudeModelProvider::new());
        let inner_tools = Arc::new(ClaudeToolProvider::new());
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.claude-code",
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
            lifecycle: ClaudeCodeLifecycleProvider::new(),
            model,
            tools,
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
    fn bootstrap_negotiates_claude_code_capabilities() {
        let integration =
            ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CLAUDE_CODE_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::TypeScriptNode)
        );
    }

    #[test]
    fn model_provider_routes_invoke_through_typescript_runtime() {
        let integration =
            ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap should succeed");
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
