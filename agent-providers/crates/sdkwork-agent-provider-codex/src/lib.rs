use sdkwork_agent_kernel::{
    KernelResult, ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelStreamChunk, ProviderHealth, ProviderManifest,
};
use std::path::PathBuf;

mod app_server;
mod local_plugins;
mod provider_sessions;

pub use app_server::{CodexInProcessThreadClient, CodexThreadClient};
pub use codex_app_server_protocol::{
    SortDirection as CodexSortDirection, Thread, ThreadActiveFlag, ThreadHistoryMode, ThreadItem,
    ThreadItemEntry, ThreadItemsListParams, ThreadItemsListResponse, ThreadListCwdFilter,
    ThreadListParams, ThreadListResponse, ThreadReadParams, ThreadReadResponse, ThreadSortKey,
    ThreadSourceKind, ThreadStatus, ThreadTurnsListParams, ThreadTurnsListResponse, Turn,
    TurnItemsView, TurnStatus,
};
pub use local_plugins::CodexLocalPluginProvider;
pub use provider_sessions::{
    map_item_page, map_thread_page, map_thread_record, map_turn_page, normalize_page_limit,
    CodexAdapter, CodexMessageAdapter, CodexMessagePage, CodexMessageRecord, CodexSessionPage,
    CodexSessionRecord, CodexThreadActivityObservation,
};

/// Reads the model id configured for the local Codex installation.
///
/// Precedence: the `CODEX_MODEL` environment override, then the top-level
/// `model` key in the Codex config file (`CODEX_HOME`/`~/.codex/config.toml`).
/// Returns `None` when the configuration cannot be read so the model catalog
/// can fall back to the built-in Codex model list.
pub fn configured_codex_model_id() -> Option<String> {
    if let Some(model) = std::env::var("CODEX_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(model);
    }
    let config_path = codex_config_path()?;
    let content = std::fs::read_to_string(config_path).ok()?;
    let document: toml::Value = content.parse().ok()?;
    document
        .get("model")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn codex_config_path() -> Option<PathBuf> {
    if let Some(codex_home) = std::env::var_os("CODEX_HOME") {
        let codex_home = PathBuf::from(codex_home);
        if !codex_home.is_dir() {
            return None;
        }
        return Some(codex_home.join("config.toml"));
    }
    let home = sdkwork_agent_provider_core::provider_user_home()?;
    Some(home.join(".codex").join("config.toml"))
}

#[derive(Clone)]
pub struct CodexModelProvider {
    default_model: String,
}

impl CodexModelProvider {
    pub fn new() -> Self {
        Self {
            default_model: configured_codex_model_id().unwrap_or_else(|| "codex-1".to_string()),
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
        let mut models = vec![
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
        ];
        // Surface the model configured for the local Codex installation first so
        // default model selection always matches what the Codex CLI would use.
        // The configured model may be provider-specific (for example a custom
        // model endpoint), so it must appear even when it is not part of the
        // built-in Codex catalog.
        if self.default_model != "codex-1"
            && !models
                .iter()
                .any(|model| model.model_id == self.default_model)
        {
            models.insert(
                0,
                ModelDescriptor::new(
                    &self.default_model,
                    "provider.model.codex",
                    &self.default_model,
                    "codex",
                )
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
            );
        }
        models
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
pub use installer::{codex_agent_installer, CODEX_CLI_PACKAGE, CODEX_CLI_VERSION};
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
        assert!(models.len() >= 3);
        assert!(models
            .iter()
            .any(|model| model.supports_capability("reasoning")));
        if let Some(configured) = configured_codex_model_id() {
            assert_eq!(models[0].model_id, configured);
        }
    }

    #[test]
    fn configured_model_precedes_builtin_catalog() {
        let provider = CodexModelProvider::new().with_default_model("gpt-5.6-sol");
        let models = provider.list_models();
        assert_eq!(models[0].model_id, "gpt-5.6-sol");
        assert!(models
            .iter()
            .any(|model| model.model_id == "codex-1"));
    }
}
