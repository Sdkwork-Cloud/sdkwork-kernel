use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const SDKWORK_PING_METHOD: &str = "sdkwork/ping";
pub const SDKWORK_CAPABILITY_INVOKE_METHOD: &str = "sdkwork/capability.invoke";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<String>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

impl JsonRpcResponse {
    pub fn into_result(self) -> Result<Value, JsonRpcErrorObject> {
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self.result.unwrap_or(Value::Null))
    }
}

/// Incremental stream frame emitted by worker processes for `model_chat_stream`.
pub const SDKWORK_STREAM_EVENT_CHUNK: &str = "stream.chunk";
pub const SDKWORK_STREAM_EVENT_DONE: &str = "stream.done";

pub fn is_stream_chunk_frame(frame: &Value) -> bool {
    frame.get("event").and_then(Value::as_str) == Some(SDKWORK_STREAM_EVENT_CHUNK)
}

pub fn is_stream_terminal_frame(frame: &Value) -> bool {
    frame.get("event").and_then(Value::as_str) == Some(SDKWORK_STREAM_EVENT_DONE)
}

pub fn stream_chunk_frame(sequence: u64, content: &str, model_request_id: Option<&str>) -> Value {
    json!({
        "event": SDKWORK_STREAM_EVENT_CHUNK,
        "sequence": sequence,
        "content": content,
        "model_request_id": model_request_id,
    })
}

pub fn stream_done_frame(finish_reason: &str) -> Value {
    json!({
        "event": SDKWORK_STREAM_EVENT_DONE,
        "finish_reason": finish_reason,
    })
}
