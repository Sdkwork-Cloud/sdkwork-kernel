use sdkwork_agent_provider_transport_ipc::{SpawnedWorker, SpawnedWorkerPool};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn spawn_idle_worker() -> Result<SpawnedWorker, sdkwork_agent_provider_transport_ipc::TransportError>
{
    let mut command = Command::new("node");
    command.args(["-e", "process.stdin.resume(); setInterval(() => {}, 1000);"]);
    SpawnedWorker::spawn(command)
}

#[test]
fn worker_pool_cancels_only_the_requested_worker() {
    if !node_available() {
        return;
    }
    let pool = SpawnedWorkerPool::new(2, spawn_idle_worker).expect("pool should be created");
    let first = pool
        .acquire("request-one", Duration::from_secs(1))
        .expect("first lease");
    let second = pool
        .acquire("request-two", Duration::from_secs(1))
        .expect("second lease");
    assert!(first.is_running());
    assert!(second.is_running());

    assert!(pool.cancel("request-one").expect("cancel should succeed"));
    assert!(!first.is_running());
    assert!(second.is_running(), "other request must remain alive");
    assert!(!pool
        .cancel("request-one")
        .expect("repeat cancel is idempotent"));
}

#[test]
fn worker_pool_enforces_concurrency_bound_and_releases_cancelled_slot() {
    if !node_available() {
        return;
    }
    let pool =
        Arc::new(SpawnedWorkerPool::new(1, spawn_idle_worker).expect("pool should be created"));
    let lease = pool
        .acquire("request-one", Duration::from_secs(1))
        .expect("first lease");
    let blocked = pool.acquire("request-two", Duration::from_millis(10));
    assert!(
        blocked.is_err(),
        "pool must not exceed configured concurrency"
    );
    assert!(pool.cancel("request-one").expect("cancel should succeed"));
    let replacement = pool
        .acquire("request-two", Duration::from_secs(1))
        .expect("cancelled slot should be released");
    assert!(replacement.is_running());
    drop(lease);
}
