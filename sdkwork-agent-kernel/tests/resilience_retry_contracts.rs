//! Contract tests for the kernel resilience and retry SPI.
//!
//! The resilience SPI provides circuit breaking, retry with exponential
//! backoff, retry budgets, and unified resilience layers. Rate-limit retry
//! strategies convert into the generic retry engine through `RetryConfig`.

use sdkwork_agent_kernel::{
    execute_with_retry, is_retryable_error, CircuitBreaker, CircuitBreakerConfig, CircuitState,
    KernelError, KernelResult, ResilienceLayer, ResilienceProfile, ResilienceRegistry, RetryBudget,
    RetryBudgetConfig, RetryConfig, RetryStrategy,
};
use std::time::Duration;

fn unavailable() -> KernelError {
    KernelError::ProviderUnavailable {
        provider_id: "contract.test".to_string(),
    }
}

#[test]
fn retry_config_presets_express_operation_character() {
    let default = RetryConfig::default();
    assert_eq!(default.max_retries, 3);
    assert!(default.jitter);

    let critical = RetryConfig::critical_write();
    assert_eq!(critical.max_retries, 2);
    assert_eq!(critical.initial_delay, Duration::from_millis(200));

    let read_only = RetryConfig::read_only();
    assert_eq!(read_only.max_retries, 5);
    assert_eq!(read_only.initial_delay, Duration::from_millis(50));

    assert_eq!(RetryConfig::no_retry().max_retries, 0);
}

#[test]
fn rate_limit_retry_strategy_converts_into_retry_engine_config() {
    let strategy = RetryStrategy::new()
        .with_max_retries(4)
        .with_initial_delay(250)
        .with_backoff(3.0);

    let config: RetryConfig = strategy.clone().into();
    assert_eq!(config.max_retries, 4);
    assert_eq!(config.initial_delay, Duration::from_millis(250));
    assert_eq!(
        config.max_delay,
        Duration::from_millis(strategy.max_delay_ms)
    );
    assert_eq!(config.backoff_factor, 3.0);

    // Rate-limit retries surface as retryable error codes.
    assert!(config
        .retryable_errors
        .contains(&"resource_exhausted".to_string()));

    let no_rate_limit: RetryConfig = RetryStrategy {
        retry_on_rate_limit: false,
        ..RetryStrategy::new()
    }
    .into();
    assert!(no_rate_limit.retryable_errors.is_empty());
}

#[test]
fn retryable_error_classification_is_stable() {
    assert!(is_retryable_error(&unavailable(), false));
    assert!(is_retryable_error(
        &KernelError::ProviderUnavailable {
            provider_id: "x".to_string()
        },
        false
    ));

    // Timeouts retry only for idempotent operations.
    let timeout = KernelError::timeout("contract timeout");
    assert!(is_retryable_error(&timeout, true));
    assert!(!is_retryable_error(&timeout, false));

    // Cancellation is never retried.
    let cancelled = KernelError::cancelled("contract cancelled");
    assert!(!is_retryable_error(&cancelled, true));
}

#[test]
fn execute_with_retry_succeeds_without_retry() {
    let mut calls = 0;
    let result = execute_with_retry::<i32, KernelError, _>(
        RetryConfig::no_retry(),
        None,
        true,
        None,
        || {
            calls += 1;
            Ok(42)
        },
    )
    .unwrap();

    assert_eq!(result.value, 42);
    assert_eq!(result.attempts, 1);
    assert!(!result.retried);
    assert_eq!(result.total_delay, Duration::ZERO);
    assert_eq!(calls, 1);
}

#[test]
fn execute_with_retry_recovers_from_retryable_failures() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(1),
        ..Default::default()
    };
    let mut calls = 0;
    let result = execute_with_retry::<i32, KernelError, _>(config, None, true, None, || {
        calls += 1;
        if calls < 3 {
            Err(unavailable())
        } else {
            Ok(7)
        }
    })
    .unwrap();

    assert_eq!(result.value, 7);
    assert_eq!(result.attempts, 3);
    assert!(result.retried);
}

#[test]
fn execute_with_retry_stops_on_non_retryable_error() {
    let config = RetryConfig {
        max_retries: 5,
        ..Default::default()
    };
    let mut calls = 0;
    let error = execute_with_retry::<i32, KernelError, _>(config, None, true, None, || {
        calls += 1;
        Err(KernelError::validation("invalid input"))
    })
    .unwrap_err();

    assert!(error.to_string().contains("invalid input"));
    assert_eq!(calls, 1);
}

#[test]
fn execute_with_retry_respects_budget() {
    let config = RetryConfig {
        max_retries: 10,
        initial_delay: Duration::from_millis(1),
        ..Default::default()
    };
    let budget = RetryBudget::new(
        "contract.service",
        RetryBudgetConfig {
            max_retries_per_window: 2,
            ..Default::default()
        },
    );
    let mut calls = 0;
    let error =
        execute_with_retry::<i32, KernelError, _>(config, Some(&budget), true, None, || {
            calls += 1;
            Err(unavailable())
        })
        .unwrap_err();

    assert!(error.to_string().contains("budget exhausted"));
    assert!(calls <= 3, "budget must bound retries, got {calls} calls");

    let status = budget.status();
    assert_eq!(status.retries_in_window, 2);
    assert!(status.budget_exhausted_count >= 1);
}

#[test]
fn circuit_breaker_opens_after_failure_threshold() {
    let breaker = CircuitBreaker::new(
        "contract.service",
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            ..Default::default()
        },
    );

    assert_eq!(breaker.state(), CircuitState::Closed);
    assert!(breaker.allow_request().is_ok());

    for _ in 0..3 {
        breaker.record_failure(&unavailable());
    }

    assert_eq!(breaker.state(), CircuitState::Open);

    // Open circuits reject requests with a provider-unavailable error.
    let error = breaker.allow_request().unwrap_err();
    assert!(error.to_string().contains("circuit_breaker"));
}

#[test]
fn circuit_breaker_recovers_through_half_open() {
    let breaker = CircuitBreaker::new(
        "contract.service",
        CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(30),
            half_open_success_threshold: 1,
            half_open_max_requests: 4,
            ..Default::default()
        },
    );

    for _ in 0..2 {
        breaker.record_failure(&unavailable());
    }
    assert_eq!(breaker.state(), CircuitState::Open);

    // After the recovery timeout, the first request transitions to half-open.
    std::thread::sleep(Duration::from_millis(50));
    assert!(breaker.allow_request().is_ok());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    // A success in half-open closes the circuit.
    breaker.record_success();
    assert_eq!(breaker.state(), CircuitState::Closed);

    // A failure in half-open reopens the circuit.
    for _ in 0..2 {
        breaker.record_failure(&unavailable());
    }
    std::thread::sleep(Duration::from_millis(50));
    assert!(breaker.allow_request().is_ok());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    breaker.record_failure(&unavailable());
    assert_eq!(breaker.state(), CircuitState::Open);
}

#[test]
fn resilience_layer_executes_with_breaker_and_retry() {
    let layer = ResilienceLayer::from_profile("contract.service", ResilienceProfile::RpcReadOnly);

    // Circuit is closed; operation succeeds on first attempt.
    let result = layer.execute(|| -> KernelResult<i32> { Ok(11) }).unwrap();
    assert_eq!(result.value, 11);
    assert_eq!(result.attempts, 1);
    assert!(!result.retried);
    assert!(result.circuit_breaker_passed);
}

#[test]
fn resilience_layer_opens_circuit_and_rejects() {
    // Critical-write profile: conservative retry (2) with a low failure
    // threshold (3), so exhausting retries trips the circuit.
    let layer =
        ResilienceLayer::from_profile("contract.service", ResilienceProfile::RpcCriticalWrite);

    // Exhaust retries so the circuit opens.
    let _ = layer.execute(|| -> KernelResult<i32> { Err(unavailable()) });
    assert_eq!(layer.circuit_breaker().state(), CircuitState::Open);

    // Subsequent requests are rejected at the breaker before reaching retry.
    let error = layer
        .execute(|| -> KernelResult<i32> { Ok(1) })
        .unwrap_err();
    assert!(error.to_string().contains("circuit_breaker"));
}

#[test]
fn resilience_layer_health_and_manual_controls() {
    let layer = ResilienceLayer::from_profile("contract.service", ResilienceProfile::RpcReadOnly);

    let health = layer.health();
    assert_eq!(health.service_name, "contract.service");
    assert_eq!(health.circuit_breaker.state, CircuitState::Closed);

    layer.open_circuit();
    assert_eq!(layer.circuit_breaker().state(), CircuitState::Open);

    layer.close_circuit();
    assert_eq!(layer.circuit_breaker().state(), CircuitState::Closed);
}

#[test]
fn resilience_registry_registers_and_looks_up_services() {
    let mut registry = ResilienceRegistry::new();
    registry.register("contract.a", ResilienceProfile::RpcDefault);
    registry.register("contract.b", ResilienceProfile::RpcReadOnly);

    assert_eq!(registry.services(), vec!["contract.a", "contract.b"]);
    assert!(registry.get("contract.a").is_some());
    assert!(registry.get("contract.missing").is_none());
    assert_eq!(registry.health_all().len(), 2);
}

#[test]
fn resilience_profiles_express_operation_character() {
    assert!(ResilienceProfile::RpcReadOnly.is_production_allowed());
    assert!(!ResilienceProfile::RpcLocalDev.is_production_allowed());

    // Read-only profile relaxes retry while critical writes stay conservative.
    let read_only = profile_retry_max(ResilienceProfile::RpcReadOnly);
    let critical = profile_retry_max(ResilienceProfile::RpcCriticalWrite);
    assert!(read_only > critical);
}

fn profile_retry_max(profile: ResilienceProfile) -> u32 {
    use sdkwork_agent_kernel::ResilienceLayerConfig;
    ResilienceLayerConfig::from_profile("contract.probe", profile)
        .retry
        .max_retries
}
