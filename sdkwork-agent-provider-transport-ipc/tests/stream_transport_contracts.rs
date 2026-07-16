use sdkwork_agent_provider_transport_ipc::{
    expand_buffered_stream_payload, is_stream_chunk_frame, is_stream_terminal_frame,
    stream_chunk_frame, stream_done_frame, stream_done_frame_with_completion, JsonRpcTransport,
    PackageStubJsonRpcTransport, MAX_STREAM_BUFFER_CHUNKS, MAX_STREAM_CHUNK_BYTES,
    MAX_STREAM_TOTAL_BYTES, SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_STREAM_EVENT_CHUNK,
    SDKWORK_STREAM_EVENT_DONE,
};
use serde_json::json;
use std::sync::{Mutex, OnceLock};

const KERNEL_PROFILE_ID_ENV: &str = "SDKWORK_KERNEL_PROFILE_ID";
const KERNEL_ENVIRONMENT_ENV: &str = "SDKWORK_KERNEL_ENVIRONMENT";
const ALLOW_MOCK_PROVIDERS_ENV: &str = "SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS";

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let previous = std::env::var(key).ok();
        match value {
            Some(next) => std::env::set_var(key, next),
            None => std::env::remove_var(key),
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn expand_buffered_stream_payload_emits_chunk_and_done_frames() {
    let payload = json!({
        "ok": true,
        "chunks": [
            { "sequence": 0, "content": "one " },
            { "sequence": 1, "content": "two " }
        ],
        "finish_reason": "stop",
        "model_request_id": "req.1",
        "native_session_id": "thread-1"
    });
    let mut frames = Vec::new();
    expand_buffered_stream_payload(payload, |frame| {
        frames.push(frame);
        Ok(true)
    })
    .expect("expand should succeed");
    assert_eq!(frames.len(), 3);
    assert!(is_stream_chunk_frame(&frames[0]));
    assert!(is_stream_chunk_frame(&frames[1]));
    assert!(is_stream_terminal_frame(&frames[2]));
    assert_eq!(
        frames[2]
            .get("model_request_id")
            .and_then(|value| value.as_str()),
        Some("req.1")
    );
    assert_eq!(
        frames[2]
            .get("native_session_id")
            .and_then(|value| value.as_str()),
        Some("thread-1")
    );
}

#[test]
fn package_stub_streaming_delivers_incremental_frames() {
    let _lock = env_lock();
    let _profile = EnvVarGuard::set(KERNEL_PROFILE_ID_ENV, None);
    let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("development"));
    let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, Some("1"));

    let transport = PackageStubJsonRpcTransport::new("openclaw", "typescript_node");
    let mut frames = Vec::new();
    transport
        .call_streaming(
            SDKWORK_CAPABILITY_INVOKE_METHOD,
            Some(json!({
                "operation": {
                    "operation": "model_chat_stream",
                    "messages": ["hello world"],
                    "model_request_id": "req.stub"
                }
            })),
            &mut |frame| {
                frames.push(frame);
                Ok(true)
            },
        )
        .expect("streaming invoke should succeed");
    assert!(frames.len() >= 3);
    assert!(frames.iter().any(|frame| {
        frame.get("event").and_then(|value| value.as_str()) == Some(SDKWORK_STREAM_EVENT_CHUNK)
    }));
    assert!(frames.iter().any(|frame| {
        frame.get("event").and_then(|value| value.as_str()) == Some(SDKWORK_STREAM_EVENT_DONE)
    }));
}

#[test]
fn stream_frame_helpers_round_trip() {
    let chunk = stream_chunk_frame(2, "delta", Some("req.2"));
    assert!(is_stream_chunk_frame(&chunk));
    let done = stream_done_frame("stop");
    assert!(is_stream_terminal_frame(&done));

    let completion = stream_done_frame_with_completion(
        "stop",
        Some("req.completion"),
        Some("thread-completion"),
    );
    assert!(is_stream_terminal_frame(&completion));
    assert_eq!(
        completion
            .get("model_request_id")
            .and_then(|value| value.as_str()),
        Some("req.completion")
    );
    assert_eq!(
        completion
            .get("native_session_id")
            .and_then(|value| value.as_str()),
        Some("thread-completion")
    );
}

#[test]
fn failed_stream_payload_is_not_expanded_into_done_frame() {
    let payload = json!({
        "ok": false,
        "error": "provider failed after emitting chunks",
        "model_request_id": "req.failed"
    });
    let mut frames = Vec::new();

    let error = expand_buffered_stream_payload(payload, |frame| {
        frames.push(frame);
        Ok(true)
    })
    .expect_err("failed provider payload must remain a transport error");

    assert_eq!(error.message, "provider failed after emitting chunks");
    assert!(frames.is_empty());
}

#[test]
fn stream_limits_reject_too_many_chunks() {
    let payload = json!({
        "ok": true,
        "chunks": vec![json!({"content": "x"}); MAX_STREAM_BUFFER_CHUNKS + 1]
    });
    let error = expand_buffered_stream_payload(payload, |_frame| Ok(true))
        .expect_err("chunk count must be bounded");
    assert!(error.message.contains("chunk limit"));
}

#[test]
fn stream_limits_reject_one_oversized_chunk() {
    let payload = json!({
        "ok": true,
        "chunks": [{"content": "x".repeat(MAX_STREAM_CHUNK_BYTES + 1)}]
    });
    let error = expand_buffered_stream_payload(payload, |_frame| Ok(true))
        .expect_err("chunk bytes must be bounded");
    assert!(error.message.contains("byte limit"));
}

#[test]
fn stream_limits_reject_aggregate_bytes() {
    let chunk = "x".repeat(MAX_STREAM_CHUNK_BYTES);
    let count = (MAX_STREAM_TOTAL_BYTES / MAX_STREAM_CHUNK_BYTES) + 1;
    let payload = json!({
        "ok": true,
        "chunks": (0..count)
            .map(|_| json!({"content": chunk}))
            .collect::<Vec<_>>()
    });
    let error = expand_buffered_stream_payload(payload, |_frame| Ok(true))
        .expect_err("aggregate stream bytes must be bounded");
    assert!(error.message.contains("total byte limit"));
}
