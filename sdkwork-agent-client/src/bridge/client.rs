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
                let local_result = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .send_message(request.clone());

                match local_result {
                    Ok(response) => Ok(response),
                    Err(local_error) => {
                        let remote_client = self.remote_client.as_ref()
                            .ok_or_else(|| "Remote client not initialized for fallback".to_string())?;

                        match fallback_strategy {
                            FallbackStrategy::Immediate => {
                                remote_client.send_message(request)
                            }
                            FallbackStrategy::RetryThenFallback { max_retries } => {
                                for attempt in 0..*max_retries {
                                    if attempt > 0 {
                                        let delay_ms = 100 * 2_u64.pow(attempt - 1);
                                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                                    }

                                    match self.local_provider.as_ref().unwrap().send_message(request.clone()) {
                                        Ok(response) => return Ok(response),
                                        Err(_) => continue,
                                    }
                                }
                                remote_client.send_message(request)
                            }
                            FallbackStrategy::LocalOnly => {
                                Err(local_error)
                            }
                        }
                    }
                }
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
            AgentClientMode::Hybrid { .. } => {
                let local_result = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .get_messages(session_id, limit);

                match local_result {
                    Ok(messages) => Ok(messages),
                    Err(_) => {
                        self.remote_client.as_ref()
                            .ok_or_else(|| "Remote client not initialized for fallback".to_string())?
                            .get_messages(session_id, limit)
                    }
                }
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
            AgentClientMode::Hybrid { .. } => {
                let local_result = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .create_session(config.clone());

                match local_result {
                    Ok(session) => Ok(session),
                    Err(_) => {
                        self.remote_client.as_ref()
                            .ok_or_else(|| "Remote client not initialized for fallback".to_string())?
                            .create_session(config)
                    }
                }
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
            AgentClientMode::Hybrid { .. } => {
                let local_result = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .close_session(session_id);

                match local_result {
                    Ok(()) => Ok(()),
                    Err(_) => {
                        self.remote_client.as_ref()
                            .ok_or_else(|| "Remote client not initialized for fallback".to_string())?
                            .close_session(session_id)
                    }
                }
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
