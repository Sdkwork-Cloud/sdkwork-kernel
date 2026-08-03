use std::time::Instant;

use sdkwork_agent_kernel::ModelProvider;
use sdkwork_agent_provider_codex::CodexSdkIntegration;
use sdkwork_agent_provider_spi::{SdkRuntimeRequest, SDK_CAPABILITY_MODEL_CHAT};

const LIVE_CWD_ENV: &str = "SDKWORK_LIVE_PROVIDER_SESSION_CWD";

/// Live smoke test: send one real message through the in-process Codex
/// app-server and verify the assistant reply comes back with a correlated
/// provider session identity.
///
/// Requires the locally installed Codex app-server runtime (codex executable
/// on PATH or `SDKWORK_KERNEL_CODEX_EXECUTABLE`) and a working directory that
/// is a git repository. Creates a real provider thread in the local Codex
/// state database.
#[tokio::test]
#[ignore = "requires the locally installed Codex app-server and a live model call"]
async fn live_send_message_and_stream_reply() {
    let cwd = std::env::var(LIVE_CWD_ENV)
        .unwrap_or_else(|_| panic!("{LIVE_CWD_ENV} must identify the project to verify"));
    let integration = CodexSdkIntegration::bootstrap().expect("bootstrap Codex app-server");
    eprintln!("codex_live_send_phase=bootstrap_complete");

    let model_id = integration
        .model
        .list_models()
        .into_iter()
        .next()
        .map(|descriptor| descriptor.model_id)
        .expect("codex app-server publishes a model");

    let request_id = format!("live-send-{}", sdkwork_utils_rust::uuid());
    let request = SdkRuntimeRequest::model_chat_with_context(
        SDK_CAPABILITY_MODEL_CHAT,
        request_id.clone(),
        vec!["Reply with exactly one word: OK".to_string()],
        None,
        Some(model_id.clone()),
        None,
        Some(cwd.clone()),
        Some(120_000),
        None,
    );

    let started = Instant::now();
    eprintln!("codex_live_send_phase=invoke_start model={model_id}");
    let response = integration
        .invoke_runtime(&request)
        .expect("live model invoke succeeds");
    eprintln!(
        "codex_live_send_phase=invoke_complete elapsed_ms={} success={}",
        started.elapsed().as_millis(),
        response.success
    );
    eprintln!(
        "codex_live_send_phase=invoke_payload payload={}",
        response
            .payload
            .as_ref()
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "null".to_string())
    );
    assert!(response.success, "live invoke must succeed: {response:?}");
    let payload = response.payload.expect("live invoke payload");
    let provider_session_id = payload
        .get("provider_session_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            payload
                .get("diagnostics")
                .and_then(|value| value.as_array())
                .and_then(|diagnostics| {
                    diagnostics.iter().find_map(|diagnostic| {
                        let diagnostic = diagnostic.as_str()?;
                        let (key, value) = diagnostic.split_once('=')?;
                        (key.trim() == "sdk_runtime_provider_session_id")
                            .then(|| value.trim().to_string())
                    })
                })
                .or_else(|| {
                    payload
                        .get("diagnostics")
                        .and_then(|value| value.as_array())
                        .and_then(|diagnostics| {
                            diagnostics.iter().find_map(|diagnostic| {
                                let diagnostic = diagnostic.as_str()?;
                                let (key, value) = diagnostic.split_once('=')?;
                                (key.trim() == "sdk_runtime_session_id")
                                    .then(|| value.trim().to_string())
                            })
                        })
                })
        })
        .expect("live invoke returns a provider session id");
    let messages = payload
        .get("messages")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    eprintln!(
        "codex_live_send_phase=reply provider_session_id={provider_session_id} messages={:?}",
        messages
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        messages
            .iter()
            .any(|message| message.as_str().is_some_and(|text| !text.trim().is_empty())),
        "live invoke must return an assistant reply"
    );

    // Second turn in the same provider session: resume must succeed and reply.
    let resumed_request_id = format!("live-resume-{}", sdkwork_utils_rust::uuid());
    let resumed_request = SdkRuntimeRequest::model_chat_with_session_identities(
        SDK_CAPABILITY_MODEL_CHAT,
        resumed_request_id.clone(),
        vec!["Reply with exactly one word: DONE".to_string()],
        None,
        Some(model_id.clone()),
        None,
        Some(provider_session_id.clone()),
        Some(cwd.clone()),
        Some(120_000),
        None,
    );
    let started = Instant::now();
    eprintln!("codex_live_send_phase=resume_start");
    let resumed_response = integration
        .invoke_runtime(&resumed_request)
        .expect("live resumed invoke succeeds");
    eprintln!(
        "codex_live_send_phase=resume_complete elapsed_ms={} success={}",
        started.elapsed().as_millis(),
        resumed_response.success
    );
    assert!(resumed_response.success, "live resume must succeed");
    eprintln!(
        "codex_live_send_phase=resume_payload payload={}",
        resumed_response
            .payload
            .as_ref()
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "null".to_string())
    );

    // Streaming path: verify incremental chunks arrive for the same thread.
    let stream_request_id = format!("live-stream-{}", sdkwork_utils_rust::uuid());
    let stream_request = SdkRuntimeRequest::model_chat_stream_with_session_identities(
        SDK_CAPABILITY_MODEL_CHAT,
        stream_request_id.clone(),
        vec!["Reply with exactly two words: STREAM WORKS".to_string()],
        None,
        Some(model_id.clone()),
        None,
        Some(provider_session_id.clone()),
        Some(cwd.clone()),
        Some(120_000),
        None,
    );
    let mut stream_chunks = Vec::new();
    let mut stream_frames = Vec::new();
    let started = Instant::now();
    eprintln!("codex_live_send_phase=stream_start");
    integration
        .runtime
        .invoke_streaming(&stream_request, &mut |frame| {
            stream_frames.push(frame.clone());
            if frame.get("event").and_then(|value| value.as_str()) == Some("stream.chunk") {
                if let Some(content) = frame.get("content").and_then(|value| value.as_str()) {
                    stream_chunks.push(content.to_string());
                }
            }
            Ok(true)
        })
        .expect("live stream invoke succeeds");
    eprintln!(
        "codex_live_send_phase=stream_complete elapsed_ms={} frames={} chunks={} text={:?}",
        started.elapsed().as_millis(),
        stream_frames.len(),
        stream_chunks.len(),
        stream_chunks.join("")
    );
    assert!(
        !stream_chunks.is_empty(),
        "live streaming must deliver incremental chunks; frames={stream_frames:?}"
    );
    let joined = stream_chunks.join("");
    assert!(
        joined.trim().to_ascii_lowercase().contains("stream"),
        "streamed reply must contain the expected text; got: {joined:?}"
    );

    println!(
        "codex_live_send_phase=parity_ok provider_session_id={provider_session_id} messages={} stream_chunks={}",
        messages.len(),
        stream_chunks.len()
    );
}
