use sdkwork_agent_kernel::{
    KernelError, KernelResult, ModelRequest, ModelResponse, ToolCall, ToolResult,
};

use crate::ids;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigBackendMode {
    FailClosed,
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackend {
    pub mode: RigBackendMode,
}

impl RigBackend {
    pub fn fail_closed() -> Self {
        Self {
            mode: RigBackendMode::FailClosed,
        }
    }

    pub fn invoke_model(&self, _request: ModelRequest) -> KernelResult<ModelResponse> {
        match self.mode {
            RigBackendMode::FailClosed => Err(KernelError::ProviderUnavailable {
                provider_id: ids::MODEL_PROVIDER_ID.to_string(),
            }),
            RigBackendMode::Live => Err(KernelError::ProviderUnavailable {
                provider_id: ids::MODEL_PROVIDER_ID.to_string(),
            }),
        }
    }

    pub fn invoke_tool(&self, call: ToolCall) -> ToolResult {
        match self.mode {
            RigBackendMode::FailClosed | RigBackendMode::Live => {
                ToolResult::failed(call.tool_call_id, "rig backend is fail-closed")
                    .with_status(sdkwork_agent_kernel::ToolCallStatus::Denied)
            }
        }
    }
}
