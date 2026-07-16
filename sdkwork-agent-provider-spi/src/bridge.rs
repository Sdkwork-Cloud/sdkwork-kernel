//! Bridges negotiated SDK runtime invocations to kernel provider SPI surfaces.

use crate::runtime::{
    SdkRuntimeError, SdkRuntimeOperation, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
};
use sdkwork_agent_kernel::{
    KernelResult, ModelProvider, ModelRequest, ModelResponse, ModelStatus, ModelStreamChunk,
    ModelStreamSink, ProviderHealth, ProviderManifest, ToolCall, ToolProvider, ToolResult,
};
use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
use sdkwork_agent_provider_core::validate_runtime_model_payload;
use sdkwork_agent_provider_transport_ipc::{
    is_stream_chunk_frame, is_stream_terminal_frame, StreamResourceBudget,
};
use serde_json::Value;
use std::sync::Arc;

pub const SDK_CAPABILITY_SESSION_LIFECYCLE: &str = "sdk.session.lifecycle";
pub const SDK_CAPABILITY_MODEL_CHAT: &str = "sdk.model.chat";
pub const SDK_CAPABILITY_TOOL_INVOKE: &str = "sdk.tool.invoke";
pub const SDK_CAPABILITY_SKILL_INVOKE: &str = "sdk.skill.invoke";

/// Provider-neutral completion metadata for one runtime-backed model stream.
///
/// A transport may complete a stream without being able to prove a native
/// provider session id. Callers that need resumable first-turn streaming must
/// require `native_session_id` instead of synthesizing one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdkRuntimeStreamCompletion {
    pub model_request_id: String,
    pub finish_reason: String,
    pub native_session_id: Option<String>,
}

/// Kernel providers wired through a negotiated [`SdkRuntimeRouter`].
pub struct RuntimeBackedProviders {
    pub model: SdkRuntimeBackedModelProvider,
    pub tools: SdkRuntimeBackedToolProvider,
}

/// Registers runtime-backed model and tool providers using the standard SDK capability ids.
pub fn wire_runtime_providers(
    runtime: Arc<SdkRuntimeRouter>,
    model_fallback: Arc<dyn ModelProvider + Send + Sync>,
    tool_fallback: Arc<dyn ToolProvider + Send + Sync>,
    model_provider_id: &str,
) -> RuntimeBackedProviders {
    RuntimeBackedProviders {
        model: SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            model_fallback,
            SDK_CAPABILITY_MODEL_CHAT,
            model_provider_id,
        ),
        tools: SdkRuntimeBackedToolProvider::new(
            runtime,
            tool_fallback,
            SDK_CAPABILITY_TOOL_INVOKE,
        ),
    }
}

/// Kernel `ModelProvider` that routes `invoke` through `SdkRuntimeRouter` with fallback.
pub struct SdkRuntimeBackedModelProvider {
    runtime: Arc<SdkRuntimeRouter>,
    fallback: Arc<dyn ModelProvider + Send + Sync>,
    capability_id: String,
    provider_id: String,
}

impl SdkRuntimeBackedModelProvider {
    pub fn new(
        runtime: Arc<SdkRuntimeRouter>,
        fallback: Arc<dyn ModelProvider + Send + Sync>,
        capability_id: impl Into<String>,
        provider_id: impl Into<String>,
    ) -> Self {
        Self {
            runtime,
            fallback,
            capability_id: capability_id.into(),
            provider_id: provider_id.into(),
        }
    }

    pub fn invoke_through_runtime(
        runtime: &SdkRuntimeRouter,
        capability_id: &str,
        request: &ModelRequest,
        provider_id: &str,
    ) -> Result<ModelResponse, SdkRuntimeError> {
        let runtime_request = SdkRuntimeRequest::from_model_request(capability_id, request)?;
        let response = runtime.invoke(&runtime_request)?;
        model_response_from_runtime(response, &request.model_request_id, provider_id)
    }

    pub fn stream_through_runtime(
        runtime: &SdkRuntimeRouter,
        capability_id: &str,
        request: &ModelRequest,
        provider_id: &str,
    ) -> Result<Vec<ModelStreamChunk>, SdkRuntimeError> {
        let runtime_request = SdkRuntimeRequest::stream_from_model_request(capability_id, request)?;
        let response = runtime.invoke(&runtime_request)?;
        stream_chunks_from_runtime(response, &request.model_request_id, provider_id)
    }

    pub fn stream_through_runtime_into(
        runtime: &SdkRuntimeRouter,
        capability_id: &str,
        request: &ModelRequest,
        _provider_id: &str,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<(), SdkRuntimeError> {
        let runtime_request = SdkRuntimeRequest::stream_from_model_request(capability_id, request)?;
        let model_request_id = request.model_request_id.clone();
        let mut budget = StreamResourceBudget::new();
        runtime.invoke_streaming(&runtime_request, &mut |frame| {
            if let Some(chunk) = model_stream_chunk_from_frame(&frame, &model_request_id) {
                if chunk.model_request_id != model_request_id {
                    return Err(SdkRuntimeError::new(
                        "stream_request_mismatch",
                        "runtime stream chunk model_request_id does not match the active request",
                    ));
                }
                budget.record_chunk(&chunk.content).map_err(|error| {
                    SdkRuntimeError::new("stream_resource_limit", error.to_string())
                })?;
                sink.push_chunk(chunk)
                    .map_err(|error| SdkRuntimeError::new("stream_sink", error.to_string()))?;
            }
            Ok(true)
        })
    }

    /// Streams through the negotiated runtime and returns the correlated
    /// terminal metadata emitted by the transport.
    ///
    /// This intentionally extends only the runtime-backed provider boundary;
    /// the stable kernel [`ModelProvider`] SPI remains chunk-only. A terminal
    /// frame without a matching `model_request_id` is rejected so completion
    /// metadata cannot be attached to another in-flight turn.
    pub fn stream_through_runtime_into_with_completion(
        runtime: &SdkRuntimeRouter,
        capability_id: &str,
        request: &ModelRequest,
        _provider_id: &str,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<SdkRuntimeStreamCompletion, SdkRuntimeError> {
        let runtime_request = SdkRuntimeRequest::stream_from_model_request(capability_id, request)?;
        let model_request_id = request.model_request_id.clone();
        let mut budget = StreamResourceBudget::new();
        let mut completion = None;

        runtime.invoke_streaming(&runtime_request, &mut |frame| {
            if is_stream_terminal_frame(&frame) {
                let next_completion =
                    runtime_stream_completion_from_terminal_frame(&frame, &model_request_id)?;
                if completion.replace(next_completion).is_some() {
                    return Err(SdkRuntimeError::new(
                        "duplicate_stream_completion",
                        "runtime stream emitted more than one terminal completion frame",
                    ));
                }
                return Ok(true);
            }

            if let Some(chunk) = model_stream_chunk_from_frame(&frame, &model_request_id) {
                if chunk.model_request_id != model_request_id {
                    return Err(SdkRuntimeError::new(
                        "stream_request_mismatch",
                        "runtime stream chunk model_request_id does not match the active request",
                    ));
                }
                budget.record_chunk(&chunk.content).map_err(|error| {
                    SdkRuntimeError::new("stream_resource_limit", error.to_string())
                })?;
                sink.push_chunk(chunk)
                    .map_err(|error| SdkRuntimeError::new("stream_sink", error.to_string()))?;
            }
            Ok(true)
        })?;

        completion.ok_or_else(|| {
            SdkRuntimeError::new(
                "missing_stream_completion",
                "runtime stream completed without a terminal completion frame",
            )
        })
    }

    pub fn cancel_through_runtime(
        runtime: &SdkRuntimeRouter,
        capability_id: &str,
        model_request_id: &str,
        provider_id: &str,
    ) -> Result<ModelResponse, SdkRuntimeError> {
        if !runtime.cancel_inflight(capability_id, model_request_id)? {
            return Err(SdkRuntimeError::new(
                "request_not_inflight",
                format!("model request is not in flight: {model_request_id}"),
            ));
        }
        Ok(ModelResponse::cancelled(model_request_id, provider_id))
    }

    /// Streams model output and returns the runtime completion metadata when
    /// the negotiated provider can prove it. This does not fall back to a
    /// synthetic provider because no fallback can truthfully provide the
    /// native session identity for the streamed execution.
    pub fn stream_into_with_completion(
        &self,
        request: ModelRequest,
        sink: &mut dyn ModelStreamSink,
    ) -> KernelResult<SdkRuntimeStreamCompletion> {
        Self::stream_through_runtime_into_with_completion(
            &self.runtime,
            &self.capability_id,
            &request,
            &self.provider_id,
            sink,
        )
        .map_err(|_| sdkwork_agent_kernel::KernelError::ProviderUnavailable {
            provider_id: self.provider_id.clone(),
        })
    }
}

impl ModelProvider for SdkRuntimeBackedModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        self.fallback.provider_manifest()
    }

    fn health(&self) -> ProviderHealth {
        self.fallback.health()
    }

    fn list_models(&self) -> Vec<sdkwork_agent_kernel::ModelDescriptor> {
        self.fallback.list_models()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let require_live_provider = request_requires_live_provider(&request);
        match Self::invoke_through_runtime(
            &self.runtime,
            &self.capability_id,
            &request,
            &self.provider_id,
        ) {
            Ok(response) => Ok(response),
            Err(_) if mock_provider_invocation_allowed() && !require_live_provider => {
                self.fallback.invoke(request)
            }
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            }),
        }
    }

    fn stream(
        &self,
        request: ModelRequest,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        let require_live_provider = request_requires_live_provider(&request);
        match Self::stream_through_runtime(
            &self.runtime,
            &self.capability_id,
            &request,
            &self.provider_id,
        ) {
            Ok(chunks) => Ok(chunks),
            Err(_) if mock_provider_invocation_allowed() && !require_live_provider => {
                self.fallback.stream(request)
            }
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            }),
        }
    }

    fn stream_into(
        &self,
        request: ModelRequest,
        sink: &mut dyn ModelStreamSink,
    ) -> KernelResult<()> {
        let require_live_provider = request_requires_live_provider(&request);
        match Self::stream_through_runtime_into(
            &self.runtime,
            &self.capability_id,
            &request,
            &self.provider_id,
            sink,
        ) {
            Ok(()) => Ok(()),
            Err(_) if mock_provider_invocation_allowed() && !require_live_provider => {
                self.fallback.stream_into(request, sink)
            }
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            }),
        }
    }

    fn cancel(&self, model_request_id: &str) -> KernelResult<ModelResponse> {
        match Self::cancel_through_runtime(
            &self.runtime,
            &self.capability_id,
            model_request_id,
            &self.provider_id,
        ) {
            Ok(response) => Ok(response),
            Err(_) if mock_provider_invocation_allowed() => self.fallback.cancel(model_request_id),
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            }),
        }
    }
}

fn request_requires_live_provider(request: &ModelRequest) -> bool {
    let Some(value) = request
        .metadata_value("sdkwork.code_engine.require_live_provider")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };

    // The runtime projection validates the value. Treat malformed values as
    // requiring a real provider here as well, so a development fallback cannot
    // silently hide a configuration error.
    !value.eq_ignore_ascii_case("false")
}

/// Kernel `ToolProvider` that routes `invoke_tool` through `SdkRuntimeRouter` with fallback.
pub struct SdkRuntimeBackedToolProvider {
    runtime: Arc<SdkRuntimeRouter>,
    fallback: Arc<dyn ToolProvider + Send + Sync>,
    capability_id: String,
}

impl SdkRuntimeBackedToolProvider {
    pub fn new(
        runtime: Arc<SdkRuntimeRouter>,
        fallback: Arc<dyn ToolProvider + Send + Sync>,
        capability_id: impl Into<String>,
    ) -> Self {
        Self {
            runtime,
            fallback,
            capability_id: capability_id.into(),
        }
    }
}

impl ToolProvider for SdkRuntimeBackedToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        self.fallback.provider_manifest()
    }

    fn health(&self) -> ProviderHealth {
        self.fallback.health()
    }

    fn list_tools(&self) -> Vec<sdkwork_agent_kernel::ToolDescriptor> {
        self.fallback.list_tools()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        let runtime_request = SdkRuntimeRequest {
            capability_id: self.capability_id.clone(),
            operation: SdkRuntimeOperation::ToolInvoke {
                tool_call_id: call.tool_call_id.clone(),
                tool_id: call.tool_id.clone(),
                arguments: call.arguments.clone(),
            },
            payload: None,
        };

        match self.runtime.invoke(&runtime_request) {
            Ok(response) => tool_result_from_runtime(response, &call.tool_call_id)
                .map_err(|error| sdkwork_agent_kernel::KernelError::validation(error.message)),
            Err(_) if mock_provider_invocation_allowed() => self.fallback.invoke_tool(call),
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.fallback.provider_manifest().provider_id.clone(),
            }),
        }
    }
}

pub fn model_response_from_runtime(
    response: SdkRuntimeResponse,
    model_request_id: &str,
    provider_id: &str,
) -> Result<ModelResponse, SdkRuntimeError> {
    if !response.success {
        return Err(SdkRuntimeError::new(
            "runtime_failure",
            response
                .message
                .unwrap_or_else(|| "runtime invoke failed".to_string()),
        ));
    }

    let payload = response.payload.ok_or_else(|| {
        SdkRuntimeError::new("missing_payload", "runtime response missing payload")
    })?;

    if payload.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(SdkRuntimeError::new(
            "sdk_live_failed",
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("runtime invoke returned ok=false"),
        ));
    }
    validate_payload_model_request_id(&payload, model_request_id)?;

    validate_runtime_model_payload(&payload)
        .map_err(|message| SdkRuntimeError::new("mock_provider_disabled", message))?;

    let messages = extract_messages(&payload);
    if messages.is_empty() {
        return Err(SdkRuntimeError::new(
            "empty_messages",
            "runtime response did not include model messages",
        ));
    }
    let tool_calls = extract_tool_calls(&payload)?;

    let mut model_response = ModelResponse {
        model_request_id: model_request_id.to_string(),
        provider_id: provider_id.to_string(),
        status: ModelStatus::Succeeded,
        messages,
        tool_calls,
        usage: None,
        finish_reason: payload
            .get("finish_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        trace_context: None,
        redaction_classification: sdkwork_agent_kernel::KernelEventRedaction::Unknown,
        diagnostics: Vec::new(),
    };

    if let Some(mode) = payload.get("mode").and_then(Value::as_str) {
        model_response
            .diagnostics
            .push(format!("sdk_runtime_mode={mode}"));
    }
    if let Some(native_session_id) = payload
        .get("native_session_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        model_response
            .diagnostics
            .push(format!("sdk_runtime_native_session_id={native_session_id}"));
    }

    Ok(model_response)
}

pub fn stream_chunks_from_runtime(
    response: SdkRuntimeResponse,
    model_request_id: &str,
    _provider_id: &str,
) -> Result<Vec<ModelStreamChunk>, SdkRuntimeError> {
    if !response.success {
        return Err(SdkRuntimeError::new(
            "runtime_failure",
            response
                .message
                .unwrap_or_else(|| "runtime stream invoke failed".to_string()),
        ));
    }

    let payload = response.payload.ok_or_else(|| {
        SdkRuntimeError::new("missing_payload", "runtime stream response missing payload")
    })?;

    if payload.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(SdkRuntimeError::new(
            "sdk_live_failed",
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("runtime stream invoke returned ok=false"),
        ));
    }
    validate_payload_model_request_id(&payload, model_request_id)?;

    if let Some(chunks) = payload.get("chunks").and_then(Value::as_array) {
        let mut budget = StreamResourceBudget::new();
        let mut mapped = Vec::with_capacity(chunks.len().min(64));
        for (index, entry) in chunks.iter().enumerate() {
            let content = entry
                .get("content")
                .or_else(|| entry.get("delta"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    SdkRuntimeError::new("invalid_stream_chunk", "stream chunk content is missing")
                })?;
            budget.record_chunk(content).map_err(|error| {
                SdkRuntimeError::new("stream_resource_limit", error.to_string())
            })?;
            let sequence = entry
                .get("sequence")
                .and_then(Value::as_u64)
                .unwrap_or(index as u64);
            mapped.push(ModelStreamChunk::output(
                model_request_id,
                sequence,
                content,
            ));
        }
        return Ok(mapped);
    }

    let mut budget = StreamResourceBudget::new();
    let mut mapped = Vec::new();
    if let Some(messages) = payload.get("messages").and_then(Value::as_array) {
        mapped.reserve(messages.len().min(64));
        for (sequence, entry) in messages.iter().enumerate() {
            let content = entry.as_str().ok_or_else(|| {
                SdkRuntimeError::new(
                    "invalid_stream_chunk",
                    "stream message content must be a string",
                )
            })?;
            budget.record_chunk(content).map_err(|error| {
                SdkRuntimeError::new("stream_resource_limit", error.to_string())
            })?;
            mapped.push(ModelStreamChunk::output(
                model_request_id,
                sequence as u64,
                content,
            ));
        }
    } else if let Some(content) = payload.get("message").and_then(Value::as_str) {
        budget
            .record_chunk(content)
            .map_err(|error| SdkRuntimeError::new("stream_resource_limit", error.to_string()))?;
        mapped.push(ModelStreamChunk::output(model_request_id, 0, content));
    }
    if mapped.is_empty() {
        return Err(SdkRuntimeError::new(
            "empty_stream",
            "runtime stream response did not include chunks or messages",
        ));
    }
    Ok(mapped)
}

fn validate_payload_model_request_id(
    payload: &Value,
    expected_model_request_id: &str,
) -> Result<(), SdkRuntimeError> {
    let Some(model_request_id) = payload
        .get("model_request_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if model_request_id == expected_model_request_id {
        return Ok(());
    }
    Err(SdkRuntimeError::new(
        "stream_request_mismatch",
        "runtime model_request_id does not match the active request",
    ))
}

pub fn model_stream_chunk_from_frame(
    frame: &Value,
    default_model_request_id: &str,
) -> Option<ModelStreamChunk> {
    if !is_stream_chunk_frame(frame) {
        return None;
    }
    let content = frame
        .get("content")
        .or_else(|| frame.get("delta"))
        .and_then(Value::as_str)?;
    let sequence = frame.get("sequence").and_then(Value::as_u64).unwrap_or(0);
    let model_request_id = frame
        .get("model_request_id")
        .and_then(Value::as_str)
        .unwrap_or(default_model_request_id);
    Some(ModelStreamChunk::output(
        model_request_id,
        sequence,
        content.to_string(),
    ))
}

fn runtime_stream_completion_from_terminal_frame(
    frame: &Value,
    expected_model_request_id: &str,
) -> Result<SdkRuntimeStreamCompletion, SdkRuntimeError> {
    let model_request_id = frame
        .get("model_request_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SdkRuntimeError::new(
                "missing_stream_request_id",
                "runtime stream completion is missing model_request_id",
            )
        })?;
    if model_request_id != expected_model_request_id {
        return Err(SdkRuntimeError::new(
            "stream_request_mismatch",
            "runtime stream completion model_request_id does not match the active request",
        ));
    }

    let finish_reason = frame
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("stop")
        .to_string();
    let native_session_id = frame
        .get("native_session_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(SdkRuntimeStreamCompletion {
        model_request_id: model_request_id.to_string(),
        finish_reason,
        native_session_id,
    })
}

pub fn tool_result_from_runtime(
    response: SdkRuntimeResponse,
    tool_call_id: &str,
) -> Result<ToolResult, SdkRuntimeError> {
    if !response.success {
        return Err(SdkRuntimeError::new(
            "runtime_failure",
            response
                .message
                .unwrap_or_else(|| "runtime invoke failed".to_string()),
        ));
    }

    let payload = response.payload.ok_or_else(|| {
        SdkRuntimeError::new("missing_payload", "runtime response missing payload")
    })?;

    let output = payload
        .get("output")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| payload.to_string());

    Ok(ToolResult::succeeded(tool_call_id, output))
}

fn extract_messages(payload: &Value) -> Vec<String> {
    if let Some(messages) = payload.get("messages").and_then(Value::as_array) {
        return messages
            .iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect();
    }

    payload
        .get("message")
        .and_then(Value::as_str)
        .map(|message| vec![message.to_string()])
        .unwrap_or_default()
}

/// Maps a provider runtime's optional tool-call list into the kernel-neutral
/// [`ToolCall`] contract. A provider must supply its own stable call id; this
/// bridge never fabricates an id because interaction replies must route back
/// to the provider-native request/checkpoint.
fn extract_tool_calls(payload: &Value) -> Result<Vec<ToolCall>, SdkRuntimeError> {
    let Some(entries) = payload
        .get("tool_calls")
        .or_else(|| payload.get("toolCalls"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };

    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| runtime_tool_call_from_value(entry, index))
        .collect()
}

fn runtime_tool_call_from_value(entry: &Value, index: usize) -> Result<ToolCall, SdkRuntimeError> {
    let tool_call_id = runtime_tool_call_string(
        entry,
        &["tool_call_id", "toolCallId", "id", "interactionId"],
    )
    .ok_or_else(|| {
        SdkRuntimeError::new(
            "invalid_tool_call",
            format!("runtime tool call at index {index} is missing a stable id"),
        )
    })?;
    let tool_id = runtime_tool_call_string(
        entry,
        &["tool_id", "toolId", "tool_name", "toolName", "name"],
    )
    .or_else(|| {
        entry
            .get("function")
            .and_then(|function| runtime_tool_call_string(function, &["name"]))
    })
    .ok_or_else(|| {
        SdkRuntimeError::new(
            "invalid_tool_call",
            format!("runtime tool call {tool_call_id} is missing a tool id"),
        )
    })?;
    let arguments = entry
        .get("arguments")
        .or_else(|| entry.get("toolArguments"))
        .or_else(|| entry.get("input"))
        .or_else(|| {
            entry
                .get("function")
                .and_then(|function| function.get("arguments"))
        })
        .map(runtime_tool_call_arguments)
        .transpose()?
        .unwrap_or_else(|| "{}".to_string());

    Ok(ToolCall::new(tool_call_id, tool_id, arguments))
}

fn runtime_tool_call_string(entry: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        entry
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn runtime_tool_call_arguments(value: &Value) -> Result<String, SdkRuntimeError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Object(_) | Value::Array(_) => serde_json::to_string(value).map_err(|error| {
            SdkRuntimeError::new(
                "invalid_tool_call",
                format!("runtime tool call arguments could not be serialized: {error}"),
            )
        }),
        Value::Null => Ok("{}".to_string()),
        _ => Err(SdkRuntimeError::new(
            "invalid_tool_call",
            "runtime tool call arguments must be a string, object, array, or null",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::SdkBackendKind;
    use crate::negotiation::SdkCapabilityNegotiation;
    use crate::runtime::{SdkBackendRuntime, SdkRuntimeOperationKind};
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

    struct StubRuntime;

    impl SdkBackendRuntime for StubRuntime {
        fn backend_kind(&self) -> SdkBackendKind {
            SdkBackendKind::RustNative
        }

        fn health(&self) -> crate::driver::SdkDriverHealth {
            crate::driver::SdkDriverHealth::healthy()
        }

        fn invoke(
            &self,
            request: &SdkRuntimeRequest,
        ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
            Ok(SdkRuntimeResponse::success(
                SdkBackendKind::RustNative,
                &request.capability_id,
                serde_json::json!({
                    "messages": ["runtime-backed"],
                    "mode": "test"
                }),
            ))
        }
    }

    #[test]
    fn model_provider_prefers_runtime_messages() {
        let negotiation = SdkCapabilityNegotiation {
            agent_id: "agent.test".to_string(),
            binding_id: "binding.agent-provider.test".to_string(),
            binding_version: "0.1.0".to_string(),
            selected: vec![crate::negotiation::NegotiatedCapability {
                capability_id: SDK_CAPABILITY_MODEL_CHAT.to_string(),
                backend_kind: SdkBackendKind::RustNative,
                driver_id: "driver.test.model.chat".to_string(),
                runtime_operations: vec![SdkRuntimeOperationKind::ModelChat],
            }],
            missing_required: Vec::new(),
            degraded_optional: Vec::new(),
        };
        let runtime =
            Arc::new(SdkRuntimeRouter::new(negotiation).with_rust_runtime(Arc::new(StubRuntime)));

        struct FallbackModel;

        impl ModelProvider for FallbackModel {
            fn provider_manifest(&self) -> ProviderManifest {
                ProviderManifest::new(
                    "provider.model.fallback",
                    "model",
                    "Fallback",
                    "0.1.0",
                    vec![],
                )
            }

            fn health(&self) -> ProviderHealth {
                ProviderHealth::available()
            }

            fn list_models(&self) -> Vec<sdkwork_agent_kernel::ModelDescriptor> {
                Vec::new()
            }

            fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
                Ok(ModelResponse::text(
                    &request.model_request_id,
                    "provider.model.fallback",
                    "fallback-response",
                ))
            }
        }

        let provider = SdkRuntimeBackedModelProvider::new(
            runtime,
            Arc::new(FallbackModel),
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.test",
        );

        let response = provider
            .invoke(ModelRequest::new("req-1", vec!["hello".to_string()]))
            .expect("invoke should succeed");
        assert_eq!(response.messages, vec!["runtime-backed".to_string()]);
    }

    #[test]
    fn production_stream_and_cancel_use_the_real_runtime() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);

        struct StreamingRuntime {
            cancelled: Arc<Mutex<Vec<String>>>,
        }

        impl SdkBackendRuntime for StreamingRuntime {
            fn backend_kind(&self) -> SdkBackendKind {
                SdkBackendKind::RustNative
            }

            fn health(&self) -> crate::driver::SdkDriverHealth {
                crate::driver::SdkDriverHealth::healthy()
            }

            fn invoke(
                &self,
                request: &SdkRuntimeRequest,
            ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
                Ok(SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    &request.capability_id,
                    serde_json::json!({
                        "ok": true,
                        "chunks": [{"sequence": 0, "content": "live"}],
                        "model_request_id": request.operation.request_id(),
                        "native_session_id": "thread-live"
                    }),
                ))
            }

            fn cancel_inflight(&self, request_id: &str) -> Result<bool, SdkRuntimeError> {
                self.cancelled
                    .lock()
                    .expect("cancelled request lock")
                    .push(request_id.to_string());
                Ok(true)
            }
        }

        struct NoFallback;

        impl ModelProvider for NoFallback {
            fn provider_manifest(&self) -> ProviderManifest {
                ProviderManifest::new("provider.no-fallback", "model", "None", "0.1.0", vec![])
            }

            fn health(&self) -> ProviderHealth {
                ProviderHealth::available()
            }

            fn invoke(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
                panic!("production real-runtime test must not invoke fallback")
            }
        }

        let cancelled = Arc::new(Mutex::new(Vec::new()));
        let negotiation = SdkCapabilityNegotiation {
            agent_id: "agent.production".to_string(),
            binding_id: "binding.production".to_string(),
            binding_version: "0.1.0".to_string(),
            selected: vec![crate::negotiation::NegotiatedCapability {
                capability_id: SDK_CAPABILITY_MODEL_CHAT.to_string(),
                backend_kind: SdkBackendKind::RustNative,
                driver_id: "driver.production".to_string(),
                runtime_operations: vec![
                    SdkRuntimeOperationKind::ModelChat,
                    SdkRuntimeOperationKind::ModelChatStream,
                ],
            }],
            missing_required: Vec::new(),
            degraded_optional: Vec::new(),
        };
        let runtime = Arc::new(
            SdkRuntimeRouter::new(negotiation).with_rust_runtime(Arc::new(StreamingRuntime {
                cancelled: cancelled.clone(),
            })),
        );
        let provider = SdkRuntimeBackedModelProvider::new(
            runtime,
            Arc::new(NoFallback),
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.production",
        );

        let chunks = provider
            .stream(ModelRequest::new("request-live", vec!["hello".to_string()]))
            .expect("real runtime streaming must remain enabled in production");
        assert_eq!(chunks[0].content, "live");

        struct CollectSink(Vec<ModelStreamChunk>);

        impl ModelStreamSink for CollectSink {
            fn push_chunk(&mut self, chunk: ModelStreamChunk) -> KernelResult<()> {
                self.0.push(chunk);
                Ok(())
            }
        }

        let mut sink = CollectSink(Vec::new());
        provider
            .stream_into(
                ModelRequest::new("request-live-into", vec!["hello".to_string()]),
                &mut sink,
            )
            .expect("real runtime stream_into must remain enabled in production");
        assert_eq!(sink.0[0].content, "live");

        let mut completion_sink = CollectSink(Vec::new());
        let completion = provider
            .stream_into_with_completion(
                ModelRequest::new("request-live-completion", vec!["hello".to_string()]),
                &mut completion_sink,
            )
            .expect("runtime completion must remain available in production");
        assert_eq!(completion_sink.0[0].content, "live");
        assert_eq!(completion.model_request_id, "request-live-completion");
        assert_eq!(completion.native_session_id.as_deref(), Some("thread-live"));

        provider
            .cancel("request-live")
            .expect("real runtime cancellation must remain enabled in production");
        assert_eq!(
            cancelled.lock().expect("cancelled request lock").as_slice(),
            ["request-live".to_string()]
        );
    }

    #[test]
    fn model_stream_chunk_from_frame_maps_delta() {
        use sdkwork_agent_provider_transport_ipc::stream_chunk_frame;

        let frame = stream_chunk_frame(2, "hello", Some("req-stream-1"));
        let chunk = model_stream_chunk_from_frame(&frame, "req-default").expect("chunk");
        assert_eq!(chunk.model_request_id, "req-stream-1");
        assert_eq!(chunk.sequence, 2);
        assert_eq!(chunk.content, "hello");
    }

    #[test]
    fn model_stream_chunk_from_frame_ignores_non_stream_frames() {
        let frame = serde_json::json!({ "type": "message", "content": "ignored" });
        assert!(model_stream_chunk_from_frame(&frame, "req-1").is_none());
    }

    #[test]
    fn stream_into_rejects_mismatched_chunk_model_request_id() {
        struct MismatchedStreamingRuntime;

        impl SdkBackendRuntime for MismatchedStreamingRuntime {
            fn backend_kind(&self) -> SdkBackendKind {
                SdkBackendKind::RustNative
            }

            fn health(&self) -> crate::driver::SdkDriverHealth {
                crate::driver::SdkDriverHealth::healthy()
            }

            fn invoke(
                &self,
                request: &SdkRuntimeRequest,
            ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
                Ok(SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    &request.capability_id,
                    serde_json::json!({
                        "ok": true,
                        "chunks": [{"sequence": 0, "content": "wrong turn"}],
                        "model_request_id": "req-other"
                    }),
                ))
            }
        }

        struct CollectSink(Vec<ModelStreamChunk>);

        impl ModelStreamSink for CollectSink {
            fn push_chunk(&mut self, chunk: ModelStreamChunk) -> KernelResult<()> {
                self.0.push(chunk);
                Ok(())
            }
        }

        let negotiation = SdkCapabilityNegotiation {
            agent_id: "agent.stream-mismatch".to_string(),
            binding_id: "binding.stream-mismatch".to_string(),
            binding_version: "0.1.0".to_string(),
            selected: vec![crate::negotiation::NegotiatedCapability {
                capability_id: SDK_CAPABILITY_MODEL_CHAT.to_string(),
                backend_kind: SdkBackendKind::RustNative,
                driver_id: "driver.stream-mismatch".to_string(),
                runtime_operations: vec![SdkRuntimeOperationKind::ModelChatStream],
            }],
            missing_required: Vec::new(),
            degraded_optional: Vec::new(),
        };
        let runtime = SdkRuntimeRouter::new(negotiation)
            .with_rust_runtime(Arc::new(MismatchedStreamingRuntime));
        let mut sink = CollectSink(Vec::new());

        let error = SdkRuntimeBackedModelProvider::stream_through_runtime_into(
            &runtime,
            SDK_CAPABILITY_MODEL_CHAT,
            &ModelRequest::new("req-active", vec!["hello".to_string()]),
            "provider.codex",
            &mut sink,
        )
        .expect_err("stream chunks must not cross-correlate turns");

        assert_eq!(error.code, "stream_request_mismatch");
        assert!(sink.0.is_empty());
    }

    #[test]
    fn stream_completion_requires_the_active_model_request_id() {
        let frame = serde_json::json!({
            "event": "stream.done",
            "finish_reason": "stop",
            "model_request_id": "req-active",
            "native_session_id": "thread-1"
        });
        let completion = runtime_stream_completion_from_terminal_frame(&frame, "req-active")
            .expect("matching stream completion");
        assert_eq!(completion.native_session_id.as_deref(), Some("thread-1"));

        let error = runtime_stream_completion_from_terminal_frame(&frame, "req-other")
            .expect_err("mismatched completion must be rejected");
        assert_eq!(error.code, "stream_request_mismatch");
    }

    #[test]
    fn stream_completion_does_not_invent_a_native_session_id() {
        let frame = serde_json::json!({
            "event": "stream.done",
            "finish_reason": "stop",
            "model_request_id": "req-active"
        });
        let completion = runtime_stream_completion_from_terminal_frame(&frame, "req-active")
            .expect("completion without native session is still well-formed");
        assert_eq!(completion.native_session_id, None);
    }

    #[test]
    fn buffered_stream_payload_rejects_mismatched_model_request_id() {
        let response = SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            SDK_CAPABILITY_MODEL_CHAT,
            serde_json::json!({
                "ok": true,
                "chunks": [{"sequence": 0, "content": "wrong turn"}],
                "model_request_id": "req-other"
            }),
        );

        let error = stream_chunks_from_runtime(response, "req-active", "provider.codex")
            .expect_err("buffered stream payload must not cross-correlate turns");

        assert_eq!(error.code, "stream_request_mismatch");
    }

    #[test]
    fn model_response_rejects_mismatched_model_request_id() {
        let response = SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            SDK_CAPABILITY_MODEL_CHAT,
            serde_json::json!({
                "ok": true,
                "messages": ["wrong turn"],
                "model_request_id": "req-other"
            }),
        );

        let error = model_response_from_runtime(response, "req-active", "provider.codex")
            .expect_err("model response payload must not cross-correlate turns");

        assert_eq!(error.code, "stream_request_mismatch");
    }

    #[test]
    fn model_chat_request_includes_wire_messages_for_structured_input() {
        use sdkwork_agent_kernel::{AgentMessage, AgentMessageRole, AgentPart};

        let mut request = ModelRequest::new("req-wire", vec!["legacy".to_string()]);
        request.input_messages = vec![AgentMessage::new(
            "msg.1",
            AgentMessageRole::User,
            vec![AgentPart::text("part.text", "structured hello")],
        )];

        let runtime_request = SdkRuntimeRequest::from_model_request("sdk.model.chat", &request)
            .expect("wire request");
        match runtime_request.operation {
            SdkRuntimeOperation::ModelChat {
                wire_messages: Some(wire),
                ..
            } => {
                assert!(wire.is_array());
                assert!(!wire.as_array().expect("array").is_empty());
            }
            other => panic!("expected model_chat with wire, got {other:?}"),
        }
    }

    #[test]
    fn model_response_includes_runtime_mode_and_native_session_diagnostics() {
        let response = SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            SDK_CAPABILITY_MODEL_CHAT,
            serde_json::json!({
                "ok": true,
                "mode": "sdk_cli",
                "messages": ["done"],
                "native_session_id": "thread-test-123"
            }),
        );

        let mapped = model_response_from_runtime(response, "req-session", "provider.codex")
            .expect("runtime response should map");
        assert_eq!(
            mapped.diagnostics,
            vec![
                "sdk_runtime_mode=sdk_cli".to_string(),
                "sdk_runtime_native_session_id=thread-test-123".to_string(),
            ]
        );
    }

    #[test]
    fn model_response_preserves_provider_neutral_tool_calls_without_synthesizing_ids() {
        let response = SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            SDK_CAPABILITY_MODEL_CHAT,
            serde_json::json!({
                "ok": true,
                "mode": "sdk_live",
                "messages": ["I need your input."],
                "tool_calls": [
                    {
                        "id": "provider-question-1",
                        "toolName": "user_question",
                        "toolArguments": {
                            "requestID": "provider-question-1",
                            "questions": [{"question": "Run unit tests?"}]
                        }
                    }
                ]
            }),
        );

        let mapped = model_response_from_runtime(response, "req-tool-call", "provider.opencode")
            .expect("valid provider tool calls should be preserved");

        assert_eq!(mapped.tool_calls.len(), 1);
        assert_eq!(mapped.tool_calls[0].tool_call_id, "provider-question-1");
        assert_eq!(mapped.tool_calls[0].tool_id, "user_question");
        assert_eq!(
            mapped.tool_calls[0].arguments,
            r#"{"questions":[{"question":"Run unit tests?"}],"requestID":"provider-question-1"}"#
        );
    }

    #[test]
    fn model_response_rejects_tool_calls_without_provider_native_ids() {
        let response = SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            SDK_CAPABILITY_MODEL_CHAT,
            serde_json::json!({
                "ok": true,
                "messages": ["I need approval."],
                "tool_calls": [{"toolName": "permission_request", "arguments": {}}]
            }),
        );

        let error = model_response_from_runtime(response, "req-tool-call", "provider.opencode")
            .expect_err("the bridge must not invent a native interaction id");

        assert_eq!(error.code, "invalid_tool_call");
    }

    #[test]
    fn request_live_provider_requirement_is_fail_closed() {
        let required = ModelRequest::new("req-live", vec!["hello".to_string()])
            .with_metadata("sdkwork.code_engine.require_live_provider", "true");
        let optional = ModelRequest::new("req-optional", vec!["hello".to_string()])
            .with_metadata("sdkwork.code_engine.require_live_provider", "false");
        let malformed = ModelRequest::new("req-malformed", vec!["hello".to_string()])
            .with_metadata("sdkwork.code_engine.require_live_provider", "sometimes");

        assert!(request_requires_live_provider(&required));
        assert!(!request_requires_live_provider(&optional));
        assert!(request_requires_live_provider(&malformed));
    }
}
