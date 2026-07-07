//! Background worker that polls typed provider health into [`BackendHealthMonitor`].
//!
//! Spawned during agent-server bootstrap so backend selection can degrade and
//! recover from stale provider health instead of relying on one-shot checks.

use sdkwork_agent_kernel::{
    AgentRuntime, BackendHealthMonitor, HealthMonitorConfig, SdkDriverHealth,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;

/// Owns a [`BackendHealthMonitor`] and a background thread that refreshes driver health.
#[derive(Clone)]
pub struct BackendHealthWorker {
    monitor: Arc<RwLock<BackendHealthMonitor>>,
    stop: Arc<AtomicBool>,
    wake: Arc<(Mutex<()>, Condvar)>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl BackendHealthWorker {
    pub fn spawn(agent_runtime: Arc<AgentRuntime>, config: HealthMonitorConfig) -> Self {
        let mut monitor = BackendHealthMonitor::new(config.clone());
        for diagnostic in agent_runtime.diagnostics().provider_diagnostics {
            monitor.register_driver(&diagnostic.provider_id);
        }

        let monitor = Arc::new(RwLock::new(monitor));
        let stop = Arc::new(AtomicBool::new(false));
        let wake = Arc::new((Mutex::new(()), Condvar::new()));
        let monitor_for_thread = monitor.clone();
        let stop_for_thread = stop.clone();
        let wake_for_thread = wake.clone();
        let interval = config.check_interval;

        let thread = std::thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                let (wake_lock, wake_signal) = &*wake_for_thread;
                let Ok(wake_guard) = wake_lock.lock() else {
                    break;
                };
                let Ok((wake_guard, _timeout)) =
                    wake_signal.wait_timeout_while(wake_guard, interval, |_| {
                        !stop_for_thread.load(Ordering::Relaxed)
                    })
                else {
                    break;
                };
                drop(wake_guard);
                if stop_for_thread.load(Ordering::Relaxed) {
                    break;
                }

                let Ok(mut guard) = monitor_for_thread.write() else {
                    continue;
                };
                if !guard.should_check() {
                    continue;
                }
                guard.mark_check();
                let diagnostics = agent_runtime.diagnostics();
                for diagnostic in diagnostics.provider_diagnostics {
                    let Some(health) = diagnostic.health.as_ref() else {
                        continue;
                    };
                    let driver_health = SdkDriverHealth::from_provider_health(health);
                    let _ = guard.record_driver_health(&diagnostic.provider_id, driver_health);
                }
            }
        });

        Self {
            monitor,
            stop,
            wake,
            thread: Arc::new(Mutex::new(Some(thread))),
        }
    }

    pub fn spawn_default(agent_runtime: Arc<AgentRuntime>) -> Self {
        Self::spawn(agent_runtime, HealthMonitorConfig::default())
    }

    pub fn monitor(&self) -> Arc<RwLock<BackendHealthMonitor>> {
        self.monitor.clone()
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let (_wake_lock, wake_signal) = &*self.wake;
        wake_signal.notify_all();
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(thread) = guard.take() {
                let _ = thread.join();
            }
        }
    }
}

impl std::fmt::Debug for BackendHealthWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let driver_count = self
            .monitor
            .read()
            .map(|monitor| monitor.registered_drivers().len())
            .unwrap_or(0);
        f.debug_struct("BackendHealthWorker")
            .field("registered_driver_count", &driver_count)
            .finish()
    }
}

impl Drop for BackendHealthWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::RuntimeState;
    use std::time::Duration;

    #[test]
    fn worker_registers_providers_from_runtime_diagnostics() {
        let runtime =
            Arc::new(crate::runtime_bootstrap::bootstrap_agent_runtime().expect("bootstrap"));
        assert_ne!(runtime.state(), RuntimeState::Failed);

        let worker = BackendHealthWorker::spawn_default(runtime);
        std::thread::sleep(Duration::from_millis(50));

        let monitor = worker.monitor();
        let guard = monitor.read().expect("monitor lock");
        let registered = guard.registered_drivers();
        assert!(!registered.is_empty());
    }

    #[test]
    fn shutdown_wakes_sleeping_worker_without_waiting_for_check_interval() {
        let runtime =
            Arc::new(crate::runtime_bootstrap::bootstrap_agent_runtime().expect("bootstrap"));
        let worker = BackendHealthWorker::spawn(
            runtime,
            HealthMonitorConfig::default().with_check_interval(Duration::from_secs(5)),
        );
        std::thread::sleep(Duration::from_millis(100));

        let started = std::time::Instant::now();
        worker.shutdown();

        assert!(
            started.elapsed() < Duration::from_millis(500),
            "backend health worker shutdown must not block until the next check interval"
        );
    }
}
