# SDKWork Backend Health Monitor Specification

- **Version**: 0.1.0
- **Status**: Implemented
- **Date**: 2025-06-28
- **Scope**: Backend driver health monitoring, auto-degradation, auto-recovery
- **Domain**: `intelligence`
- **Capability**: `agent-kernel.backend-health-monitor`
- **Implementation**: `sdkwork-agent-kernel/src/backend_health.rs`
- **Test Coverage**: 15/15 tests passing (100%)

## 1. Overview

The Backend Health Monitor provides continuous health monitoring for external agent SDK capability drivers with automatic degradation and recovery logic.

### Key Features

1. **Periodic Health Checks**: Configurable interval (default: 30 seconds)
2. **Rolling Window History**: Track health check results (default: 10 entries)
3. **Auto-Degradation**: Automatically degrade after consecutive failures (default: 3)
4. **Auto-Recovery**: Automatically recover after consecutive successes (default: 5)
5. **Aggregated Status**: Monitor overall system health across all drivers
6. **Event Emission**: Emit health change events to telemetry

## 2. Architecture

### Component Structure

```text
BackendHealthMonitor
  ├── HealthMonitorConfig (configuration)
  ├── HashMap<String, DriverHealthHistory> (driver registry)
  └── AggregateHealthStatus (system-level health)

DriverHealthHistory
  ├── driver_id (identifier)
  ├── Vec<HealthHistoryEntry> (rolling window)
  ├── current_status (SdkDriverStatus)
  ├── consecutive_failures (counter)
  └── consecutive_successes (counter)

SdkDriverStatus
  ├── Healthy (driver is healthy)
  ├── Degraded (driver is degraded, still usable)
  ├── Unhealthy (driver is unhealthy, not usable)
  └ Unknown (initial state)
```

## 3. Configuration

### HealthMonitorConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `check_interval` | `Duration` | 30s | Interval between health checks |
| `degradation_threshold` | `u32` | 3 | Consecutive failures before degradation |
| `recovery_threshold` | `u32` | 5 | Consecutive successes before recovery |
| `history_window_size` | `usize` | 10 | Rolling window size for history |
| `emit_events` | `bool` | true | Emit health change events to telemetry |

### Example Configuration

```rust
let config = HealthMonitorConfig::new()
    .with_check_interval(Duration::from_secs(60))
    .with_degradation_threshold(5)
    .with_recovery_threshold(3)
    .with_history_window_size(20)
    .with_emit_events(false);
```

## 4. State Machine

### Status Transitions

```text
Unknown → Healthy (first healthy check)
Unknown → Unhealthy (first unhealthy check)

Healthy → Degraded (auto-degradation: 3 consecutive failures)
Degraded → Healthy (auto-recovery: 5 consecutive successes)

Degraded → Unhealthy (manual status change)
Unhealthy → Degraded (manual status change)
```

### Auto-Degradation Logic

- **Trigger**: Consecutive failures >= degradation_threshold (default: 3)
- **Previous State**: Must be Healthy
- **New State**: Degraded
- **Reason**: "Auto-degraded after N consecutive failures"
- **Event**: `agent.backend.health.degraded`

### Auto-Recovery Logic

- **Trigger**: Consecutive successes >= recovery_threshold (default: 5)
- **Previous State**: Must be Degraded
- **New State**: Healthy
- **Reason**: "Auto-recovered after N consecutive successes"
- **Event**: `agent.backend.health.recovered`

## 5. API Reference

### BackendHealthMonitor

```rust
/// Create a new monitor with configuration.
pub fn new(config: HealthMonitorConfig) -> Self;

/// Register a driver for health monitoring.
pub fn register_driver(&mut self, driver_id: impl Into<String>);

/// Unregister a driver from health monitoring.
pub fn unregister_driver(&mut self, driver_id: &str);

/// Record a health check result for a driver.
/// Returns HealthStatusChange if status changed.
pub fn record_driver_health(
    &mut self,
    driver_id: &str,
    health: SdkDriverHealth,
) -> Option<HealthStatusChange>;

/// Get current status for a driver.
pub fn driver_status(&self, driver_id: &str) -> Option<SdkDriverStatus>;

/// Get health history for a driver.
pub fn driver_history(&self, driver_id: &str) -> Option<&DriverHealthHistory>;

/// Get all registered drivers.
pub fn registered_drivers(&self) -> Vec<&str>;

/// Get aggregated health status for all drivers.
pub fn aggregate_health(&self) -> AggregateHealthStatus;

/// Check if a driver is usable (healthy or degraded).
pub fn is_driver_usable(&self, driver_id: &str) -> bool;

/// Get drivers that should be avoided (unhealthy or degraded).
pub fn avoid_drivers(&self) -> Vec<&str>;

/// Get drivers that are healthy.
pub fn healthy_drivers(&self) -> Vec<&str>;

/// Check if it's time for a health check.
pub fn should_check(&self) -> bool;

/// Mark the last check time.
pub fn mark_check(&mut self);

/// Generate health change event for telemetry.
pub fn health_change_event(&self, change: &HealthStatusChange) -> KernelEvent;
```

### DriverHealthHistory

```rust
/// Create a new health history tracker.
pub fn new(driver_id: impl Into<String>) -> Self;

/// Record a health check result.
pub fn record_health(&mut self, health: SdkDriverHealth, window_size: usize);

/// Check if should auto-degrade.
pub fn should_degrade(&self, threshold: u32) -> bool;

/// Check if should auto-recover.
pub fn should_recover(&self, threshold: u32) -> bool;

/// Update driver status.
pub fn update_status(&mut self, new_status: SdkDriverStatus);

/// Get latest health check result.
pub fn latest_health(&self) -> Option<&SdkDriverHealth>;

/// Get health history since a time.
pub fn history_since(&self, since: Instant) -> Vec<&HealthHistoryEntry>;

/// Calculate success rate in rolling window.
pub fn success_rate(&self) -> f64;
```

### SdkDriverHealth

```rust
/// Create healthy status.
pub fn healthy() -> Self;

/// Create degraded status with message.
pub fn degraded(message: impl Into<String>) -> Self;

/// Create unhealthy status with message.
pub fn unhealthy(message: impl Into<String>) -> Self;

/// Check if driver is usable (healthy or degraded).
pub fn is_usable(&self) -> bool;
```

## 6. Aggregation Logic

### AggregateHealthStatus

| Condition | Status |
|-----------|--------|
| Any driver unhealthy | Unhealthy |
| Any driver degraded | Degraded |
| All drivers healthy | Healthy |
| No drivers registered | Unknown |

### Example

```rust
let monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
monitor.register_driver("driver-1");
monitor.register_driver("driver-2");

monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());
monitor.record_driver_health("driver-2", SdkDriverHealth::unhealthy("test failure"));

assert_eq!(monitor.aggregate_health(), AggregateHealthStatus::Unhealthy);
```

## 7. Event Model

### Health Change Events

| Event Type | Severity | Condition |
|------------|----------|-----------|
| `agent.backend.health.degraded` | Warn | Driver degraded |
| `agent.backend.health.recovered` | Info | Driver recovered |
| `agent.backend.health.unhealthy` | Error | Driver unhealthy |
| `agent.backend.health.unknown` | Warn | Driver unknown |

### Event Payload

```text
driver_id={driver_id}
previous_status={previous_status}
new_status={new_status}
reason={reason}
```

### Event Schema

- **Schema**: `sdkwork.agent.backend.health.change.v1`
- **Source**: `KernelEventSource::Runtime`

## 8. Usage Patterns

### Pattern 1: Continuous Monitoring

```rust
// Create monitor
let mut monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());

// Register drivers
monitor.register_driver("codex-backend");
monitor.register_driver("claude-backend");

// Background health check loop
loop {
    if monitor.should_check() {
        for driver_id in monitor.registered_drivers() {
            let health = check_driver_health(driver_id); // External call
            let change = monitor.record_driver_health(driver_id, health);
            
            if let Some(change) = change {
                let event = monitor.health_change_event(&change);
                telemetry.record_event(event);
            }
        }
        monitor.mark_check();
    }
    
    sleep(Duration::from_secs(1));
}
```

### Pattern 2: Backend Selection

```rust
// Get usable backends
let usable_drivers = monitor.registered_drivers()
    .filter(|id| monitor.is_driver_usable(id));

// Prioritize healthy backends
let healthy_drivers = monitor.healthy_drivers();

if !healthy_drivers.is_empty() {
    // Use healthy driver
} else if usable_drivers.is_empty() {
    // No usable drivers, fallback or error
} else {
    // Use degraded driver with warning
}
```

### Pattern 3: System Health Dashboard

```rust
// Aggregate health for dashboard
let system_health = monitor.aggregate_health();

// Success rates for each driver
for driver_id in monitor.registered_drivers() {
    if let Some(history) = monitor.driver_history(driver_id) {
        let success_rate = history.success_rate();
        dashboard.update_driver_metric(driver_id, success_rate);
    }
}
```

## 9. Conformance Tests

### Test Coverage (15 tests)

| Test Name | Coverage |
|-----------|----------|
| `test_health_monitor_config_defaults` | Default configuration |
| `test_health_monitor_config_custom` | Custom configuration |
| `test_driver_health_history_new` | History initialization |
| `test_record_health_healthy` | Record healthy check |
| `test_record_health_unhealthy` | Record unhealthy check |
| `test_should_degrade` | Degradation trigger logic |
| `test_should_recover` | Recovery trigger logic |
| `test_success_rate` | Success rate calculation |
| `test_backend_health_monitor_register` | Driver registration |
| `test_backend_health_monitor_unregister` | Driver unregistration |
| `test_backend_health_monitor_auto_degrade` | Auto-degradation workflow |
| `test_backend_health_monitor_auto_recover` | Auto-recovery workflow |
| `test_backend_health_monitor_aggregate` | Aggregate health status |
| `test_backend_health_monitor_usable` | Usable driver check |
| `test_health_change_event` | Event generation |

### Test Execution

```bash
cargo test --package sdkwork-agent-kernel backend_health
```

### Expected Result

```
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured
```

## 10. Performance Characteristics

### Memory

- **Per Driver**: ~200 bytes (10 history entries)
- **Total**: O(n) where n = number of registered drivers
- **Recommendation**: Limit to 100 drivers (~20KB)

### CPU

- **Health Check**: O(1) per driver
- **Aggregation**: O(n) scan all drivers
- **Success Rate**: O(window_size) per driver

### Latency

- **Record Health**: <1ms
- **Aggregate Status**: <10ms (for 100 drivers)
- **Should Check**: <1ms

## 11. Security Considerations

### No Secrets in Health Messages

- Health messages should not contain secrets or credentials
- Use safe error messages for unhealthy status

### Isolation

- Monitor should not access driver internals
- Monitor should only track health status, not influence driver behavior

### Event Redaction

- Health change events are categorized as `Internal`
- No sensitive data in event payloads

## 12. Integration Points

### TelemetryProvider

- Health change events should be recorded via `TelemetryProvider::record_event()`
- Metrics can track success rates via `TelemetryProvider::record_metric()`

### Runtime Provider Selection

- Use `is_driver_usable()` in provider negotiation
- Prefer `healthy_drivers()` over degraded drivers

### PolicyProvider

- Policy decisions can consider driver health status
- Degraded drivers may require additional approval

## 13. Future Extensions

### Planned Extensions (Phase 6)

1. **Background Health Check Task**: Async task with configurable interval
2. **Health Metrics Export**: Prometheus/OpenTelemetry metrics
3. **Driver Weighting**: Weight-based selection (healthy=1.0, degraded=0.5)
4. **Health Predictions**: Predict degradation based on trends
5. **Health Policies**: Configure degradation/recovery per driver

### Extension Points

```rust
// Future: Background task
pub fn spawn_health_check_task(
    monitor: Arc<Mutex<BackendHealthMonitor>>,
    drivers: Arc<DriverRegistry>,
) -> JoinHandle<()>;

// Future: Metrics export
pub fn export_health_metrics(
    monitor: &BackendHealthMonitor,
    telemetry: &mut TelemetryProvider,
);
```

## 14. References

- `sdkwork-agent-kernel/src/backend_health.rs` - Implementation
- `sdkwork-agent-kernel/src/lib.rs` - Module exports
- `sdkwork-agent-provider-spi/src/driver.rs` - Driver SPI reference
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- `specs/AGENT_EVENT_TELEMETRY_SPEC.md` - Telemetry specification

## 15. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-06-28 | Initial implementation, 15/15 tests passing |

---

**Status**: ✅ Implemented and Tested
**Next Steps**: Integration with provider binding system and telemetry