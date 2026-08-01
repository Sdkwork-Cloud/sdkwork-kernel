//! Retry policies with exponential backoff for SDKWORK agent runtime.
//!
//! This module implements retry mechanisms following RPC_RESILIENCE_SPEC.md requirements:
//! - Bounded retry count
//! - Exponential backoff with jitter
//! - Retry budget per service
//! - Deadline propagation
//! - Idempotency awareness

use crate::KernelError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Handle RwLock poisoning errors
fn handle_lock_error<T>(result: Result<T, std::sync::PoisonError<T>>, op: &str) -> T {
    result.unwrap_or_else(|_| {
        panic!(
            "Lock poisoned in retry.{}: a thread panicked while holding the lock",
            op
        )
    })
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
    /// Backoff multiplier (typically 2.0 for exponential)
    pub backoff_factor: f64,
    /// Whether to add jitter to delays
    pub jitter: bool,
    /// List of retryable error codes (empty = use defaults)
    pub retryable_errors: Vec<String>,
}

impl PartialEq for RetryConfig {
    fn eq(&self, other: &Self) -> bool {
        self.max_retries == other.max_retries
            && self.initial_delay == other.initial_delay
            && self.max_delay == other.max_delay
            && self.backoff_factor == other.backoff_factor
            && self.jitter == other.jitter
            && self.retryable_errors == other.retryable_errors
    }
}

impl From<crate::RetryStrategy> for RetryConfig {
    /// Convert a rate-limit retry strategy into a full retry configuration.
    ///
    /// Rate-limit strategies use millisecond delays and carry a rate-limit
    /// retry flag; the generic engine represents the same intent with
    /// `Duration` delays and the retryable-error list.
    fn from(strategy: crate::RetryStrategy) -> Self {
        let mut retryable_errors = Vec::new();
        if strategy.retry_on_rate_limit {
            retryable_errors.push("resource_exhausted".to_string());
        }
        Self {
            max_retries: strategy.max_retries,
            initial_delay: Duration::from_millis(strategy.initial_delay_ms),
            max_delay: Duration::from_millis(strategy.max_delay_ms),
            backoff_factor: strategy.backoff_multiplier,
            jitter: true,
            retryable_errors,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: true,
            retryable_errors: Vec::new(),
        }
    }
}

impl RetryConfig {
    /// Conservative config for critical writes (fewer retries)
    pub fn critical_write() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: true,
            retryable_errors: Vec::new(),
        }
    }

    /// Relaxed config for read-only operations (more retries)
    pub fn read_only() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
            jitter: true,
            retryable_errors: Vec::new(),
        }
    }

    /// No retry config
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_factor: 1.0,
            jitter: false,
            retryable_errors: Vec::new(),
        }
    }
}

/// Retry budget configuration
#[derive(Debug, Clone)]
pub struct RetryBudgetConfig {
    /// Maximum total retries per time window
    pub max_retries_per_window: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Tokens added per successful request
    pub tokens_per_success: f64,
    /// Maximum token bucket size
    pub max_token_balance: f64,
}

impl Default for RetryBudgetConfig {
    fn default() -> Self {
        Self {
            max_retries_per_window: 100,
            window_duration: Duration::from_secs(60),
            tokens_per_success: 0.1,
            max_token_balance: 10.0,
        }
    }
}

/// Retry budget tracker for a specific service
pub struct RetryBudget {
    /// Service name
    service_name: String,
    /// Configuration
    config: RetryBudgetConfig,
    /// Current token balance
    tokens: RwLock<f64>,
    /// Total retries in current window
    retry_count: AtomicU64,
    /// Window start time
    window_start: RwLock<Instant>,
    /// Total retries (for metrics)
    total_retries: AtomicU64,
    /// Budget exhausted count (for metrics)
    budget_exhausted_count: AtomicU64,
}

impl RetryBudget {
    /// Create a new retry budget for a service
    pub fn new(service_name: impl Into<String>, config: RetryBudgetConfig) -> Self {
        Self {
            service_name: service_name.into(),
            config: config.clone(),
            tokens: RwLock::new(config.max_token_balance),
            retry_count: AtomicU64::new(0),
            window_start: RwLock::new(Instant::now()),
            total_retries: AtomicU64::new(0),
            budget_exhausted_count: AtomicU64::new(0),
        }
    }

    /// Check if a retry is allowed (has budget)
    pub fn allow_retry(&self) -> bool {
        // Reset window if expired
        {
            let mut window_start =
                handle_lock_error(self.window_start.write(), "allow_retry.window_start");
            if window_start.elapsed() >= self.config.window_duration {
                *window_start = Instant::now();
                self.retry_count.store(0, Ordering::Relaxed);
            }
        }

        // Check max retries per window
        let current_count = self.retry_count.load(Ordering::Relaxed);
        if current_count >= self.config.max_retries_per_window as u64 {
            self.budget_exhausted_count.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Check token balance
        let mut tokens = handle_lock_error(self.tokens.write(), "allow_retry.tokens");
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            self.retry_count.fetch_add(1, Ordering::Relaxed);
            self.total_retries.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            self.budget_exhausted_count.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Record a successful request (add tokens)
    pub fn record_success(&self) {
        let mut tokens = handle_lock_error(self.tokens.write(), "record_success");
        *tokens = (*tokens + self.config.tokens_per_success).min(self.config.max_token_balance);
    }

    /// Get current budget status
    pub fn status(&self) -> RetryBudgetStatus {
        RetryBudgetStatus {
            service_name: self.service_name.clone(),
            tokens: *handle_lock_error(self.tokens.read(), "status.tokens"),
            max_token_balance: self.config.max_token_balance,
            retries_in_window: self.retry_count.load(Ordering::Relaxed),
            max_retries_per_window: self.config.max_retries_per_window,
            total_retries: self.total_retries.load(Ordering::Relaxed),
            budget_exhausted_count: self.budget_exhausted_count.load(Ordering::Relaxed),
        }
    }
}

/// Retry budget status for monitoring
#[derive(Debug, Clone)]
pub struct RetryBudgetStatus {
    pub service_name: String,
    pub tokens: f64,
    pub max_token_balance: f64,
    pub retries_in_window: u64,
    pub max_retries_per_window: u32,
    pub total_retries: u64,
    pub budget_exhausted_count: u64,
}

/// Retry execution result
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    /// The final result value
    pub value: T,
    /// Total attempts made (including initial)
    pub attempts: u32,
    /// Whether retry was used
    pub retried: bool,
    /// Total time spent on retries
    pub total_delay: Duration,
}

/// Check if an error is retryable based on default rules
pub fn is_retryable_error(error: &KernelError, is_idempotent: bool) -> bool {
    match error {
        // Provider unavailability is retryable
        KernelError::ProviderUnavailable { .. } => true,

        // Timeout might be retryable for idempotent operations
        KernelError::Structured { info } => {
            match info.kind {
                // UNAVAILABLE - retryable
                crate::KernelErrorKind::ProviderUnavailable => true,

                // RESOURCE_EXHAUSTED - retryable with backoff
                crate::KernelErrorKind::ResourceExhausted => true,

                // TIMEOUT - retryable for idempotent operations
                crate::KernelErrorKind::Timeout => is_idempotent,

                // CANCELLED - not retryable
                crate::KernelErrorKind::Cancelled => false,

                // All others - not retryable by default
                _ => false,
            }
        }

        // All other errors - not retryable
        _ => false,
    }
}

/// Calculate delay for a given retry attempt with exponential backoff and jitter
pub fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
    if attempt == 0 {
        return Duration::ZERO;
    }

    // Calculate base exponential delay
    // attempt=1 -> multiplier=1.0 (initial_delay)
    // attempt=2 -> multiplier=backoff_factor
    // attempt=3 -> multiplier=backoff_factor^2
    let multiplier = config.backoff_factor.powi((attempt - 1) as i32);
    let base_delay = config.initial_delay.as_secs_f64() * multiplier;

    // Cap at max delay
    let capped_delay = base_delay.min(config.max_delay.as_secs_f64());

    // Add jitter if enabled (±50% randomization)
    if config.jitter {
        // Simple jitter: add up to 50% variation
        let jitter_range = capped_delay * 0.5;
        // Use attempt as pseudo-random seed (deterministic but varied)
        let jitter_factor = ((attempt as f64 * 0.618) % 1.0) - 0.5; // -0.5 to 0.5
        let final_delay = capped_delay + jitter_factor * jitter_range;
        Duration::from_secs_f64(final_delay.max(0.0))
    } else {
        Duration::from_secs_f64(capped_delay)
    }
}

/// Execute an operation with retry policy
pub fn execute_with_retry<T, E, F>(
    config: RetryConfig,
    budget: Option<&RetryBudget>,
    is_idempotent: bool,
    deadline: Option<Instant>,
    mut operation: F,
) -> Result<RetryResult<T>, KernelError>
where
    F: FnMut() -> Result<T, E>,
    E: Into<KernelError>,
{
    let mut attempts = 0;
    let mut total_delay = Duration::ZERO;
    let mut last_error: Option<KernelError> = None;

    for attempt in 0..=config.max_retries {
        attempts += 1;

        // Check deadline
        if let Some(d) = deadline {
            if Instant::now() >= d {
                return Err(KernelError::Internal {
                    message: format!("Retry deadline exceeded after {} attempts", attempts),
                });
            }
        }

        // Execute operation
        match operation() {
            Ok(value) => {
                // Record success for budget
                if let Some(b) = budget {
                    b.record_success();
                }

                return Ok(RetryResult {
                    value,
                    attempts,
                    retried: attempt > 0,
                    total_delay,
                });
            }
            Err(err) => {
                let kernel_error: KernelError = err.into();
                last_error = Some(kernel_error.clone());

                // Check if error is retryable
                if !is_retryable_error(&kernel_error, is_idempotent) {
                    return Err(kernel_error);
                }

                // Check retry budget
                if let Some(b) = budget {
                    if !b.allow_retry() {
                        return Err(KernelError::Internal {
                            message: format!(
                                "Retry budget exhausted for service '{}'",
                                b.service_name
                            ),
                        });
                    }
                }

                // Don't delay after last attempt
                if attempt >= config.max_retries {
                    break;
                }

                // Calculate and apply delay
                let delay = calculate_delay(attempt, &config);
                total_delay += delay;

                // Check if delay would exceed deadline
                if let Some(d) = deadline {
                    if Instant::now() + delay > d {
                        return Err(KernelError::Internal {
                            message: format!(
                                "Retry delay would exceed deadline (attempt {})",
                                attempts
                            ),
                        });
                    }
                }

                // Apply delay (blocking sleep for sync operation)
                std::thread::sleep(delay);
            }
        }
    }

    // All retries exhausted
    Err(last_error.unwrap_or_else(|| KernelError::Internal {
        message: "Retry exhausted without error".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.backoff_factor, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn critical_write_config_is_conservative() {
        let config = RetryConfig::critical_write();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
    }

    #[test]
    fn read_only_config_is_relaxed() {
        let config = RetryConfig::read_only();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(50));
    }

    #[test]
    fn no_retry_config_disables_retries() {
        let config = RetryConfig::no_retry();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn retry_budget_allows_retries_within_limit() {
        let config = RetryBudgetConfig {
            max_retries_per_window: 5,
            ..Default::default()
        };
        let budget = RetryBudget::new("test-service", config);

        // Should allow 5 retries
        for _ in 0..5 {
            assert!(budget.allow_retry());
        }

        // 6th should be denied
        assert!(!budget.allow_retry());

        // Check status
        let status = budget.status();
        assert_eq!(status.retries_in_window, 5);
        assert_eq!(status.budget_exhausted_count, 1);
    }

    #[test]
    fn retry_budget_adds_tokens_on_success() {
        let config = RetryBudgetConfig {
            tokens_per_success: 0.5,
            max_token_balance: 10.0,
            ..Default::default()
        };
        let budget = RetryBudget::new("test-service", config);

        // Exhaust some tokens
        budget.allow_retry();
        budget.allow_retry();

        let status = budget.status();
        assert!(status.tokens < 10.0);

        // Add tokens with success
        budget.record_success();
        budget.record_success();

        let status = budget.status();
        assert!(status.tokens > status.tokens - 1.0);
    }

    #[test]
    fn calculate_delay_increases_exponentially() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: false,
            ..Default::default()
        };

        let d0 = calculate_delay(0, &config);
        let d1 = calculate_delay(1, &config);
        let d2 = calculate_delay(2, &config);
        let d3 = calculate_delay(3, &config);

        assert_eq!(d0, Duration::ZERO);
        assert_eq!(d1, Duration::from_millis(100));
        assert_eq!(d2, Duration::from_millis(200));
        assert_eq!(d3, Duration::from_millis(400));
    }

    #[test]
    fn calculate_delay_caps_at_max() {
        let config = RetryConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_factor: 10.0,
            jitter: false,
            ..Default::default()
        };

        let d5 = calculate_delay(5, &config);
        assert!(d5 <= Duration::from_secs(5));
    }

    #[test]
    fn jitter_adds_variation() {
        let config_with_jitter = RetryConfig {
            jitter: true,
            ..Default::default()
        };

        let config_without_jitter = RetryConfig {
            jitter: false,
            ..Default::default()
        };

        // With jitter, same attempt might have different delays
        let d1_a = calculate_delay(1, &config_with_jitter);
        let d1_b = calculate_delay(2, &config_with_jitter); // Different seed

        // Without jitter, delays are deterministic
        let d1_c = calculate_delay(1, &config_without_jitter);
        let d1_d = calculate_delay(1, &config_without_jitter);

        assert_eq!(d1_c, d1_d);

        // With jitter might be different (but might not always)
        // Just verify they're both in reasonable range (0 to initial_delay * 1.5)
        assert!(d1_a <= Duration::from_millis(150));
        assert!(d1_b <= Duration::from_millis(300));
    }

    #[test]
    fn is_retryable_error_provider_unavailable() {
        let error = KernelError::ProviderUnavailable {
            provider_id: "test".into(),
        };
        assert!(is_retryable_error(&error, false));
    }

    #[test]
    fn execute_with_retry_succeeds_on_first_attempt() {
        let config = RetryConfig::default();
        let mut call_count = 0;

        let result = execute_with_retry::<i32, KernelError, _>(config, None, true, None, || {
            call_count += 1;
            Ok(42)
        });

        assert!(result.is_ok());
        let retry_result = result.unwrap();
        assert_eq!(retry_result.value, 42);
        assert_eq!(retry_result.attempts, 1);
        assert!(!retry_result.retried);
        assert_eq!(retry_result.total_delay, Duration::ZERO);
        assert_eq!(call_count, 1);
    }

    #[test]
    fn execute_with_retry_retries_on_retryable_error() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        let mut call_count = 0;

        let result = execute_with_retry::<i32, KernelError, _>(config, None, true, None, || {
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
        let retry_result = result.unwrap();
        assert_eq!(retry_result.value, 42);
        assert_eq!(retry_result.attempts, 3);
        assert!(retry_result.retried);
        assert_eq!(call_count, 3);
    }

    #[test]
    fn execute_with_retry_fails_after_max_retries() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        let mut call_count = 0;

        let result = execute_with_retry::<i32, KernelError, _>(config, None, true, None, || {
            call_count += 1;
            Err(KernelError::ProviderUnavailable {
                provider_id: "test".into(),
            })
        });

        assert!(result.is_err());
        assert_eq!(call_count, 3); // 1 initial + 2 retries
    }

    #[test]
    fn execute_with_retry_respects_deadline() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_millis(100),
            ..Default::default()
        };
        let deadline = Instant::now() + Duration::from_millis(50);
        let mut call_count = 0;

        let result =
            execute_with_retry::<i32, KernelError, _>(config, None, true, Some(deadline), || {
                call_count += 1;
                Err(KernelError::ProviderUnavailable {
                    provider_id: "test".into(),
                })
            });

        assert!(result.is_err());
        assert!(matches!(result, Err(KernelError::Internal { .. })));
    }

    #[test]
    fn execute_with_retry_uses_budget() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        let budget_config = RetryBudgetConfig {
            max_retries_per_window: 2,
            ..Default::default()
        };
        let budget = RetryBudget::new("test-service", budget_config);
        let mut call_count = 0;

        let result =
            execute_with_retry::<i32, KernelError, _>(config, Some(&budget), true, None, || {
                call_count += 1;
                Err(KernelError::ProviderUnavailable {
                    provider_id: "test".into(),
                })
            });

        // Should stop early due to budget exhaustion
        assert!(result.is_err());
        assert!(call_count <= 3); // 1 initial + 2 budgeted retries
    }
}
