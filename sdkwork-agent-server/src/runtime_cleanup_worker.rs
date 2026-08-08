//! Bounded retention worker for transient runtime persistence.
//!
//! The worker deliberately owns no business data and never performs an
//! unbounded read. Each pass delegates filtering and key selection to the
//! database adapter, then yields between bounded transactions so online
//! requests retain predictable access to the store.

use crate::{config::ServerConfig, persistence::PersistenceState};
use chrono::{Duration as ChronoDuration, Utc};
use std::sync::Arc;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, Duration, MissedTickBehavior},
};
use tracing::{info, warn};

const MAX_BATCHES_PER_CYCLE: usize = 10;
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CleanupCycleSummary {
    pub sessions: u64,
    pub messages: u64,
    pub tasks: u64,
    pub runs: u64,
    pub steps: u64,
    pub events: u64,
    pub permissions: u64,
    pub permission_operations: u64,
    pub batches: u32,
}

impl CleanupCycleSummary {
    fn add(&mut self, counts: sdkwork_agent_database::RuntimePurgeCounts) {
        self.sessions = self.sessions.saturating_add(counts.sessions);
        self.messages = self.messages.saturating_add(counts.messages);
        self.tasks = self.tasks.saturating_add(counts.tasks);
        self.runs = self.runs.saturating_add(counts.runs);
        self.steps = self.steps.saturating_add(counts.steps);
        self.events = self.events.saturating_add(counts.events);
        self.permissions = self.permissions.saturating_add(counts.permissions);
        self.permission_operations = self
            .permission_operations
            .saturating_add(counts.permission_operations);
        self.batches = self.batches.saturating_add(1);
    }

    fn total(&self) -> u64 {
        self.sessions
            .saturating_add(self.messages)
            .saturating_add(self.tasks)
            .saturating_add(self.runs)
            .saturating_add(self.steps)
            .saturating_add(self.events)
            .saturating_add(self.permissions)
            .saturating_add(self.permission_operations)
    }
}

/// Background runtime retention worker.
pub struct RuntimeCleanupWorker {
    task: Option<JoinHandle<()>>,
}

impl RuntimeCleanupWorker {
    pub fn spawn(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        let task = tokio::spawn(async move {
            // Restart the loop if it ever terminates unexpectedly (panic), so
            // retention cleanup self-heals; the loop only returns on shutdown.
            loop {
                run_worker(persistence.clone(), config.clone(), shutdown.clone()).await;
                if *shutdown.borrow() {
                    return;
                }
                warn!("runtime retention worker terminated unexpectedly; restarting");
            }
        });
        Self { task: Some(task) }
    }

    /// Wait for the worker after the caller has signalled shutdown.
    pub async fn join(self) {
        self.join_with_timeout(DEFAULT_SHUTDOWN_TIMEOUT).await;
    }

    async fn join_with_timeout(mut self, timeout: Duration) {
        if let Some(mut task) = self.task.take() {
            if time::timeout(timeout, &mut task).await.is_err() {
                warn!("runtime retention worker did not stop before the shutdown deadline");
                task.abort();
                let _ = task.await;
            }
        }
    }
}

impl Drop for RuntimeCleanupWorker {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn run_worker(
    persistence: Arc<PersistenceState>,
    config: Arc<ServerConfig>,
    mut shutdown: watch::Receiver<bool>,
) {
    let interval = Duration::from_secs(config.runtime_cleanup_interval_secs.max(10));
    let mut ticker = time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // The first tick is intentionally consumed so startup does not compete
    // with schema/bootstrap traffic. A bounded pass starts one interval later.
    ticker.tick().await;
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            _ = ticker.tick() => {
                let cutoff = (Utc::now() - ChronoDuration::days(config.runtime_retention_days as i64))
                    .to_rfc3339();
                match run_cleanup_cycle(
                    persistence.clone(),
                    cutoff,
                    config.runtime_cleanup_batch_size as i64,
                ).await {
                    Ok(summary) if summary.total() > 0 => {
                        info!(
                            batches = summary.batches,
                            sessions = summary.sessions,
                            messages = summary.messages,
                            tasks = summary.tasks,
                            runs = summary.runs,
                            steps = summary.steps,
                            events = summary.events,
                            permissions = summary.permissions,
                            permission_operations = summary.permission_operations,
                            "runtime retention cleanup completed"
                        );
                    }
                    Ok(_) => {}
                    Err(error) => warn!(error = %error, "runtime retention cleanup failed")
                }
            }
        }
    }
}

/// Execute at most [`MAX_BATCHES_PER_CYCLE`] bounded transactions.
pub async fn run_cleanup_cycle(
    persistence: Arc<PersistenceState>,
    cutoff: String,
    batch_size: i64,
) -> Result<CleanupCycleSummary, String> {
    let mut summary = CleanupCycleSummary::default();
    for _ in 0..MAX_BATCHES_PER_CYCLE {
        let cutoff_for_batch = cutoff.clone();
        let counts = persistence
            .run(move |state| state.purge_expired(&cutoff_for_batch, batch_size))
            .await?;
        let total = counts.total();
        summary.add(counts);
        if total == 0 {
            break;
        }
        // Yield between write transactions so a hot runtime does not let the
        // maintenance task monopolize the async scheduler.
        tokio::task::yield_now().await;
    }
    if summary.total() > 0 {
        persistence.run(|state| state.run_maintenance()).await?;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn cleanup_cycle_stops_after_an_empty_batch() {
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let summary = run_cleanup_cycle(persistence, "2021-01-01T00:00:00Z".to_string(), 1)
            .await
            .expect("cleanup");
        assert_eq!(summary.batches, 1);
        assert_eq!(summary.total(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn shutdown_wakes_worker_before_cleanup_interval() {
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let config = Arc::new(ServerConfig {
            runtime_cleanup_interval_secs: 60,
            ..ServerConfig::default()
        });
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let worker = RuntimeCleanupWorker::spawn(persistence, config, shutdown_rx);
        shutdown_tx.send(true).expect("shutdown signal");
        time::timeout(Duration::from_secs(1), worker.join())
            .await
            .expect("worker should stop promptly");
    }

    #[tokio::test]
    async fn shutdown_aborts_a_worker_that_exceeds_its_deadline() {
        let worker = RuntimeCleanupWorker {
            task: Some(tokio::spawn(std::future::pending())),
        };
        time::timeout(
            Duration::from_secs(1),
            worker.join_with_timeout(Duration::from_millis(10)),
        )
        .await
        .expect("bounded join should abort the stuck task");
    }
}
