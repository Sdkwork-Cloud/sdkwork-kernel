use sdkwork_agent_provider_transport_ipc::{
    is_stream_chunk_frame, is_stream_kernel_event_frame, is_stream_terminal_frame,
    InMemoryJsonRpcTransport, JsonRpcTransport, SpawnedWorker, MAX_IPC_FRAME_BYTES,
    SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_PING_METHOD,
};
use serde_json::json;
use std::process::Command;
use std::sync::{mpsc, Arc};
use std::thread;
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
fn stdio_streaming_preserves_kernel_events_before_chunks_and_completion() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});rl.on('line',line=>{const r=JSON.parse(line);for(const result of [{event:'stream.event',model_request_id:'req.rich',kernel_event:{event_id:'event.req.rich.0'}},{event:'stream.chunk',sequence:0,content:'hello',model_request_id:'req.rich'},{event:'stream.done',finish_reason:'stop',model_request_id:'req.rich'}])process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result})+'\\n');});",
    ]);
    let worker = SpawnedWorker::spawn(command).expect("spawn rich stream worker");
    let mut frames = Vec::new();

    worker
        .transport()
        .call_streaming(
            SDKWORK_CAPABILITY_INVOKE_METHOD,
            Some(json!({
                "operation": {
                    "operation": "model_chat_stream",
                    "model_request_id": "req.rich",
                    "messages": ["hello"]
                }
            })),
            &mut |frame| {
                frames.push(frame);
                Ok(true)
            },
        )
        .expect("rich stream frames should remain ordered");

    assert_eq!(frames.len(), 3);
    assert!(is_stream_kernel_event_frame(&frames[0]));
    assert!(is_stream_chunk_frame(&frames[1]));
    assert!(is_stream_terminal_frame(&frames[2]));
    assert_eq!(
        frames[0]
            .get("model_request_id")
            .and_then(|value| value.as_str()),
        Some("req.rich")
    );
    assert!(worker.is_reusable());
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

#[test]
fn stdio_transport_allows_unary_control_during_active_stream() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});let stream=null;rl.on('line',line=>{const r=JSON.parse(line);if(r.method==='stream'){stream=r;process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{event:'stream.event',kernel_event:{event_id:'paused'}}})+'\\n');return;}if(r.method==='control'){process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{ok:true,request:r.params.request}})+'\\n');process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:stream.id,result:{event:'stream.done',finish_reason:'stop'}})+'\\n');}});",
    ]);
    let worker = Arc::new(SpawnedWorker::spawn(command).expect("spawn duplex worker"));
    let stream_transport = worker.transport();
    let (frame_tx, frame_rx) = mpsc::channel();
    let stream = thread::spawn(move || {
        stream_transport.call_streaming("stream", None, &mut |frame| {
            frame_tx.send(frame).expect("send observed frame");
            Ok(true)
        })
    });

    let paused = frame_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("stream should pause before control response");
    assert!(is_stream_kernel_event_frame(&paused));

    let control = worker
        .transport()
        .call("control", Some(json!({"request": "approval-1"})))
        .expect("unary control should complete while stream remains active");
    assert_eq!(control.get("ok"), Some(&json!(true)));
    assert_eq!(control.get("request"), Some(&json!("approval-1")));

    stream
        .join()
        .expect("stream thread should not panic")
        .expect("stream should finish after control response");
    assert!(worker.is_reusable());
}

#[test]
fn stdio_transport_demultiplexes_reversed_unary_responses() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});const requests=[];rl.on('line',line=>{requests.push(JSON.parse(line));if(requests.length===2){for(const r of [...requests].reverse())process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{method:r.method}})+'\\n');}});",
    ]);
    let worker = Arc::new(SpawnedWorker::spawn(command).expect("spawn reverse-order worker"));
    let first_worker = worker.clone();
    let first = thread::spawn(move || first_worker.transport().call("first", None));
    let second_worker = worker.clone();
    let second = thread::spawn(move || second_worker.transport().call("second", None));

    let first = first
        .join()
        .expect("first call thread")
        .expect("first call");
    let second = second
        .join()
        .expect("second call thread")
        .expect("second call");
    assert_eq!(first.get("method"), Some(&json!("first")));
    assert_eq!(second.get("method"), Some(&json!("second")));
    assert!(worker.is_reusable());
}

#[test]
fn stdio_transport_poisoned_by_unknown_response_id_fails_pending_call() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});rl.on('line',()=>process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:'unknown',result:{ok:true}})+'\\n'));",
    ]);
    let worker = SpawnedWorker::spawn(command).expect("spawn mismatched-id worker");
    let error = worker
        .transport()
        .call("probe", None)
        .expect_err("unknown response id must poison the session");
    assert!(error.message.contains("unknown or duplicate response id"));
    assert!(!worker.is_reusable());
}

#[test]
fn stdio_transport_disconnect_fails_all_active_calls() {
    if !node_available() {
        return;
    }
    let mut command = Command::new("node");
    command.args([
        "-e",
        "const rl=require('readline').createInterface({input:process.stdin});let count=0;rl.on('line',()=>{count+=1;if(count===2)process.exit(0);});",
    ]);
    let worker = Arc::new(SpawnedWorker::spawn(command).expect("spawn disconnecting worker"));
    let first_worker = worker.clone();
    let first = thread::spawn(move || first_worker.transport().call("first", None));
    let second_worker = worker.clone();
    let second = thread::spawn(move || second_worker.transport().call("second", None));

    let first_error = first
        .join()
        .expect("first call thread")
        .expect_err("first call must fail on disconnect");
    let second_error = second
        .join()
        .expect("second call thread")
        .expect_err("second call must fail on disconnect");
    assert!(first_error.message.contains("closed stdout"));
    assert!(second_error.message.contains("closed stdout"));
    assert!(!worker.is_reusable());
}
