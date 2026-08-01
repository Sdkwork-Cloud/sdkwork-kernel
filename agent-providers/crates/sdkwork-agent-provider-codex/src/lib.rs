use sdkwork_agent_kernel::{
    KernelResult, ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelStreamChunk, ProviderHealth, ProviderManifest,
};

mod app_server;
mod local_plugins;
mod provider_sessions;

pub use app_server::{CodexInProcessThreadClient, CodexThreadClient};
pub use codex_app_server_protocol::{
    SortDirection as CodexSortDirection, Thread, ThreadActiveFlag, ThreadHistoryMode, ThreadItem,
    ThreadItemEntry, ThreadItemsListParams, ThreadItemsListResponse, ThreadListParams,
    ThreadListResponse, ThreadReadParams, ThreadReadResponse, ThreadSortKey, ThreadSourceKind,
    ThreadStatus, ThreadTurnsListParams, ThreadTurnsListResponse, Turn, TurnItemsView, TurnStatus,
};
pub use local_plugins::CodexLocalPluginProvider;
pub use provider_sessions::{
    map_item_page, map_thread_page, map_thread_record, normalize_page_limit, CodexAdapter,
    CodexMessageAdapter, CodexMessagePage, CodexMessageRecord, CodexSessionPage,
    CodexSessionRecord, CodexThreadActivityObservation,
};

#[derive(Clone)]
pub struct CodexModelProvider {
    default_model: String,
}

impl CodexModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: "codex-1".to_string(),
        }
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for CodexModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for CodexModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.codex",
            "model",
            "Codex Model Provider",
            "0.1.0",
            vec![
                "model.chat".to_string(),
                "model.stream".to_string(),
                "model.tool_call".to_string(),
                "model.reasoning".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new("codex-mini", "provider.model.codex", "Codex Mini", "codex")
                .with_version("mini")
                .with_capability("chat")
                .with_capability("tool_call")
                .with_context_window_tokens(128000)
                .with_max_output_tokens(16000)
                .with_input_mode("text")
                .with_output_mode("text")
                .with_response_format(ModelResponseFormat::Text)
                .with_tool_capability("function_calling"),
            ModelDescriptor::new("codex-1", "provider.model.codex", "Codex 1", "codex")
                .with_version("1.0")
                .with_capability("chat")
                .with_capability("tool_call")
                .with_capability("reasoning")
                .with_context_window_tokens(200000)
                .with_max_output_tokens(32000)
                .with_input_mode("text")
                .with_output_mode("text")
                .with_response_format(ModelResponseFormat::Text)
                .with_response_format(ModelResponseFormat::Json)
                .with_tool_capability("function_calling"),
            ModelDescriptor::new(
                "codex-1-pro",
                "provider.model.codex",
                "Codex 1 Pro",
                "codex",
            )
            .with_version("1.0")
            .with_capability("chat")
            .with_capability("tool_call")
            .with_capability("reasoning")
            .with_context_window_tokens(200000)
            .with_max_output_tokens(64000)
            .with_input_mode("text")
            .with_output_mode("text")
            .with_response_format(ModelResponseFormat::Text)
            .with_response_format(ModelResponseFormat::Json)
            .with_tool_capability("function_calling"),
        ]
    }

    fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        sdkwork_agent_provider_core::reject_in_process_model_invoke("provider.model.codex")
    }

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        sdkwork_agent_provider_core::reject_in_process_model_stream("provider.model.codex")
    }
}

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(CodexLifecycleProvider, "codex");

mod agent_definition;
mod configuration;
mod conformance;
pub mod ids;
mod installer;
mod manifest;
mod package;
pub mod sdk_integration;

pub use agent_definition::{codex_agent_definition, codex_agent_manifest};
pub use configuration::CodexConfigurationProvider;
pub use conformance::codex_conformance_profile;
pub use installer::{codex_agent_installer, CODEX_SDK_PACKAGE, CODEX_SDK_VERSION};
pub use manifest::{codex_kernel_plugin_manifest, codex_provider_manifests, CodexKernelPlugin};
pub use package::codex_package_manifest;
pub use sdk_integration::{codex_binding_manifest, CodexSdkIntegration};

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{KernelError, SessionKind, SessionSource};
    use sdkwork_agent_provider_core::{SessionConfig, SessionLifecycleProvider};

    #[test]
    fn lifecycle_provider_create_and_resume() {
        let provider = CodexLifecycleProvider::new();
        let config = SessionConfig::new()
            .with_source(SessionSource::Cli)
            .with_kind(SessionKind::Main);
        let created = provider
            .create_session("agent.1", Some("user.1"), config)
            .expect("create session");

        let resumed = provider
            .resume_session(&created.session_id)
            .expect("resume session");
        assert!(resumed.state.is_active());
    }

    #[test]
    fn model_provider_invoke_requires_transport_worker() {
        let provider = CodexModelProvider::new();
        let request = ModelRequest::new("req.1", vec!["Explain lifetimes".to_string()])
            .with_model_id("codex-1");
        let error = provider
            .invoke(request)
            .expect_err("in-process invoke is forbidden");
        assert!(matches!(error, KernelError::ProviderUnavailable { .. }));
    }

    #[test]
    fn model_provider_lists_supported_models() {
        let models = CodexModelProvider::new().list_models();
        assert_eq!(models.len(), 3);
        assert!(models
            .iter()
            .any(|model| model.supports_capability("reasoning")));
    }
}
