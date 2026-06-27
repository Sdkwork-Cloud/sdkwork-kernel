//! Bridges negotiated SDK runtime invocations to kernel provider SPI surfaces.

use crate::runtime::{
    SdkRuntimeError, SdkRuntimeOperation, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
};
use sdkwork_agent_kernel::{
    KernelResult, ModelProvider, ModelRequest, ModelResponse, ModelStatus, ProviderHealth,
    ProviderManifest, ToolCall, ToolProvider, ToolResult,
};
use sdkwork_agent_provider_core::validate_runtime_model_payload;
use sdkwork_agent_provider_core::{
    mock_provider_invocation_allowed, reject_direct_mock_provider_invocation,
};
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
        let runtime_request = SdkRuntimeRequest::model_chat(
            capability_id,
            &request.model_request_id,
            request.messages.clone(),
        );
        let response = runtime.invoke(&runtime_request)?;
        model_response_from_runtime(response, &request.model_request_id, provider_id)
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
        match Self::invoke_through_runtime(
            &self.runtime,
            &self.capability_id,
            &request,
            &self.provider_id,
        ) {
            Ok(response) => Ok(response),
            Err(_) if mock_provider_invocation_allowed() => self.fallback.invoke(request),
            Err(_) => Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            }),
        }
    }

    fn stream(
        &self,
        request: ModelRequest,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        reject_direct_mock_provider_invocation(&self.provider_id)?;
        self.fallback.stream(request)
    }
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

    Ok(model_response)
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
    use crate::runtime::SdkBackendRuntime;

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
}
