> Migrated from `docs/superpowers/plans/2026-06-14-multi-mode-agent-system.md` on 2026-06-24.
> Owner: SDKWork maintainers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a multi-mode agent system supporting client/server modes, local/remote agent capabilities, and multiple agent framework implementations (OpenClaw, ZeroClaw, Hermes).

**Architecture:** Bridge provider pattern using SDKWork Agent Kernel's Provider SPI. Each agent framework implements `AgentBridgeProvider` trait (inherits `ChatClient`). Client mode supports remote (SDKWork Agent Server), local (bridge providers), and hybrid (fallback) modes. Server mode exposes HTTP/REST + SSE API.

**Tech Stack:** Rust, SDKWork Agent Kernel, SDKWork Agent Client, chrono, uuid, serde, axum (HTTP), tokio-tungstenite (WebSocket)

---

## Scope

This plan covers **Phase 1 (Core Infrastructure)** and **Phase 2 (Built-in Plugins)** from the design document. Phase 3 (Server Mode), Phase 4 (Kernel Integration), and Phase 5 (Packaging) are separate efforts.

**In Scope:**
- AgentBridgeProvider trait and supporting types
- AgentBridgePlugin trait and plugin registry
- AgentClient with Remote/Local/Hybrid modes
- OpenClaw, ZeroClaw, and Hermes bridge providers

**Out of Scope:**
- Server mode (HTTP/REST, gRPC, WebSocket)
- Kernel integration (BridgeProviderAdapter, KernelIntegrator)
- Packaging and deployment

---

## File Structure

### New Files to Create

```
sdkwork-agent-client/
├── src/
│   ├── bridge/
│   │   ├── mod.rs                    # Module exports
│   │   ├── provider.rs               # AgentBridgeProvider trait
│   │   ├── plugin.rs                 # AgentBridgePlugin trait
│   │   ├── registry.rs               # AgentBridgePluginRegistry
│   │   ├── types.rs                  # AgentBridgeType, Health, Metadata, Config
│   │   └── client.rs                 # AgentClient (Remote/Local/Hybrid)
│   ├── plugins/
│   │   ├── mod.rs                    # Module exports
│   │   ├── openclaw/
│   │   │   ├── mod.rs                # OpenClawPlugin + OpenClawProvider
│   │   │   └── runtime.rs            # OpenClawRuntime (placeholder)
│   │   ├── zeroclaw/
│   │   │   ├── mod.rs                # ZeroClawPlugin + ZeroClawProvider
│   │   │   └── runtime.rs            # ZeroClawRuntime (placeholder)
│   │   └── hermes/
│   │       ├── mod.rs                # HermesPlugin + HermesProvider
│   │       └── runtime.rs            # HermesRuntime (placeholder)
│   └── lib.rs                        # Add bridge and plugins modules
└── Cargo.toml                        # Add chrono, uuid dependencies
```

### Files to Modify

```
sdkwork-agent-client/
├── src/
│   ├── lib.rs                        # Add bridge and plugins module declarations
│   └── types.rs                      # Add AgentAuth type (if needed)
└── Cargo.toml                        # Add new dependencies
```

---

## Tasks

### Task 1: Add Dependencies to Cargo.toml

**Files:**
- Modify: `sdkwork-agent-client/Cargo.toml`

- [ ] **Step 1: Read current Cargo.toml**

Read `sdkwork-agent-client/Cargo.toml` to understand current dependencies.

- [ ] **Step 2: Add new dependencies**

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: Verify dependencies compile**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/Cargo.toml
git commit -m "feat(agent-client): add chrono, uuid, serde dependencies for bridge system"
```

---

### Task 2: Create Bridge Types

**Files:**
- Create: `sdkwork-agent-client/src/bridge/mod.rs`
- Create: `sdkwork-agent-client/src/bridge/types.rs`

- [ ] **Step 1: Create bridge module directory**

```bash
mkdir -p sdkwork-agent-client/src/bridge
```

- [ ] **Step 2: Create types.rs with all bridge types**

```rust
use std::collections::HashMap;
use std::fmt;

/// Agent bridge type enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentBridgeType {
    OpenClaw,
    ZeroClaw,
    Hermes,
    Custom(String),
}

impl fmt::Display for AgentBridgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenClaw => write!(f, "openclaw"),
            Self::ZeroClaw => write!(f, "zeroclaw"),
            Self::Hermes => write!(f, "hermes"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Agent bridge status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentBridgeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl fmt::Display for AgentBridgeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Agent bridge health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBridgeHealth {
    pub status: AgentBridgeStatus,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl AgentBridgeHealth {
    pub fn healthy() -> Self {
        Self {
            status: AgentBridgeStatus::Healthy,
            message: None,
            last_check: chrono::Utc::now(),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: AgentBridgeStatus::Unhealthy,
            message: Some(message.into()),
            last_check: chrono::Utc::now(),
        }
    }
}

/// Agent bridge metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBridgeMetadata {
    pub bridge_id: String,
    pub bridge_type: AgentBridgeType,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub config_schema: Option<String>,
}

/// Agent bridge configuration
#[derive(Debug, Clone)]
pub struct AgentBridgeConfig {
    pub bridge_id: String,
    pub bridge_type: AgentBridgeType,
    pub settings: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

impl AgentBridgeConfig {
    pub fn new(
        bridge_id: impl Into<String>,
        bridge_type: AgentBridgeType,
    ) -> Self {
        Self {
            bridge_id: bridge_id.into(),
            bridge_type,
            settings: HashMap::new(),
            secrets: HashMap::new(),
        }
    }

    pub fn with_setting(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.insert(key.into(), value.into());
        self
    }

    pub fn with_secret(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.secrets.insert(key.into(), value.into());
        self
    }
}

/// Agent protocol enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentProtocol {
    HttpRestSse,
    Grpc,
    WebSocket,
}

/// Agent authentication type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAuthType {
    ApiKey,
    BearerToken,
    BasicAuth,
    OAuth2,
}

/// Agent authentication information
#[derive(Debug, Clone)]
pub struct AgentAuth {
    pub auth_type: AgentAuthType,
    pub credentials: HashMap<String, String>,
}

impl AgentAuth {
    pub fn api_key(key: impl Into<String>) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("api_key".to_string(), key.into());
        Self {
            auth_type: AgentAuthType::ApiKey,
            credentials,
        }
    }
}

/// Fallback strategy for hybrid mode
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    Immediate,
    RetryThenFallback { max_retries: u32 },
    LocalOnly,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self::Immediate
    }
}
```

- [ ] **Step 3: Create mod.rs**

```rust
mod types;

pub use types::*;
```

- [ ] **Step 4: Verify types compile**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add sdkwork-agent-client/src/bridge/
git commit -m "feat(agent-client): add bridge types (AgentBridgeType, Health, Metadata, Config)"
```

---

### Task 3: Create AgentBridgeProvider Trait

**Files:**
- Create: `sdkwork-agent-client/src/bridge/provider.rs`
- Modify: `sdkwork-agent-client/src/bridge/mod.rs`

- [ ] **Step 1: Create provider.rs with trait definition**

```rust
use std::sync::Arc;
use crate::chat::ChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use super::types::{AgentBridgeType, AgentBridgeHealth, AgentBridgeMetadata, AgentBridgeConfig};

/// Unified Agent Bridge Provider trait
/// Inherits from ChatClient — the ChatClient implementation IS the bridge logic
pub trait AgentBridgeProvider: ChatClient + Send + Sync {
    /// Return bridge provider identifier
    fn bridge_id(&self) -> &str;
    
    /// Return bridge provider type
    fn bridge_type(&self) -> AgentBridgeType;
    
    /// Return bridge provider version
    fn version(&self) -> &str;
    
    /// Return supported capabilities
    fn capabilities(&self) -> Vec<String>;
    
    /// Check if specified capability is supported
    fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities().iter().any(|c| c == capability)
    }
    
    /// Initialize bridge provider (uses interior mutability)
    fn initialize(&self) -> Result<(), String>;
    
    /// Shutdown bridge provider (uses interior mutability)
    fn shutdown(&self) -> Result<(), String>;
    
    /// Health check
    fn health_check(&self) -> AgentBridgeHealth;
    
    /// Get provider metadata
    fn metadata(&self) -> AgentBridgeMetadata;
}
```

- [ ] **Step 2: Update mod.rs to include provider**

```rust
mod types;
mod provider;

pub use types::*;
pub use provider::AgentBridgeProvider;
```

- [ ] **Step 3: Verify trait compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/src/bridge/provider.rs sdkwork-agent-client/src/bridge/mod.rs
git commit -m "feat(agent-client): add AgentBridgeProvider trait"
```

---

### Task 4: Create AgentBridgePlugin Trait

**Files:**
- Create: `sdkwork-agent-client/src/bridge/plugin.rs`
- Modify: `sdkwork-agent-client/src/bridge/mod.rs`

- [ ] **Step 1: Create plugin.rs with trait definition**

```rust
use std::sync::Arc;
use super::types::{AgentBridgeType, AgentBridgeConfig};
use super::provider::AgentBridgeProvider;

/// Agent Bridge Plugin trait
/// Supports runtime dynamic loading via dynamic libraries
pub trait AgentBridgePlugin: Send + Sync {
    /// Return plugin unique identifier
    fn plugin_id(&self) -> &str;
    
    /// Return plugin name
    fn name(&self) -> &str;
    
    /// Return plugin version
    fn version(&self) -> &str;
    
    /// Return supported bridge types
    fn supported_bridge_types(&self) -> Vec<AgentBridgeType>;
    
    /// Create bridge provider instance
    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String>;
    
    /// Return plugin configuration schema
    fn config_schema(&self) -> Option<String>;
    
    /// Validate configuration
    fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String>;
}
```

- [ ] **Step 2: Update mod.rs to include plugin**

```rust
mod types;
mod provider;
mod plugin;

pub use types::*;
pub use provider::AgentBridgeProvider;
pub use plugin::AgentBridgePlugin;
```

- [ ] **Step 3: Verify trait compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/src/bridge/plugin.rs sdkwork-agent-client/src/bridge/mod.rs
git commit -m "feat(agent-client): add AgentBridgePlugin trait"
```

---

### Task 5: Create AgentBridgePluginRegistry

**Files:**
- Create: `sdkwork-agent-client/src/bridge/registry.rs`
- Modify: `sdkwork-agent-client/src/bridge/mod.rs`

- [ ] **Step 1: Create registry.rs with implementation**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use super::types::{AgentBridgeType, AgentBridgeConfig, AgentBridgeHealth};
use super::provider::AgentBridgeProvider;
use super::plugin::AgentBridgePlugin;

/// Plugin registry
pub struct AgentBridgePluginRegistry {
    plugins: HashMap<String, Arc<dyn AgentBridgePlugin>>,
    providers: HashMap<String, Arc<dyn AgentBridgeProvider>>,
}

impl AgentBridgePluginRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            providers: HashMap::new(),
        }
    }
    
    /// Register plugin
    pub fn register_plugin(&mut self, plugin: Arc<dyn AgentBridgePlugin>) -> Result<(), String> {
        let plugin_id = plugin.plugin_id().to_string();
        if self.plugins.contains_key(&plugin_id) {
            return Err(format!("Plugin already registered: {}", plugin_id));
        }
        self.plugins.insert(plugin_id, plugin);
        Ok(())
    }
    
    /// Unregister plugin
    pub fn unregister_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(format!("Plugin not found: {}", plugin_id));
        }
        self.plugins.remove(plugin_id);
        Ok(())
    }
    
    /// Create provider instance
    pub fn create_provider(
        &mut self,
        plugin_id: &str,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<String, String> {
        let plugin = self.plugins.get(plugin_id)
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;
        
        let bridge_id = config.bridge_id.clone();
        let provider = plugin.create_provider(bridge_type, config)?;
        self.providers.insert(bridge_id.clone(), provider);
        Ok(bridge_id)
    }
    
    /// Get provider instance
    pub fn get_provider(&self, bridge_id: &str) -> Option<Arc<dyn AgentBridgeProvider>> {
        self.providers.get(bridge_id).cloned()
    }
    
    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<Arc<dyn AgentBridgePlugin>> {
        self.plugins.values().cloned().collect()
    }
    
    /// List all created providers
    pub fn list_providers(&self) -> Vec<Arc<dyn AgentBridgeProvider>> {
        self.providers.values().cloned().collect()
    }
    
    /// Health check all providers
    pub fn health_check_all(&self) -> HashMap<String, AgentBridgeHealth> {
        self.providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.health_check()))
            .collect()
    }
}

impl Default for AgentBridgePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Update mod.rs to include registry**

```rust
mod types;
mod provider;
mod plugin;
mod registry;

pub use types::*;
pub use provider::AgentBridgeProvider;
pub use plugin::AgentBridgePlugin;
pub use registry::AgentBridgePluginRegistry;
```

- [ ] **Step 3: Verify registry compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/src/bridge/registry.rs sdkwork-agent-client/src/bridge/mod.rs
git commit -m "feat(agent-client): add AgentBridgePluginRegistry"
```

---

### Task 6: Create AgentClient

**Files:**
- Create: `sdkwork-agent-client/src/bridge/client.rs`
- Modify: `sdkwork-agent-client/src/bridge/mod.rs`

- [ ] **Step 1: Create client.rs with AgentClient implementation**

```rust
use std::sync::Arc;
use crate::chat::ChatClient;
use crate::sse::SseChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use super::types::{
    AgentBridgeType, AgentBridgeConfig, AgentBridgeMetadata, AgentBridgeHealth,
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
            Ok(Box::new(SseChatClient::new(server_url)?))
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
```

- [ ] **Step 2: Update mod.rs to include client**

```rust
mod types;
mod provider;
mod plugin;
mod registry;
mod client;

pub use types::*;
pub use provider::AgentBridgeProvider;
pub use plugin::AgentBridgePlugin;
pub use registry::AgentBridgePluginRegistry;
pub use client::{AgentClient, AgentClientMode};
```

- [ ] **Step 3: Verify client compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/src/bridge/client.rs sdkwork-agent-client/src/bridge/mod.rs
git commit -m "feat(agent-client): add AgentClient with Remote/Local/Hybrid modes"
```

---

### Task 7: Create OpenClaw Plugin and Provider

**Files:**
- Create: `sdkwork-agent-client/src/plugins/mod.rs`
- Create: `sdkwork-agent-client/src/plugins/openclaw/mod.rs`
- Create: `sdkwork-agent-client/src/plugins/openclaw/runtime.rs`
- Modify: `sdkwork-agent-client/src/lib.rs`

- [ ] **Step 1: Create plugins module directory**

```bash
mkdir -p sdkwork-agent-client/src/plugins/openclaw
```

- [ ] **Step 2: Create OpenClawRuntime placeholder**

```rust
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};

/// OpenClaw runtime handle
pub struct OpenClawRuntime {
    config: AgentBridgeConfig,
}

impl OpenClawRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
        })
    }
    
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        // TODO: Implement OpenClaw API call
        Err("OpenClaw runtime not implemented".to_string())
    }
    
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String> {
        // TODO: Implement OpenClaw API call
        Err("OpenClaw runtime not implemented".to_string())
    }
    
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        // TODO: Implement OpenClaw API call
        Err("OpenClaw runtime not implemented".to_string())
    }
    
    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        // TODO: Implement OpenClaw API call
        Err("OpenClaw runtime not implemented".to_string())
    }
    
    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Unknown,
            message: Some("OpenClaw runtime not implemented".to_string()),
            last_check: chrono::Utc::now(),
        }
    }
    
    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 3: Create OpenClawProvider**

```rust
mod runtime;

use std::sync::{Arc, Mutex};
use crate::chat::ChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{
    AgentBridgeProvider, AgentBridgeType, AgentBridgeHealth, AgentBridgeStatus,
    AgentBridgeMetadata, AgentBridgeConfig,
};
use runtime::OpenClawRuntime;

/// OpenClaw Provider implementation
pub struct OpenClawProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<OpenClawRuntime>>,
}

impl OpenClawProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            runtime: Mutex::new(None),
        })
    }
}

impl ChatClient for OpenClawProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }
    
    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }
    
    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }
    
    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }
    
    fn health(&self) -> Result<bool, String> {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => Ok(rt.health_check().status == AgentBridgeStatus::Healthy),
            None => Ok(false),
        }
    }
}

impl AgentBridgeProvider for OpenClawProvider {
    fn bridge_id(&self) -> &str {
        &self.config.bridge_id
    }
    
    fn bridge_type(&self) -> AgentBridgeType {
        AgentBridgeType::OpenClaw
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn capabilities(&self) -> Vec<String> {
        vec![
            "chat".to_string(),
            "tool_call".to_string(),
            "memory".to_string(),
            "planning".to_string(),
        ]
    }
    
    fn initialize(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(OpenClawRuntime::new(&self.config)?);
        }
        Ok(())
    }
    
    fn shutdown(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }
    
    fn health_check(&self) -> AgentBridgeHealth {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => rt.health_check(),
            None => AgentBridgeHealth {
                status: AgentBridgeStatus::Unknown,
                message: Some("Runtime not initialized".to_string()),
                last_check: chrono::Utc::now(),
            },
        }
    }
    
    fn metadata(&self) -> AgentBridgeMetadata {
        AgentBridgeMetadata {
            bridge_id: self.config.bridge_id.clone(),
            bridge_type: AgentBridgeType::OpenClaw,
            version: "1.0.0".to_string(),
            description: "OpenClaw agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities(),
            config_schema: None,
        }
    }
}
```

- [ ] **Step 4: Create OpenClawPlugin**

```rust
use std::sync::Arc;
use crate::bridge::{AgentBridgePlugin, AgentBridgeType, AgentBridgeConfig, AgentBridgeProvider};
use super::OpenClawProvider;

/// OpenClaw plugin implementation
pub struct OpenClawPlugin;

impl OpenClawPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl AgentBridgePlugin for OpenClawPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.openclaw"
    }
    
    fn name(&self) -> &str {
        "OpenClaw Bridge Plugin"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::OpenClaw]
    }
    
    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::OpenClaw => {
                Ok(Arc::new(OpenClawProvider::new(config)?))
            }
            _ => Err(format!("Unsupported bridge type: {:?}", bridge_type)),
        }
    }
    
    fn config_schema(&self) -> Option<String> {
        None
    }
    
    fn validate_config(&self, _config: &AgentBridgeConfig) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 5: Create plugins/mod.rs**

```rust
pub mod openclaw;

pub use openclaw::{OpenClawPlugin, OpenClawProvider};
```

- [ ] **Step 6: Update lib.rs to include plugins**

Add to `sdkwork-agent-client/src/lib.rs`:
```rust
pub mod bridge;
pub mod plugins;
```

- [ ] **Step 7: Verify OpenClaw plugin compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add sdkwork-agent-client/src/plugins/
git add sdkwork-agent-client/src/lib.rs
git commit -m "feat(agent-client): add OpenClaw plugin and provider"
```

---

### Task 8: Create ZeroClaw Plugin and Provider

**Files:**
- Create: `sdkwork-agent-client/src/plugins/zeroclaw/mod.rs`
- Create: `sdkwork-agent-client/src/plugins/zeroclaw/runtime.rs`
- Modify: `sdkwork-agent-client/src/plugins/mod.rs`

- [ ] **Step 1: Create zeroclaw module directory**

```bash
mkdir -p sdkwork-agent-client/src/plugins/zeroclaw
```

- [ ] **Step 2: Create ZeroClawRuntime placeholder**

```rust
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};

/// ZeroClaw runtime handle (uses PyO3)
pub struct ZeroClawRuntime {
    config: AgentBridgeConfig,
}

impl ZeroClawRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
        })
    }
    
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        // TODO: Implement ZeroClaw Python API call via PyO3
        Err("ZeroClaw runtime not implemented".to_string())
    }
    
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String> {
        // TODO: Implement ZeroClaw Python API call via PyO3
        Err("ZeroClaw runtime not implemented".to_string())
    }
    
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        // TODO: Implement ZeroClaw Python API call via PyO3
        Err("ZeroClaw runtime not implemented".to_string())
    }
    
    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        // TODO: Implement ZeroClaw Python API call via PyO3
        Err("ZeroClaw runtime not implemented".to_string())
    }
    
    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Unknown,
            message: Some("ZeroClaw runtime not implemented".to_string()),
            last_check: chrono::Utc::now(),
        }
    }
    
    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 3: Create ZeroClawProvider**

```rust
mod runtime;

use std::sync::{Arc, Mutex};
use crate::chat::ChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{
    AgentBridgeProvider, AgentBridgeType, AgentBridgeHealth, AgentBridgeStatus,
    AgentBridgeMetadata, AgentBridgeConfig,
};
use runtime::ZeroClawRuntime;

/// ZeroClaw Provider implementation
pub struct ZeroClawProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<ZeroClawRuntime>>,
}

impl ZeroClawProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            runtime: Mutex::new(None),
        })
    }
}

impl ChatClient for ZeroClawProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }
    
    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }
    
    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }
    
    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }
    
    fn health(&self) -> Result<bool, String> {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => Ok(rt.health_check().status == AgentBridgeStatus::Healthy),
            None => Ok(false),
        }
    }
}

impl AgentBridgeProvider for ZeroClawProvider {
    fn bridge_id(&self) -> &str {
        &self.config.bridge_id
    }
    
    fn bridge_type(&self) -> AgentBridgeType {
        AgentBridgeType::ZeroClaw
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn capabilities(&self) -> Vec<String> {
        vec![
            "chat".to_string(),
            "tool_call".to_string(),
        ]
    }
    
    fn initialize(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(ZeroClawRuntime::new(&self.config)?);
        }
        Ok(())
    }
    
    fn shutdown(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }
    
    fn health_check(&self) -> AgentBridgeHealth {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => rt.health_check(),
            None => AgentBridgeHealth {
                status: AgentBridgeStatus::Unknown,
                message: Some("Runtime not initialized".to_string()),
                last_check: chrono::Utc::now(),
            },
        }
    }
    
    fn metadata(&self) -> AgentBridgeMetadata {
        AgentBridgeMetadata {
            bridge_id: self.config.bridge_id.clone(),
            bridge_type: AgentBridgeType::ZeroClaw,
            version: "1.0.0".to_string(),
            description: "ZeroClaw agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities(),
            config_schema: None,
        }
    }
}
```

- [ ] **Step 4: Create ZeroClawPlugin**

```rust
use std::sync::Arc;
use crate::bridge::{AgentBridgePlugin, AgentBridgeType, AgentBridgeConfig, AgentBridgeProvider};
use super::ZeroClawProvider;

/// ZeroClaw plugin implementation
pub struct ZeroClawPlugin;

impl ZeroClawPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl AgentBridgePlugin for ZeroClawPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.zeroclaw"
    }
    
    fn name(&self) -> &str {
        "ZeroClaw Bridge Plugin"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::ZeroClaw]
    }
    
    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::ZeroClaw => {
                Ok(Arc::new(ZeroClawProvider::new(config)?))
            }
            _ => Err(format!("Unsupported bridge type: {:?}", bridge_type)),
        }
    }
    
    fn config_schema(&self) -> Option<String> {
        None
    }
    
    fn validate_config(&self, _config: &AgentBridgeConfig) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 5: Update plugins/mod.rs**

```rust
pub mod openclaw;
pub mod zeroclaw;

pub use openclaw::{OpenClawPlugin, OpenClawProvider};
pub use zeroclaw::{ZeroClawPlugin, ZeroClawProvider};
```

- [ ] **Step 6: Verify ZeroClaw plugin compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 7: Commit**

```bash
git add sdkwork-agent-client/src/plugins/zeroclaw/
git add sdkwork-agent-client/src/plugins/mod.rs
git commit -m "feat(agent-client): add ZeroClaw plugin and provider"
```

---

### Task 9: Create Hermes Plugin and Provider

**Files:**
- Create: `sdkwork-agent-client/src/plugins/hermes/mod.rs`
- Create: `sdkwork-agent-client/src/plugins/hermes/runtime.rs`
- Modify: `sdkwork-agent-client/src/plugins/mod.rs`

- [ ] **Step 1: Create hermes module directory**

```bash
mkdir -p sdkwork-agent-client/src/plugins/hermes
```

- [ ] **Step 2: Create HermesRuntime placeholder**

```rust
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{AgentBridgeConfig, AgentBridgeHealth, AgentBridgeStatus};

/// Hermes runtime handle
pub struct HermesRuntime {
    config: AgentBridgeConfig,
}

impl HermesRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
        })
    }
    
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        // TODO: Implement Hermes Agent Rust API call
        Err("Hermes runtime not implemented".to_string())
    }
    
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String> {
        // TODO: Implement Hermes Agent Rust API call
        Err("Hermes runtime not implemented".to_string())
    }
    
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        // TODO: Implement Hermes Agent Rust API call
        Err("Hermes runtime not implemented".to_string())
    }
    
    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        // TODO: Implement Hermes Agent Rust API call
        Err("Hermes runtime not implemented".to_string())
    }
    
    pub fn health_check(&self) -> AgentBridgeHealth {
        AgentBridgeHealth {
            status: AgentBridgeStatus::Unknown,
            message: Some("Hermes runtime not implemented".to_string()),
            last_check: chrono::Utc::now(),
        }
    }
    
    pub fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 3: Create HermesProvider**

```rust
mod runtime;

use std::sync::{Arc, Mutex};
use crate::chat::ChatClient;
use crate::types::{ChatRequest, ChatResponse, ChatMessage, SessionConfig, SessionInfo};
use crate::bridge::{
    AgentBridgeProvider, AgentBridgeType, AgentBridgeHealth, AgentBridgeStatus,
    AgentBridgeMetadata, AgentBridgeConfig,
};
use runtime::HermesRuntime;

/// Hermes Provider implementation
pub struct HermesProvider {
    config: AgentBridgeConfig,
    runtime: Mutex<Option<HermesRuntime>>,
}

impl HermesProvider {
    pub fn new(config: AgentBridgeConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            runtime: Mutex::new(None),
        })
    }
}

impl ChatClient for HermesProvider {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }
    
    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }
    
    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }
    
    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }
    
    fn health(&self) -> Result<bool, String> {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => Ok(rt.health_check().status == AgentBridgeStatus::Healthy),
            None => Ok(false),
        }
    }
}

impl AgentBridgeProvider for HermesProvider {
    fn bridge_id(&self) -> &str {
        &self.config.bridge_id
    }
    
    fn bridge_type(&self) -> AgentBridgeType {
        AgentBridgeType::Hermes
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn capabilities(&self) -> Vec<String> {
        vec![
            "chat".to_string(),
            "tool_call".to_string(),
            "memory".to_string(),
        ]
    }
    
    fn initialize(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(HermesRuntime::new(&self.config)?);
        }
        Ok(())
    }
    
    fn shutdown(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }
    
    fn health_check(&self) -> AgentBridgeHealth {
        let runtime = self.runtime.lock().unwrap();
        match runtime.as_ref() {
            Some(rt) => rt.health_check(),
            None => AgentBridgeHealth {
                status: AgentBridgeStatus::Unknown,
                message: Some("Runtime not initialized".to_string()),
                last_check: chrono::Utc::now(),
            },
        }
    }
    
    fn metadata(&self) -> AgentBridgeMetadata {
        AgentBridgeMetadata {
            bridge_id: self.config.bridge_id.clone(),
            bridge_type: AgentBridgeType::Hermes,
            version: "1.0.0".to_string(),
            description: "Hermes agent bridge provider".to_string(),
            author: "SDKWork".to_string(),
            capabilities: self.capabilities(),
            config_schema: None,
        }
    }
}
```

- [ ] **Step 4: Create HermesPlugin**

```rust
use std::sync::Arc;
use crate::bridge::{AgentBridgePlugin, AgentBridgeType, AgentBridgeConfig, AgentBridgeProvider};
use super::HermesProvider;

/// Hermes plugin implementation
pub struct HermesPlugin;

impl HermesPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl AgentBridgePlugin for HermesPlugin {
    fn plugin_id(&self) -> &str {
        "builtin.hermes"
    }
    
    fn name(&self) -> &str {
        "Hermes Bridge Plugin"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn supported_bridge_types(&self) -> Vec<AgentBridgeType> {
        vec![AgentBridgeType::Hermes]
    }
    
    fn create_provider(
        &self,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<Arc<dyn AgentBridgeProvider>, String> {
        match bridge_type {
            AgentBridgeType::Hermes => {
                Ok(Arc::new(HermesProvider::new(config)?))
            }
            _ => Err(format!("Unsupported bridge type: {:?}", bridge_type)),
        }
    }
    
    fn config_schema(&self) -> Option<String> {
        None
    }
    
    fn validate_config(&self, _config: &AgentBridgeConfig) -> Result<(), String> {
        Ok(())
    }
}
```

- [ ] **Step 5: Update plugins/mod.rs**

```rust
pub mod openclaw;
pub mod zeroclaw;
pub mod hermes;

pub use openclaw::{OpenClawPlugin, OpenClawProvider};
pub use zeroclaw::{ZeroClawPlugin, ZeroClawProvider};
pub use hermes::{HermesPlugin, HermesProvider};
```

- [ ] **Step 6: Verify Hermes plugin compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 7: Commit**

```bash
git add sdkwork-agent-client/src/plugins/hermes/
git add sdkwork-agent-client/src/plugins/mod.rs
git commit -m "feat(agent-client): add Hermes plugin and provider"
```

---

### Task 10: Create BuiltinPlugins Helper

**Files:**
- Create: `sdkwork-agent-client/src/plugins/builtin.rs`
- Modify: `sdkwork-agent-client/src/plugins/mod.rs`

- [ ] **Step 1: Create builtin.rs with BuiltinPlugins**

```rust
use std::sync::Arc;
use crate::bridge::{AgentBridgePlugin, AgentBridgePluginRegistry};
use super::{OpenClawPlugin, ZeroClawPlugin, HermesPlugin};

/// Built-in plugins package
pub struct BuiltinPlugins {
    plugins: Vec<Arc<dyn AgentBridgePlugin>>,
}

impl BuiltinPlugins {
    /// Create all built-in plugins
    pub fn create_all() -> Self {
        Self {
            plugins: vec![
                Arc::new(OpenClawPlugin::new()),
                Arc::new(ZeroClawPlugin::new()),
                Arc::new(HermesPlugin::new()),
            ],
        }
    }
    
    /// Get all built-in plugins
    pub fn plugins(&self) -> &[Arc<dyn AgentBridgePlugin>] {
        &self.plugins
    }
    
    /// Register all built-in plugins to registry
    pub fn register_all(&self, registry: &mut AgentBridgePluginRegistry) -> Result<(), String> {
        for plugin in &self.plugins {
            registry.register_plugin(plugin.clone())?;
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Update plugins/mod.rs**

```rust
pub mod openclaw;
pub mod zeroclaw;
pub mod hermes;
pub mod builtin;

pub use openclaw::{OpenClawPlugin, OpenClawProvider};
pub use zeroclaw::{ZeroClawPlugin, ZeroClawProvider};
pub use hermes::{HermesPlugin, HermesProvider};
pub use builtin::BuiltinPlugins;
```

- [ ] **Step 3: Verify BuiltinPlugins compiles**

Run: `cargo check --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add sdkwork-agent-client/src/plugins/builtin.rs sdkwork-agent-client/src/plugins/mod.rs
git commit -m "feat(agent-client): add BuiltinPlugins helper"
```

---

### Task 11: Add Tests for Bridge System

**Files:**
- Create: `sdkwork-agent-client/tests/bridge_tests.rs`

- [ ] **Step 1: Create bridge_tests.rs**

```rust
use std::sync::Arc;
use sdkwork_agent_client::bridge::{
    AgentBridgeType, AgentBridgeStatus, AgentBridgeHealth, AgentBridgeMetadata,
    AgentBridgeConfig, AgentBridgePluginRegistry, AgentClient, AgentClientMode,
    FallbackStrategy,
};
use sdkwork_agent_client::plugins::BuiltinPlugins;
use sdkwork_agent_client::chat::ChatClient;

#[test]
fn test_bridge_type_display() {
    assert_eq!(AgentBridgeType::OpenClaw.to_string(), "openclaw");
    assert_eq!(AgentBridgeType::ZeroClaw.to_string(), "zeroclaw");
    assert_eq!(AgentBridgeType::Hermes.to_string(), "hermes");
    assert_eq!(AgentBridgeType::Custom("test".to_string()).to_string(), "test");
}

#[test]
fn test_bridge_status_display() {
    assert_eq!(AgentBridgeStatus::Healthy.to_string(), "healthy");
    assert_eq!(AgentBridgeStatus::Degraded.to_string(), "degraded");
    assert_eq!(AgentBridgeStatus::Unhealthy.to_string(), "unhealthy");
    assert_eq!(AgentBridgeStatus::Unknown.to_string(), "unknown");
}

#[test]
fn test_bridge_health_healthy() {
    let health = AgentBridgeHealth::healthy();
    assert_eq!(health.status, AgentBridgeStatus::Healthy);
    assert!(health.message.is_none());
}

#[test]
fn test_bridge_health_unhealthy() {
    let health = AgentBridgeHealth::unhealthy("test error");
    assert_eq!(health.status, AgentBridgeStatus::Unhealthy);
    assert_eq!(health.message, Some("test error".to_string()));
}

#[test]
fn test_bridge_config_new() {
    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw);
    assert_eq!(config.bridge_id, "test");
    assert_eq!(config.bridge_type, AgentBridgeType::OpenClaw);
    assert!(config.settings.is_empty());
    assert!(config.secrets.is_empty());
}

#[test]
fn test_bridge_config_with_settings() {
    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw)
        .with_setting("key1", "value1")
        .with_secret("secret1", "value1");
    
    assert_eq!(config.settings.get("key1"), Some(&"value1".to_string()));
    assert_eq!(config.secrets.get("secret1"), Some(&"value1".to_string()));
}

#[test]
fn test_plugin_registry_new() {
    let registry = AgentBridgePluginRegistry::new();
    assert!(registry.list_plugins().is_empty());
    assert!(registry.list_providers().is_empty());
}

#[test]
fn test_builtin_plugins_create_all() {
    let plugins = BuiltinPlugins::create_all();
    assert_eq!(plugins.plugins().len(), 3);
}

#[test]
fn test_builtin_plugins_register_all() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();
    
    plugins.register_all(&mut registry).unwrap();
    assert_eq!(registry.list_plugins().len(), 3);
}

#[test]
fn test_plugin_registry_create_provider() {
    let plugins = BuiltinPlugins::create_all();
    let mut registry = AgentBridgePluginRegistry::new();
    
    plugins.register_all(&mut registry).unwrap();
    
    let config = AgentBridgeConfig::new("test-openclaw", AgentBridgeType::OpenClaw);
    let bridge_id = registry.create_provider("builtin.openclaw", AgentBridgeType::OpenClaw, config).unwrap();
    
    assert_eq!(bridge_id, "test-openclaw");
    assert!(registry.get_provider(&bridge_id).is_some());
}

#[test]
fn test_plugin_registry_create_provider_not_found() {
    let mut registry = AgentBridgePluginRegistry::new();
    
    let config = AgentBridgeConfig::new("test", AgentBridgeType::OpenClaw);
    let result = registry.create_provider("nonexistent", AgentBridgeType::OpenClaw, config);
    
    assert!(result.is_err());
}

#[test]
fn test_fallback_strategy_default() {
    let strategy = FallbackStrategy::default();
    match strategy {
        FallbackStrategy::Immediate => {},
        _ => panic!("Expected Immediate"),
    }
}

#[test]
fn test_agent_client_mode_local_not_found() {
    let registry = Arc::new(AgentBridgePluginRegistry::new());
    let mode = AgentClientMode::Local {
        bridge_id: "nonexistent".to_string(),
    };
    
    let result = AgentClient::new(mode, registry);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path sdkwork-agent-client/Cargo.toml --test bridge_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add sdkwork-agent-client/tests/bridge_tests.rs
git commit -m "test(agent-client): add bridge system tests"
```

---

### Task 12: Final Verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run all tests**

Run: `cargo test --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --manifest-path sdkwork-agent-client/Cargo.toml`
Expected: No warnings

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --manifest-path sdkwork-agent-client/Cargo.toml -- --check`
Expected: No formatting issues

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(agent-client): complete multi-mode agent system (Phase 1 & 2)"
```

---

## Summary

This plan implements **Phase 1 (Core Infrastructure)** and **Phase 2 (Built-in Plugins)** of the multi-mode agent system:

1. **Bridge Types** - AgentBridgeType, AgentBridgeHealth, AgentBridgeMetadata, AgentBridgeConfig, etc.
2. **AgentBridgeProvider Trait** - Unified interface for local agent frameworks
3. **AgentBridgePlugin Trait** - Plugin interface for runtime dynamic loading
4. **AgentBridgePluginRegistry** - Manages plugins and providers
5. **AgentClient** - Supports Remote, Local, and Hybrid modes
6. **OpenClaw Plugin** - Bridge provider for OpenClaw agent framework
7. **ZeroClaw Plugin** - Bridge provider for ZeroClaw agent framework
8. **Hermes Plugin** - Bridge provider for Hermes Agent framework
9. **BuiltinPlugins** - Helper to register all built-in plugins

**Next Steps (Future Plans):**
- Phase 3: Server Mode (HTTP/REST + SSE, gRPC, WebSocket)
- Phase 4: Kernel Integration (BridgeProviderAdapter, KernelIntegrator)
- Phase 5: Packaging and Deployment (Optional)

