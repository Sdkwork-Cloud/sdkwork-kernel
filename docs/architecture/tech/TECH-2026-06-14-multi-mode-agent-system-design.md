> Migrated from `docs/superpowers/specs/2026-06-14-multi-mode-agent-system-design.md` on 2026-06-24.
> Owner: SDKWork maintainers

## Overview

This document describes the design of a multi-mode agent system for SDKWork that supports client/server modes, local/remote agent capabilities, and multiple agent framework implementations.

## Prerequisites and Existing Types

### Existing Types (from sdkwork-agent-kernel and sdkwork-agent-client)

The following types are **existing** and should be imported from their respective crates:

- `ChatClient`, `ChatRequest`, `ChatResponse`, `ChatMessage`, `SessionConfig`, `SessionInfo`, `ChatStatus`, `TokenUsage` — from `sdkwork-agent-client`
- `SseChatClient`, `WebSocketChatClient` — from `sdkwork-agent-client` (for remote client implementations)
- `AgentRuntime`, `RuntimeBuilder`, `ProviderManifest`, `ProviderRegistration`, `ModelProvider`, `ToolProvider`, `PolicyProvider`, `ContextProvider`, `MemoryProvider`, `KnowledgeProvider`, `PlanningProvider`, `HostProvider`, `ProtocolAdapter`, `McpProvider`, `AgentSkillProvider`, `AgentCollaborationProvider`, `TelemetryProvider`, `AgentInstaller`, `AgentConfigurationProvider` — from `sdkwork-agent-kernel`
- `ModelRequest`, `ModelResponse`, `ModelUsage`, `ToolCall`, `ToolResult`, `ToolDescriptor` — from `sdkwork-agent-kernel`

**Required Changes to Existing Types**:

1. `SseChatClient::new` must be extended to accept optional `AgentAuth` parameter
2. `WebSocketChatClient::new` must be extended to accept optional `AgentAuth` parameter

**Note**: `ChatRequest` already derives `Clone` in the existing codebase, so no change is needed for hybrid mode retry logic.

### External Dependencies

- `chrono` — for timestamp handling
- `libloading` — for dynamic plugin loading
- `axum` — for HTTP/REST server
- `tonic` — for gRPC server
- `tokio-tungstenite` — for WebSocket server
- `serde`, `serde_json` — for serialization
- `uuid` — for generating unique IDs

### New Types to Build

The following types are **new** and will be built as part of this design:

- `AgentBridgeProvider`, `AgentBridgeType`, `AgentBridgeHealth`, `AgentBridgeStatus`, `AgentBridgeMetadata`
- `AgentBridgePlugin`, `AgentBridgeConfig`, `AgentBridgePluginRegistry`
- `AgentClientMode`, `AgentProtocol`, `AgentAuth`, `AgentAuthType`, `FallbackStrategy`, `AgentClient`
- `AgentServerConfig`, `TlsConfig`, `ServerAuthConfig`, `ServerAuthType`, `AgentBridgePluginConfig`, `AgentServer`, `ServerHealth`, `ServerHealthStatus`, `ServerStatus`, `ServerState`, `ConnectionStats`
- `BuiltinPlugins`, `OpenClawPlugin`, `OpenClawProvider`, `ZeroClawPlugin`, `ZeroClawProvider`, `HermesPlugin`, `HermesProvider`
- `AppPackageConfig`, `BuiltinPluginConfig`, `AppPackager`, `PackageManifest`, `PluginManifest`, `ValidationReport`, `AppInstaller`, `InstallReport`, `UninstallReport`, `UpgradeReport`
- `BridgeProviderAdapter`, `KernelIntegrator`

### Placeholder Types (to be defined during implementation)

The following types are **placeholders** and will be defined during implementation. Each has a minimum interface contract:

#### Runtime Types

```rust
/// OpenClaw runtime handle
pub struct OpenClawRuntime { /* internal state */ }
impl OpenClawRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String>;
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String>;
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String>;
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String>;
    pub fn close_session(&self, session_id: &str) -> Result<(), String>;
    pub fn health_check(&self) -> AgentBridgeHealth;
    pub fn shutdown(&mut self) -> Result<(), String>;
}

/// ZeroClaw runtime handle (uses PyO3)
pub struct ZeroClawRuntime { /* Python interpreter state */ }
impl ZeroClawRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String>;
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String>;
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String>;
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String>;
    pub fn close_session(&self, session_id: &str) -> Result<(), String>;
    pub fn health_check(&self) -> AgentBridgeHealth;
    pub fn shutdown(&mut self) -> Result<(), String>;
}

/// Hermes runtime handle
pub struct HermesRuntime { /* internal state */ }
impl HermesRuntime {
    pub fn new(config: &AgentBridgeConfig) -> Result<Self, String>;
    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String>;
    pub fn get_messages(&self, session_id: &str, limit: Option<u32>) -> Result<Vec<ChatMessage>, String>;
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String>;
    pub fn close_session(&self, session_id: &str) -> Result<(), String>;
    pub fn health_check(&self) -> AgentBridgeHealth;
    pub fn shutdown(&mut self) -> Result<(), String>;
}
```

#### Server Types

- `HttpServer` — HTTP/REST server implementation using `axum`. Contains router, middleware, and connection pool.
- `GrpcServer` — gRPC server implementation using `tonic`. Contains service definitions and connection pool. Deferred to Phase 3.
- `WebSocketServer` — WebSocket server implementation using `tokio-tungstenite`. Contains connection handler and message routing.
- `OAuth2Config` — OAuth2 configuration struct with client_id, client_secret, token_url, and scopes.
- `HttpRouter` — HTTP router type (alias for `axum::Router`).
- `GrpcService` — gRPC service type (alias for `tonic::transport::Server`).
- `WebSocketHandler` — WebSocket connection handler type.

## Requirements

### Functional Requirements

1. **Client Mode**:
   - Call remote SDKWork Agent Server services
   - Call local agent capabilities through bridge providers
   - Support hybrid mode with fallback strategies

2. **Server Mode**:
   - Provide standard agent service API
   - Support multiple protocols: HTTP/REST + SSE, gRPC, WebSocket
   - Support TLS and authentication

3. **Local Agent Capabilities**:
   - Create local bridge layer for each agent framework
   - Unified interface using Rust implementation
   - Based on Provider SPI architecture
   - Reuse existing ChatClient trait

4. **Built-in Integration**:
   - Built-in three agent frameworks: OpenClaw, ZeroClaw, Hermes Agent
   - As runtime dynamic plugin modules
   - SDKWork application packages include these plugins

### Non-Functional Requirements

1. **Extensibility**: Support adding new agent frameworks through plugins
2. **Performance**: Minimal overhead for local agent calls
3. **Reliability**: Health checks and monitoring for all components
4. **Security**: Support TLS, authentication, and authorization

## Architecture

### Overall Architecture

```text
SDKWork Application
  ├── Client Mode
  │   ├── Remote Call → SDKWork Agent Server (HTTP/REST + SSE, gRPC, WebSocket)
  │   └── Local Call → Local Agent Bridge Layer (Based on Provider SPI)
  │       ├── OpenClaw Bridge Provider
  │       ├── ZeroClaw Bridge Provider
  │       └── Hermes Agent Bridge Provider
  └── Server Mode
      └── SDKWork Agent Server
          ├── Standard Agent API
          └── Local Agent Bridge Layer
```

### Core Components

1. **AgentBridgeProvider**: Unified bridge provider trait, inherits from ChatClient
2. **AgentBridgePluginRegistry**: Bridge plugin registry, manages all local agent bridges
3. **AgentClientMode**: Client mode enum (Remote/Local/Hybrid)
4. **AgentBridgePlugin**: Plugin interface, supports runtime dynamic loading

## Detailed Design

### 1. Bridge Provider Interface

**Design Decision**: `AgentBridgeProvider` inherits from `ChatClient`. The `ChatClient` implementation on each bridge provider IS the actual bridge logic — there is no separate bridge-specific method. When `AgentClient` delegates to a local provider, it calls the provider's `ChatClient::send_message` method, which internally bridges to the underlying agent framework (OpenClaw, ZeroClaw, Hermes).

**Sync vs Async Design**: `ChatClient` is a synchronous trait. All methods return `Result<T, String>` directly. This simplifies implementation and avoids async runtime complexity. For server mode, the HTTP/gRPC/WebSocket servers will wrap synchronous calls in `tokio::task::spawn_blocking` or equivalent.

**Mutability Design**: `initialize()` and `shutdown()` use interior mutability via `std::sync::Mutex` or `std::sync::RwLock` for internal state. The trait methods take `&self` instead of `&mut self` to work with `Arc<dyn AgentBridgeProvider>`.

```rust
/// Unified Agent Bridge Provider trait
/// Inherits from ChatClient — the ChatClient implementation IS the bridge logic
pub trait AgentBridgeProvider: ChatClient + Send + Sync {
    /// Return bridge provider identifier
    fn bridge_id(&self) -> &str;
    
    /// Return bridge provider type (openclaw, zeroclaw, hermes, etc.)
    fn bridge_type(&self) -> AgentBridgeType;
    
    /// Return bridge provider version
    fn version(&self) -> &str;
    
    /// Return supported capabilities
    fn capabilities(&self) -> Vec<String>;
    
    /// Check if specified capability is supported
    fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities().contains(&capability.to_string())
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

/// Agent bridge type enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentBridgeType {
    OpenClaw,
    ZeroClaw,
    Hermes,
    Custom(String),
}

/// Agent bridge health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBridgeHealth {
    pub status: AgentBridgeStatus,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Agent bridge status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentBridgeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for AgentBridgeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
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
```

### 2. Plugin System

**Loading Mechanism**: Plugins are loaded using Rust's `libloading` crate for dynamic library (.so/.dll/.dylib) loading. Each plugin library must export a `create_plugin` function that returns a `Box<dyn AgentBridgePlugin>`. For built-in plugins (OpenClaw, ZeroClaw, Hermes), they are statically linked and registered at compile time.

**Concurrency Model**: 
- Bridge providers use `Mutex<Option<Runtime>>` for interior mutability
- Server mode uses `tokio::task::spawn_blocking` to wrap synchronous `ChatClient` calls
- Multiple concurrent requests to the same bridge provider will serialize on the `Mutex`
- `initialize()` and `send_message()` will contend on the same `Mutex` — initialize must complete before send_message succeeds
- For high-concurrency scenarios, consider using `RwLock` instead of `Mutex` to allow concurrent reads (health checks) while serializing writes (initialization, message sending)

```rust
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

/// Agent bridge configuration
#[derive(Debug, Clone)]
pub struct AgentBridgeConfig {
    pub bridge_id: String,
    pub bridge_type: AgentBridgeType,
    pub settings: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

/// Plugin registry
pub struct AgentBridgePluginRegistry {
    plugins: HashMap<String, Arc<dyn AgentBridgePlugin>>,
    providers: HashMap<String, Arc<dyn AgentBridgeProvider>>,
}

impl AgentBridgePluginRegistry {
    /// Register plugin
    pub fn register_plugin(&mut self, plugin: Arc<dyn AgentBridgePlugin>) -> Result<(), String>;
    
    /// Unregister plugin
    pub fn unregister_plugin(&mut self, plugin_id: &str) -> Result<(), String>;
    
    /// Create provider instance
    pub fn create_provider(
        &mut self,
        plugin_id: &str,
        bridge_type: AgentBridgeType,
        config: AgentBridgeConfig,
    ) -> Result<String, String>;
    
    /// Get provider instance
    pub fn get_provider(&self, bridge_id: &str) -> Option<Arc<dyn AgentBridgeProvider>>;
    
    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<Arc<dyn AgentBridgePlugin>>;
    
    /// List all created providers
    pub fn list_providers(&self) -> Vec<Arc<dyn AgentBridgeProvider>>;
    
    /// Health check all providers
    pub fn health_check_all(&self) -> HashMap<String, AgentBridgeHealth>;
}
```

### 3. Client Mode

```rust
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

/// Agent protocol enum
/// Note: Grpc variant is defined for future use but will return error in Phase 1/2
#[derive(Debug, Clone)]
pub enum AgentProtocol {
    HttpRestSse,
    Grpc,       // Deferred to Phase 3 - will return error if used
    WebSocket,
}

/// Agent authentication information
#[derive(Debug, Clone)]
pub struct AgentAuth {
    pub auth_type: AgentAuthType,
    pub credentials: HashMap<String, String>,
}

/// Agent authentication type
#[derive(Debug, Clone)]
pub enum AgentAuthType {
    ApiKey,
    BearerToken,
    BasicAuth,
    OAuth2,
}

/// Fallback strategy
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// Local fails, immediately fallback to remote (default)
    Immediate,
    /// Local fails, retry N additional times then fallback to remote
    /// Uses exponential backoff: 100ms, 200ms, 400ms, etc.
    /// max_retries = 3 means: initial attempt + 3 retries = 4 total attempts
    RetryThenFallback { max_retries: u32 },
    /// Only use local, no fallback
    LocalOnly,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self::Immediate
    }
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
    /// Resolves bridge_id to provider at construction time
    pub fn new(
        mode: AgentClientMode,
        plugin_registry: Arc<AgentBridgePluginRegistry>,
    ) -> Result<Self, String> {
        let (remote_client, local_provider) = match &mode {
            AgentClientMode::Remote { server_url, protocol, auth } => {
                // Create remote client based on protocol
                let remote = create_remote_client(server_url, protocol, auth)?;
                (Some(remote), None)
            }
            AgentClientMode::Local { bridge_id } => {
                // Resolve bridge_id to provider at construction time
                let provider = plugin_registry.get_provider(bridge_id)
                    .ok_or_else(|| format!("Bridge provider not found: {}", bridge_id))?;
                (None, Some(provider))
            }
            AgentClientMode::Hybrid { bridge_id, server_url, protocol, auth, .. } => {
                // Create both remote and local clients
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
        // Initialize local provider if present
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
        // Recreate clients for new mode
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
/// Note: SseChatClient and WebSocketChatClient constructors need to be extended
/// to accept optional auth parameters. This is a required change to existing types.
/// 
/// Decision: gRPC client is deferred to Phase 3. Initial implementation focuses on HTTP/REST + SSE.
fn create_remote_client(
    server_url: &str,
    protocol: &AgentProtocol,
    auth: &Option<AgentAuth>,
) -> Result<Box<dyn ChatClient>, String> {
    match protocol {
        AgentProtocol::HttpRestSse => {
            // TODO: Extend SseChatClient::new to accept optional auth
            // Current: SseChatClient::new(url)
            // Required: SseChatClient::new(url, auth)
            // For now, auth is ignored in initial implementation
            Ok(Box::new(SseChatClient::new(server_url)?))
        }
        AgentProtocol::Grpc => {
            // Deferred to Phase 3
            Err("gRPC client not yet implemented - deferred to Phase 3".to_string())
        }
        AgentProtocol::WebSocket => {
            #[cfg(feature = "websocket")]
            {
                // TODO: Extend WebSocketChatClient::new to accept optional auth
                // Current: WebSocketChatClient::new(url)
                // Required: WebSocketChatClient::new(url, auth)
                // For now, auth is ignored in initial implementation
                Ok(Box::new(WebSocketChatClient::new(server_url)?))
            }
            #[cfg(not(feature = "websocket"))]
            {
                Err("WebSocket feature not enabled".to_string())
            }
        }
    }
}

impl ChatClient for AgentClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        match &self.mode {
            AgentClientMode::Remote { .. } => {
                // Call remote server
                self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .send_message(request)
            }
            AgentClientMode::Local { .. } => {
                // Call local provider (ChatClient implementation IS the bridge logic)
                self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .send_message(request)
            }
            AgentClientMode::Hybrid { fallback_strategy, .. } => {
                // Hybrid mode: prefer local, fallback based on strategy
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
                                // Retry logic with exponential backoff
                                // Initial failure counts as attempt 0
                                // Using synchronous sleep since ChatClient is sync
                                for attempt in 0..*max_retries {
                                    // Exponential backoff: 100ms, 200ms, 400ms, etc.
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
                // For read operations, prefer local
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
                // For session creation, prefer local
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
                // For session close, prefer local
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
                // For health check, check both
                let local_health = self.local_provider.as_ref()
                    .ok_or_else(|| "Local provider not initialized".to_string())?
                    .health();
                let remote_health = self.remote_client.as_ref()
                    .ok_or_else(|| "Remote client not initialized".to_string())?
                    .health();
                
                // Return healthy if either is healthy
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

### 4. Server Mode

**Server API Contract**: The SDKWork Agent Server exposes the following endpoints:

#### HTTP/REST + SSE Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/sessions` | Create a new session |
| GET | `/api/v1/sessions/{session_id}` | Get session info |
| DELETE | `/api/v1/sessions/{session_id}` | Close session |
| POST | `/api/v1/sessions/{session_id}/messages` | Send message (returns SSE stream for streaming responses) |
| GET | `/api/v1/sessions/{session_id}/messages` | Get message history |
| GET | `/api/v1/health` | Health check |
| GET | `/api/v1/plugins` | List loaded plugins |
| GET | `/api/v1/plugins/{plugin_id}/health` | Plugin health check |

#### Message Format

```json
{
  "message_id": "msg.123",
  "session_id": "session.456",
  "content": "Hello, world!",
  "model": "gpt-4",
  "stream": false
}
```

#### SSE Stream Format

```
data: {"type":"message_start","message_id":"msg.123"}
data: {"type":"content_delta","content":"Hello"}
data: {"type":"content_delta","content":" world"}
data: {"type":"message_end","message_id":"msg.123","usage":{"input_tokens":10,"output_tokens":5}}
```

#### gRPC Service Definition

**Note**: The gRPC service definition is provided as a contract artifact. The `.proto` file should be committed in Phase 1 as a contract, but implementation is deferred to Phase 3.

```protobuf
service AgentService {
  rpc CreateSession(CreateSessionRequest) returns (SessionInfo);
  rpc GetSession(GetSessionRequest) returns (SessionInfo);
  rpc CloseSession(CloseSessionRequest) returns (google.protobuf.Empty);
  rpc SendMessage(SendMessageRequest) returns (stream MessageChunk);
  rpc GetMessages(GetMessagesRequest) returns (GetMessagesResponse);
  rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
  rpc ListPlugins(ListPluginsRequest) returns (ListPluginsResponse);
}

// Message types
message CreateSessionRequest {
  string agent_id = 1;
  string model = 2;
  map<string, string> config = 3;
}

message GetSessionRequest {
  string session_id = 1;
}

message CloseSessionRequest {
  string session_id = 1;
}

message SendMessageRequest {
  string session_id = 1;
  string content = 2;
  string model = 3;
  bool stream = 4;
}

message MessageChunk {
  string message_id = 1;
  string session_id = 2;
  string content = 3;
  string status = 4;
  TokenUsage usage = 5;
}

message GetMessagesRequest {
  string session_id = 1;
  int32 limit = 2;
}

message GetMessagesResponse {
  repeated ChatMessage messages = 1;
}

message HealthCheckRequest {}

message HealthCheckResponse {
  bool healthy = 1;
  string message = 2;
}

message ListPluginsRequest {}

message ListPluginsResponse {
  repeated PluginInfo plugins = 1;
}

message ChatMessage {
  string message_id = 1;
  string session_id = 2;
  string content = 3;
  string role = 4;
  string timestamp = 5;
}

message TokenUsage {
  int32 input_tokens = 1;
  int32 output_tokens = 2;
  int32 total_tokens = 3;
}

message PluginInfo {
  string plugin_id = 1;
  string name = 2;
  string version = 3;
  repeated string capabilities = 4;
}
```

#### WebSocket Protocol

WebSocket connections use JSON messages:

```json
// Client -> Server
{"type": "create_session", "config": {"agent_id": "agent.1", "model": "gpt-4"}}
{"type": "send_message", "session_id": "session.456", "content": "Hello"}

// Server -> Client
{"type": "session_created", "session_id": "session.456"}
{"type": "message_start", "message_id": "msg.123"}
{"type": "content_delta", "content": "Hello"}
{"type": "message_end", "message_id": "msg.123"}
```

```rust
/// Agent server configuration
#[derive(Debug, Clone)]
pub struct AgentServerConfig {
    pub server_id: String,
    pub bind_address: String,
    pub port: u16,
    pub protocols: Vec<AgentProtocol>,
    pub tls: Option<TlsConfig>,
    pub auth: Option<ServerAuthConfig>,
    pub bridge_plugins: Vec<AgentBridgePluginConfig>,
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: Option<String>,
}

/// Server authentication configuration
#[derive(Debug, Clone)]
pub struct ServerAuthConfig {
    pub auth_type: ServerAuthType,
    pub api_keys: Option<Vec<String>>,
    pub jwt_secret: Option<String>,
    pub oauth2_config: Option<OAuth2Config>,
}

/// Server authentication type
/// Initial implementation: ApiKey only
/// Jwt and OAuth2 are defined for future extension
#[derive(Debug, Clone)]
pub enum ServerAuthType {
    None,
    ApiKey,
    Jwt,      // Deferred to future phase
    OAuth2,   // Deferred to future phase
}

/// Bridge plugin configuration
#[derive(Debug, Clone)]
pub struct AgentBridgePluginConfig {
    pub plugin_id: String,
    pub plugin_path: String,
    pub auto_load: bool,
    pub bridge_configs: Vec<AgentBridgeConfig>,
}

/// Agent server
pub struct AgentServer {
    config: AgentServerConfig,
    plugin_registry: Arc<AgentBridgePluginRegistry>,
    http_server: Option<HttpServer>,
    grpc_server: Option<GrpcServer>,
    websocket_server: Option<WebSocketServer>,
}

impl AgentServer {
    /// Create new agent server
    pub fn new(config: AgentServerConfig) -> Result<Self, String>;
    
    /// Load bridge plugins
    pub fn load_plugins(&mut self) -> Result<(), String>;
    
    /// Start server
    pub async fn start(&mut self) -> Result<(), String>;
    
    /// Stop server
    pub async fn stop(&mut self) -> Result<(), String>;
    
    /// Health check
    pub fn health(&self) -> ServerHealth;
    
    /// Get server status
    pub fn status(&self) -> ServerStatus;
    
    /// Get loaded plugins list
    pub fn loaded_plugins(&self) -> Vec<AgentBridgeMetadata>;
    
    /// Get active connections count
    pub fn active_connections(&self) -> u32;
}

/// Server status
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub server_id: String,
    pub state: ServerState,
    pub uptime: Duration,
    pub active_connections: u32,
    pub loaded_plugins: Vec<String>,
    pub protocols: Vec<AgentProtocol>,
}

/// Server state enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Server health status
#[derive(Debug, Clone)]
pub struct ServerHealth {
    pub status: ServerHealthStatus,
    pub uptime: Duration,
    pub plugins_health: HashMap<String, AgentBridgeHealth>,
    pub connections: ConnectionStats,
}

/// Server health status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total: u32,
    pub active: u32,
    pub idle: u32,
    pub failed: u32,
}
```

### 5. Built-in Plugins

**Implementation Note**: ZeroClaw and Hermes implementations follow the same `ChatClient`-based interface as OpenClaw. All three bridge providers implement `ChatClient` methods that internally delegate to their respective agent frameworks. The key difference is in the underlying runtime initialization and configuration:

- **OpenClaw**: Uses OpenClaw's native Rust API for agent operations. Has real external API.
- **ZeroClaw**: Uses ZeroClaw's Python API via PyO3 bindings. Has real external API. Requires `pyo3` crate.
- **Hermes**: Uses Hermes Agent's Rust API for agent operations. Has real external API.

All three share the same `AgentBridgeProvider` trait and can be used interchangeably through the unified `AgentClient` interface.

#### ZeroClaw Implementation Skeleton

```rust
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
        // Call ZeroClaw Python API via PyO3
        rt.send_message(request)
    }
    
    // Other ChatClient methods follow same pattern
}

impl AgentBridgeProvider for ZeroClawProvider {
    fn bridge_id(&self) -> &str { &self.config.bridge_id }
    fn bridge_type(&self) -> AgentBridgeType { AgentBridgeType::ZeroClaw }
    fn version(&self) -> &str { "1.0.0" }
    fn capabilities(&self) -> Vec<String> {
        vec!["chat".to_string(), "tool_call".to_string()]
    }
    
    fn initialize(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            // Initialize Python interpreter and load ZeroClaw
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

#### Hermes Implementation Skeleton

```rust
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
        // Call Hermes Agent Rust API
        rt.send_message(request)
    }
    
    // Other ChatClient methods follow same pattern
}

impl AgentBridgeProvider for HermesProvider {
    fn bridge_id(&self) -> &str { &self.config.bridge_id }
    fn bridge_type(&self) -> AgentBridgeType { AgentBridgeType::Hermes }
    fn version(&self) -> &str { "1.0.0" }
    fn capabilities(&self) -> Vec<String> {
        vec!["chat".to_string(), "tool_call".to_string(), "memory".to_string()]
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

```rust
/// Built-in plugins package
/// Contains bridge implementations for openclaw, zeroclaw, hermes agent
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

/// OpenClaw plugin implementation
pub struct OpenClawPlugin {
    metadata: AgentBridgeMetadata,
}

impl OpenClawPlugin {
    pub fn new() -> Self {
        Self {
            metadata: AgentBridgeMetadata {
                bridge_id: "openclaw".to_string(),
                bridge_type: AgentBridgeType::OpenClaw,
                version: "1.0.0".to_string(),
                description: "OpenClaw agent bridge provider".to_string(),
                author: "SDKWork".to_string(),
                capabilities: vec![
                    "chat".to_string(),
                    "tool_call".to_string(),
                    "memory".to_string(),
                    "planning".to_string(),
                ],
                // Config schema will be defined as a separate JSON file during implementation
                config_schema: None,
            },
        }
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
        &self.metadata.version
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
        self.metadata.config_schema.clone()
    }
    
    fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String> {
        // Validate OpenClaw configuration
        Ok(())
    }
}

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
        // Call OpenClaw runtime to send message
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.send_message(request)
    }
    
    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        // Call OpenClaw runtime to get messages
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.get_messages(session_id, limit)
    }
    
    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        // Call OpenClaw runtime to create session
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.create_session(config)
    }
    
    fn close_session(&self, session_id: &str) -> Result<(), String> {
        // Call OpenClaw runtime to close session
        let runtime = self.runtime.lock().unwrap();
        let rt = runtime.as_ref().ok_or("Runtime not initialized")?;
        rt.close_session(session_id)
    }
    
    fn health(&self) -> Result<bool, String> {
        // Check OpenClaw runtime health
        // Note: ChatClient::health() returns Result<bool, String>
        // We convert AgentBridgeHealth to bool (healthy = true)
        // Diagnostic information is lost in this conversion
        // For detailed health info, use AgentBridgeProvider::health_check()
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
        // Initialize OpenClaw runtime using interior mutability
        // The runtime is stored in a Mutex<Option<OpenClawRuntime>>
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(OpenClawRuntime::new(&self.config)?);
        }
        Ok(())
    }
    
    fn shutdown(&self) -> Result<(), String> {
        // Shutdown OpenClaw runtime using interior mutability
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(mut rt) = runtime.take() {
            rt.shutdown()?;
        }
        Ok(())
    }
    
    fn health_check(&self) -> AgentBridgeHealth {
        // Check OpenClaw runtime health
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

/// ZeroClaw plugin implementation
pub struct ZeroClawPlugin {
    metadata: AgentBridgeMetadata,
}

impl ZeroClawPlugin {
    pub fn new() -> Self {
        Self {
            metadata: AgentBridgeMetadata {
                bridge_id: "zeroclaw".to_string(),
                bridge_type: AgentBridgeType::ZeroClaw,
                version: "1.0.0".to_string(),
                description: "ZeroClaw agent bridge provider".to_string(),
                author: "SDKWork".to_string(),
                capabilities: vec![
                    "chat".to_string(),
                    "tool_call".to_string(),
                ],
                config_schema: None,
            },
        }
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
        &self.metadata.version
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
        self.metadata.config_schema.clone()
    }
    
    fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String> {
        // Validate ZeroClaw configuration
        Ok(())
    }
}

/// Hermes plugin implementation
pub struct HermesPlugin {
    metadata: AgentBridgeMetadata,
}

impl HermesPlugin {
    pub fn new() -> Self {
        Self {
            metadata: AgentBridgeMetadata {
                bridge_id: "hermes".to_string(),
                bridge_type: AgentBridgeType::Hermes,
                version: "1.0.0".to_string(),
                description: "Hermes agent bridge provider".to_string(),
                author: "SDKWork".to_string(),
                capabilities: vec![
                    "chat".to_string(),
                    "tool_call".to_string(),
                    "memory".to_string(),
                ],
                config_schema: None,
            },
        }
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
        &self.metadata.version
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
        self.metadata.config_schema.clone()
    }
    
    fn validate_config(&self, config: &AgentBridgeConfig) -> Result<(), String> {
        // Validate Hermes configuration
        Ok(())
    }
}
```

### 6. Packaging and Deployment

**Note**: This section describes a packaging/deployment subsystem that is **optional** for the initial implementation. The core multi-mode agent system (bridge providers, plugin registry, client mode) can be implemented without this subsystem. This section is included for completeness and can be implemented as a separate effort.

```rust
/// Application packaging configuration
#[derive(Debug, Clone)]
pub struct AppPackageConfig {
    pub app_id: String,
    pub app_version: String,
    pub builtin_plugins: Vec<BuiltinPluginConfig>,
    pub plugin_search_paths: Vec<String>,
    pub auto_load_plugins: bool,
}

/// Built-in plugin configuration
#[derive(Debug, Clone)]
pub struct BuiltinPluginConfig {
    pub plugin_id: String,
    pub plugin_path: String,
    pub enabled: bool,
    pub default_bridge_configs: Vec<AgentBridgeConfig>,
}

/// Application packager
pub struct AppPackager {
    config: AppPackageConfig,
    plugin_registry: AgentBridgePluginRegistry,
}

impl AppPackager {
    /// Create new application packager
    pub fn new(config: AppPackageConfig) -> Self;
    
    /// Add built-in plugin
    pub fn add_builtin_plugin(&mut self, plugin_config: BuiltinPluginConfig);
    
    /// Package application
    pub fn package(&self, output_path: &str) -> Result<PackageManifest, String>;
    
    /// Validate package
    pub fn validate(&self, package_path: &str) -> Result<ValidationReport, String>;
}

/// Package manifest
#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub package_id: String,
    pub package_version: String,
    pub app_id: String,
    pub app_version: String,
    pub plugins: Vec<PluginManifest>,
    pub created_at: String,
    pub checksum: String,
}

/// Plugin manifest
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub plugin_id: String,
    pub plugin_version: String,
    pub plugin_path: String,
    pub enabled: bool,
    pub dependencies: Vec<String>,
}

/// Validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Application installer
pub struct AppInstaller {
    package_path: String,
    install_path: String,
}

impl AppInstaller {
    /// Create new application installer
    pub fn new(package_path: &str, install_path: &str) -> Self;
    
    /// Install application
    pub fn install(&self) -> Result<InstallReport, String>;
    
    /// Uninstall application
    pub fn uninstall(&self) -> Result<UninstallReport, String>;
    
    /// Upgrade application
    pub fn upgrade(&self, new_package_path: &str) -> Result<UpgradeReport, String>;
}

/// Install report
#[derive(Debug, Clone)]
pub struct InstallReport {
    pub success: bool,
    pub installed_plugins: Vec<String>,
    pub install_path: String,
    pub message: String,
}

/// Uninstall report
#[derive(Debug, Clone)]
pub struct UninstallReport {
    pub success: bool,
    pub removed_plugins: Vec<String>,
    pub message: String,
}

/// Upgrade report
#[derive(Debug, Clone)]
pub struct UpgradeReport {
    pub success: bool,
    pub upgraded_plugins: Vec<String>,
    pub from_version: String,
    pub to_version: String,
    pub message: String,
}
```

### 7. Integration with SDKWork Kernel

```rust
/// Integration with sdkwork-agent-kernel
/// Register Agent Bridge Providers as kernel Providers
use sdkwork_agent_kernel::{
    AgentRuntime, RuntimeBuilder, ProviderManifest, ProviderRegistration,
    ModelProvider, ToolProvider, PolicyProvider, ContextProvider,
    MemoryProvider, KnowledgeProvider, PlanningProvider, HostProvider,
    ProtocolAdapter, McpProvider, AgentSkillProvider, AgentCollaborationProvider,
    TelemetryProvider, AgentInstaller, AgentConfigurationProvider,
    ModelRequest, ModelResponse, ModelUsage, ToolCall, ToolResult, ToolDescriptor,
    ProviderHealth, KernelResult, KernelError,
};
use sdkwork_agent_client::{ChatClient, ChatRequest, ChatResponse};

/// Bridge Provider adapter
/// Adapts AgentBridgeProvider to kernel Provider interfaces
/// Uses Arc for shared ownership instead of Clone on trait objects
#[derive(Clone)]
pub struct BridgeProviderAdapter {
    bridge_provider: Arc<dyn AgentBridgeProvider>,
}

impl BridgeProviderAdapter {
    pub fn new(bridge_provider: Arc<dyn AgentBridgeProvider>) -> Self {
        Self { bridge_provider }
    }
}

/// Implement kernel ModelProvider interface
/// Note: ProviderHealth has { status: String } and ProviderHealth::available() constructor
/// This is verified against sdkwork-agent-kernel/src/manifest.rs:116-127
impl ModelProvider for BridgeProviderAdapter {
    fn provider_manifest(&self) -> ProviderManifest {
        // Return bridge provider manifest
        ProviderManifest::new(
            self.bridge_provider.bridge_id(),
            "model",
            "bridge-model-provider",
            self.bridge_provider.version(),
            self.bridge_provider.capabilities(),
        )
    }
    
    fn health(&self) -> ProviderHealth {
        // Convert bridge health to provider health
        let bridge_health = self.bridge_provider.health_check();
        match bridge_health.status {
            AgentBridgeStatus::Healthy => ProviderHealth::available(),
            _ => ProviderHealth {
                status: bridge_health.status.to_string(),
            },
        }
    }
    
    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        // Convert ModelRequest to ChatRequest and delegate to bridge provider
        // ModelRequest.messages is a Vec<String>, we join them into a single content string
        let content = request.messages.join("\n");
        let session_id = request.session_id.clone()
            .unwrap_or_else(|| format!("session.{}", uuid::Uuid::new_v4()));
        
        let chat_request = ChatRequest {
            session_id,
            content,
            model: request.model_id,
            stream: false,
        };
        
        let chat_response = self.bridge_provider.send_message(chat_request)
            .map_err(|e| KernelError::Internal { message: e })?;
        
        // Convert ChatResponse to ModelResponse
        // Note: ModelResponse has many required fields, we use constructors and defaults
        let mut response = ModelResponse::text(
            request.model_request_id,
            self.bridge_provider.bridge_id(),
            chat_response.content,
        );
        
        // Set usage if available
        if let Some(usage) = chat_response.usage {
            response.usage = Some(ModelUsage::new(
                usage.input_tokens,
                usage.output_tokens,
            ));
        }
        
        Ok(response)
    }
    
    // Other ModelProvider methods use default implementations:
    // - list_models() -> Vec<ModelDescriptor> (default: empty)
    // - describe_model(model_id) -> KernelResult<ModelDescriptor> (default: searches list_models)
    // - stream(request) -> KernelResult<Vec<ModelStreamChunk>> (default: CapabilityMissing error)
    // - cancel(model_request_id) -> KernelResult<ModelResponse> (default: CapabilityMissing error)
    // - prepare(model_id) -> KernelResult<()> (default: CapabilityMissing error)
    // - validate_structured_output(request, response) -> KernelResult<ModelStructuredOutputValidation> (default: CapabilityMissing error)
}

/// Implement kernel ToolProvider interface
impl ToolProvider for BridgeProviderAdapter {
    fn provider_manifest(&self) -> ProviderManifest {
        // Return bridge provider manifest
        ProviderManifest::new(
            self.bridge_provider.bridge_id(),
            "tool",
            "bridge-tool-provider",
            self.bridge_provider.version(),
            self.bridge_provider.capabilities(),
        )
    }
    
    fn list_tools(&self) -> Vec<ToolDescriptor> {
        // Bridge providers don't expose tools directly
        // Tools are managed by the underlying agent framework
        Vec::new()
    }
    
    fn health(&self) -> ProviderHealth {
        // Convert bridge health to provider health
        let bridge_health = self.bridge_provider.health_check();
        match bridge_health.status {
            AgentBridgeStatus::Healthy => ProviderHealth::available(),
            _ => ProviderHealth {
                status: bridge_health.status.to_string(),
            },
        }
    }
    
    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        // Bridge providers don't support direct tool invocation
        // Tools are invoked through the agent framework's chat interface
        Err(KernelError::CapabilityMissing {
            capability_id: "tool.invoke".to_string(),
        })
    }
}

/// Capability to Kernel Provider mapping:
/// - "chat" or "model.chat" → ModelProvider (via invoke method)
/// - "tool_call" or "tool.invoke" → ToolProvider (via invoke_tool method)
/// - Other capabilities map to their respective kernel providers

/// Kernel integrator
/// Responsible for integrating bridge providers into SDKWork kernel
/// Note: RuntimeBuilder uses consuming builder pattern (mut self -> Self)
pub struct KernelIntegrator {
    runtime_builder: RuntimeBuilder,
    plugin_registry: Arc<AgentBridgePluginRegistry>,
}

impl KernelIntegrator {
    /// Create new kernel integrator
    pub fn new(
        runtime_builder: RuntimeBuilder,
        plugin_registry: Arc<AgentBridgePluginRegistry>,
    ) -> Self {
        Self {
            runtime_builder,
            plugin_registry,
        }
    }
    
    /// Integrate all bridge providers into kernel
    /// Consumes self and returns the updated KernelIntegrator
    pub fn integrate_all(mut self) -> Result<Self, String> {
        // Get all registered bridge providers as Arc references
        let providers: Vec<Arc<dyn AgentBridgeProvider>> = self.plugin_registry.list_providers();
        
        // Accumulate providers first, then fold
        let mut builder = self.runtime_builder;
        for provider in providers {
            let adapter = BridgeProviderAdapter::new(provider.clone());
            
            for capability in provider.capabilities() {
                match capability.as_str() {
                    "chat" | "model.chat" => {
                        builder = builder
                            .register_model_provider(
                                provider.bridge_id(),
                                provider.version(),
                                adapter.clone(),
                            );
                    }
                    "tool_call" | "tool.invoke" => {
                        builder = builder
                            .register_tool_provider(
                                provider.bridge_id(),
                                provider.version(),
                                adapter.clone(),
                            );
                    }
                    _ => {
                        // Ignore unsupported capabilities
                    }
                }
            }
        }
        
        Ok(Self {
            runtime_builder: builder,
            plugin_registry: self.plugin_registry,
        })
    }
    
    /// Integrate single bridge provider into kernel
    /// Consumes self and returns the updated KernelIntegrator
    pub fn integrate_provider(
        mut self,
        provider: Arc<dyn AgentBridgeProvider>,
    ) -> Result<Self, String> {
        // Create adapter with Arc for shared ownership
        let adapter = BridgeProviderAdapter::new(provider.clone());
        
        // Register to different kernel providers based on capabilities
        // Note: RuntimeBuilder uses consuming builder pattern
        for capability in provider.capabilities() {
            match capability.as_str() {
                "chat" | "model.chat" => {
                    // Register as ModelProvider
                    self.runtime_builder = self.runtime_builder
                        .register_model_provider(
                            provider.bridge_id(),
                            provider.version(),
                            adapter.clone(),
                        );
                }
                "tool_call" | "tool.invoke" => {
                    // Register as ToolProvider
                    self.runtime_builder = self.runtime_builder
                        .register_tool_provider(
                            provider.bridge_id(),
                            provider.version(),
                            adapter.clone(),
                        );
                }
                // ... Other capability registrations
                _ => {
                    // Ignore unsupported capabilities
                }
            }
        }
        
        Ok(self)
    }
    
    /// Build integrated runtime
    pub fn build(self) -> Result<AgentRuntime, String> {
        let report = self.runtime_builder.bootstrap()
            .map_err(|e| format!("Failed to bootstrap runtime: {:?}", e))?;
        
        Ok(report.runtime)
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure

1. Define `AgentBridgeProvider` trait
2. Define `AgentBridgePlugin` trait
3. Implement `AgentBridgePluginRegistry`
4. Implement `AgentClient` with mode support

### Phase 2: Built-in Plugins

1. Implement `OpenClawPlugin` and `OpenClawProvider`
2. Implement `ZeroClawPlugin` and `ZeroClawProvider`
3. Implement `HermesPlugin` and `HermesProvider`

### Phase 3: Server Mode

1. Implement `AgentServer` with multi-protocol support
2. Implement HTTP/REST + SSE server
3. Implement gRPC server
4. Implement WebSocket server

### Phase 4: Kernel Integration

1. Implement `BridgeProviderAdapter`
2. Implement `KernelIntegrator`
3. Integrate with existing SDKWork Agent Kernel

### Phase 5: Packaging and Deployment (Optional)

1. Implement `AppPackager`
2. Implement `AppInstaller`
3. Create deployment scripts and tools

## Testing Strategy

### Unit Tests

1. Test `AgentBridgeProvider` implementations
2. Test `AgentBridgePlugin` implementations
3. Test `AgentBridgePluginRegistry` operations
4. Test `AgentClient` mode switching

### Integration Tests

1. Test client-server communication
2. Test plugin loading and unloading
3. Test kernel integration
4. Test packaging and deployment

### Performance Tests

1. Test local bridge provider performance
2. Test remote server performance
3. Test concurrent connection handling

## Security Considerations

1. **Authentication**: Support API key, JWT, OAuth2
2. **Authorization**: Simple API key-based access control (no complex RBAC for initial implementation)
3. **Encryption**: TLS for all communications
4. **Secrets Management**: Secure storage of credentials using environment variables or secret managers
5. **Audit Logging**: Track all agent operations for debugging and compliance

## Monitoring and Observability

1. **Health Checks**: Regular health checks for all components
2. **Metrics**: Collect performance metrics
3. **Logging**: Structured logging for debugging
4. **Tracing**: Distributed tracing for request flow

## Future Extensions

1. **Additional Agent Frameworks**: Support for codex, claude code, gemini cli, rig
2. **Advanced Features**: Multi-agent collaboration, tool chaining
3. **UI Integration**: Kernel UI components for agent management
4. **Cloud Deployment**: Support for cloud-native deployment

## Conclusion

This design provides a comprehensive multi-mode agent system that supports client/server modes, local/remote agent capabilities, and multiple agent framework implementations. The architecture is extensible, performant, and secure, with clear separation of concerns and well-defined interfaces.

