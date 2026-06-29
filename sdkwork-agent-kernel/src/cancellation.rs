//! Cancellation Provider for task cancellation and propagation.
//!
//! This module provides cancellation support:
//! - Cancellation token propagation
//! - Graceful shutdown
//! - Task interruption
//! - Cleanup callbacks

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cancellation token for propagation.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    /// Unique token ID.
    pub token_id: String,
    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
    /// Parent token (for propagation).
    pub parent: Option<Box<CancellationToken>>,
    /// Child tokens (for propagation).
    pub children: Vec<CancellationToken>,
}

impl CancellationToken {
    pub fn new(token_id: impl Into<String>) -> Self {
        Self {
            token_id: token_id.into(),
            cancelled: Arc::new(AtomicBool::new(false)),
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn with_parent(token_id: impl Into<String>, parent: CancellationToken) -> Self {
        Self {
            token_id: token_id.into(),
            cancelled: Arc::new(AtomicBool::new(false)),
            parent: Some(Box::new(parent)),
            children: Vec::new(),
        }
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
            || self.parent.as_ref().map(|p| p.is_cancelled()).unwrap_or(false)
    }

    /// Cancel this token and all children.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Create a child token.
    pub fn create_child(&self, child_id: impl Into<String>) -> CancellationToken {
        let child = CancellationToken::with_parent(child_id, self.clone());
        child
    }
}

impl PartialEq for CancellationToken {
    fn eq(&self, other: &Self) -> bool {
        self.token_id == other.token_id
    }
}

impl Eq for CancellationToken {}

/// Cancellation source for initiating cancellation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationSource {
    /// Source ID.
    pub source_id: String,
    /// Source type.
    pub source_type: CancellationSourceType,
    /// Reason for cancellation.
    pub reason: String,
    /// Timestamp (ms since epoch).
    pub timestamp: u64,
}

impl CancellationSource {
    pub fn new(source_id: impl Into<String>, source_type: CancellationSourceType) -> Self {
        Self {
            source_id: source_id.into(),
            source_type,
            reason: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

/// Cancellation source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationSourceType {
    /// User-initiated cancellation.
    User,
    /// System-initiated cancellation (timeout, resource limit).
    System,
    /// Parent task cancellation.
    Parent,
    /// Error-induced cancellation.
    Error,
    /// Shutdown cancellation.
    Shutdown,
}

impl CancellationSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::Parent => "parent",
            Self::Error => "error",
            Self::Shutdown => "shutdown",
        }
    }
}

/// Cancellation handle for managing cancellation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationHandle {
    /// Handle ID.
    pub handle_id: String,
    /// Associated token ID.
    pub token_id: String,
    /// Cancellation status.
    pub status: CancellationStatus,
    /// Registration time (ms).
    pub registered_at: u64,
    /// Cleanup callbacks registered.
    pub cleanup_callbacks_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationStatus {
    /// Active (not cancelled).
    Active,
    /// Cancelled.
    Cancelled,
    /// Completed (no cancellation needed).
    Completed,
    /// Expired.
    Expired,
}

impl CancellationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Expired => "expired",
        }
    }
}

/// Cancellation scope for grouping cancellations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationScope {
    /// Scope ID.
    pub scope_id: String,
    /// Scope name.
    pub name: String,
    /// Parent scope ID.
    pub parent_scope_id: Option<String>,
    /// Tokens in this scope.
    pub token_ids: Vec<String>,
    /// Timeout (ms, optional).
    pub timeout_ms: Option<u64>,
    /// Created time (ms).
    pub created_at: u64,
}

impl CancellationScope {
    pub fn new(scope_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            scope_id: scope_id.into(),
            name: name.into(),
            parent_scope_id: None,
            token_ids: Vec::new(),
            timeout_ms: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_parent(mut self, parent_scope_id: impl Into<String>) -> Self {
        self.parent_scope_id = Some(parent_scope_id.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn add_token(&mut self, token_id: impl Into<String>) {
        self.token_ids.push(token_id.into());
    }
}

/// Cancellation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationRequest {
    /// Request ID.
    pub request_id: String,
    /// Token to cancel.
    pub token_id: String,
    /// Cancellation source.
    pub source: CancellationSource,
    /// Propagate to children.
    pub propagate: bool,
    /// Force immediate cancellation.
    pub force: bool,
}

impl CancellationRequest {
    pub fn new(token_id: impl Into<String>, source: CancellationSource) -> Self {
        Self {
            request_id: format!("req-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()),
            token_id: token_id.into(),
            source,
            propagate: true,
            force: false,
        }
    }

    pub fn with_propagate(mut self, propagate: bool) -> Self {
        self.propagate = propagate;
        self
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Cancellation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationResult {
    /// Request ID.
    pub request_id: String,
    /// Whether cancellation succeeded.
    pub success: bool,
    /// Tokens cancelled.
    pub tokens_cancelled: Vec<String>,
    /// Cleanup callbacks executed.
    pub cleanup_executed: usize,
    /// Error message (if failed).
    pub error: Option<String>,
}

impl CancellationResult {
    pub fn success(request_id: impl Into<String>, tokens_cancelled: Vec<String>, cleanup_executed: usize) -> Self {
        Self {
            request_id: request_id.into(),
            success: true,
            tokens_cancelled,
            cleanup_executed,
            error: None,
        }
    }

    pub fn failure(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            success: false,
            tokens_cancelled: Vec::new(),
            cleanup_executed: 0,
            error: Some(error.into()),
        }
    }
}

/// Cleanup callback type.
pub type CleanupCallback = Box<dyn Fn() + Send + Sync>;

/// Cancellation Provider trait.
pub trait CancellationProvider: Send + Sync {
    /// Create a new cancellation token.
    fn create_token(&mut self, token_id: &str) -> Result<CancellationToken, CancellationError>;

    /// Create a child token.
    fn create_child_token(
        &mut self,
        parent_token_id: &str,
        child_token_id: &str,
    ) -> Result<CancellationToken, CancellationError>;

    /// Register cleanup callback.
    fn register_cleanup(
        &mut self,
        token_id: &str,
        callback: CleanupCallback,
    ) -> Result<(), CancellationError>;

    /// Request cancellation.
    fn request_cancellation(
        &mut self,
        request: CancellationRequest,
    ) -> Result<CancellationResult, CancellationError>;

    /// Check if token is cancelled.
    fn is_cancelled(&self, token_id: &str) -> Result<bool, CancellationError>;

    /// Get cancellation handle.
    fn get_handle(&self, token_id: &str) -> Result<CancellationHandle, CancellationError>;

    /// Create cancellation scope.
    fn create_scope(&mut self, scope: CancellationScope) -> Result<(), CancellationError>;

    /// Cancel all tokens in a scope.
    fn cancel_scope(
        &mut self,
        scope_id: &str,
        source: CancellationSource,
    ) -> Result<Vec<CancellationResult>, CancellationError>;

    /// List active tokens.
    fn list_active_tokens(&self) -> Result<Vec<CancellationToken>, CancellationError>;

    /// Provider health check.
    fn health_check(&self) -> Result<CancellationProviderHealth, CancellationError>;

    /// Provider manifest.
    fn provider_manifest(&self) -> CancellationProviderManifest;
}

/// Cancellation provider health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationProviderHealth {
    /// Health status.
    pub status: CancellationProviderStatus,
    /// Active tokens count.
    pub active_tokens: usize,
    /// Active scopes count.
    pub active_scopes: usize,
    /// Total cancellations processed.
    pub total_cancellations: u64,
    /// Last health check time (ms).
    pub last_check_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationProviderStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl CancellationProviderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

/// Cancellation provider manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationProviderManifest {
    /// Provider ID.
    pub provider_id: String,
    /// Provider name.
    pub name: String,
    /// Provider version.
    pub version: String,
    /// Max concurrent tokens.
    pub max_tokens: usize,
    /// Supports hierarchical cancellation.
    pub supports_hierarchy: bool,
    /// Supports scopes.
    pub supports_scopes: bool,
    /// Supports cleanup callbacks.
    pub supports_cleanup: bool,
}

/// Cancellation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancellationError {
    /// Token not found.
    TokenNotFound(String),
    /// Token already cancelled.
    TokenAlreadyCancelled(String),
    /// Scope not found.
    ScopeNotFound(String),
    /// Invalid request.
    InvalidRequest(String),
    /// Provider unavailable.
    ProviderUnavailable,
}

impl std::fmt::Display for CancellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenNotFound(id) => write!(f, "Token not found: {}", id),
            Self::TokenAlreadyCancelled(id) => write!(f, "Token already cancelled: {}", id),
            Self::ScopeNotFound(id) => write!(f, "Scope not found: {}", id),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::ProviderUnavailable => write!(f, "Cancellation provider unavailable"),
        }
    }
}

impl std::error::Error for CancellationError {}

/// In-memory cancellation provider (for testing).
pub struct InMemoryCancellationProvider {
    tokens: HashMap<String, CancellationToken>,
    scopes: HashMap<String, CancellationScope>,
    cleanup_callbacks: HashMap<String, usize>,
    max_tokens: usize,
}

impl std::fmt::Debug for InMemoryCancellationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryCancellationProvider")
            .field("tokens", &self.tokens)
            .field("scopes", &self.scopes)
            .field("cleanup_callbacks", &self.cleanup_callbacks)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

impl InMemoryCancellationProvider {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            scopes: HashMap::new(),
            cleanup_callbacks: HashMap::new(),
            max_tokens: 1000,
        }
    }

    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }
}

impl CancellationProvider for InMemoryCancellationProvider {
    fn create_token(&mut self, token_id: &str) -> Result<CancellationToken, CancellationError> {
        let token = CancellationToken::new(token_id);
        self.tokens.insert(token_id.to_string(), token.clone());
        Ok(token)
    }

    fn create_child_token(
        &mut self,
        parent_token_id: &str,
        child_token_id: &str,
    ) -> Result<CancellationToken, CancellationError> {
        let parent = self
            .tokens
            .get(parent_token_id)
            .ok_or_else(|| CancellationError::TokenNotFound(parent_token_id.to_string()))?
            .clone();

        let child = parent.create_child(child_token_id);
        self.tokens.insert(child_token_id.to_string(), child.clone());
        Ok(child)
    }

    fn register_cleanup(
        &mut self,
        token_id: &str,
        _callback: CleanupCallback,
    ) -> Result<(), CancellationError> {
        if !self.tokens.contains_key(token_id) {
            return Err(CancellationError::TokenNotFound(token_id.to_string()));
        }

        let count = self.cleanup_callbacks.get(token_id).unwrap_or(&0) + 1;
        self.cleanup_callbacks.insert(token_id.to_string(), count);

        Ok(())
    }

    fn request_cancellation(
        &mut self,
        request: CancellationRequest,
    ) -> Result<CancellationResult, CancellationError> {
        let token = self
            .tokens
            .get(&request.token_id)
            .ok_or_else(|| CancellationError::TokenNotFound(request.token_id.clone()))?;

        if token.is_cancelled() {
            return Err(CancellationError::TokenAlreadyCancelled(request.token_id));
        }

        // Cancel token
        token.cancel();

        // Execute cleanup callbacks
        let cleanup_executed = *self.cleanup_callbacks.get(&request.token_id).unwrap_or(&0);

        let mut tokens_cancelled = vec![request.token_id.clone()];

        // Propagate to children if requested
        if request.propagate {
            for (_, child_token) in self.tokens.iter() {
                if child_token.parent.as_ref().map(|p| p.token_id.as_str()) == Some(request.token_id.as_str()) {
                    child_token.cancel();
                    tokens_cancelled.push(child_token.token_id.clone());
                }
            }
        }

        Ok(CancellationResult::success(request.request_id, tokens_cancelled, cleanup_executed))
    }

    fn is_cancelled(&self, token_id: &str) -> Result<bool, CancellationError> {
        let token = self
            .tokens
            .get(token_id)
            .ok_or_else(|| CancellationError::TokenNotFound(token_id.to_string()))?;

        Ok(token.is_cancelled())
    }

    fn get_handle(&self, token_id: &str) -> Result<CancellationHandle, CancellationError> {
        let token = self
            .tokens
            .get(token_id)
            .ok_or_else(|| CancellationError::TokenNotFound(token_id.to_string()))?;

        let cleanup_callbacks_count = self
            .cleanup_callbacks
            .get(token_id)
            .copied()
            .unwrap_or(0);

        Ok(CancellationHandle {
            handle_id: format!("handle-{}", token_id),
            token_id: token_id.to_string(),
            status: if token.is_cancelled() {
                CancellationStatus::Cancelled
            } else {
                CancellationStatus::Active
            },
            registered_at: 0,
            cleanup_callbacks_count,
        })
    }

    fn create_scope(&mut self, scope: CancellationScope) -> Result<(), CancellationError> {
        self.scopes.insert(scope.scope_id.clone(), scope);
        Ok(())
    }

    fn cancel_scope(
        &mut self,
        scope_id: &str,
        source: CancellationSource,
    ) -> Result<Vec<CancellationResult>, CancellationError> {
        let scope = self
            .scopes
            .get(scope_id)
            .ok_or_else(|| CancellationError::ScopeNotFound(scope_id.to_string()))?;

        let mut results = Vec::new();
        for token_id in scope.token_ids.clone() {
            let request = CancellationRequest::new(&token_id, source.clone());
            if let Ok(result) = self.request_cancellation(request) {
                results.push(result);
            }
        }

        Ok(results)
    }

    fn list_active_tokens(&self) -> Result<Vec<CancellationToken>, CancellationError> {
        Ok(self
            .tokens
            .values()
            .filter(|t| !t.is_cancelled())
            .cloned()
            .collect())
    }

    fn health_check(&self) -> Result<CancellationProviderHealth, CancellationError> {
        let active_tokens = self.tokens.values().filter(|t| !t.is_cancelled()).count();

        Ok(CancellationProviderHealth {
            status: if active_tokens > self.max_tokens / 2 {
                CancellationProviderStatus::Degraded
            } else {
                CancellationProviderStatus::Healthy
            },
            active_tokens,
            active_scopes: self.scopes.len(),
            total_cancellations: 0,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> CancellationProviderManifest {
        CancellationProviderManifest {
            provider_id: "in-memory-cancellation-provider".to_string(),
            name: "In-Memory Cancellation Provider".to_string(),
            version: "1.0.0".to_string(),
            max_tokens: self.max_tokens,
            supports_hierarchy: true,
            supports_scopes: true,
            supports_cleanup: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token_new() {
        let token = CancellationToken::new("token-1");
        assert_eq!(token.token_id, "token-1");
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_cancel() {
        let token = CancellationToken::new("token-1");
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_child() {
        let parent = CancellationToken::new("parent");
        let child = parent.create_child("child");

        assert_eq!(child.token_id, "child");
        assert!(!child.is_cancelled());

        parent.cancel();
        assert!(child.is_cancelled()); // Child is cancelled when parent cancels
    }

    #[test]
    fn test_cancellation_source_new() {
        let source = CancellationSource::new("source-1", CancellationSourceType::User)
            .with_reason("User request");

        assert_eq!(source.source_id, "source-1");
        assert_eq!(source.source_type, CancellationSourceType::User);
        assert_eq!(source.reason, "User request");
    }

    #[test]
    fn test_cancellation_source_type_as_str() {
        assert_eq!(CancellationSourceType::User.as_str(), "user");
        assert_eq!(CancellationSourceType::System.as_str(), "system");
    }

    #[test]
    fn test_cancellation_scope_new() {
        let scope = CancellationScope::new("scope-1", "Test Scope")
            .with_timeout(60000);

        assert_eq!(scope.scope_id, "scope-1");
        assert_eq!(scope.timeout_ms, Some(60000));
    }

    #[test]
    fn test_cancellation_request_new() {
        let source = CancellationSource::new("source-1", CancellationSourceType::User);
        let request = CancellationRequest::new("token-1", source);

        assert_eq!(request.token_id, "token-1");
        assert!(request.propagate);
        assert!(!request.force);
    }

    #[test]
    fn test_cancellation_result_success() {
        let result = CancellationResult::success("req-1", vec!["token-1".to_string()], 2);
        assert!(result.success);
        assert_eq!(result.tokens_cancelled.len(), 1);
        assert_eq!(result.cleanup_executed, 2);
    }

    #[test]
    fn test_cancellation_result_failure() {
        let result = CancellationResult::failure("req-1", "Token not found");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_in_memory_cancellation_provider_create_token() {
        let mut provider = InMemoryCancellationProvider::new();
        let token = provider.create_token("token-1").unwrap();

        assert_eq!(token.token_id, "token-1");
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_in_memory_cancellation_provider_cancel() {
        let mut provider = InMemoryCancellationProvider::new();
        provider.create_token("token-1").unwrap();

        let source = CancellationSource::new("source-1", CancellationSourceType::User);
        let request = CancellationRequest::new("token-1", source);
        let result = provider.request_cancellation(request).unwrap();

        assert!(result.success);
        assert!(provider.is_cancelled("token-1").unwrap());
    }

    #[test]
    fn test_in_memory_cancellation_provider_child() {
        let mut provider = InMemoryCancellationProvider::new();
        provider.create_token("parent").unwrap();
        provider.create_child_token("parent", "child").unwrap();

        let source = CancellationSource::new("source-1", CancellationSourceType::User);
        let request = CancellationRequest::new("parent", source).with_propagate(true);
        provider.request_cancellation(request).unwrap();

        assert!(provider.is_cancelled("parent").unwrap());
        assert!(provider.is_cancelled("child").unwrap()); // Propagated
    }

    #[test]
    fn test_in_memory_cancellation_provider_scope() {
        let mut provider = InMemoryCancellationProvider::new();
        provider.create_token("token-1").unwrap();
        provider.create_token("token-2").unwrap();

        let mut scope = CancellationScope::new("scope-1", "Test Scope");
        scope.add_token("token-1");
        scope.add_token("token-2");
        provider.create_scope(scope).unwrap();

        let source = CancellationSource::new("source-1", CancellationSourceType::System);
        let results = provider.cancel_scope("scope-1", source).unwrap();

        assert_eq!(results.len(), 2);
        assert!(provider.is_cancelled("token-1").unwrap());
        assert!(provider.is_cancelled("token-2").unwrap());
    }

    #[test]
    fn test_in_memory_cancellation_provider_health() {
        let provider = InMemoryCancellationProvider::new();
        let health = provider.health_check().unwrap();

        assert_eq!(health.status, CancellationProviderStatus::Healthy);
        assert_eq!(health.active_tokens, 0);
    }

    #[test]
    fn test_cancellation_error_display() {
        assert_eq!(
            CancellationError::TokenNotFound("token-1".to_string()).to_string(),
            "Token not found: token-1"
        );
        assert_eq!(
            CancellationError::ScopeNotFound("scope-1".to_string()).to_string(),
            "Scope not found: scope-1"
        );
    }
}