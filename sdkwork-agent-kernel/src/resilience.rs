//! Resilience patterns for SDKWORK agent runtime.
//!
//! This module implements Circuit Breaker, Retry, and related resilience patterns
//! following RPC_RESILIENCE_SPEC.md requirements.

use crate::{KernelError, KernelResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

/// Handle RwLock poisoning errors gracefully
fn handle_lock_poisoned(op: &str) -> KernelError {
    KernelError::Internal {
        message: format!(
            "Lock poisoned in circuit breaker.{}: a thread panicked while holding the lock",
            op
        ),
    }
}

/// Read an RwLock with poison recovery
fn read_lock<'a, T>(lock: &'a RwLock<T>, op: &str) -> KernelResult<RwLockReadGuard<'a, T>> {
    lock.read().map_err(|_| handle_lock_poisoned(op))
}

/// Write an RwLock with poison recovery
fn write_lock<'a, T>(lock: &'a RwLock<T>, op: &str) -> KernelResult<RwLockWriteGuard<'a, T>> {
    lock.write().map_err(|_| handle_lock_poisoned(op))
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CircuitState {
    /// Normal operation, requests flow through
    #[default]
    Closed,
    /// Failure threshold exceeded, requests are blocked
    Open,
    /// Recovery timeout passed, allowing probe requests
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait in Open state before transitioning to HalfOpen
    pub recovery_timeout: Duration,
    /// Number of successful requests in HalfOpen before closing
    pub half_open_success_threshold: u32,
    /// Maximum number of probe requests allowed in HalfOpen state
    pub half_open_max_requests: u32,
    /// Time window for counting failures (rolling window)
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_success_threshold: 3,
            half_open_max_requests: 5,
            failure_window: Duration::from_secs(60),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a conservative config for critical writes
    pub fn critical_write() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            half_open_success_threshold: 5,
            half_open_max_requests: 2,
            failure_window: Duration::from_secs(120),
        }
    }

    /// Create a relaxed config for read-only operations
    pub fn read_only() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(15),
            half_open_success_threshold: 2,
            half_open_max_requests: 10,
            failure_window: Duration::from_secs(30),
        }
    }
}

/// Failure record with timestamp for rolling window
#[derive(Debug, Clone)]
struct FailureRecord {
    timestamp: Instant,
    #[allow(dead_code)]
    error_message: String,
}

/// Circuit breaker for a specific service or provider
pub struct CircuitBreaker {
    /// Service name for logging and metrics
    service_name: String,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Current state
    state: RwLock<CircuitState>,
    /// Consecutive failure count
    consecutive_failures: RwLock<u32>,
    /// Rolling window of failure records
    failures: RwLock<Vec<FailureRecord>>,
    /// Success count in half-open state
    half_open_successes: RwLock<u32>,
    /// Request count in half-open state
    half_open_requests: RwLock<u32>,
    /// Time when circuit opened
    opened_at: RwLock<Option<Instant>>,
    /// Total requests (for metrics)
    total_requests: AtomicU64,
    /// Total failures (for metrics)
    total_failures: AtomicU64,
    /// Total rejected requests (for metrics)
    total_rejected: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker for a service
    pub fn new(service_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            service_name: service_name.into(),
            config,
            state: RwLock::new(CircuitState::Closed),
            consecutive_failures: RwLock::new(0),
            failures: RwLock::new(Vec::new()),
            half_open_successes: RwLock::new(0),
            half_open_requests: RwLock::new(0),
            opened_at: RwLock::new(None),
            total_requests: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
        }
    }

    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        self.state
            .read()
            .map_or(CircuitState::Closed, |state| *state)
    }

    /// Check if a request is allowed through the circuit breaker
    pub fn allow_request(&self) -> KernelResult<()> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        let mut state = write_lock(&self.state, "allow_request")?;

        match *state {
            CircuitState::Closed => {
                // In closed state, all requests are allowed
                Ok(())
            }
            CircuitState::Open => {
                // Check if recovery timeout has passed
                let opened_at = read_lock(&self.opened_at, "allow_request.opened_at")?;
                if let Some(opened_time) = *opened_at {
                    if opened_time.elapsed() >= self.config.recovery_timeout {
                        // Transition to half-open
                        *state = CircuitState::HalfOpen;
                        drop(state);
                        drop(opened_at);

                        // Reset half-open counters
                        *write_lock(&self.half_open_successes, "allow_request.reset_successes")? =
                            0;
                        *write_lock(&self.half_open_requests, "allow_request.reset_requests")? = 0;

                        Ok(())
                    } else {
                        // Still in open state, reject request
                        self.total_rejected.fetch_add(1, Ordering::Relaxed);
                        Err(KernelError::ProviderUnavailable {
                            provider_id: format!("circuit_breaker.{}.open", self.service_name),
                        })
                    }
                } else {
                    // Should not happen, but handle gracefully
                    *state = CircuitState::Closed;
                    Ok(())
                }
            }
            CircuitState::HalfOpen => {
                // Check if we've exceeded max requests in half-open
                let mut half_open_requests =
                    write_lock(&self.half_open_requests, "allow_request.half_open_requests")?;
                if *half_open_requests >= self.config.half_open_max_requests {
                    self.total_rejected.fetch_add(1, Ordering::Relaxed);
                    Err(KernelError::ProviderUnavailable {
                        provider_id: format!("circuit_breaker.{}.half_open_max", self.service_name),
                    })
                } else {
                    *half_open_requests += 1;
                    Ok(())
                }
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let _ = self.state.read().map(|state| {
            match *state {
                CircuitState::Closed => {
                    // Reset consecutive failures on success
                    if let Ok(mut consecutive) = self.consecutive_failures.write() {
                        *consecutive = 0;
                    }
                }
                CircuitState::HalfOpen => {
                    // Increment success count in half-open
                    if let Ok(mut successes) = self.half_open_successes.write() {
                        *successes += 1;

                        // Check if we should close the circuit
                        if *successes >= self.config.half_open_success_threshold {
                            drop(successes);
                            drop(state);
                            self.close_circuit();
                        }
                    }
                }
                CircuitState::Open => {
                    // Should not happen, but ignore
                }
            }
        });
    }

    /// Record a failed request
    pub fn record_failure(&self, error: &KernelError) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);

        if let Ok(state) = self.state.read() {
            match *state {
                CircuitState::Closed => {
                    // Increment consecutive failures
                    if let Ok(mut consecutive) = self.consecutive_failures.write() {
                        *consecutive += 1;

                        // Add to rolling window
                        if let Ok(mut failures) = self.failures.write() {
                            failures.push(FailureRecord {
                                timestamp: Instant::now(),
                                error_message: error.to_string(),
                            });

                            // Prune old failures outside the window
                            let cutoff = Instant::now() - self.config.failure_window;
                            failures.retain(|f| f.timestamp > cutoff);

                            // Check if we should open the circuit
                            if *consecutive >= self.config.failure_threshold {
                                drop(failures);
                                drop(consecutive);
                                drop(state);
                                self.open_circuit();
                            }
                        }
                    }
                }
                CircuitState::HalfOpen => {
                    // Any failure in half-open immediately reopens the circuit
                    drop(state);
                    self.open_circuit();
                }
                CircuitState::Open => {
                    // Already open, just track the failure
                }
            }
        }
    }

    /// Manually open the circuit (for testing or admin override)
    pub fn open_circuit(&self) {
        if let Ok(mut state) = self.state.write() {
            *state = CircuitState::Open;
        }
        if let Ok(mut opened_at) = self.opened_at.write() {
            *opened_at = Some(Instant::now());
        }
        if let Ok(mut value) = self.consecutive_failures.write() {
            *value = 0;
        }
        if let Ok(mut value) = self.half_open_successes.write() {
            *value = 0;
        }
        if let Ok(mut value) = self.half_open_requests.write() {
            *value = 0;
        }
    }

    /// Manually close the circuit (for testing or admin override)
    pub fn close_circuit(&self) {
        if let Ok(mut state) = self.state.write() {
            *state = CircuitState::Closed;
        }
        if let Ok(mut opened_at) = self.opened_at.write() {
            *opened_at = None;
        }
        if let Ok(mut value) = self.consecutive_failures.write() {
            *value = 0;
        }
        if let Ok(mut value) = self.half_open_successes.write() {
            *value = 0;
        }
        if let Ok(mut value) = self.half_open_requests.write() {
            *value = 0;
        }
        if let Ok(mut failures) = self.failures.write() {
            failures.clear();
        }
    }

    /// Check health status
    pub fn health(&self) -> CircuitBreakerHealth {
        let state = self.state.read().map_or(CircuitState::Closed, |s| *s);
        let consecutive = self.consecutive_failures.read().map_or(0, |c| *c);

        CircuitBreakerHealth {
            state,
            consecutive_failures: consecutive,
            failure_threshold: self.config.failure_threshold,
            recovery_timeout: self.config.recovery_timeout,
            opened_at: self.opened_at.read().map_or(None, |o| *o),
        }
    }

    /// Get metrics for monitoring
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            service_name: self.service_name.clone(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
        }
    }
}

/// Circuit breaker metrics
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub service_name: String,
    pub total_requests: u64,
    pub total_failures: u64,
    pub total_rejected: u64,
}

/// Circuit breaker health status
#[derive(Debug, Clone)]
pub struct CircuitBreakerHealth {
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub opened_at: Option<Instant>,
}

/// Resilience profile names as defined in RPC_RESILIENCE_SPEC.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResilienceProfile {
    /// Standard internal unary orchestration
    RpcDefault,
    /// List/retrieve/query methods with no write side effects
    RpcReadOnly,
    /// Manifest-marked idempotent commands
    RpcIdempotentWrite,
    /// Money-moving or security-sensitive writes; conservative retry
    RpcCriticalWrite,
    /// Server/bidi streaming methods
    RpcStream,
    /// Loopback development only; relaxed retry with explicit env gate
    RpcLocalDev,
}

impl ResilienceProfile {
    /// Get circuit breaker config for this profile
    pub fn circuit_breaker_config(&self) -> CircuitBreakerConfig {
        match self {
            ResilienceProfile::RpcDefault => CircuitBreakerConfig::default(),
            ResilienceProfile::RpcReadOnly => CircuitBreakerConfig::read_only(),
            ResilienceProfile::RpcIdempotentWrite => CircuitBreakerConfig::default(),
            ResilienceProfile::RpcCriticalWrite => CircuitBreakerConfig::critical_write(),
            ResilienceProfile::RpcStream => CircuitBreakerConfig::default(),
            ResilienceProfile::RpcLocalDev => CircuitBreakerConfig {
                failure_threshold: 20,
                recovery_timeout: Duration::from_secs(5),
                half_open_success_threshold: 1,
                half_open_max_requests: 20,
                failure_window: Duration::from_secs(10),
            },
        }
    }

    /// Check if this profile is allowed in production
    pub fn is_production_allowed(&self) -> bool {
        !matches!(self, ResilienceProfile::RpcLocalDev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new("test-service", CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn circuit_breaker_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-service", config);

        // Record 3 failures
        for _ in 0..3 {
            cb.record_failure(&KernelError::Internal {
                message: "test error".into(),
            });
        }

        // Should be open now
        assert_eq!(cb.state(), CircuitState::Open);

        // Requests should be rejected
        let result = cb.allow_request();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(KernelError::ProviderUnavailable { .. })
        ));
    }

    #[test]
    fn circuit_breaker_transitions_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-service", config);

        // Open the circuit
        cb.record_failure(&KernelError::Internal {
            message: "error 1".into(),
        });
        cb.record_failure(&KernelError::Internal {
            message: "error 2".into(),
        });
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for recovery timeout
        sleep(Duration::from_millis(150));

        // Should allow request (transition to half-open)
        assert!(cb.allow_request().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn circuit_breaker_closes_after_successful_probes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            half_open_success_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-service", config);

        // Open the circuit
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });

        // Wait for recovery
        sleep(Duration::from_millis(100));

        // Allow request (half-open)
        cb.allow_request().unwrap();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record 2 successes
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_reopens_on_failure_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-service", config);

        // Open the circuit
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });

        // Wait for recovery
        sleep(Duration::from_millis(100));

        // Transition to half-open
        cb.allow_request().unwrap();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record failure in half-open
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });

        // Should be open again
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn circuit_breaker_success_resets_consecutive_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-service", config);

        // Record some failures
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });
        cb.record_failure(&KernelError::Internal {
            message: "error".into(),
        });

        let health = cb.health();
        assert_eq!(health.consecutive_failures, 2);

        // Record success
        cb.record_success();

        let health = cb.health();
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.state, CircuitState::Closed);
    }

    #[test]
    fn resilience_profile_production_check() {
        assert!(ResilienceProfile::RpcDefault.is_production_allowed());
        assert!(ResilienceProfile::RpcReadOnly.is_production_allowed());
        assert!(ResilienceProfile::RpcCriticalWrite.is_production_allowed());
        assert!(!ResilienceProfile::RpcLocalDev.is_production_allowed());
    }

    #[test]
    fn critical_write_config_is_conservative() {
        let config = ResilienceProfile::RpcCriticalWrite.circuit_breaker_config();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.recovery_timeout, Duration::from_secs(60));
        assert_eq!(config.half_open_success_threshold, 5);
        assert_eq!(config.half_open_max_requests, 2);
    }

    #[test]
    fn read_only_config_is_relaxed() {
        let config = ResilienceProfile::RpcReadOnly.circuit_breaker_config();
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.recovery_timeout, Duration::from_secs(15));
        assert_eq!(config.half_open_success_threshold, 2);
        assert_eq!(config.half_open_max_requests, 10);
    }
}
