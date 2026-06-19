use sdkwork_agent_kernel::{ModelProvider, ModelRequest, ModelResponse, ToolCall, ToolProvider};
use sdkwork_agent_sdk_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError, SdkRuntimeOperation,
    SdkRuntimeRequest, SdkRuntimeResponse,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub struct ProviderBackedRustHandler {
    model: Mutex<Arc<dyn ModelProvider + Send + Sync>>,
    tools: Mutex<Arc<dyn ToolProvider + Send + Sync>>,
    default_model: String,
}

impl ProviderBackedRustHandler {
    pub fn new(
        model: Arc<dyn ModelProvider + Send + Sync>,
        tools: Arc<dyn ToolProvider + Send + Sync>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            model: Mutex::new(model),
            tools: Mutex::new(tools),
            default_model: default_model.into(),
        }
    }

    fn invoke_inner(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        match &request.operation {
            SdkRuntimeOperation::Ping => Ok(SdkRuntimeResponse::success(
                SdkBackendKind::RustNative,
                &request.capability_id,
                json!({ "backend": "rust_native", "ok": true }),
            )),
            SdkRuntimeOperation::ModelChat {
                model_request_id,
                messages,
            } => {
                let mut model_request =
                    ModelRequest::new(model_request_id.clone(), messages.clone());
                if let Some(model_id) = request
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("model_id"))
                    .and_then(Value::as_str)
                {
                    model_request = model_request.with_model_id(model_id);
                }
                let response = self
                    .model
                    .lock()
                    .map_err(|error| SdkRuntimeError::new("lock_error", error.to_string()))?
                    .invoke(model_request)
                    .map_err(map_kernel_error)?;
                Ok(SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    &request.capability_id,
                    model_response_json(&response),
                ))
            }
            SdkRuntimeOperation::ToolInvoke {
                tool_call_id,
                tool_id,
                arguments,
            } => {
                let call = ToolCall::new(tool_call_id, tool_id, arguments);
                let result = self
                    .tools
                    .lock()
                    .map_err(|error| SdkRuntimeError::new("lock_error", error.to_string()))?
                    .invoke_tool(call)
                    .map_err(map_kernel_error)?;
                Ok(SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    &request.capability_id,
                    json!({
                        "tool_call_id": result.tool_call_id,
                        "output": result.output,
                        "status": result.status,
                    }),
                ))
            }
            SdkRuntimeOperation::SessionCreate { agent_id, user_ref } => {
                Ok(SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    &request.capability_id,
                    json!({
                        "agent_id": agent_id,
                        "user_ref": user_ref,
                        "default_model": self.default_model,
                    }),
                ))
            }
        }
    }
}

pub struct InProcessRustSdkRuntime {
    handler: Arc<ProviderBackedRustHandler>,
}

impl InProcessRustSdkRuntime {
    pub fn new(handler: Arc<ProviderBackedRustHandler>) -> Self {
        Self { handler }
    }
}

impl SdkBackendRuntime for InProcessRustSdkRuntime {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::RustNative
    }

    fn health(&self) -> SdkDriverHealth {
        let model_health = self
            .handler
            .model
            .lock()
            .map(|model| model.health())
            .unwrap_or_else(|_| sdkwork_agent_kernel::ProviderHealth {
                status: "unavailable".to_string(),
            });
        if model_health.status == "available" {
            SdkDriverHealth::healthy()
        } else {
            SdkDriverHealth::degraded("model provider unavailable")
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        self.handler.invoke_inner(request)
    }
}

fn map_kernel_error(error: sdkwork_agent_kernel::KernelError) -> SdkRuntimeError {
    SdkRuntimeError::new("kernel_error", error.to_string())
}

fn model_response_json(response: &ModelResponse) -> Value {
    json!({
        "model_request_id": response.model_request_id,
        "provider_id": response.provider_id,
        "messages": response.messages,
        "finish_reason": response.finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{KernelResult, ModelResponse, ProviderHealth, ProviderManifest};

    struct StubModelProvider;

    impl ModelProvider for StubModelProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new("provider.model.stub", "model", "Stub", "0.1.0", vec![])
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
                "provider.model.stub",
                "stub-response",
            ))
        }
    }

    struct StubToolProvider;

    impl ToolProvider for StubToolProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new("provider.tool.stub", "tool", "Stub", "0.1.0", vec![])
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::available()
        }

        fn list_tools(&self) -> Vec<sdkwork_agent_kernel::ToolDescriptor> {
            Vec::new()
        }

        fn invoke_tool(&self, call: ToolCall) -> KernelResult<sdkwork_agent_kernel::ToolResult> {
            Ok(sdkwork_agent_kernel::ToolResult::succeeded(
                &call.tool_call_id,
                "stub-tool-output",
            ))
        }
    }

    #[test]
    fn model_chat_routes_through_kernel_provider() {
        let handler = Arc::new(ProviderBackedRustHandler::new(
            Arc::new(StubModelProvider) as Arc<dyn ModelProvider + Send + Sync>,
            Arc::new(StubToolProvider) as Arc<dyn ToolProvider + Send + Sync>,
            "stub-model",
        ));
        let runtime = InProcessRustSdkRuntime::new(handler);
        let response = runtime
            .invoke(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req-1",
                vec!["hello".to_string()],
            ))
            .expect("invoke should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::RustNative);
    }
}
