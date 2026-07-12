//! Rate Limit Provider for resource usage control.
//!
//! This module provides rate limiting support:
//! - Quota management
//! - Rate limit policies
//! - Retry strategies
//! - Backpressure control

use std::collections::HashMap;

/// Rate limit policy configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimitPolicy {
    /// Policy ID.
    pub policy_id: String,
    /// Policy name.
    pub name: String,
    /// Resource type being limited.
    pub resource_type: ResourceType,
    /// Maximum requests per window.
    pub max_requests: u64,
    /// Time window (ms).
    pub window_ms: u64,
    /// Burst size (additional allowance).
    pub burst_size: u64,
    /// Retry strategy.
    pub retry_strategy: RetryStrategy,
    /// Penalty multiplier on violation.
    pub penalty_multiplier: f64,
}

impl RateLimitPolicy {
    pub fn new(
        policy_id: impl Into<String>,
        name: impl Into<String>,
        resource_type: ResourceType,
        max_requests: u64,
        window_ms: u64,
    ) -> Self {
        Self {
            policy_id: policy_id.into(),
            name: name.into(),
            resource_type,
            max_requests,
            window_ms,
            burst_size: 0,
            retry_strategy: RetryStrategy::default(),
            penalty_multiplier: 1.0,
        }
    }

    pub fn with_burst(mut self, burst_size: u64) -> Self {
        self.burst_size = burst_size;
        self
    }

    pub fn with_retry(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }

    pub fn with_penalty(mut self, multiplier: f64) -> Self {
        self.penalty_multiplier = multiplier;
        self
    }

    /// Check if request is within limits.
    pub fn is_within_limits(&self, current_usage: u64) -> bool {
        current_usage < self.max_requests + self.burst_size
    }

    /// Calculate wait time for next available slot.
    pub fn calculate_wait_time(&self, current_usage: u64) -> u64 {
        if current_usage < self.max_requests {
            return 0;
        }

        // Simple calculation: proportion of window remaining
        let overflow = current_usage - self.max_requests;
        let wait_per_unit = self.window_ms / self.max_requests;
        overflow * wait_per_unit
    }
}

/// Resource type being rate limited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// API requests.
    ApiRequest,
    /// Model tokens.
    ModelTokens,
    /// Tool executions.
    ToolExecution,
    /// Storage operations.
    StorageOperation,
    /// Network bandwidth (bytes).
    NetworkBandwidth,
    /// Compute resources (CPU/memory).
    ComputeResource,
    /// Generic resource.
    Generic,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiRequest => "api_request",
            Self::ModelTokens => "model_tokens",
            Self::ToolExecution => "tool_execution",
            Self::StorageOperation => "storage_operation",
            Self::NetworkBandwidth => "network_bandwidth",
            Self::ComputeResource => "compute_resource",
            Self::Generic => "generic",
        }
    }
}

/// Retry strategy configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryStrategy {
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Initial delay (ms).
    pub initial_delay_ms: u64,
    /// Maximum delay (ms).
    pub max_delay_ms: u64,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
    /// Retry on rate limit.
    pub retry_on_rate_limit: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            retry_on_rate_limit: true,
        }
    }
}

impl RetryStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    pub fn with_backoff(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate delay for retry attempt.
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        delay.min(self.max_delay_ms as f64) as u64
    }
}

/// Rate limit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRequest {
    /// Request ID.
    pub request_id: String,
    /// Policy to apply.
    pub policy_id: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Requester identity.
    pub requester: String,
    /// Requested amount.
    pub requested_amount: u64,
    /// Timestamp (ms).
    pub timestamp: u64,
    /// Metadata.
    pub metadata: HashMap<String, String>,
}

impl RateLimitRequest {
    pub fn new(
        policy_id: impl Into<String>,
        resource_id: impl Into<String>,
        requester: impl Into<String>,
        requested_amount: u64,
    ) -> Self {
        Self {
            request_id: format!("req-{}", sdkwork_utils_rust::uuid()),
            policy_id: policy_id.into(),
            resource_id: resource_id.into(),
            requester: requester.into(),
            requested_amount,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Rate limit result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitResult {
    /// Request ID.
    pub request_id: String,
    /// Whether request was allowed.
    pub allowed: bool,
    /// Current usage after request.
    pub current_usage: u64,
    /// Remaining quota.
    pub remaining: u64,
    /// Wait time before retry (ms).
    pub retry_after_ms: u64,
    /// Retry attempt (if retrying).
    pub retry_attempt: u32,
    /// Violation reason (if denied).
    pub violation_reason: Option<String>,
}

impl RateLimitResult {
    pub fn allowed(request_id: impl Into<String>, current_usage: u64, remaining: u64) -> Self {
        Self {
            request_id: request_id.into(),
            allowed: true,
            current_usage,
            remaining,
            retry_after_ms: 0,
            retry_attempt: 0,
            violation_reason: None,
        }
    }

    pub fn denied(
        request_id: impl Into<String>,
        current_usage: u64,
        retry_after_ms: u64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            allowed: false,
            current_usage,
            remaining: 0,
            retry_after_ms,
            retry_attempt: 0,
            violation_reason: Some(reason.into()),
        }
    }

    pub fn with_retry(mut self, attempt: u32) -> Self {
        self.retry_attempt = attempt;
        self
    }
}

/// Usage quota tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaUsage {
    /// Resource ID.
    pub resource_id: String,
    /// Policy ID.
    pub policy_id: String,
    /// Current usage count.
    pub current_usage: u64,
    /// Window start time (ms).
    pub window_start: u64,
    /// Window end time (ms).
    pub window_end: u64,
    /// Violation count.
    pub violations: u64,
    /// Last request time (ms).
    pub last_request: u64,
}

impl QuotaUsage {
    pub fn new(
        resource_id: impl Into<String>,
        policy_id: impl Into<String>,
        window_ms: u64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            resource_id: resource_id.into(),
            policy_id: policy_id.into(),
            current_usage: 0,
            window_start: now,
            window_end: now + window_ms,
            violations: 0,
            last_request: now,
        }
    }

    /// Check if window has expired.
    pub fn is_window_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now >= self.window_end
    }

    /// Reset window.
    pub fn reset_window(&mut self, window_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.current_usage = 0;
        self.window_start = now;
        self.window_end = now + window_ms;
        self.violations = 0;
    }

    /// Increment usage.
    pub fn increment(&mut self, amount: u64) {
        self.current_usage += amount;
        self.last_request = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Record violation.
    pub fn record_violation(&mut self) {
        self.violations += 1;
    }
}

/// Rate limit provider trait.
pub trait RateLimitProvider: Send + Sync {
    /// Create or update a rate limit policy.
    fn create_policy(&mut self, policy: RateLimitPolicy) -> Result<(), RateLimitError>;

    /// Get policy by ID.
    fn get_policy(&self, policy_id: &str) -> Result<RateLimitPolicy, RateLimitError>;

    /// Check rate limit for a request.
    fn check_rate_limit(
        &mut self,
        request: RateLimitRequest,
    ) -> Result<RateLimitResult, RateLimitError>;

    /// Record actual usage (for tracking).
    fn record_usage(
        &mut self,
        resource_id: &str,
        policy_id: &str,
        amount: u64,
    ) -> Result<(), RateLimitError>;

    /// Get current quota usage.
    fn get_usage(&self, resource_id: &str, policy_id: &str) -> Result<QuotaUsage, RateLimitError>;

    /// Reset quota for a resource.
    fn reset_quota(&mut self, resource_id: &str, policy_id: &str) -> Result<(), RateLimitError>;

    /// List all policies.
    fn list_policies(&self) -> Result<Vec<RateLimitPolicy>, RateLimitError>;

    /// Delete a policy.
    fn delete_policy(&mut self, policy_id: &str) -> Result<(), RateLimitError>;

    /// Provider health check.
    fn health_check(&self) -> Result<RateLimitProviderHealth, RateLimitError>;

    /// Provider manifest.
    fn provider_manifest(&self) -> RateLimitProviderManifest;
}

/// Rate limit provider health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitProviderHealth {
    /// Health status.
    pub status: RateLimitProviderStatus,
    /// Active policies count.
    pub active_policies: usize,
    /// Active quotas count.
    pub active_quotas: usize,
    /// Total requests processed.
    pub total_requests_processed: u64,
    /// Total violations.
    pub total_violations: u64,
    /// Last health check time (ms).
    pub last_check_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitProviderStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl RateLimitProviderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

/// Rate limit provider manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitProviderManifest {
    /// Provider ID.
    pub provider_id: String,
    /// Provider name.
    pub name: String,
    /// Provider version.
    pub version: String,
    /// Max policies supported.
    pub max_policies: usize,
    /// Supports distributed rate limiting.
    pub supports_distributed: bool,
    /// Supports custom strategies.
    pub supports_custom_strategies: bool,
    /// Supports real-time monitoring.
    pub supports_realtime_monitoring: bool,
}

/// Rate limit error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    /// Policy not found.
    PolicyNotFound(String),
    /// Quota exceeded.
    QuotaExceeded(String),
    /// Invalid request.
    InvalidRequest(String),
    /// Provider unavailable.
    ProviderUnavailable,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyNotFound(id) => write!(f, "Policy not found: {}", id),
            Self::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::ProviderUnavailable => write!(f, "Rate limit provider unavailable"),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// In-memory rate limit provider (for testing).
#[derive(Debug, Clone)]
pub struct InMemoryRateLimitProvider {
    policies: HashMap<String, RateLimitPolicy>,
    quotas: HashMap<String, QuotaUsage>,
    max_policies: usize,
}

impl InMemoryRateLimitProvider {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            quotas: HashMap::new(),
            max_policies: 100,
        }
    }

    pub fn with_max_policies(mut self, max: usize) -> Self {
        self.max_policies = max;
        self
    }

    fn get_quota_key(resource_id: &str, policy_id: &str) -> String {
        format!("{}:{}", resource_id, policy_id)
    }
}

impl Default for InMemoryRateLimitProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitProvider for InMemoryRateLimitProvider {
    fn create_policy(&mut self, policy: RateLimitPolicy) -> Result<(), RateLimitError> {
        if self.policies.len() >= self.max_policies {
            return Err(RateLimitError::ProviderUnavailable);
        }
        self.policies.insert(policy.policy_id.clone(), policy);
        Ok(())
    }

    fn get_policy(&self, policy_id: &str) -> Result<RateLimitPolicy, RateLimitError> {
        self.policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| RateLimitError::PolicyNotFound(policy_id.to_string()))
    }

    fn check_rate_limit(
        &mut self,
        request: RateLimitRequest,
    ) -> Result<RateLimitResult, RateLimitError> {
        let policy = self.get_policy(&request.policy_id)?;

        let quota_key = Self::get_quota_key(&request.resource_id, &request.policy_id);

        // Get or create quota
        let quota = self.quotas.entry(quota_key.clone()).or_insert_with(|| {
            QuotaUsage::new(&request.resource_id, &request.policy_id, policy.window_ms)
        });

        // Reset if window expired
        if quota.is_window_expired() {
            quota.reset_window(policy.window_ms);
        }

        // Check limits
        let new_usage = quota.current_usage + request.requested_amount;
        if policy.is_within_limits(new_usage) {
            quota.increment(request.requested_amount);
            let remaining = policy.max_requests + policy.burst_size - new_usage;
            Ok(RateLimitResult::allowed(
                &request.request_id,
                new_usage,
                remaining,
            ))
        } else {
            quota.record_violation();
            let wait_time = policy.calculate_wait_time(new_usage);
            Ok(RateLimitResult::denied(
                &request.request_id,
                quota.current_usage,
                wait_time,
                "Rate limit exceeded",
            ))
        }
    }

    fn record_usage(
        &mut self,
        resource_id: &str,
        policy_id: &str,
        amount: u64,
    ) -> Result<(), RateLimitError> {
        let quota_key = Self::get_quota_key(resource_id, policy_id);
        let policy = self.get_policy(policy_id)?;

        let quota = self
            .quotas
            .entry(quota_key)
            .or_insert_with(|| QuotaUsage::new(resource_id, policy_id, policy.window_ms));

        quota.increment(amount);
        Ok(())
    }

    fn get_usage(&self, resource_id: &str, policy_id: &str) -> Result<QuotaUsage, RateLimitError> {
        let quota_key = Self::get_quota_key(resource_id, policy_id);
        self.quotas
            .get(&quota_key)
            .cloned()
            .ok_or_else(|| RateLimitError::InvalidRequest("Quota not found".to_string()))
    }

    fn reset_quota(&mut self, resource_id: &str, policy_id: &str) -> Result<(), RateLimitError> {
        let policy = self.get_policy(policy_id)?;
        let quota_key = Self::get_quota_key(resource_id, policy_id);

        self.quotas
            .entry(quota_key)
            .and_modify(|q| q.reset_window(policy.window_ms));

        Ok(())
    }

    fn list_policies(&self) -> Result<Vec<RateLimitPolicy>, RateLimitError> {
        Ok(self.policies.values().cloned().collect())
    }

    fn delete_policy(&mut self, policy_id: &str) -> Result<(), RateLimitError> {
        self.policies
            .remove(policy_id)
            .map(|_| ())
            .ok_or_else(|| RateLimitError::PolicyNotFound(policy_id.to_string()))
    }

    fn health_check(&self) -> Result<RateLimitProviderHealth, RateLimitError> {
        let total_violations = self.quotas.values().map(|q| q.violations).sum::<u64>();

        Ok(RateLimitProviderHealth {
            status: if total_violations > 100 {
                RateLimitProviderStatus::Degraded
            } else {
                RateLimitProviderStatus::Healthy
            },
            active_policies: self.policies.len(),
            active_quotas: self.quotas.len(),
            total_requests_processed: 0,
            total_violations,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> RateLimitProviderManifest {
        RateLimitProviderManifest {
            provider_id: "in-memory-rate-limit-provider".to_string(),
            name: "In-Memory Rate Limit Provider".to_string(),
            version: "1.0.0".to_string(),
            max_policies: self.max_policies,
            supports_distributed: false,
            supports_custom_strategies: true,
            supports_realtime_monitoring: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_policy_new() {
        let policy = RateLimitPolicy::new(
            "policy-1",
            "API Limit",
            ResourceType::ApiRequest,
            100,
            60000,
        );
        assert_eq!(policy.policy_id, "policy-1");
        assert_eq!(policy.max_requests, 100);
        assert_eq!(policy.window_ms, 60000);
    }

    #[test]
    fn test_rate_limit_policy_with_burst() {
        let policy =
            RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000).with_burst(5);

        assert_eq!(policy.burst_size, 5);
        assert!(policy.is_within_limits(14));
        assert!(!policy.is_within_limits(16));
    }

    #[test]
    fn test_rate_limit_policy_within_limits() {
        let policy = RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000);
        assert!(policy.is_within_limits(9));
        assert!(!policy.is_within_limits(10));
    }

    #[test]
    fn test_resource_type_as_str() {
        assert_eq!(ResourceType::ApiRequest.as_str(), "api_request");
        assert_eq!(ResourceType::ModelTokens.as_str(), "model_tokens");
    }

    #[test]
    fn test_retry_strategy_default() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.initial_delay_ms, 1000);
    }

    #[test]
    fn test_retry_strategy_calculate_delay() {
        let strategy = RetryStrategy::new()
            .with_initial_delay(1000)
            .with_backoff(2.0);

        assert_eq!(strategy.calculate_delay(0), 1000);
        assert_eq!(strategy.calculate_delay(1), 2000);
        assert_eq!(strategy.calculate_delay(2), 4000);
    }

    #[test]
    fn test_rate_limit_request_new() {
        let request = RateLimitRequest::new("policy-1", "resource-1", "agent-1", 10);
        assert_eq!(request.policy_id, "policy-1");
        assert_eq!(request.requested_amount, 10);
        assert!(sdkwork_utils_rust::is_uuid(
            request
                .request_id
                .strip_prefix("req-")
                .expect("request id prefix")
        ));
    }

    #[test]
    fn test_rate_limit_result_allowed() {
        let result = RateLimitResult::allowed("req-1", 5, 95);
        assert!(result.allowed);
        assert_eq!(result.remaining, 95);
        assert!(result.violation_reason.is_none());
    }

    #[test]
    fn test_rate_limit_result_denied() {
        let result = RateLimitResult::denied("req-1", 10, 5000, "Rate limit exceeded");
        assert!(!result.allowed);
        assert_eq!(result.retry_after_ms, 5000);
        assert!(result.violation_reason.is_some());
    }

    #[test]
    fn test_quota_usage_new() {
        let quota = QuotaUsage::new("resource-1", "policy-1", 60000);
        assert_eq!(quota.resource_id, "resource-1");
        assert_eq!(quota.current_usage, 0);
    }

    #[test]
    fn test_quota_usage_increment() {
        let mut quota = QuotaUsage::new("r1", "p1", 1000);
        quota.increment(5);
        assert_eq!(quota.current_usage, 5);
        quota.increment(3);
        assert_eq!(quota.current_usage, 8);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_create_policy() {
        let mut provider = InMemoryRateLimitProvider::new();
        let policy = RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000);
        provider.create_policy(policy).unwrap();

        assert_eq!(provider.list_policies().unwrap().len(), 1);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_check_limit() {
        let mut provider = InMemoryRateLimitProvider::new();
        let policy = RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000);
        provider.create_policy(policy).unwrap();

        // First request: 5 units (total = 5, within limit)
        let request = RateLimitRequest::new("p1", "r1", "agent", 5);
        let result = provider.check_rate_limit(request).unwrap();
        assert!(result.allowed);
        assert_eq!(result.current_usage, 5);

        // Second request: 5 units (total would be 10, which equals max_requests)
        // With strict inequality (<), this should be denied
        let request2 = RateLimitRequest::new("p1", "r1", "agent", 5);
        let result2 = provider.check_rate_limit(request2).unwrap();
        assert!(!result2.allowed); // 10 is not < 10

        // Third request: 1 unit (total would be 6, within limit)
        let request3 = RateLimitRequest::new("p1", "r1", "agent", 1);
        let result3 = provider.check_rate_limit(request3).unwrap();
        assert!(result3.allowed); // 6 < 10
        assert_eq!(result3.current_usage, 6);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_with_burst() {
        let mut provider = InMemoryRateLimitProvider::new();
        let policy =
            RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000).with_burst(5);
        provider.create_policy(policy).unwrap();

        // Should allow burst
        let request = RateLimitRequest::new("p1", "r1", "agent", 14);
        let result = provider.check_rate_limit(request).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_reset_quota() {
        let mut provider = InMemoryRateLimitProvider::new();
        let policy = RateLimitPolicy::new("p1", "Test", ResourceType::Generic, 10, 1000);
        provider.create_policy(policy).unwrap();

        // Use quota
        let request = RateLimitRequest::new("p1", "r1", "agent", 8);
        provider.check_rate_limit(request).unwrap();

        // Reset
        provider.reset_quota("r1", "p1").unwrap();

        // Should have fresh quota
        let usage = provider.get_usage("r1", "p1").unwrap();
        assert_eq!(usage.current_usage, 0);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_health() {
        let provider = InMemoryRateLimitProvider::new();
        let health = provider.health_check().unwrap();

        assert_eq!(health.status, RateLimitProviderStatus::Healthy);
        assert_eq!(health.active_policies, 0);
    }

    #[test]
    fn test_in_memory_rate_limit_provider_manifest() {
        let provider = InMemoryRateLimitProvider::new();
        let manifest = provider.provider_manifest();

        assert_eq!(manifest.provider_id, "in-memory-rate-limit-provider");
        assert!(manifest.supports_custom_strategies);
    }

    #[test]
    fn test_rate_limit_error_display() {
        assert_eq!(
            RateLimitError::PolicyNotFound("p1".to_string()).to_string(),
            "Policy not found: p1"
        );
        assert_eq!(
            RateLimitError::QuotaExceeded("Limit reached".to_string()).to_string(),
            "Quota exceeded: Limit reached"
        );
    }
}
