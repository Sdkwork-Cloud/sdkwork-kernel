use sdkwork_agent_kernel::{ModelProvider, ModelRequest, ModelResponse, ToolCall, ToolProvider};
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError, SdkRuntimeOperation,
    SdkRuntimeRequest, SdkRuntimeResponse,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ProviderBackedRustHandler {
    model: Arc<dyn ModelProvider + Send + Sync>,
    tools: Option<Arc<dyn ToolProvider + Send + Sync>>,
    default_model: String,
}

impl ProviderBackedRustHandler {
    pub fn new(
        model: Arc<dyn ModelProvider + Send + Sync>,
        tools: Arc<dyn ToolProvider + Send + Sync>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            model,
            tools: Some(tools),
            default_model: default_model.into(),
        }
    }

    pub fn model_only(
        model: Arc<dyn ModelProvider + Send + Sync>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            model,
            tools: None,
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
                wire_messages,
                ..
            } => {
                self.invoke_model_chat(model_request_id, messages, wire_messages.as_ref(), request)
            }
            SdkRuntimeOperation::ModelChatStream {
                model_request_id,
                messages,
                wire_messages,
                ..
            } => self.invoke_model_chat_stream(
                model_request_id,
                messages,
                wire_messages.as_ref(),
                request,
            ),
            SdkRuntimeOperation::ToolInvoke {
                tool_call_id,
                tool_id,
                arguments,
            } => {
                let tools = self.tools.as_ref().ok_or_else(|| {
                    SdkRuntimeError::new(
                        "operation_not_supported",
                        "tool_invoke is not configured for this Rust provider runtime",
                    )
                })?;
                let call = ToolCall::new(tool_call_id, tool_id, arguments);
                let result = tools.invoke_tool(call).map_err(map_kernel_error)?;
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
            SdkRuntimeOperation::SessionCreate { agent_id, user_ref } => Err(SdkRuntimeError::new(
                "unsupported_operation",
                format!(
                    "SessionCreate is not supported on rust_native transport; \
                     create sessions through the kernel runtime HTTP API \
                    (agent_id={agent_id}, user_ref={user_ref:?})"
                ),
            )),
            SdkRuntimeOperation::SessionList { .. }
            | SdkRuntimeOperation::SessionHistory { .. } => Err(SdkRuntimeError::new(
                "unsupported_operation",
                "provider Session discovery is not configured for this Rust provider runtime",
            )),
            SdkRuntimeOperation::SessionInterrupt { .. }
            | SdkRuntimeOperation::SessionCompact { .. }
            | SdkRuntimeOperation::SessionFork { .. } => Err(SdkRuntimeError::new(
                "unsupported_operation",
                "provider session control is not configured for this Rust provider runtime",
            )),
            SdkRuntimeOperation::SkillInvoke {
                skill_id,
                arguments,
            } => Err(SdkRuntimeError::new(
                "unsupported_operation",
                format!(
                    "SkillInvoke is not supported on rust_native transport; \
                     route skill execution through the provider worker transport \
                     (skill_id={skill_id}, arguments={arguments:?})"
                ),
            )),
        }
    }

    fn build_model_request(
        &self,
        model_request_id: &str,
        messages: &[String],
        _wire_messages: Option<&Value>,
        request: &SdkRuntimeRequest,
    ) -> ModelRequest {
        let mut model_request = ModelRequest::new(model_request_id.to_string(), messages.to_vec());
        let (operation_model_id, session_id, provider_session_id, turn_id, auth_token, access_token) =
            match &request.operation {
                SdkRuntimeOperation::ModelChat {
                    model_id,
                    session_id,
                    provider_session_id,
                    turn_id,
                    auth_token,
                    access_token,
                    ..
                }
                | SdkRuntimeOperation::ModelChatStream {
                    model_id,
                    session_id,
                    provider_session_id,
                    turn_id,
                    auth_token,
                    access_token,
                    ..
                } => (
                    model_id.as_deref(),
                    session_id.as_deref(),
                    provider_session_id.as_deref(),
                    turn_id.as_deref(),
                    auth_token.as_deref(),
                    access_token.as_deref(),
                ),
                _ => (None, None, None, None, None, None),
            };
        if let Some(model_id) = operation_model_id.or_else(|| {
            request
                .payload
                .as_ref()
                .and_then(|payload| payload.get("model_id"))
                .and_then(Value::as_str)
        }) {
            model_request = model_request.with_model_id(model_id);
        } else {
            model_request = model_request.with_model_id(&self.default_model);
        }
        if let Some(session_id) = session_id {
            model_request = model_request.for_session(session_id);
        }
        if let Some(provider_session_id) = provider_session_id {
            model_request = model_request.for_provider_session(provider_session_id);
        }
        if let Some(turn_id) = turn_id {
            model_request = model_request.for_step(turn_id);
        }
        if auth_token.is_some() || access_token.is_some() {
            model_request = model_request.for_caller(
                auth_token.map(str::to_string),
                access_token.map(str::to_string),
            );
        }
        model_request
    }

    fn invoke_model_chat(
        &self,
        model_request_id: &str,
        messages: &[String],
        wire_messages: Option<&Value>,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        let model_request =
            self.build_model_request(model_request_id, messages, wire_messages, request);
        let response = self.model.invoke(model_request).map_err(map_kernel_error)?;
        Ok(SdkRuntimeResponse::success(
            SdkBackendKind::RustNative,
            &request.capability_id,
            model_response_json(&response),
        ))
    }

    fn invoke_model_chat_stream(
        &self,
        model_request_id: &str,
        messages: &[String],
        wire_messages: Option<&Value>,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        let model_request =
            self.build_model_request(model_request_id, messages, wire_messages, request);
        let chunks = self.model.stream(model_request).map_err(map_kernel_error)?;
        Ok(SdkRuntimeResponse::success(
            SdkBackendKind::RustNative,
            &request.capability_id,
            stream_response_json(&chunks),
        ))
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
        let model_health = self.handler.model.health();
        if model_health.status == "available" {
            SdkDriverHealth::healthy()
        } else {
            SdkDriverHealth::degraded("model provider unavailable")
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        self.handler.invoke_inner(request)
    }

    fn cancel_inflight(&self, request_id: &str) -> Result<bool, SdkRuntimeError> {
        self.handler
            .model
            .cancel(request_id)
            .map(|_| true)
            .map_err(map_kernel_error)
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

fn stream_response_json(chunks: &[sdkwork_agent_kernel::ModelStreamChunk]) -> Value {
    json!({
        "chunks": chunks
            .iter()
            .map(|chunk| json!({
                "sequence": chunk.sequence,
                "content": chunk.content,
            }))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{KernelResult, ModelResponse, ProviderHealth, ProviderManifest};
    use sdkwork_agent_provider_spi::{SdkRuntimeOperation, SdkRuntimeRequest};

    struct StubModelProvider;

    impl ModelProvider for StubModelProvider {
        fn provider_manifest(&self) -> ProviderManifest {
            ProviderManifest::new("provider.stub", "model", "Stub", "0.1.0", vec![])
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
                "provider.stub",
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

    #[test]
    fn session_create_is_unsupported_on_rust_native_transport() {
        let handler = Arc::new(ProviderBackedRustHandler::new(
            Arc::new(StubModelProvider) as Arc<dyn ModelProvider + Send + Sync>,
            Arc::new(StubToolProvider) as Arc<dyn ToolProvider + Send + Sync>,
            "stub-model",
        ));
        let runtime = InProcessRustSdkRuntime::new(handler);
        let error = runtime
            .invoke(&SdkRuntimeRequest {
                capability_id: "sdk.session.create".to_string(),
                operation: SdkRuntimeOperation::SessionCreate {
                    agent_id: "agent.1".to_string(),
                    user_ref: Some("user.1".to_string()),
                },
                payload: None,
            })
            .expect_err("session create must fail closed on rust_native");
        assert_eq!(error.code, "unsupported_operation");
    }

    #[test]
    fn session_discovery_is_unsupported_on_generic_rust_native_transport() {
        let handler = Arc::new(ProviderBackedRustHandler::new(
            Arc::new(StubModelProvider) as Arc<dyn ModelProvider + Send + Sync>,
            Arc::new(StubToolProvider) as Arc<dyn ToolProvider + Send + Sync>,
            "stub-model",
        ));
        let runtime = InProcessRustSdkRuntime::new(handler);
        let operations = [
            SdkRuntimeOperation::SessionList {
                working_directory: None,
                cursor: None,
                limit: 20,
                source_kinds: None,
                section_id: None,
                archived: None,
                search_term: None,
                sort_key: None,
                sort_direction: None,
                model_providers: None,
            },
            SdkRuntimeOperation::SessionHistory {
                provider_session_id: "provider-session.1".to_string(),
                working_directory: None,
                cursor: None,
                limit: 20,
            },
        ];

        for operation in operations {
            let error = runtime
                .invoke(&SdkRuntimeRequest {
                    capability_id: "sdk.session.lifecycle".to_string(),
                    operation,
                    payload: None,
                })
                .expect_err("Session discovery must fail closed on generic rust_native");
            assert_eq!(error.code, "unsupported_operation");
        }
    }

    #[test]
    fn session_control_is_unsupported_on_generic_rust_native_transport() {
        let handler = Arc::new(ProviderBackedRustHandler::new(
            Arc::new(StubModelProvider) as Arc<dyn ModelProvider + Send + Sync>,
            Arc::new(StubToolProvider) as Arc<dyn ToolProvider + Send + Sync>,
            "stub-model",
        ));
        let runtime = InProcessRustSdkRuntime::new(handler);
        let operations = vec![
            SdkRuntimeOperation::SessionInterrupt {
                control_request_id: "control.interrupt".to_string(),
                session_id: "session.1".to_string(),
                provider_session_id: "provider-session.1".to_string(),
                policy_decision_id: "policy.1".to_string(),
                reason: None,
                working_directory: None,
                timeout_ms: None,
            },
            SdkRuntimeOperation::SessionCompact {
                control_request_id: "control.compact".to_string(),
                session_id: "session.1".to_string(),
                provider_session_id: "provider-session.1".to_string(),
                policy_decision_id: "policy.1".to_string(),
                focus: None,
                working_directory: None,
                timeout_ms: None,
            },
            SdkRuntimeOperation::SessionFork {
                control_request_id: "control.fork".to_string(),
                session_id: "session.1".to_string(),
                provider_session_id: "provider-session.1".to_string(),
                policy_decision_id: "policy.1".to_string(),
                before_message_id: None,
                working_directory: None,
                timeout_ms: None,
            },
        ];

        for operation in operations {
            let error = runtime
                .invoke(&SdkRuntimeRequest {
                    capability_id: "sdk.session.control".to_string(),
                    operation,
                    payload: None,
                })
                .expect_err("session control must fail closed on generic rust_native");
            assert_eq!(error.code, "unsupported_operation");
        }
    }

    #[test]
    fn skill_invoke_is_unsupported_on_rust_native_transport() {
        let handler = Arc::new(ProviderBackedRustHandler::new(
            Arc::new(StubModelProvider) as Arc<dyn ModelProvider + Send + Sync>,
            Arc::new(StubToolProvider) as Arc<dyn ToolProvider + Send + Sync>,
            "stub-model",
        ));
        let runtime = InProcessRustSdkRuntime::new(handler);
        let error = runtime
            .invoke(&SdkRuntimeRequest {
                capability_id: "sdk.skill.invoke".to_string(),
                operation: SdkRuntimeOperation::SkillInvoke {
                    skill_id: "skill.example".to_string(),
                    arguments: Some(r#"{"input":"hello"}"#.to_string()),
                },
                payload: None,
            })
            .expect_err("skill invoke must fail closed on rust_native");
        assert_eq!(error.code, "unsupported_operation");
    }
}
