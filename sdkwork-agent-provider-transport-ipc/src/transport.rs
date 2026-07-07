use crate::protocol::{
    is_stream_chunk_frame, is_stream_terminal_frame, stream_chunk_frame, stream_done_frame,
    JsonRpcRequest, JsonRpcResponse, SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_PING_METHOD,
};
use sdkwork_agent_kernel::mock_provider_invocation_allowed_from_env;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_STREAM_BUFFER_CHUNKS: usize = 4096;
const MAX_STREAM_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    pub message: String,
}

impl TransportError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TransportError {}

pub trait JsonRpcTransport {
    fn call(&self, method: &str, params: Option<Value>) -> Result<Value, TransportError>;

    /// Delivers one or more stream frames for capability operations that support
    /// incremental worker output. Default implementation expands a single buffered
    /// `call` response into chunk/done frames.
    fn call_streaming(
        &self,
        method: &str,
        params: Option<Value>,
        sink: &mut dyn FnMut(Value) -> Result<bool, TransportError>,
    ) -> Result<(), TransportError> {
        let result = self.call(method, params)?;
        expand_buffered_stream_payload(result, |frame| sink(frame))
    }
}

/// Expands a buffered stream payload or passes through explicit stream frames.
pub fn expand_buffered_stream_payload<F>(
    payload: Value,
    mut on_frame: F,
) -> Result<(), TransportError>
where
    F: FnMut(Value) -> Result<bool, TransportError>,
{
    if is_stream_chunk_frame(&payload) {
        return if on_frame(payload)? { Ok(()) } else { Ok(()) };
    }
    if is_stream_terminal_frame(&payload) {
        on_frame(payload)?;
        return Ok(());
    }
    if payload.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(TransportError::new(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("stream invoke failed"),
        ));
    }
    if let Some(chunks) = payload.get("chunks").and_then(Value::as_array) {
        let model_request_id = payload.get("model_request_id").and_then(Value::as_str);
        for (index, entry) in chunks.iter().enumerate() {
            if index >= MAX_STREAM_BUFFER_CHUNKS {
                return Err(TransportError::new(format!(
                    "worker stream exceeded chunk limit ({MAX_STREAM_BUFFER_CHUNKS})"
                )));
            }
            let content = entry
                .get("content")
                .or_else(|| entry.get("delta"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if content.len() > MAX_STREAM_CHUNK_BYTES {
                return Err(TransportError::new(format!(
                    "worker stream chunk exceeded byte limit ({MAX_STREAM_CHUNK_BYTES})"
                )));
            }
            let sequence = entry
                .get("sequence")
                .and_then(Value::as_u64)
                .unwrap_or(index as u64);
            let frame = stream_chunk_frame(sequence, content, model_request_id);
            if !on_frame(frame)? {
                return Ok(());
            }
        }
        let finish_reason = payload
            .get("finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("stop");
        on_frame(stream_done_frame(finish_reason))?;
        return Ok(());
    }
    if let Some(messages) = payload.get("messages").and_then(Value::as_array) {
        let model_request_id = payload.get("model_request_id").and_then(Value::as_str);
        for (index, entry) in messages.iter().enumerate() {
            if index >= MAX_STREAM_BUFFER_CHUNKS {
                return Err(TransportError::new(format!(
                    "worker stream exceeded chunk limit ({MAX_STREAM_BUFFER_CHUNKS})"
                )));
            }
            let Some(content) = entry.as_str() else {
                continue;
            };
            if content.len() > MAX_STREAM_CHUNK_BYTES {
                return Err(TransportError::new(format!(
                    "worker stream chunk exceeded byte limit ({MAX_STREAM_CHUNK_BYTES})"
                )));
            }
            let frame = stream_chunk_frame(index as u64, content, model_request_id);
            if !on_frame(frame)? {
                return Ok(());
            }
        }
        on_frame(stream_done_frame("stop"))?;
        return Ok(());
    }
    if !on_frame(payload)? {
        return Ok(());
    }
    on_frame(stream_done_frame("stop"))?;
    Ok(())
}

pub struct InMemoryJsonRpcTransport {
    responses: HashMap<String, Value>,
}

impl InMemoryJsonRpcTransport {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    pub fn stub_method(mut self, method: &str, result: Value) -> Self {
        self.responses.insert(method.to_string(), result);
        self
    }
}

impl Default for InMemoryJsonRpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonRpcTransport for InMemoryJsonRpcTransport {
    fn call(&self, method: &str, _params: Option<Value>) -> Result<Value, TransportError> {
        self.responses
            .get(method)
            .cloned()
            .ok_or_else(|| TransportError::new(format!("no stub response for method {method}")))
    }
}

/// In-memory transport that mirrors worker capability dispatch for CI fallback paths.
pub struct PackageStubJsonRpcTransport {
    package_name: String,
    backend_kind: &'static str,
}

impl PackageStubJsonRpcTransport {
    pub fn new(package_name: impl Into<String>, backend_kind: &'static str) -> Self {
        Self {
            package_name: package_name.into(),
            backend_kind,
        }
    }
}

impl JsonRpcTransport for PackageStubJsonRpcTransport {
    fn call(&self, method: &str, params: Option<Value>) -> Result<Value, TransportError> {
        if !mock_provider_invocation_allowed_from_env() {
            if method == SDKWORK_CAPABILITY_INVOKE_METHOD {
                return Ok(fail_closed_provider_payload(
                    "mock provider fallback is disabled for this runtime profile",
                ));
            }
            return Err(TransportError::new(fail_closed_provider_message(
                "mock provider fallback is disabled for this runtime profile",
            )));
        }

        match method {
            SDKWORK_PING_METHOD => Ok(json!({
                "ok": true,
                "backend": self.backend_kind,
                "package": self.package_name,
                "package_resolved": false,
            })),
            SDKWORK_CAPABILITY_INVOKE_METHOD => {
                let operation = params
                    .as_ref()
                    .and_then(|value| value.get("operation"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(stub_capability_invoke_result(
                    &self.package_name,
                    self.backend_kind,
                    &operation,
                ))
            }
            _ => Err(TransportError::new(format!(
                "no stub response for method {method}"
            ))),
        }
    }
}

pub fn stub_capability_invoke_result(
    package_name: &str,
    backend_kind: &str,
    operation: &Value,
) -> Value {
    let op = operation
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    if op == "ping" {
        return json!({
            "ok": true,
            "backend": backend_kind,
            "package": package_name,
            "package_resolved": false,
        });
    }

    if op == "session_create" {
        return json!({
            "ok": true,
            "mode": "stub",
            "agent_id": operation.get("agent_id"),
            "user_ref": operation.get("user_ref"),
            "package": package_name,
        });
    }

    if op == "model_chat" || op == "model_chat_stream" {
        let messages = operation
            .get("messages")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let prompt = messages.join("\n");
        let wire_note = operation
            .get("wire_messages")
            .and_then(Value::as_array)
            .filter(|entries| !entries.is_empty())
            .map(|entries| format!(" wire_messages={}", entries.len()))
            .unwrap_or_default();
        let body = if op == "model_chat_stream" {
            let text = format!("[{package_name} stub] {prompt}{wire_note}");
            let chunks = text
                .split_whitespace()
                .enumerate()
                .map(|(sequence, word)| {
                    json!({
                        "sequence": sequence as u64,
                        "content": format!("{word} "),
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "ok": true,
                "mode": "stub",
                "chunks": chunks,
                "finish_reason": "stop",
                "package": package_name,
                "model_request_id": operation.get("model_request_id"),
            })
        } else {
            json!({
                "ok": true,
                "mode": "stub",
                "messages": [format!("[{package_name} stub] {prompt}{wire_note}")],
                "finish_reason": "stop",
                "package": package_name,
                "model_request_id": operation.get("model_request_id"),
            })
        };
        return body;
    }

    if op == "tool_invoke" {
        return json!({
            "ok": true,
            "mode": "stub",
            "output": json!({
                "tool_id": operation.get("tool_id"),
                "arguments": operation.get("arguments"),
                "package": package_name,
            }).to_string(),
            "package": package_name,
            "tool_call_id": operation.get("tool_call_id"),
        });
    }

    json!({
        "ok": true,
        "mode": "unknown_operation",
        "operation": op,
        "package": package_name,
    })
}

pub struct StdioJsonRpcSession {
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<std::process::ChildStdout>>,
    /// Serializes full request/response pairs so concurrent callers cannot interleave stdio I/O.
    call_lock: Mutex<()>,
    next_id: AtomicU64,
}

impl StdioJsonRpcSession {
    pub fn spawn(mut command: Command) -> Result<(Self, Child), TransportError> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|error| TransportError::new(format!("failed to spawn worker: {error}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::new("worker stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::new("worker stdout unavailable"))?;
        Ok((
            Self {
                stdin: Mutex::new(stdin),
                stdout: Mutex::new(BufReader::new(stdout)),
                call_lock: Mutex::new(()),
                next_id: AtomicU64::new(1),
            },
            child,
        ))
    }

    fn next_id(&self) -> String {
        self.next_id.fetch_add(1, Ordering::Relaxed).to_string()
    }
}

impl JsonRpcTransport for StdioJsonRpcSession {
    fn call(&self, method: &str, params: Option<Value>) -> Result<Value, TransportError> {
        let _call_guard = self
            .call_lock
            .lock()
            .map_err(|error| TransportError::new(format!("call lock failed: {error}")))?;

        let request = JsonRpcRequest::new(self.next_id(), method, params);
        let encoded = serde_json::to_string(&request)
            .map_err(|error| TransportError::new(format!("encode request failed: {error}")))?;

        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|error| TransportError::new(format!("stdin lock failed: {error}")))?;
            stdin
                .write_all(encoded.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush())
                .map_err(|error| TransportError::new(format!("worker write failed: {error}")))?;
        }

        let mut line = String::new();
        {
            let mut stdout = self
                .stdout
                .lock()
                .map_err(|error| TransportError::new(format!("stdout lock failed: {error}")))?;
            stdout
                .read_line(&mut line)
                .map_err(|error| TransportError::new(format!("worker read failed: {error}")))?;
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim()).map_err(|error| {
            TransportError::new(format!("decode worker response failed: {error}"))
        })?;

        response
            .into_result()
            .map_err(|error| TransportError::new(error.message))
    }

    fn call_streaming(
        &self,
        method: &str,
        params: Option<Value>,
        sink: &mut dyn FnMut(Value) -> Result<bool, TransportError>,
    ) -> Result<(), TransportError> {
        let _call_guard = self
            .call_lock
            .lock()
            .map_err(|error| TransportError::new(format!("call lock failed: {error}")))?;

        let request_id = self.next_id();
        let request = JsonRpcRequest::new(request_id.clone(), method, params);
        let encoded = serde_json::to_string(&request)
            .map_err(|error| TransportError::new(format!("encode request failed: {error}")))?;

        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|error| TransportError::new(format!("stdin lock failed: {error}")))?;
            stdin
                .write_all(encoded.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush())
                .map_err(|error| TransportError::new(format!("worker write failed: {error}")))?;
        }

        loop {
            let mut line = String::new();
            {
                let mut stdout = self
                    .stdout
                    .lock()
                    .map_err(|error| TransportError::new(format!("stdout lock failed: {error}")))?;
                stdout
                    .read_line(&mut line)
                    .map_err(|error| TransportError::new(format!("worker read failed: {error}")))?;
            }

            let response: JsonRpcResponse = serde_json::from_str(line.trim()).map_err(|error| {
                TransportError::new(format!("decode worker response failed: {error}"))
            })?;
            if response.id != request_id {
                return Err(TransportError::new(format!(
                    "worker response id mismatch: expected {request_id}, got {}",
                    response.id
                )));
            }

            let result = response
                .into_result()
                .map_err(|error| TransportError::new(error.message))?;

            if is_stream_chunk_frame(&result) {
                if !sink(result)? {
                    return Ok(());
                }
                continue;
            }
            if is_stream_terminal_frame(&result) {
                sink(result)?;
                return Ok(());
            }

            expand_buffered_stream_payload(result, |frame| sink(frame))?;
            return Ok(());
        }
    }
}

pub struct FailClosedJsonRpcTransport {
    reason: String,
}

impl FailClosedJsonRpcTransport {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl JsonRpcTransport for FailClosedJsonRpcTransport {
    fn call(&self, method: &str, _params: Option<Value>) -> Result<Value, TransportError> {
        if method == SDKWORK_CAPABILITY_INVOKE_METHOD {
            return Ok(fail_closed_provider_payload(&self.reason));
        }

        Err(TransportError::new(fail_closed_provider_message(
            &self.reason,
        )))
    }
}

fn fail_closed_provider_payload(reason: &str) -> Value {
    json!({
        "ok": false,
        "mode": "sdk_live_failed",
        "error": fail_closed_provider_message(reason),
    })
}

fn fail_closed_provider_message(reason: &str) -> String {
    format!("sdk backend unavailable in production profile: {reason}")
}

pub struct SharedJsonRpcTransport {
    inner: Arc<dyn JsonRpcTransport + Send + Sync>,
}

impl SharedJsonRpcTransport {
    pub fn new(inner: Arc<dyn JsonRpcTransport + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl JsonRpcTransport for SharedJsonRpcTransport {
    fn call(&self, method: &str, params: Option<Value>) -> Result<Value, TransportError> {
        self.inner.call(method, params)
    }

    fn call_streaming(
        &self,
        method: &str,
        params: Option<Value>,
        sink: &mut dyn FnMut(Value) -> Result<bool, TransportError>,
    ) -> Result<(), TransportError> {
        self.inner.call_streaming(method, params, sink)
    }
}
