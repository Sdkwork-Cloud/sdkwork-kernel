//! Runtime invocation protocol for external agent SDK backends.

use crate::backend::SdkBackendKind;
use crate::driver::SdkDriverHealth;
use crate::negotiation::SdkCapabilityNegotiation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        Self {
            capability_id: capability_id.into(),
            operation: SdkRuntimeOperation::ModelChat {
                model_request_id: model_request_id.into(),
                messages,
            },
            payload: None,
        }
    }
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

        let runtime = self
            .runtime_for(selected.backend_kind)
            .ok_or_else(|| SdkRuntimeError::backend_unavailable(selected.backend_kind))?;

        runtime.invoke(request)
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

/// Canonical router alias for provider transport dispatch.
pub type ProviderTransportRouter = SdkRuntimeRouter;
