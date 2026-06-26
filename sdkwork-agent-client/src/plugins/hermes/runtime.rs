use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};
use crate::session::SdkModelBridgeRuntime;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, SessionConfig, SessionInfo};
use sdkwork_agent_provider_hermes::HermesSdkIntegration;
use sdkwork_agent_kernel::ModelProvider;
use sdkwork_agent_provider_spi::SdkRuntimeRequest;
use std::sync::Arc;

/// Hermes runtime handle backed by negotiated SDK model routing and SQLite session storage.
pub struct HermesRuntime {
    _integration: Arc<HermesSdkIntegration>,
    inner: SdkModelBridgeRuntime,
}

impl HermesRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        let integration = Arc::new(
            HermesSdkIntegration::bootstrap()
                .map_err(|error| format!("hermes sdk bootstrap failed: {error}"))?,
        );
        let invoke_integration = Arc::clone(&integration);
        let invoke_model = Arc::new(move |request| {
            invoke_integration
                .model
                .invoke(request)
                .map_err(|error| format!("hermes model invoke failed: {error}"))
        });
        let health_integration = Arc::clone(&integration);
        let health_probe = Arc::new(move || runtime_health_from_ping(&health_integration));

        Ok(Self {
            inner: SdkModelBridgeRuntime::new(
                "hermes",
                &config.bridge_id,
                invoke_model,
                health_probe,
            )?,
            _integration: integration,
        })
    }

    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        self.inner.send_message(request)
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.inner.get_messages(session_id, limit)
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        self.inner.create_session(config)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.inner.close_session(session_id)
    }

    pub fn health_check(&self) -> AgentBridgeHealth {
        self.inner.health_check()
    }

    pub fn list_sessions(
        &self,
        query: &crate::session::BridgeSessionQuery,
    ) -> Result<Vec<SessionInfo>, String> {
        self.inner.list_sessions(query)
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn runtime_health_from_ping(integration: &HermesSdkIntegration) -> AgentBridgeHealth {
    match integration.invoke_runtime(&SdkRuntimeRequest::ping("sdk.session.lifecycle")) {
        Ok(response) if response.success => AgentBridgeHealth::healthy(),
        Ok(response) => AgentBridgeHealth::unhealthy(format!(
            "hermes runtime ping failed: {}",
            response.message.unwrap_or_else(|| "unknown error".to_string())
        )),
        Err(error) => AgentBridgeHealth {
            status: AgentBridgeStatus::Degraded,
            message: Some(format!("hermes runtime unavailable: {error}")),
            last_check: chrono::Utc::now(),
        },
    }
}
