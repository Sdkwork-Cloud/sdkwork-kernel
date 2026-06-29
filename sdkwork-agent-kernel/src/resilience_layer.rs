//! Unified resilience layer combining Circuit Breaker and Retry policies.
//!
//! This module provides a cohesive resilience framework that integrates
//! circuit breaking and retry mechanisms following RPC_RESILIENCE_SPEC.md.

use crate::{CircuitBreaker, CircuitBreakerConfig, KernelError, ResilienceProfile};
use crate::{RetryBudget, RetryBudgetConfig, RetryConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Unified resilience configuration for a service
#[derive(Debug, Clone)]
pub struct ResilienceLayerConfig {
    /// Service name
    pub service_name: String,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Retry budget configuration
    pub retry_budget: RetryBudgetConfig,
    /// Whether operation is idempotent
    pub is_idempotent: bool,
    /// Default deadline for operations
    pub default_deadline: Option<Duration>,
}

impl ResilienceLayerConfig {
    /// Create configuration from resilience profile
    pub fn from_profile(service_name: impl Into<String>, profile: ResilienceProfile) -> Self {
        let service_name = service_name.into();

        let circuit_breaker = profile.circuit_breaker_config();

        let retry = match profile {
            ResilienceProfile::RpcDefault => RetryConfig::default(),
            ResilienceProfile::RpcReadOnly => RetryConfig::read_only(),
            ResilienceProfile::RpcIdempotentWrite => RetryConfig::default(),
            ResilienceProfile::RpcCriticalWrite => RetryConfig::critical_write(),
            ResilienceProfile::RpcStream => RetryConfig::no_retry(), // Streams don't retry
            ResilienceProfile::RpcLocalDev => RetryConfig {
                max_retries: 10,
                initial_delay: Duration::from_millis(10),
                max_delay: Duration::from_secs(1),
                backoff_factor: 1.5,
                jitter: true,
                retryable_errors: Vec::new(),
            },
        };

        let retry_budget = RetryBudgetConfig::default();

        let is_idempotent = matches!(
            profile,
            ResilienceProfile::RpcReadOnly
                | ResilienceProfile::RpcIdempotentWrite
                | ResilienceProfile::RpcLocalDev
        );

        Self {
            service_name,
            circuit_breaker,
            retry,
            retry_budget,
            is_idempotent,
            default_deadline: Some(Duration::from_secs(30)),
        }
    }

    /// Create default configuration
    pub fn default_for_service(service_name: impl Into<String>) -> Self {
        Self::from_profile(service_name, ResilienceProfile::RpcDefault)
    }

    /// Create critical write configuration
    pub fn critical_write(service_name: impl Into<String>) -> Self {
        Self::from_profile(service_name, ResilienceProfile::RpcCriticalWrite)
    }

    /// Create read-only configuration
    pub fn read_only(service_name: impl Into<String>) -> Self {
        Self::from_profile(service_name, ResilienceProfile::RpcReadOnly)
    }
}

/// Unified resilience layer managing circuit breaker and retry
pub struct ResilienceLayer {
    /// Configuration
    config: ResilienceLayerConfig,
    /// Circuit breaker instance
    circuit_breaker: Arc<CircuitBreaker>,
    /// Retry budget instance
    retry_budget: Arc<RetryBudget>,
}

impl ResilienceLayer {
    /// Create a new resilience layer
    pub fn new(config: ResilienceLayerConfig) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            &config.service_name,
            config.circuit_breaker.clone(),
        ));

        let retry_budget = Arc::new(RetryBudget::new(
            &config.service_name,
            config.retry_budget.clone(),
        ));

        Self {
            config,
            circuit_breaker,
            retry_budget,
        }
    }

    /// Create from resilience profile
    pub fn from_profile(service_name: impl Into<String>, profile: ResilienceProfile) -> Self {
        Self::new(ResilienceLayerConfig::from_profile(service_name, profile))
    }

    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.config.service_name
    }

    /// Get circuit breaker reference
    pub fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }

    /// Get retry budget reference
    pub fn retry_budget(&self) -> &Arc<RetryBudget> {
        &self.retry_budget
    }

    /// Check if request is allowed (circuit breaker check)
    pub fn allow_request(&self) -> crate::KernelResult<()> {
        self.circuit_breaker.allow_request()
    }

    /// Execute operation with full resilience (circuit breaker + retry)
    pub fn execute<T, E, F>(&self, operation: F) -> Result<ResilienceResult<T>, KernelError>
    where
        F: FnMut() -> Result<T, E>,
        E: Into<KernelError>,
    {
        self.execute_with_deadline(operation, self.config.default_deadline)
    }

    /// Execute operation with explicit deadline
    pub fn execute_with_deadline<T, E, F>(
        &self,
        mut operation: F,
        deadline: Option<Duration>,
    ) -> Result<ResilienceResult<T>, KernelError>
    where
        F: FnMut() -> Result<T, E>,
        E: Into<KernelError>,
    {
        // Step 1: Check circuit breaker
        self.circuit_breaker.allow_request()?;

        // Step 2: Calculate absolute deadline
        let absolute_deadline = deadline.map(|d| Instant::now() + d);

        // Step 3: Execute with retry
        let retry_result = crate::execute_with_retry(
            self.config.retry.clone(),
            Some(&*self.retry_budget),
            self.config.is_idempotent,
            absolute_deadline,
            || {
                let result = operation();
                match result {
                    Ok(value) => {
                        self.circuit_breaker.record_success();
                        Ok(value)
                    }
                    Err(err) => {
                        let kernel_error: KernelError = err.into();
                        self.circuit_breaker.record_failure(&kernel_error);
                        Err(kernel_error)
                    }
                }
            },
        )?;

        Ok(ResilienceResult {
            value: retry_result.value,
            attempts: retry_result.attempts,
            retried: retry_result.retried,
            total_delay: retry_result.total_delay,
            circuit_breaker_passed: true,
        })
    }

    /// Get health status
    pub fn health(&self) -> ResilienceHealth {
        ResilienceHealth {
            service_name: self.config.service_name.clone(),
            circuit_breaker: self.circuit_breaker.health(),
            retry_budget: self.retry_budget.status(),
        }
    }

    /// Manually open circuit (for testing or admin)
    pub fn open_circuit(&self) {
        self.circuit_breaker.open_circuit();
    }

    /// Manually close circuit (for testing or admin)
    pub fn close_circuit(&self) {
        self.circuit_breaker.close_circuit();
    }
}

/// Resilience execution result
#[derive(Debug, Clone)]
pub struct ResilienceResult<T> {
    /// Final value
    pub value: T,
    /// Total attempts
    pub attempts: u32,
    /// Whether retry was used
    pub retried: bool,
    /// Total delay from retries
    pub total_delay: Duration,
    /// Whether circuit breaker check passed
    pub circuit_breaker_passed: bool,
}

/// Resilience health status
#[derive(Debug, Clone)]
pub struct ResilienceHealth {
    pub service_name: String,
    pub circuit_breaker: crate::CircuitBreakerHealth,
    pub retry_budget: crate::RetryBudgetStatus,
}

/// Resilience registry managing multiple service layers
pub struct ResilienceRegistry {
    layers: Vec<ResilienceLayer>,
}

impl ResilienceRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Register a service with resilience profile
    pub fn register(
        &mut self,
        service_name: impl Into<String>,
        profile: ResilienceProfile,
    ) -> &ResilienceLayer {
        let service = service_name.into();
        let layer = ResilienceLayer::from_profile(&service, profile);
        self.layers.push(layer);
        self.layers.last()
            .expect("resilience registry layer list should not be empty after push")
    }

    /// Register a service with custom configuration
    pub fn register_with_config(&mut self, config: ResilienceLayerConfig) -> &ResilienceLayer {
        let layer = ResilienceLayer::new(config);
        self.layers.push(layer);
        self.layers.last()
            .expect("resilience registry layer list should not be empty after push")
    }

    /// Get resilience layer for a service
    pub fn get(&self, service_name: &str) -> Option<&ResilienceLayer> {
        self.layers.iter().find(|l| l.service_name() == service_name)
    }

    /// Get all registered services
    pub fn services(&self) -> Vec<&str> {
        self.layers.iter().map(|l| l.service_name()).collect()
    }

    /// Get health status for all services
    pub fn health_all(&self) -> Vec<ResilienceHealth> {
        self.layers.iter().map(|l| l.health()).collect()
    }
}

impl Default for ResilienceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resilience_layer_config_from_profile() {
        let config = ResilienceLayerConfig::from_profile("test-service", ResilienceProfile::RpcDefault);

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.circuit_breaker.failure_threshold, 5);
        assert_eq!(config.retry.max_retries, 3);
        assert!(!config.is_idempotent);
    }

    #[test]
    fn resilience_layer_config_critical_write() {
        let config = ResilienceLayerConfig::critical_write("critical-service");

        assert_eq!(config.circuit_breaker.failure_threshold, 3);
        assert_eq!(config.retry.max_retries, 2);
        assert!(!config.is_idempotent);
    }

    #[test]
    fn resilience_layer_config_read_only() {
        let config = ResilienceLayerConfig::read_only("read-service");

        assert_eq!(config.circuit_breaker.failure_threshold, 10);
        assert_eq!(config.retry.max_retries, 5);
        assert!(config.is_idempotent);
    }

    #[test]
    fn resilience_layer_allows_request_when_circuit_closed() {
        let layer = ResilienceLayer::from_profile("test-service", ResilienceProfile::RpcDefault);

        assert!(layer.allow_request().is_ok());
    }

    #[test]
    fn resilience_layer_blocks_request_when_circuit_open() {
        let layer = ResilienceLayer::from_profile("test-service", ResilienceProfile::RpcDefault);

        // Open circuit manually
        layer.open_circuit();

        // Request should be blocked
        let result = layer.allow_request();
        assert!(result.is_err());
        assert!(matches!(result, Err(KernelError::ProviderUnavailable { .. })));
    }

    #[test]
    fn resilience_layer_execute_succeeds() {
        let layer = ResilienceLayer::from_profile("test-service", ResilienceProfile::RpcDefault);

        let result: Result<ResilienceResult<i32>, KernelError> = layer.execute(|| Ok::<i32, KernelError>(42));

        assert!(result.is_ok());
        let resilience_result = result.unwrap();
        assert_eq!(resilience_result.value, 42);
        assert_eq!(resilience_result.attempts, 1);
        assert!(!resilience_result.retried);
        assert!(resilience_result.circuit_breaker_passed);
    }

    #[test]
    fn resilience_layer_execute_retries_on_failure() {
        let config = ResilienceLayerConfig {
            retry: RetryConfig {
                max_retries: 3,
                initial_delay: Duration::from_millis(10),
                ..Default::default()
            },
            ..ResilienceLayerConfig::default_for_service("test-service")
        };
        let layer = ResilienceLayer::new(config);

        let mut call_count = 0;
        let result = layer.execute(|| {
            call_count += 1;
            if call_count < 3 {
                Err(KernelError::ProviderUnavailable {
                    provider_id: "test".into(),
                })
            } else {
                Ok(42)
            }
        });

        assert!(result.is_ok());
        assert_eq!(call_count, 3);
    }

    #[test]
    fn resilience_registry_registers_services() {
        let mut registry = ResilienceRegistry::new();

        registry.register("service-a", ResilienceProfile::RpcDefault);
        registry.register("service-b", ResilienceProfile::RpcReadOnly);

        assert_eq!(registry.services().len(), 2);
        assert!(registry.get("service-a").is_some());
        assert!(registry.get("service-b").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn resilience_registry_health_all() {
        let mut registry = ResilienceRegistry::new();

        registry.register("service-a", ResilienceProfile::RpcDefault);
        registry.register("service-b", ResilienceProfile::RpcReadOnly);

        let health = registry.health_all();
        assert_eq!(health.len(), 2);
        assert_eq!(health[0].service_name, "service-a");
        assert_eq!(health[1].service_name, "service-b");
    }

    #[test]
    fn resilience_result_tracking() {
        let layer = ResilienceLayer::from_profile("test-service", ResilienceProfile::RpcDefault);

        let result = layer.execute(|| Ok::<&str, KernelError>("success")).unwrap();

        assert_eq!(result.value, "success");
        assert_eq!(result.attempts, 1);
        assert!(!result.retried);
        assert_eq!(result.total_delay, Duration::ZERO);
        assert!(result.circuit_breaker_passed);
    }
}