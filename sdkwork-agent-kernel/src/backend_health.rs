//! Backend Health Monitor for continuous health checking and auto-degradation/recovery.
//!
//! This module implements a background health monitoring system that:
//! - Periodically checks backend health via AgentSdkCapabilityDriver::health()
//! - Tracks health history in a rolling window
//! - Auto-degrades backends after consecutive failures
//! - Auto-recovers backends after consecutive successes
//! - Emits health change events to telemetry

use crate::{KernelEvent, KernelEventSeverity, KernelEventSource};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Driver status for backend health monitoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkDriverStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Driver health result from backend health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkDriverHealth {
    pub status: SdkDriverStatus,
    pub message: Option<String>,
}

impl SdkDriverHealth {
    pub fn healthy() -> Self {
        Self {
            status: SdkDriverStatus::Healthy,
            message: None,
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: SdkDriverStatus::Degraded,
            message: Some(message.into()),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: SdkDriverStatus::Unhealthy,
            message: Some(message.into()),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            SdkDriverStatus::Healthy | SdkDriverStatus::Degraded
        )
    }
}

/// Configuration for the Backend Health Monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthMonitorConfig {
    /// Interval between health checks (default: 30 seconds).
    pub check_interval: Duration,
    /// Number of consecutive failures before degradation (default: 3).
    pub degradation_threshold: u32,
    /// Number of consecutive successes before recovery (default: 5).
    pub recovery_threshold: u32,
    /// Rolling window size for health history (default: 10).
    pub history_window_size: usize,
    /// Enable telemetry event emission (default: true).
    pub emit_events: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            degradation_threshold: 3,
            recovery_threshold: 5,
            history_window_size: 10,
            emit_events: true,
        }
    }
}

impl HealthMonitorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    pub fn with_degradation_threshold(mut self, threshold: u32) -> Self {
        self.degradation_threshold = threshold.max(1);
        self
    }

    pub fn with_recovery_threshold(mut self, threshold: u32) -> Self {
        self.recovery_threshold = threshold.max(1);
        self
    }

    pub fn with_history_window_size(mut self, size: usize) -> Self {
        self.history_window_size = size.max(1);
        self
    }

    pub fn with_emit_events(mut self, emit_events: bool) -> Self {
        self.emit_events = emit_events;
        self
    }
}

/// Health history entry for a single backend check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthHistoryEntry {
    /// Time of the health check.
    pub checked_at: Instant,
    /// Health result from the driver.
    pub health: SdkDriverHealth,
}

/// Health history tracker for a single backend driver.
#[derive(Debug, Clone)]
pub struct DriverHealthHistory {
    /// Driver identifier.
    pub driver_id: String,
    /// Rolling window of health history entries.
    pub history: Vec<HealthHistoryEntry>,
    /// Current driver status (may differ from latest health if auto-degraded).
    pub current_status: SdkDriverStatus,
    /// Consecutive failure count (for degradation).
    pub consecutive_failures: u32,
    /// Consecutive success count (for recovery).
    pub consecutive_successes: u32,
}

impl DriverHealthHistory {
    pub fn new(driver_id: impl Into<String>) -> Self {
        Self {
            driver_id: driver_id.into(),
            history: Vec::new(),
            current_status: SdkDriverStatus::Unknown,
            consecutive_failures: 0,
            consecutive_successes: 0,
        }
    }

    /// Record a new health check result.
    pub fn record_health(&mut self, health: SdkDriverHealth, window_size: usize) {
        let entry = HealthHistoryEntry {
            checked_at: Instant::now(),
            health: health.clone(),
        };

        // Add to rolling window
        self.history.push(entry);
        if self.history.len() > window_size {
            self.history.remove(0);
        }

        // Update counters
        match health.status {
            SdkDriverStatus::Healthy => {
                self.consecutive_failures = 0;
                self.consecutive_successes += 1;
            }
            SdkDriverStatus::Degraded => {
                // Degraded counts as failure for auto-degradation, but not for auto-recovery
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;
            }
            SdkDriverStatus::Unhealthy | SdkDriverStatus::Unknown => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;
            }
        }
    }

    /// Check if the driver should be auto-degraded.
    pub fn should_degrade(&self, threshold: u32) -> bool {
        self.consecutive_failures >= threshold
            && self.current_status == SdkDriverStatus::Healthy
    }

    /// Check if the driver should be auto-recovered.
    pub fn should_recover(&self, threshold: u32) -> bool {
        self.consecutive_successes >= threshold
            && self.current_status == SdkDriverStatus::Degraded
    }

    /// Update driver status.
    pub fn update_status(&mut self, new_status: SdkDriverStatus) {
        self.current_status = new_status;
    }

    /// Get the latest health check result.
    pub fn latest_health(&self) -> Option<&SdkDriverHealth> {
        self.history.last().map(|entry| &entry.health)
    }

    /// Get health history entries within a time range.
    pub fn history_since(&self, since: Instant) -> Vec<&HealthHistoryEntry> {
        self.history
            .iter()
            .filter(|entry| entry.checked_at >= since)
            .collect()
    }

    /// Calculate health success rate in the rolling window.
    pub fn success_rate(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let healthy_count = self
            .history
            .iter()
            .filter(|entry| entry.health.status == SdkDriverStatus::Healthy)
            .count();

        healthy_count as f64 / self.history.len() as f64
    }
}

/// Backend Health Monitor that tracks driver health over time.
#[derive(Debug, Clone)]
pub struct BackendHealthMonitor {
    /// Configuration for the monitor.
    config: HealthMonitorConfig,
    /// Health history for each driver.
    drivers: HashMap<String, DriverHealthHistory>,
    /// Last health check time.
    last_check: Option<Instant>,
}

impl BackendHealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            config,
            drivers: HashMap::new(),
            last_check: None,
        }
    }

    /// Register a driver for health monitoring.
    pub fn register_driver(&mut self, driver_id: impl Into<String>) {
        let driver_id = driver_id.into();
        if !self.drivers.contains_key(&driver_id) {
            self.drivers.insert(driver_id.clone(), DriverHealthHistory::new(driver_id));
        }
    }

    /// Unregister a driver from health monitoring.
    pub fn unregister_driver(&mut self, driver_id: &str) {
        self.drivers.remove(driver_id);
    }

    /// Record a health check for a driver.
    pub fn record_driver_health(
        &mut self,
        driver_id: &str,
        health: SdkDriverHealth,
    ) -> Option<HealthStatusChange> {
        let driver_history = self.drivers.get_mut(driver_id)?;

        let previous_status = driver_history.current_status.clone();
        driver_history.record_health(health.clone(), self.config.history_window_size);

        // Handle initial Unknown state - accept driver's reported status
        if previous_status == SdkDriverStatus::Unknown {
            driver_history.update_status(health.status.clone());
            return Some(HealthStatusChange {
                driver_id: driver_id.to_string(),
                previous_status,
                new_status: health.status.clone(),
                reason: "Initial health status from driver".to_string(),
                health: Some(health),
            });
        }

        // Check for auto-degradation (only from Healthy state)
        if driver_history.should_degrade(self.config.degradation_threshold)
            && previous_status == SdkDriverStatus::Healthy
        {
            driver_history.update_status(SdkDriverStatus::Degraded);
            return Some(HealthStatusChange {
                driver_id: driver_id.to_string(),
                previous_status,
                new_status: SdkDriverStatus::Degraded,
                reason: format!(
                    "Auto-degraded after {} consecutive failures",
                    self.config.degradation_threshold
                ),
                health: driver_history.latest_health().cloned(),
            });
        }

        // Check for auto-recovery (only from Degraded state)
        if driver_history.should_recover(self.config.recovery_threshold)
            && previous_status == SdkDriverStatus::Degraded
        {
            driver_history.update_status(SdkDriverStatus::Healthy);
            return Some(HealthStatusChange {
                driver_id: driver_id.to_string(),
                previous_status,
                new_status: SdkDriverStatus::Healthy,
                reason: format!(
                    "Auto-recovered after {} consecutive successes",
                    self.config.recovery_threshold
                ),
                health: driver_history.latest_health().cloned(),
            });
        }

        None
    }

    /// Get the current status for a driver.
    pub fn driver_status(&self, driver_id: &str) -> Option<SdkDriverStatus> {
        self.drivers.get(driver_id).map(|history| history.current_status.clone())
    }

    /// Get the health history for a driver.
    pub fn driver_history(&self, driver_id: &str) -> Option<&DriverHealthHistory> {
        self.drivers.get(driver_id)
    }

    /// Get all registered drivers.
    pub fn registered_drivers(&self) -> Vec<&str> {
        self.drivers.keys().map(|s| s.as_str()).collect()
    }

    /// Get aggregated health status for all drivers.
    pub fn aggregate_health(&self) -> AggregateHealthStatus {
        let total = self.drivers.len();
        if total == 0 {
            return AggregateHealthStatus::Unknown;
        }

        let healthy_count = self
            .drivers
            .values()
            .filter(|history| history.current_status == SdkDriverStatus::Healthy)
            .count();

        let degraded_count = self
            .drivers
            .values()
            .filter(|history| history.current_status == SdkDriverStatus::Degraded)
            .count();

        let unhealthy_count = self
            .drivers
            .values()
            .filter(|history| history.current_status == SdkDriverStatus::Unhealthy)
            .count();

        // If any driver is unhealthy, aggregate is unhealthy
        if unhealthy_count > 0 {
            return AggregateHealthStatus::Unhealthy;
        }

        // If any driver is degraded, aggregate is degraded
        if degraded_count > 0 {
            return AggregateHealthStatus::Degraded;
        }

        // If all drivers are healthy, aggregate is healthy
        if healthy_count == total {
            return AggregateHealthStatus::Healthy;
        }

        // Otherwise, unknown
        AggregateHealthStatus::Unknown
    }

    /// Check if a driver is usable (healthy or degraded).
    pub fn is_driver_usable(&self, driver_id: &str) -> bool {
        self.driver_status(driver_id)
            .map(|status| matches!(status, SdkDriverStatus::Healthy | SdkDriverStatus::Degraded))
            .unwrap_or(false)
    }

    /// Get drivers that should be avoided (unhealthy or degraded).
    pub fn avoid_drivers(&self) -> Vec<&str> {
        self.drivers
            .iter()
            .filter(|(_, history)| {
                matches!(
                    history.current_status,
                    SdkDriverStatus::Unhealthy | SdkDriverStatus::Degraded
                )
            })
            .map(|(driver_id, _)| driver_id.as_str())
            .collect()
    }

    /// Get drivers that are healthy.
    pub fn healthy_drivers(&self) -> Vec<&str> {
        self.drivers
            .iter()
            .filter(|(_, history)| history.current_status == SdkDriverStatus::Healthy)
            .map(|(driver_id, _)| driver_id.as_str())
            .collect()
    }

    /// Mark the last check time.
    pub fn mark_check(&mut self) {
        self.last_check = Some(Instant::now());
    }

    /// Check if it's time for a health check.
    pub fn should_check(&self) -> bool {
        match self.last_check {
            None => true,
            Some(last) => last.elapsed() >= self.config.check_interval,
        }
    }

    /// Generate health change event.
    pub fn health_change_event(&self, change: &HealthStatusChange) -> KernelEvent {
        let event_id = format!("health.change.{}.{}", change.driver_id, Instant::now().elapsed().as_millis());
        let event_type = match change.new_status {
            SdkDriverStatus::Healthy => "agent.backend.health.recovered",
            SdkDriverStatus::Degraded => "agent.backend.health.degraded",
            SdkDriverStatus::Unhealthy => "agent.backend.health.unhealthy",
            SdkDriverStatus::Unknown => "agent.backend.health.unknown",
        };

        let severity = match change.new_status {
            SdkDriverStatus::Healthy => KernelEventSeverity::Info,
            SdkDriverStatus::Degraded => KernelEventSeverity::Warn,
            SdkDriverStatus::Unhealthy => KernelEventSeverity::Error,
            SdkDriverStatus::Unknown => KernelEventSeverity::Warn,
        };

        KernelEvent::new(
            event_id,
            event_type,
            severity,
            format!(
                "driver_id={};previous_status={};new_status={};reason={}",
                change.driver_id,
                change.previous_status.as_str(),
                change.new_status.as_str(),
                change.reason
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_payload_schema("sdkwork.agent.backend.health.change.v1")
    }
}

/// Health status change event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatusChange {
    /// Driver that changed status.
    pub driver_id: String,
    /// Previous status.
    pub previous_status: SdkDriverStatus,
    /// New status.
    pub new_status: SdkDriverStatus,
    /// Reason for the change.
    pub reason: String,
    /// Latest health check result.
    pub health: Option<SdkDriverHealth>,
}

/// Aggregate health status for all monitored drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl AggregateHealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
}

impl SdkDriverStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_config_defaults() {
        let config = HealthMonitorConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(30));
        assert_eq!(config.degradation_threshold, 3);
        assert_eq!(config.recovery_threshold, 5);
        assert_eq!(config.history_window_size, 10);
        assert!(config.emit_events);
    }

    #[test]
    fn test_health_monitor_config_custom() {
        let config = HealthMonitorConfig::new()
            .with_check_interval(Duration::from_secs(60))
            .with_degradation_threshold(5)
            .with_recovery_threshold(3)
            .with_history_window_size(20)
            .with_emit_events(false);

        assert_eq!(config.check_interval, Duration::from_secs(60));
        assert_eq!(config.degradation_threshold, 5);
        assert_eq!(config.recovery_threshold, 3);
        assert_eq!(config.history_window_size, 20);
        assert!(!config.emit_events);
    }

    #[test]
    fn test_driver_health_history_new() {
        let history = DriverHealthHistory::new("test-driver");
        assert_eq!(history.driver_id, "test-driver");
        assert!(history.history.is_empty());
        assert_eq!(history.current_status, SdkDriverStatus::Unknown);
        assert_eq!(history.consecutive_failures, 0);
        assert_eq!(history.consecutive_successes, 0);
    }

    #[test]
    fn test_record_health_healthy() {
        let mut history = DriverHealthHistory::new("test-driver");
        history.record_health(SdkDriverHealth::healthy(), 10);

        assert_eq!(history.history.len(), 1);
        assert_eq!(history.consecutive_failures, 0);
        assert_eq!(history.consecutive_successes, 1);
    }

    #[test]
    fn test_record_health_unhealthy() {
        let mut history = DriverHealthHistory::new("test-driver");
        history.record_health(SdkDriverHealth::unhealthy("test failure"), 10);

        assert_eq!(history.history.len(), 1);
        assert_eq!(history.consecutive_failures, 1);
        assert_eq!(history.consecutive_successes, 0);
    }

    #[test]
    fn test_should_degrade() {
        let mut history = DriverHealthHistory::new("test-driver");
        history.update_status(SdkDriverStatus::Healthy);

        // Record 2 failures (below threshold)
        for _ in 0..2 {
            history.record_health(SdkDriverHealth::unhealthy("test failure"), 10);
        }
        assert!(!history.should_degrade(3));

        // Record 3rd failure (at threshold)
        history.record_health(SdkDriverHealth::unhealthy("test failure"), 10);
        assert!(history.should_degrade(3));
    }

    #[test]
    fn test_should_recover() {
        let mut history = DriverHealthHistory::new("test-driver");
        history.update_status(SdkDriverStatus::Degraded);

        // Record 4 successes (below threshold)
        for _ in 0..4 {
            history.record_health(SdkDriverHealth::healthy(), 10);
        }
        assert!(!history.should_recover(5));

        // Record 5th success (at threshold)
        history.record_health(SdkDriverHealth::healthy(), 10);
        assert!(history.should_recover(5));
    }

    #[test]
    fn test_backend_health_monitor_register() {
        let mut monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
        monitor.register_driver("driver-1");
        monitor.register_driver("driver-2");

        assert_eq!(monitor.registered_drivers().len(), 2);
        assert!(monitor.driver_status("driver-1").is_some());
    }

    #[test]
    fn test_backend_health_monitor_unregister() {
        let mut monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
        monitor.register_driver("driver-1");
        monitor.unregister_driver("driver-1");

        assert_eq!(monitor.registered_drivers().len(), 0);
        assert!(monitor.driver_status("driver-1").is_none());
    }

    #[test]
    fn test_backend_health_monitor_auto_degrade() {
        let mut monitor = BackendHealthMonitor::new(
            HealthMonitorConfig::new().with_degradation_threshold(3),
        );
        monitor.register_driver("driver-1");

        // Start with healthy
        monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Healthy)
        );

        // Record 3 failures - should trigger auto-degradation
        for _ in 0..2 {
            monitor.record_driver_health("driver-1", SdkDriverHealth::unhealthy("test"));
        }
        // After 2 failures, still healthy
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Healthy)
        );

        // 3rd failure triggers degradation
        let change = monitor.record_driver_health("driver-1", SdkDriverHealth::unhealthy("test"));
        assert!(change.is_some());
        let change = change.unwrap();
        assert_eq!(change.new_status, SdkDriverStatus::Degraded);
        assert_eq!(change.previous_status, SdkDriverStatus::Healthy);

        // Check auto-degradation
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Degraded)
        );
    }

    #[test]
    fn test_backend_health_monitor_auto_recover() {
        let mut monitor = BackendHealthMonitor::new(
            HealthMonitorConfig::new()
                .with_degradation_threshold(3)
                .with_recovery_threshold(5),
        );
        monitor.register_driver("driver-1");

        // Start with healthy
        monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());

        // Degrade first
        for _ in 0..3 {
            monitor.record_driver_health("driver-1", SdkDriverHealth::unhealthy("test"));
        }
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Degraded)
        );

        // Record 4 successes - not enough for recovery
        for _ in 0..4 {
            monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());
        }
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Degraded)
        );

        // 5th success triggers recovery
        let change = monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());
        assert!(change.is_some());
        let change = change.unwrap();
        assert_eq!(change.new_status, SdkDriverStatus::Healthy);
        assert_eq!(change.previous_status, SdkDriverStatus::Degraded);

        // Check auto-recovery
        assert_eq!(
            monitor.driver_status("driver-1"),
            Some(SdkDriverStatus::Healthy)
        );
    }

    #[test]
    fn test_backend_health_monitor_aggregate() {
        let mut monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
        monitor.register_driver("driver-1");
        monitor.register_driver("driver-2");

        // Set driver-1 healthy
        monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());

        // Set driver-2 unhealthy
        monitor.record_driver_health("driver-2", SdkDriverHealth::unhealthy("test"));

        // Aggregate should be unhealthy
        assert_eq!(monitor.aggregate_health(), AggregateHealthStatus::Unhealthy);
    }

    #[test]
    fn test_backend_health_monitor_usable() {
        let mut monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
        monitor.register_driver("driver-1");

        // Healthy is usable
        monitor.record_driver_health("driver-1", SdkDriverHealth::healthy());
        assert!(monitor.is_driver_usable("driver-1"));

        // Degrade
        for _ in 0..3 {
            monitor.record_driver_health("driver-1", SdkDriverHealth::unhealthy("test"));
        }

        // Degraded is usable
        assert!(monitor.is_driver_usable("driver-1"));

        // Unhealthy is not usable
        monitor.drivers.get_mut("driver-1").unwrap().update_status(SdkDriverStatus::Unhealthy);
        assert!(!monitor.is_driver_usable("driver-1"));
    }

    #[test]
    fn test_success_rate() {
        let mut history = DriverHealthHistory::new("test-driver");

        // 7 healthy, 3 unhealthy
        for _ in 0..7 {
            history.record_health(SdkDriverHealth::healthy(), 10);
        }
        for _ in 0..3 {
            history.record_health(SdkDriverHealth::unhealthy("test"), 10);
        }

        assert_eq!(history.success_rate(), 0.7);
    }

    #[test]
    fn test_health_change_event() {
        let monitor = BackendHealthMonitor::new(HealthMonitorConfig::default());
        let change = HealthStatusChange {
            driver_id: "test-driver".to_string(),
            previous_status: SdkDriverStatus::Healthy,
            new_status: SdkDriverStatus::Degraded,
            reason: "Auto-degraded after 3 consecutive failures".to_string(),
            health: Some(SdkDriverHealth::unhealthy("test")),
        };

        let event = monitor.health_change_event(&change);
        assert_eq!(event.event_type, "agent.backend.health.degraded");
        assert_eq!(event.severity, KernelEventSeverity::Warn);
        assert_eq!(event.source, KernelEventSource::Runtime);
    }
}