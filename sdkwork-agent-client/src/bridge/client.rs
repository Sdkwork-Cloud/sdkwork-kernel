use std::sync::Arc;
use crate::chat::ChatClient;
use crate::sse::SseChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use super::types::{
    AgentBridgeMetadata,
    AgentProtocol, AgentAuth, FallbackStrategy,
};
use super::provider::AgentBridgeProvider;
use super::registry::AgentBridgePluginRegistry;

/// Agent client mode enum
#[derive(Debug, Clone)]
pub enum AgentClientMode {
    /// Remote mode: call SDKWork Agent Server
    Remote {
        server_url: String,
        protocol: AgentProtocol,
        auth: Option<AgentAuth>,
    },
    /// Local mode: call local bridge provider
    Local {
        bridge_id: String,
    },
    /// Hybrid mode: prefer local, fallback to remote
    Hybrid {
        bridge_id: String,
        server_url: String,
        protocol: AgentProtocol,
        auth: Option<AgentAuth>,
        fallback_strategy: FallbackStrategy,
    },
}

/// Unified agent client
pub struct AgentClient {
    mode: AgentClientMode,
    plugin_registry: Arc<AgentBridgePluginRegistry>,
    remote_client: Option<Box<dyn ChatClient>>,
    local_provider: Option<Arc<dyn AgentBridgeProvider>>,
}

impl AgentClient {
    /// Create new agent client
    pub fn new(
        mode: AgentClientMode,
        plugin_registry: Arc<AgentBridgePluginRegistry>,
    ) -> Result<Self, String> {
        let (remote_client, local_provider) = match &mode {
            AgentClientMode::Remote { server_url, protocol, auth } => {
                let remote = create_remote_client(server_url, protocol, auth)?;
                (Some(remote), None)
            }
            AgentClientMode::Local { bridge_id } => {
                let provider = plugin_registry.get_provider(bridge_id)
                    .ok_or_else(|| format!("Bridge provider not found: {}", bridge_id))?;
                (None, Some(provider))
            }
            AgentClientMode::Hybrid { bridge_id, server_url, protocol, auth, .. } => {
                let remote = create_remote_client(server_url, protocol, auth)?;
                let provider = plugin_registry.get_provider(bridge_id)
                    .ok_or_else(|| format!("Bridge provider not found: {}", bridge_id))?;
                (Some(remote), Some(provider))
            }
        };

        Ok(Self {
            mode,
            plugin_registry,
            remote_client,
            local_provider,
        })
    }

    /// Initialize client
    pub fn initialize(&self) -> Result<(), String> {
        if let Some(provider) = &self.local_provider {
            provider.initialize()?;
        }
        Ok(())
    }

    /// Get active provider information
    pub fn active_provider_info(&self) -> Option<AgentBridgeMetadata> {
        self.local_provider.as_ref().map(|p| p.metadata())
    }

    /// Switch mode
    pub fn switch_mode(&mut self, mode: AgentClientMode) -> Result<(), String> {
        let (remote_client, local_provider) = match &mode {
            AgentClientMode::Remote { server_url, protocol, auth } => {
                let remote = create_remote_client(server_url, protocol, auth)?;
                (Some(remote), None)
            }
            AgentClientMode::Local { bridge_id } => {
                let provider = self.plugin_registry.get_provider(bridge_id)
                    .ok_or_else(|| format!("Bridge provider not found: {}", bridge_id))?;
                (None, Some(provider))
            }
            AgentClientMode::Hybrid { bridge_id, server_url, protocol, auth, .. } => {
                let remote = create_remote_client(server_url, protocol, auth)?;
                let provider = self.plugin_registry.get_provider(bridge_id)
                    .ok_or_else(|| format!("Bridge provider not found: {}", bridge_id))?;
                (Some(remote), Some(provider))
            }
        };

        self.mode = mode;
        self.remote_client = remote_client;
        self.local_provider = local_provider;
        Ok(())
    }

    /// Helper: dispatch with fallback based on FallbackStrategy
    fn dispatch_with_fallback<T>(
        &self,
        fallback_strategy: &FallbackStrategy,
        local_fn: impl Fn(&dyn ChatClient) -> Result<T, String>,
        remote_fn: impl Fn(&dyn ChatClient) -> Result<T, String>,
    ) -> Result<T, String> {
        let local_provider = self.local_provider.as_ref()
            .ok_or_else(|| "Local provider not initialized".to_string())?;

        let local_result = local_fn(local_provider.as_ref());

        match local_result {
            Ok(result) => Ok(result),
            Err(local_error) => {
                let remote_client = self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized for fallback".to_string())?;

                match fallback_strategy {
                    FallbackStrategy::Immediate => {
                        remote_fn(remote_client.as_ref())
                    }
                    FallbackStrategy::RetryThenFallback { max_retries } => {
                        for attempt in 1..=*max_retries {
                            let delay_ms = 100 * 2_u64.pow(attempt - 1);
                            std::thread::sleep(std::time::Duration::from_millis(delay_ms));

                            match local_fn(local_provider.as_ref()) {
                                Ok(result) => return Ok(result),
                                Err(_) => continue,
                            }
                        }
                        remote_fn(remote_client.as_ref())
                    }
                    FallbackStrategy::LocalOnly => {
                        Err(local_error)
                    }
                }
            }
        }
    }
}

/// Helper function to create remote client based on protocol
fn create_remote_client(
    server_url: &str,
    protocol: &AgentProtocol,
    _auth: &Option<AgentAuth>,
) -> Result<Box<dyn ChatClient>, String> {
    match protocol {
        AgentProtocol::HttpRestSse => {
            Ok(Box::new(SseChatClient::new(server_url)))
        }
        AgentProtocol::Grpc => {
            Err("gRPC client not yet implemented - deferred to Phase 3".to_string())
        }
        AgentProtocol::WebSocket => {
            Err("WebSocket client not yet implemented".to_string())
        }
    }
}

impl ChatClient for AgentClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .send_message(request)
            }
            AgentClientMode::Local { .. } => {
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .send_message(request)
            }
            AgentClientMode::Hybrid { fallback_strategy, .. } => {
                self.dispatch_with_fallback(
                    fallback_strategy,
                    |client| client.send_message(request.clone()),
                    |client| client.send_message(request),
                )
            }
        }
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .get_messages(session_id, limit)
            }
            AgentClientMode::Local { .. } => {
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .get_messages(session_id, limit)
            }
            AgentClientMode::Hybrid { fallback_strategy, .. } => {
                self.dispatch_with_fallback(
                    fallback_strategy,
                    |client| client.get_messages(session_id, limit),
                    |client| client.get_messages(session_id, limit),
                )
            }
        }
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .create_session(config)
            }
            AgentClientMode::Local { .. } => {
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .create_session(config)
            }
            AgentClientMode::Hybrid { fallback_strategy, .. } => {
                self.dispatch_with_fallback(
                    fallback_strategy,
                    |client| client.create_session(config.clone()),
                    |client| client.create_session(config),
                )
            }
        }
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .close_session(session_id)
            }
            AgentClientMode::Local { .. } => {
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .close_session(session_id)
            }
            AgentClientMode::Hybrid { fallback_strategy, .. } => {
                self.dispatch_with_fallback(
                    fallback_strategy,
                    |client| client.close_session(session_id),
                    |client| client.close_session(session_id),
                )
            }
        }
    }

    fn health(&self) -> Result<bool, String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .health()
            }
            AgentClientMode::Local { .. } => {
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .health()
            }
            AgentClientMode::Hybrid { .. } => {
                // For health, check both and return healthy if either is healthy
                let local_health = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .health();
                let remote_health = self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .health();

                match (local_health, remote_health) {
                    (Ok(true), _) | (_, Ok(true)) => Ok(true),
                    (Ok(false), Ok(false)) => Ok(false),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            }
        }
    }
}
