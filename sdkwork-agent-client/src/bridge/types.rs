use std::collections::HashMap;
use std::fmt;

/// Agent bridge type enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentBridgeType {
    OpenClaw,
    ZeroCloud,
    Hermes,
    Codex,
    ClaudeCode,
    OpenCode,
    GeminiCli,
    Custom(String),
}

impl fmt::Display for AgentBridgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenClaw => write!(f, "openclaw"),
            Self::ZeroCloud => write!(f, "zeroclaw"),
            Self::Hermes => write!(f, "hermes"),
            Self::Codex => write!(f, "codex"),
            Self::ClaudeCode => write!(f, "claude-code"),
            Self::OpenCode => write!(f, "opencode"),
            Self::GeminiCli => write!(f, "gemini-cli"),
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
    pub fn new(bridge_id: impl Into<String>, bridge_type: AgentBridgeType) -> Self {
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

    pub fn bearer_token(token: impl Into<String>) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("token".to_string(), token.into());
        Self {
            auth_type: AgentAuthType::BearerToken,
            credentials,
        }
    }

    /// Ingress token plus tenant/user identity for non-loopback Signed mode.
    pub fn ingress_session(
        token: impl Into<String>,
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Self {
        Self::bearer_token(token).with_tenant_user(tenant_id, user_id)
    }

    /// Platform-issued ingress JWT; server validates claims and ignores identity MAC headers.
    pub fn ingress_jwt(token: impl Into<String>) -> Self {
        let mut auth = Self::bearer_token(token);
        auth.credentials
            .insert("ingress_profile".to_string(), "jwt".to_string());
        auth
    }

    pub fn with_tenant_user(
        mut self,
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Self {
        self.credentials
            .insert("tenant_id".to_string(), tenant_id.into());
        self.credentials
            .insert("user_id".to_string(), user_id.into());
        self
    }
}

/// Fallback strategy for hybrid mode
#[derive(Debug, Clone, Default)]
pub enum FallbackStrategy {
    #[default]
    Immediate,
    RetryThenFallback {
        max_retries: u32,
    },
    LocalOnly,
}
