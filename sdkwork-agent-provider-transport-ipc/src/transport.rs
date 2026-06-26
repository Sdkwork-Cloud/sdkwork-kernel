use crate::protocol::{
    JsonRpcRequest, JsonRpcResponse, SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_PING_METHOD,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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

    if op == "model_chat" {
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
        return json!({
            "ok": true,
            "mode": "stub",
            "messages": [format!("[{package_name} stub] {prompt}")],
            "finish_reason": "stop",
            "package": package_name,
            "model_request_id": operation.get("model_request_id"),
        });
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
    fn call(&self, _method: &str, _params: Option<Value>) -> Result<Value, TransportError> {
        Err(TransportError::new(format!(
            "sdk backend unavailable in production profile: {}",
            self.reason
        )))
    }
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
}
