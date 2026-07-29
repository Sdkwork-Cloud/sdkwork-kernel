//! Runtime invocation protocol for external agent SDK backends.

use crate::backend::SdkBackendKind;
use crate::driver::SdkDriverHealth;
use crate::negotiation::SdkCapabilityNegotiation;
use sdkwork_agent_provider_transport_ipc::TransportError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Optional execution and generation controls forwarded to an external model runtime.
///
/// The fields intentionally remain provider-neutral. A provider may choose not to
/// translate a generation control to its native CLI/API when that control is not
/// supported, but the runtime protocol must preserve it so that an adapter can make
/// that decision explicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SdkRuntimeExecutionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approvals_reviewer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_git_repo_check: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_live_provider: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
}

impl SdkRuntimeExecutionOptions {
    fn is_empty(&self) -> bool {
        self.approval_policy.is_none()
            && self.approvals_reviewer.is_none()
            && self.sandbox_mode.is_none()
            && self.full_auto.is_none()
            && self.skip_git_repo_check.is_none()
            && self.ephemeral.is_none()
            && self.require_live_provider.is_none()
            && self.max_output_bytes.is_none()
            && self.temperature.is_none()
            && self.top_p.is_none()
            && self.max_tokens.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SdkRuntimeOperation {
    Ping,
    SessionCreate {
        agent_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_ref: Option<String>,
    },
    ModelChat {
        model_request_id: String,
        messages: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wire_messages: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution_options: Option<SdkRuntimeExecutionOptions>,
    },
    ModelChatStream {
        model_request_id: String,
        messages: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wire_messages: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution_options: Option<SdkRuntimeExecutionOptions>,
    },
    ToolInvoke {
        tool_call_id: String,
        tool_id: String,
        arguments: String,
    },
    SkillInvoke {
        skill_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arguments: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkRuntimeOperationKind {
    Ping,
    SessionCreate,
    ModelChat,
    ModelChatStream,
    ToolInvoke,
    SkillInvoke,
}

impl SdkRuntimeOperation {
    pub fn kind(&self) -> SdkRuntimeOperationKind {
        match self {
            Self::Ping => SdkRuntimeOperationKind::Ping,
            Self::SessionCreate { .. } => SdkRuntimeOperationKind::SessionCreate,
            Self::ModelChat { .. } => SdkRuntimeOperationKind::ModelChat,
            Self::ModelChatStream { .. } => SdkRuntimeOperationKind::ModelChatStream,
            Self::ToolInvoke { .. } => SdkRuntimeOperationKind::ToolInvoke,
            Self::SkillInvoke { .. } => SdkRuntimeOperationKind::SkillInvoke,
        }
    }

    /// Returns the request-scoped identifier used for isolation and cancellation.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::ModelChat {
                model_request_id, ..
            }
            | Self::ModelChatStream {
                model_request_id, ..
            } => Some(model_request_id),
            Self::ToolInvoke { tool_call_id, .. } => Some(tool_call_id),
            Self::Ping | Self::SessionCreate { .. } | Self::SkillInvoke { .. } => None,
        }
    }
}

impl SdkRuntimeOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::SessionCreate => "session_create",
            Self::ModelChat => "model_chat",
            Self::ModelChatStream => "model_chat_stream",
            Self::ToolInvoke => "tool_invoke",
            Self::SkillInvoke => "skill_invoke",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdkRuntimeRequest {
    pub capability_id: String,
    pub operation: SdkRuntimeOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

impl SdkRuntimeRequest {
    pub fn ping(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            operation: SdkRuntimeOperation::Ping,
            payload: None,
        }
    }

    pub fn model_chat(
        capability_id: impl Into<String>,
        model_request_id: impl Into<String>,
        messages: Vec<String>,
    ) -> Self {
        Self::model_chat_with_wire(capability_id, model_request_id, messages, None)
    }

    pub fn model_chat_with_wire(
        capability_id: impl Into<String>,
        model_request_id: impl Into<String>,
        messages: Vec<String>,
        wire_messages: Option<Value>,
    ) -> Self {
        Self::model_chat_with_context(
            capability_id,
            model_request_id,
            messages,
            wire_messages,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn model_chat_with_context(
        capability_id: impl Into<String>,
        model_request_id: impl Into<String>,
        messages: Vec<String>,
        wire_messages: Option<Value>,
        model_id: Option<String>,
        session_id: Option<String>,
        working_directory: Option<String>,
        timeout_ms: Option<u64>,
        execution_options: Option<SdkRuntimeExecutionOptions>,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            operation: SdkRuntimeOperation::ModelChat {
                model_request_id: model_request_id.into(),
                messages,
                wire_messages,
                model_id: normalize_optional_string(model_id),
                session_id: normalize_optional_string(session_id),
                working_directory: normalize_optional_string(working_directory),
                timeout_ms,
                execution_options: normalize_execution_options(execution_options),
            },
            payload: None,
        }
    }

    pub fn model_chat_stream_with_wire(
        capability_id: impl Into<String>,
        model_request_id: impl Into<String>,
        messages: Vec<String>,
        wire_messages: Option<Value>,
    ) -> Self {
        Self::model_chat_stream_with_context(
            capability_id,
            model_request_id,
            messages,
            wire_messages,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn model_chat_stream_with_context(
        capability_id: impl Into<String>,
        model_request_id: impl Into<String>,
        messages: Vec<String>,
        wire_messages: Option<Value>,
        model_id: Option<String>,
        session_id: Option<String>,
        working_directory: Option<String>,
        timeout_ms: Option<u64>,
        execution_options: Option<SdkRuntimeExecutionOptions>,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            operation: SdkRuntimeOperation::ModelChatStream {
                model_request_id: model_request_id.into(),
                messages,
                wire_messages,
                model_id: normalize_optional_string(model_id),
                session_id: normalize_optional_string(session_id),
                working_directory: normalize_optional_string(working_directory),
                timeout_ms,
                execution_options: normalize_execution_options(execution_options),
            },
            payload: None,
        }
    }

    pub fn from_model_request(
        capability_id: impl Into<String>,
        request: &sdkwork_agent_kernel::ModelRequest,
    ) -> Result<Self, SdkRuntimeError> {
        let (model_request_id, messages, wire_messages) =
            sdkwork_agent_provider_core::build_model_chat_operation(request).map_err(|error| {
                SdkRuntimeError::new("invalid_model_request", error.to_string())
            })?;
        let execution_options = execution_options_from_model_request(request)?;
        Ok(Self::model_chat_with_context(
            capability_id,
            model_request_id,
            messages,
            wire_messages,
            request.model_id.clone(),
            request.session_id.clone(),
            metadata_string(request, "sdkwork.code_engine.working_directory"),
            request.timeout_ms,
            execution_options,
        ))
    }

    pub fn stream_from_model_request(
        capability_id: impl Into<String>,
        request: &sdkwork_agent_kernel::ModelRequest,
    ) -> Result<Self, SdkRuntimeError> {
        let (model_request_id, messages, wire_messages) =
            sdkwork_agent_provider_core::build_model_chat_operation(request).map_err(|error| {
                SdkRuntimeError::new("invalid_model_request", error.to_string())
            })?;
        let execution_options = execution_options_from_model_request(request)?;
        Ok(Self::model_chat_stream_with_context(
            capability_id,
            model_request_id,
            messages,
            wire_messages,
            request.model_id.clone(),
            request.session_id.clone(),
            metadata_string(request, "sdkwork.code_engine.working_directory"),
            request.timeout_ms,
            execution_options,
        ))
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn normalize_execution_options(
    options: Option<SdkRuntimeExecutionOptions>,
) -> Option<SdkRuntimeExecutionOptions> {
    options.filter(|options| !options.is_empty())
}

fn metadata_string(request: &sdkwork_agent_kernel::ModelRequest, key: &str) -> Option<String> {
    request
        .metadata_value(key)
        .and_then(|value| normalize_optional_string(Some(value.to_string())))
}

fn execution_options_from_model_request(
    request: &sdkwork_agent_kernel::ModelRequest,
) -> Result<Option<SdkRuntimeExecutionOptions>, SdkRuntimeError> {
    let options = SdkRuntimeExecutionOptions {
        approval_policy: metadata_string(request, "sdkwork.code_engine.approval_policy"),
        approvals_reviewer: metadata_string(request, "sdkwork.code_engine.approvals_reviewer"),
        sandbox_mode: metadata_string(request, "sdkwork.code_engine.sandbox_mode"),
        full_auto: parse_metadata_bool(request, "sdkwork.code_engine.full_auto")?,
        skip_git_repo_check: parse_metadata_bool(
            request,
            "sdkwork.code_engine.skip_git_repo_check",
        )?,
        ephemeral: parse_metadata_bool(request, "sdkwork.code_engine.ephemeral")?,
        require_live_provider: parse_metadata_bool(
            request,
            "sdkwork.code_engine.require_live_provider",
        )?,
        max_output_bytes: parse_metadata_u64(request, "sdkwork.code_engine.max_output_bytes")?,
        temperature: parse_metadata_f64(request, "sdkwork.code_engine.temperature")?,
        top_p: parse_metadata_f64(request, "sdkwork.code_engine.top_p")?,
        max_tokens: parse_metadata_u64(request, "sdkwork.code_engine.max_tokens")?,
    };
    Ok(normalize_execution_options(Some(options)))
}

fn metadata_value<'a>(
    request: &'a sdkwork_agent_kernel::ModelRequest,
    key: &str,
) -> Option<&'a str> {
    request
        .metadata_value(key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_metadata_bool(
    request: &sdkwork_agent_kernel::ModelRequest,
    key: &str,
) -> Result<Option<bool>, SdkRuntimeError> {
    let Some(value) = metadata_value(request, key) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => Err(SdkRuntimeError::new(
            "invalid_model_request_metadata",
            format!("metadata {key} must be true or false"),
        )),
    }
}

fn parse_metadata_u64(
    request: &sdkwork_agent_kernel::ModelRequest,
    key: &str,
) -> Result<Option<u64>, SdkRuntimeError> {
    let Some(value) = metadata_value(request, key) else {
        return Ok(None);
    };
    value.parse::<u64>().map(Some).map_err(|_| {
        SdkRuntimeError::new(
            "invalid_model_request_metadata",
            format!("metadata {key} must be an unsigned integer"),
        )
    })
}

fn parse_metadata_f64(
    request: &sdkwork_agent_kernel::ModelRequest,
    key: &str,
) -> Result<Option<f64>, SdkRuntimeError> {
    let Some(value) = metadata_value(request, key) else {
        return Ok(None);
    };
    let parsed = value.parse::<f64>().map_err(|_| {
        SdkRuntimeError::new(
            "invalid_model_request_metadata",
            format!("metadata {key} must be a number"),
        )
    })?;
    if !parsed.is_finite() {
        return Err(SdkRuntimeError::new(
            "invalid_model_request_metadata",
            format!("metadata {key} must be a finite number"),
        ));
    }
    Ok(Some(parsed))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkRuntimeResponse {
    pub success: bool,
    pub backend_kind: SdkBackendKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SdkRuntimeResponse {
    pub fn success(
        backend_kind: SdkBackendKind,
        capability_id: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            success: true,
            backend_kind,
            capability_id: Some(capability_id.into()),
            payload: Some(payload),
            message: None,
        }
    }

    pub fn failure(backend_kind: SdkBackendKind, message: impl Into<String>) -> Self {
        Self {
            success: false,
            backend_kind,
            capability_id: None,
            payload: None,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkRuntimeError {
    pub code: String,
    pub message: String,
}

impl SdkRuntimeError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn backend_unavailable(kind: SdkBackendKind) -> Self {
        Self {
            code: "backend_unavailable".to_string(),
            message: format!("no runtime registered for backend {}", kind.as_str()),
        }
    }

    pub fn capability_not_negotiated(capability_id: &str) -> Self {
        Self {
            code: "capability_not_negotiated".to_string(),
            message: format!("capability not negotiated: {capability_id}"),
        }
    }

    pub fn operation_not_supported(
        capability_id: &str,
        operation: SdkRuntimeOperationKind,
    ) -> Self {
        Self {
            code: "operation_not_supported".to_string(),
            message: format!(
                "capability {capability_id} does not declare runtime operation {}",
                operation.as_str()
            ),
        }
    }
}

impl std::fmt::Display for SdkRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SdkRuntimeError {}

/// Executes SDK capability operations through a concrete backend transport.
pub trait SdkBackendRuntime: Send + Sync {
    fn backend_kind(&self) -> SdkBackendKind;
    fn health(&self) -> SdkDriverHealth;
    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError>;

    /// Delivers incremental stream frames for streaming capability operations.
    fn invoke_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(serde_json::Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        let response = self.invoke(request)?;
        if !response.success {
            return Err(SdkRuntimeError::new(
                "runtime_failure",
                response
                    .message
                    .unwrap_or_else(|| "runtime stream invoke failed".to_string()),
            ));
        }
        let payload = response.payload.unwrap_or(serde_json::Value::Null);
        let sink_error = std::cell::RefCell::new(None);
        sdkwork_agent_provider_transport_ipc::expand_buffered_stream_payload(payload, |frame| {
            sink(frame).map_err(|error| {
                let message = error.message.clone();
                sink_error.replace(Some(error));
                TransportError::new(message)
            })
        })
        .map_err(|error| {
            sink_error
                .into_inner()
                .unwrap_or_else(|| SdkRuntimeError::new("stream_transport", error.message))
        })
    }

    fn cancel_inflight(&self, _request_id: &str) -> Result<bool, SdkRuntimeError> {
        Ok(false)
    }
}

/// Routes runtime requests to the negotiated backend implementation.
pub struct SdkRuntimeRouter {
    negotiation: SdkCapabilityNegotiation,
    rust_runtime: Option<std::sync::Arc<dyn SdkBackendRuntime>>,
    typescript_runtime: Option<std::sync::Arc<dyn SdkBackendRuntime>>,
    python_runtime: Option<std::sync::Arc<dyn SdkBackendRuntime>>,
    http_runtime: Option<std::sync::Arc<dyn SdkBackendRuntime>>,
    ipc_runtime: Option<std::sync::Arc<dyn SdkBackendRuntime>>,
}

impl SdkRuntimeRouter {
    pub fn new(negotiation: SdkCapabilityNegotiation) -> Self {
        Self {
            negotiation,
            rust_runtime: None,
            typescript_runtime: None,
            python_runtime: None,
            http_runtime: None,
            ipc_runtime: None,
        }
    }

    pub fn with_rust_runtime(mut self, runtime: std::sync::Arc<dyn SdkBackendRuntime>) -> Self {
        self.rust_runtime = Some(runtime);
        self
    }

    pub fn with_typescript_runtime(
        mut self,
        runtime: std::sync::Arc<dyn SdkBackendRuntime>,
    ) -> Self {
        self.typescript_runtime = Some(runtime);
        self
    }

    pub fn with_python_runtime(mut self, runtime: std::sync::Arc<dyn SdkBackendRuntime>) -> Self {
        self.python_runtime = Some(runtime);
        self
    }

    pub fn with_http_runtime(mut self, runtime: std::sync::Arc<dyn SdkBackendRuntime>) -> Self {
        self.http_runtime = Some(runtime);
        self
    }

    pub fn with_ipc_runtime(mut self, runtime: std::sync::Arc<dyn SdkBackendRuntime>) -> Self {
        self.ipc_runtime = Some(runtime);
        self
    }

    pub fn invoke(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        let selected = self
            .negotiation
            .selected_driver(&request.capability_id)
            .ok_or_else(|| SdkRuntimeError::capability_not_negotiated(&request.capability_id))?;
        validate_selected_operation(selected, request)?;

        let runtime = self
            .runtime_for(selected.backend_kind)
            .ok_or_else(|| SdkRuntimeError::backend_unavailable(selected.backend_kind))?;

        runtime.invoke(request)
    }

    pub fn invoke_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(serde_json::Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        let selected = self
            .negotiation
            .selected_driver(&request.capability_id)
            .ok_or_else(|| SdkRuntimeError::capability_not_negotiated(&request.capability_id))?;
        validate_selected_operation(selected, request)?;

        let runtime = self
            .runtime_for(selected.backend_kind)
            .ok_or_else(|| SdkRuntimeError::backend_unavailable(selected.backend_kind))?;

        runtime.invoke_streaming(request, sink)
    }

    pub fn cancel_inflight(
        &self,
        capability_id: &str,
        request_id: &str,
    ) -> Result<bool, SdkRuntimeError> {
        let selected = self
            .negotiation
            .selected_driver(capability_id)
            .ok_or_else(|| SdkRuntimeError::capability_not_negotiated(capability_id))?;

        let runtime = self
            .runtime_for(selected.backend_kind)
            .ok_or_else(|| SdkRuntimeError::backend_unavailable(selected.backend_kind))?;

        runtime.cancel_inflight(request_id)
    }

    fn runtime_for(&self, kind: SdkBackendKind) -> Option<&std::sync::Arc<dyn SdkBackendRuntime>> {
        match kind {
            SdkBackendKind::RustNative => self.rust_runtime.as_ref(),
            SdkBackendKind::TypeScriptNode => self.typescript_runtime.as_ref(),
            SdkBackendKind::PythonProcess => self.python_runtime.as_ref(),
            SdkBackendKind::HttpOpenApi => self.http_runtime.as_ref(),
            SdkBackendKind::IpcProtocol => self.ipc_runtime.as_ref(),
        }
    }
}

fn validate_selected_operation(
    selected: &crate::negotiation::NegotiatedCapability,
    request: &SdkRuntimeRequest,
) -> Result<(), SdkRuntimeError> {
    let operation = request.operation.kind();
    if selected.runtime_operations.contains(&operation) {
        return Ok(());
    }

    Err(SdkRuntimeError::operation_not_supported(
        &request.capability_id,
        operation,
    ))
}

/// Canonical router alias for provider transport dispatch.
pub type ProviderTransportRouter = SdkRuntimeRouter;

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::ModelRequest;
    use serde_json::json;

    #[test]
    fn legacy_model_chat_constructor_keeps_optional_context_omitted() {
        let request = SdkRuntimeRequest::model_chat(
            "sdk.model.chat",
            "req-legacy",
            vec!["hello".to_string()],
        );
        let value = serde_json::to_value(&request).expect("serialize runtime request");
        let operation = value.get("operation").expect("operation");

        assert_eq!(operation.get("operation"), Some(&json!("model_chat")));
        for key in [
            "model_id",
            "session_id",
            "working_directory",
            "timeout_ms",
            "execution_options",
        ] {
            assert!(operation.get(key).is_none(), "{key} should stay omitted");
        }
    }

    #[test]
    fn legacy_model_chat_json_deserializes_with_empty_optional_context() {
        let request: SdkRuntimeRequest = serde_json::from_value(json!({
            "capability_id": "sdk.model.chat",
            "operation": {
                "operation": "model_chat",
                "model_request_id": "req-legacy-json",
                "messages": ["hello"]
            }
        }))
        .expect("legacy request should deserialize");

        match request.operation {
            SdkRuntimeOperation::ModelChat {
                model_id,
                session_id,
                working_directory,
                timeout_ms,
                execution_options,
                ..
            } => {
                assert_eq!(model_id, None);
                assert_eq!(session_id, None);
                assert_eq!(working_directory, None);
                assert_eq!(timeout_ms, None);
                assert_eq!(execution_options, None);
            }
            other => panic!("expected model_chat, got {other:?}"),
        }
    }

    #[test]
    fn from_model_request_projects_code_engine_context_and_generation_options() {
        let request = ModelRequest::new("req-context", vec!["hello".to_string()])
            .with_model_id("codex-test")
            .for_session("provider-session")
            .with_timeout_ms(42_000)
            .with_metadata("sdkwork.code_engine.working_directory", " C:/workspace ")
            .with_metadata("sdkwork.code_engine.approval_policy", "on-request")
            .with_metadata("sdkwork.code_engine.sandbox_mode", "workspace-write")
            .with_metadata("sdkwork.code_engine.full_auto", "true")
            .with_metadata("sdkwork.code_engine.skip_git_repo_check", "false")
            .with_metadata("sdkwork.code_engine.ephemeral", "true")
            .with_metadata("sdkwork.code_engine.require_live_provider", "true")
            .with_metadata("sdkwork.code_engine.max_output_bytes", "1048576")
            .with_metadata("sdkwork.code_engine.temperature", "0.25")
            .with_metadata("sdkwork.code_engine.top_p", "0.9")
            .with_metadata("sdkwork.code_engine.max_tokens", "4096");

        let runtime_request =
            SdkRuntimeRequest::from_model_request("sdk.model.chat", &request).expect("projection");
        match runtime_request.operation {
            SdkRuntimeOperation::ModelChat {
                model_id,
                session_id,
                working_directory,
                timeout_ms,
                execution_options: Some(options),
                ..
            } => {
                assert_eq!(model_id.as_deref(), Some("codex-test"));
                assert_eq!(session_id.as_deref(), Some("provider-session"));
                assert_eq!(working_directory.as_deref(), Some("C:/workspace"));
                assert_eq!(timeout_ms, Some(42_000));
                assert_eq!(options.approval_policy.as_deref(), Some("on-request"));
                assert_eq!(options.sandbox_mode.as_deref(), Some("workspace-write"));
                assert_eq!(options.full_auto, Some(true));
                assert_eq!(options.skip_git_repo_check, Some(false));
                assert_eq!(options.ephemeral, Some(true));
                assert_eq!(options.require_live_provider, Some(true));
                assert_eq!(options.max_output_bytes, Some(1_048_576));
                assert_eq!(options.temperature, Some(0.25));
                assert_eq!(options.top_p, Some(0.9));
                assert_eq!(options.max_tokens, Some(4_096));
            }
            other => panic!("expected model_chat with execution options, got {other:?}"),
        }
    }

    #[test]
    fn stream_from_model_request_projects_the_same_context() {
        let request = ModelRequest::new("req-stream", vec!["hello".to_string()])
            .with_model_id("codex-stream")
            .for_session("stream-session")
            .with_timeout_ms(9_000)
            .with_metadata("sdkwork.code_engine.working_directory", "C:/stream")
            .with_metadata("sdkwork.code_engine.max_tokens", "512");

        let runtime_request =
            SdkRuntimeRequest::stream_from_model_request("sdk.model.chat", &request)
                .expect("stream projection");
        match runtime_request.operation {
            SdkRuntimeOperation::ModelChatStream {
                model_id,
                session_id,
                working_directory,
                timeout_ms,
                execution_options: Some(options),
                ..
            } => {
                assert_eq!(model_id.as_deref(), Some("codex-stream"));
                assert_eq!(session_id.as_deref(), Some("stream-session"));
                assert_eq!(working_directory.as_deref(), Some("C:/stream"));
                assert_eq!(timeout_ms, Some(9_000));
                assert_eq!(options.max_tokens, Some(512));
            }
            other => panic!("expected model_chat_stream with context, got {other:?}"),
        }
    }

    #[test]
    fn malformed_code_engine_metadata_returns_a_typed_error() {
        for (key, value) in [
            ("sdkwork.code_engine.full_auto", "sometimes"),
            ("sdkwork.code_engine.max_output_bytes", "lots"),
            ("sdkwork.code_engine.temperature", "NaN"),
        ] {
            let request = ModelRequest::new("req-invalid", vec!["hello".to_string()])
                .with_metadata(key, value);
            let error = SdkRuntimeRequest::from_model_request("sdk.model.chat", &request)
                .expect_err("invalid metadata should fail");
            assert_eq!(error.code, "invalid_model_request_metadata");
            assert!(error.message.contains(key));
        }
    }
}
