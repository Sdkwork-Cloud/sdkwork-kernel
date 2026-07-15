use sdkwork_agent_provider_transport_ipc::{
    InMemoryJsonRpcTransport, JsonRpcTransport, SpawnedWorker, MAX_IPC_FRAME_BYTES,
    SDKWORK_PING_METHOD,
};
use serde_json::json;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn in_memory_transport_returns_stubbed_ping() {
    let transport = InMemoryJsonRpcTransport::new()
        .stub_method(SDKWORK_PING_METHOD, serde_json::json!({ "ok": true }));
    let result = transport
        .call(SDKWORK_PING_METHOD, None)
        .expect("ping should succeed");
    assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn stdio_transport_rejects_oversized_request_before_write() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args(["-e", "process.stdin.resume(); setInterval(() => {}, 1000);"]);
    let worker = SpawnedWorker::spawn(command).expect("spawn idle node worker");
    let error = worker
        .transport()
        .call(
            SDKWORK_PING_METHOD,
            Some(json!({"payload": "x".repeat(MAX_IPC_FRAME_BYTES)})),
        )
        .expect_err("oversized request must fail before worker write");
    assert!(error.message.contains("request exceeded frame byte limit"));
}

#[test]
fn stdio_transport_stops_reading_at_response_frame_limit() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});rl.on('line',line=>{const r=JSON.parse(line);process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{payload:'x'.repeat(8388608)}})+'\\n');});",
    ]);
    let worker = SpawnedWorker::spawn(command).expect("spawn oversized response worker");
    let error = worker
        .transport()
        .call(SDKWORK_PING_METHOD, None)
        .expect_err("oversized response must be rejected");
    assert!(error.message.contains("response exceeded frame byte limit"));
    assert!(
        !worker.is_reusable(),
        "protocol-violating worker must be poisoned"
    );
}

#[test]
fn worker_lease_timeout_terminates_and_reaps_unresponsive_process() {
    if !node_available() {
        return;
    }
    let pool = sdkwork_agent_provider_transport_ipc::SpawnedWorkerPool::new(1, move || {
        let mut command = Command::new("node");
        command.args(["-e", "process.stdin.resume(); setInterval(() => {}, 1000);"]);
        SpawnedWorker::spawn(command)
    })
    .expect("create worker pool");
    let lease = pool
        .acquire("request.timeout", Duration::from_secs(1))
        .expect("acquire worker");

    let started = Instant::now();
    let error = lease
        .call_with_timeout(SDKWORK_PING_METHOD, None, Duration::from_millis(100))
        .expect_err("unresponsive worker must time out");

    assert!(error.message.contains("timed out after 100 ms"));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!lease.is_running(), "timed-out worker must be reaped");
}
