use crate::bridge::{
    AgentBridgeConfig, AgentBridgeHealth, AgentBridgeMetadata, AgentBridgePlugin,
    AgentBridgeProvider, AgentBridgeStatus, AgentBridgeType,
};
use crate::chat::ChatClient;
use crate::runtime_guard::lock_runtime_mutex;
use crate::session::{BridgeSessionQuery, SdkModelBridgeRuntime};
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};
use sdkwork_agent_kernel::ModelProvider;
use sdkwork_agent_provider_claude_code::ClaudeCodeSdkIntegration;
use sdkwork_agent_provider_gemini_cli::GeminiCliSdkIntegration;
use sdkwork_agent_provider_opencode::OpenCodeSdkIntegration;
use sdkwork_agent_provider_spi::{SdkRuntimeError, SdkRuntimeRequest, SdkRuntimeResponse};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
enum SdkBackedKind {
    ClaudeCode,
    OpenCode,
    GeminiCli,
}

impl SdkBackedKind {
    fn provider_id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::OpenCode => "opencode",
            Self::GeminiCli => "gemini-cli",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::GeminiCli => "Gemini CLI",
        }
    }
}

struct SdkBackedRuntime {
    _integration: Arc<dyn Send + Sync>,
    inner: SdkModelBridgeRuntime,
}

impl SdkBackedRuntime {
    fn new(config: &AgentBridgeConfig, kind: SdkBackedKind) -> Result<Self, String> {
        match kind {
            SdkBackedKind::ClaudeCode => {
                let integration = Arc::new(
                    ClaudeCodeSdkIntegration::bootstrap()
                        .map_err(|error| format!("claude-code sdk bootstrap failed: {error}"))?,
                );
                let invoke_integration = Arc::clone(&integration);
                let invoke_model = Arc::new(move |request| {
                    invoke_integration
                        .model
                        .invoke(request)
                        .map_err(|error| format!("claude-code model invoke failed: {error}"))
                });
                let health_integration = Arc::clone(&integration);
                let health_probe = Arc::new(move || {
                    runtime_health_from_ping(
                        "claude-code",
                        health_integration
                            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.session.lifecycle")),
                    )
                });
                Self::from_parts(config, kind, integration, invoke_model, health_probe)
            }
            SdkBackedKind::OpenCode => {
                let integration = Arc::new(
                    OpenCodeSdkIntegration::bootstrap()
                        .map_err(|error| format!("opencode sdk bootstrap failed: {error}"))?,
                );
                let invoke_integration = Arc::clone(&integration);
                let invoke_model = Arc::new(move |request| {
                    invoke_integration
                        .model
                        .invoke(request)
                        .map_err(|error| format!("opencode model invoke failed: {error}"))
                });
                let health_integration = Arc::clone(&integration);
                let health_probe = Arc::new(move || {
                    runtime_health_from_ping(
                        "opencode",
                        health_integration
                            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.session.lifecycle")),
                    )
                });
                Self::from_parts(config, kind, integration, invoke_model, health_probe)
            }
            SdkBackedKind::GeminiCli => {
                let integration = Arc::new(
                    GeminiCliSdkIntegration::bootstrap()
                        .map_err(|error| format!("gemini-cli sdk bootstrap failed: {error}"))?,
                );
                let invoke_integration = Arc::clone(&integration);
                let invoke_model = Arc::new(move |request| {
                    invoke_integration
                        .model
                        .invoke(request)
                        .map_err(|error| format!("gemini-cli model invoke failed: {error}"))
                });
                let health_integration = Arc::clone(&integration);
                let health_probe = Arc::new(move || {
                    runtime_health_from_ping(
                        "gemini-cli",
                        health_integration
                            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.session.lifecycle")),
                    )
                });
                Self::from_parts(config, kind, integration, invoke_model, health_probe)
            }
        }
    }

    fn from_parts<I>(
        config: &AgentBridgeConfig,
        kind: SdkBackedKind,
        integration: Arc<I>,
        invoke_model: crate::session::ModelInvokeFn,
        health_probe: crate::session::HealthProbeFn,
    ) -> Result<Self, String>
    where
        I: Send + Sync + 'static,
    {
        Ok(Self {
            inner: SdkModelBridgeRuntime::new(
                kind.provider_id(),
                &config.bridge_id,
                invoke_model,
                health_probe,
            )?,
            _integration: integration,
        })
    }
}

fn runtime_health_from_ping(
    provider: &str,
    result: Result<SdkRuntimeResponse, SdkRuntimeError>,
) -> AgentBridgeHealth {
    match result {
        Ok(response) if response.success => AgentBridgeHealth::healthy(),
        Ok(response) => AgentBridgeHealth::unhealthy(format!(
            "{provider} runtime ping failed: {}",
            response
                .message
                .unwrap_or_else(|| "unknown error".to_string())
        )),
        Err(error) => AgentBridgeHealth {
            status: AgentBridgeStatus::Degraded,
            message: Some(format!("{provider} runtime unavailable: {error}")),
            last_check: chrono::Utc::now(),
        },
    }
}

struct SdkBackedProvider {
    config: AgentBridgeConfig,
    kind: SdkBackedKind,
    runtime: Mutex<Option<SdkBackedRuntime>>,
    capabilities: Vec<String>,
}

impl SdkBackedProvider {
    fn new(config: AgentBridgeConfig, kind: SdkBackedKind) -> Result<Self, String> {
        Ok(Self {
            config,
            kind,
            runtime: Mutex::new(None),
            capabilities: vec![
                "chat".to_string(),
                "tool_call".to_string(),
                "session_history".to_string(),
            ],
        })
    }
}

impl ChatClient for SdkBackedProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        runtime
            .as_ref()
            .ok_or_else(|| "Runtime not initialized".to_string())?
            .inner
            .send_message(request)
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        runtime
            .as_ref()
            .ok_or_else(|| "Runtime not initialized".to_string())?
            .inner
            .get_messages(session_id, limit)
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        runtime
            .as_ref()
            .ok_or_else(|| "Runtime not initialized".to_string())?
            .inner
            .create_session(config)
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        runtime
            .as_ref()
            .ok_or_else(|| "Runtime not initialized".to_string())?
            .inner
            .close_session(session_id)
    }

    fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        let runtime = lock_runtime_mutex(&self.runtime)?;
        runtime
            .as_ref()
            .ok_or_else(|| "Runtime not initialized".to_string())?
            .inner
            .list_sessions(query)
    }

    fn health(&self) -> Result<bool, String> {
        Ok(self.health_check().status == AgentBridgeStatus::Healthy)
    }
}

impl AgentBridgeProvider for SdkBackedProvider {
    fn bridge_id(&self) -> &str {
        &self.config.bridge_id
    }

    fn bridge_type(&self) -> &AgentBridgeType {
        &self.config.bridge_type
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn initialize(&self) -> Result<(), String> {
        let mut runtime = lock_runtime_mutex(&self.runtime)?;
        if runtime.is_none() {
            *runtime = Some(SdkBackedRuntime::new(&self.config, self.kind)?);
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), String> {
        lock_runtime_mutex(&self.runtime)?.take();
        Ok(())
    }

    fn health_check(&self) -> AgentBridgeHealth {
        match lock_runtime_mutex(&self.runtime) {
            Ok(runtime) => match runtime.as_ref() {
                Some(runtime) => runtime.inner.health_check(),
                None => AgentBridgeHealth {
                    status: AgentBridgeStatus::Unknown,
                    message: Some("Runtime not initialized".to_string()),
                    last_check: chrono::Utc::now(),
                },
            },
            Err(message) => AgentBridgeHealth {
                status: AgentBridgeStatus::Unknown,
                message: Some(message),
                last_check: chrono::Utc::now(),
            },
        }
    }

    fn metadata(&self) -> AgentBridgeMetadata {
        AgentBridgeMetadata {
            bridge_id: self.config.bridge_id.clone(),
            bridge_type: self.config.bridge_type.clone(),
            version: "1.0.0".to_string(),
            description: format!("{} agent bridge provider", self.kind.display_name()),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities.clone(),
            config_schema: None,
        }
    }
}

macro_rules! define_sdk_backed_plugin {
    ($provider:ident, $plugin:ident, $kind:ident, $bridge_type:ident, $plugin_id:literal, $name:literal) => {
        pub struct $provider {
            inner: SdkBackedProvider,
        }

        impl $provider {
            pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
                Ok(Self {
                    inner: SdkBackedProvider::new(config, SdkBackedKind::$kind)?,
                })
            }
        }

        impl ChatClient for $provider {
            fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
                self.inner.send_message(request)
            }

            fn get_messages(
                &self,
                session_id: &str,
                limit: Option<u32>,
            ) -> Result<Vec<ChatMessage>, String> {
                self.inner.get_messages(session_id, limit)
            }

            fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
                self.inner.create_session(config)
            }

            fn close_session(&self, session_id: &str) -> Result<(), String> {
                self.inner.close_session(session_id)
            }

            fn list_sessions(
                &self,
                query: &BridgeSessionQuery,
            ) -> Result<Vec<SessionInfo>, String> {
                self.inner.list_sessions(query)
            }

            fn health(&self) -> Result<bool, String> {
                self.inner.health()
            }
        }

        impl AgentBridgeProvider for $provider {
            fn bridge_id(&self) -> &str {
                self.inner.bridge_id()
            }

            fn bridge_type(&self) -> &AgentBridgeType {
                self.inner.bridge_type()
            }

            fn version(&self) -> &str {
                self.inner.version()
            }

            fn capabilities(&self) -> &[String] {
                self.inner.capabilities()
            }

            fn initialize(&self) -> Result<(), String> {
                self.inner.initialize()
            }

            fn shutdown(&self) -> Result<(), String> {
                self.inner.shutdown()
            }

            fn health_check(&self) -> AgentBridgeHealth {
                self.inner.health_check()
            }

            fn metadata(&self) -> AgentBridgeMetadata {
                self.inner.metadata()
            }
        }

        pub struct $plugin;

        impl $plugin {
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $plugin {
            fn default() -> Self {
                Self::new()
            }
        }

        impl AgentBridgePlugin for $plugin {
            fn plugin_id(&self) -> &str {
                $plugin_id
            }

            fn name(&self) -> &str {
                $name
            }

            fn version(&self) -> &str {
                "1.0.0"
            }

            fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
                vec![AgentBridgeType::$bridge_type]
            }

            fn create_provider(
                &self,
                bridge_type: AgentBridgeType,
                config: AgentBridgeConfig,
            ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
                if bridge_type != AgentBridgeType::$bridge_type {
                    return Err(format!("Unsupported bridge type: {bridge_type:?}"));
                }
                Ok(Arc::new($provider::new(config)?))
            }

            fn config_schema(&self) -> Option<String> {
                None
            }

            fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String> {
                if config.bridge_type != AgentBridgeType::$bridge_type {
                    return Err(format!(
                        "{} plugin requires {} bridge type",
                        $name,
                        AgentBridgeType::$bridge_type
                    ));
                }
                Ok(())
            }
        }
    };
}

define_sdk_backed_plugin!(
    ClaudeCodeProvider,
    ClaudeCodePlugin,
    ClaudeCode,
    ClaudeCode,
    "builtin.claude-code",
    "Claude Code Bridge Plugin"
);
define_sdk_backed_plugin!(
    OpenCodeProvider,
    OpenCodePlugin,
    OpenCode,
    OpenCode,
    "builtin.opencode",
    "OpenCode Bridge Plugin"
);
define_sdk_backed_plugin!(
    GeminiCliProvider,
    GeminiCliPlugin,
    GeminiCli,
    GeminiCli,
    "builtin.gemini-cli",
    "Gemini CLI Bridge Plugin"
);
