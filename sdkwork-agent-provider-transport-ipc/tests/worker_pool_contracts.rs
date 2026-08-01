use sdkwork_agent_provider_transport_ipc::{JsonRpcTransport, SpawnedWorker, SpawnedWorkerPool};
use serde_json::json;
use std::process::Command;
use std::sync::{mpsc, Arc};
use std::thread;
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

#[test]
fn worker_pool_controls_only_the_worker_for_the_exact_active_request() {
    if !node_available() {
        return;
    }
    let pool = SpawnedWorkerPool::new(1, || {
        let mut command = Command::new("node");
        command.args([
            "-e",
            "const rl=require('readline').createInterface({input:process.stdin});let stream=null;rl.on('line',line=>{const r=JSON.parse(line);if(r.method==='stream'){stream=r;process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{event:'stream.event',kernel_event:{event_id:'paused'}}})+'\\n');return;}if(r.method==='control'){process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{ok:true,model_request_id:r.params.model_request_id}})+'\\n');process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:stream.id,result:{event:'stream.done',finish_reason:'stop'}})+'\\n');}});",
        ]);
        SpawnedWorker::spawn(command)
    })
    .expect("pool should be created");
    let lease = pool
        .acquire("model-request-1", Duration::from_secs(1))
        .expect("active request lease");
    let (frame_tx, frame_rx) = mpsc::channel();
    let stream = thread::spawn(move || {
        lease
            .transport()
            .call_streaming("stream", None, &mut |frame| {
                frame_tx.send(frame).expect("send stream frame");
                Ok(true)
            })
    });
    frame_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("stream should reach paused state");

    let missing = pool
        .control(
            "model-request-stale",
            "control",
            Some(json!({"model_request_id": "model-request-stale"})),
            Duration::from_secs(1),
        )
        .expect_err("stale request must not acquire another worker");
    assert!(missing
        .message
        .contains("no active provider worker for request"));

    let response = pool
        .control(
            "model-request-1",
            "control",
            Some(json!({"model_request_id": "model-request-1"})),
            Duration::from_secs(1),
        )
        .expect("control should reach the exact active worker");
    assert_eq!(
        response.get("model_request_id"),
        Some(&json!("model-request-1"))
    );
    stream
        .join()
        .expect("stream thread should not panic")
        .expect("stream should continue after control");
}
