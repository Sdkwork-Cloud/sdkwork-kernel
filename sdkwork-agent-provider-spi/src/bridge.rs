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
use sdkwork_agent_provider_transport_ipc::{is_stream_chunk_frame, StreamResourceBudget};
use serde_json::Value;
use std::sync::Arc;

pub const SDK_CAPABILITY_SESSION_LIFECYCLE: &str = "sdk.session.lifecycle";
pub const SDK_CAPABILITY_MODEL_CHAT: &str = "sdk.model.chat";
pub const SDK_CAPABILITY_TOOL_INVOKE: &str = "sdk.tool.invoke";
pub const SDK_CAPABILITY_SKILL_INVOKE: &str = "sdk.skill.invoke";

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
                budget.record_chunk(&chunk.content).map_err(|error| {
                    SdkRuntimeError::new("stream_resource_limit", error.to_string())
                })?;
                sink.push_chunk(chunk)
                    .map_err(|error| SdkRuntimeError::new("stream_sink", error.to_string()))?;
            }
            Ok(true)
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

    validate_runtime_model_payload(&payload)
        .map_err(|message| SdkRuntimeError::new("mock_provider_disabled", message))?;

    let messages = extract_messages(&payload);
    if messages.is_empty() {
        return Err(SdkRuntimeError::new(
            "empty_messages",
            "runtime response did not include model messages",
        ));
    }

    let mut model_response = ModelResponse {
        model_request_id: model_request_id.to_string(),
        provider_id: provider_id.to_string(),
        status: ModelStatus::Succeeded,
        messages,
        tool_calls: Vec::new(),
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
                        "chunks": [{"sequence": 0, "content": "live"}]
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
