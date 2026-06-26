use sdkwork_agent_kernel::{
    AgentConfigValue, AgentConfiguration, KernelError, KernelResult, ModelRequest, ModelResponse,
    PolicyCategory, ToolCall, ToolResult,
};

use crate::ids;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigBackendMode {
    FailClosed,
    Live,
}

impl RigBackendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FailClosed => "fail_closed",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigBackendExecutionState {
    FailClosed,
    LivePending,
}

impl RigBackendExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FailClosed => "fail_closed",
            Self::LivePending => "live_pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackendExecutionStatus {
    pub mode: RigBackendMode,
    pub state: RigBackendExecutionState,
    pub fail_closed: bool,
    pub safe_reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigBackendBootstrapState {
    FailClosed,
    LivePending,
}

impl RigBackendBootstrapState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FailClosed => "fail_closed",
            Self::LivePending => "live_pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackendBootstrapPlan {
    pub backend_mode: RigBackendMode,
    pub state: RigBackendBootstrapState,
    pub provider_id: Option<String>,
    pub required_secret_refs: Vec<String>,
    pub secret_refs: Vec<(String, String)>,
    pub policy_categories: Vec<String>,
    pub fail_closed: bool,
    pub safe_summary: String,
}

impl RigBackendBootstrapPlan {
    pub fn execution_status(&self) -> RigBackendExecutionStatus {
        execution_status_for_mode(self.backend_mode)
    }

    pub fn secret_ref_value(&self, field_key: &str) -> Option<&str> {
        self.secret_refs
            .iter()
            .find(|(key, _)| key == field_key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackend {
    pub mode: RigBackendMode,
    config: RigBackendConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackendConfig {
    pub mode: RigBackendMode,
    pub provider_id: Option<String>,
    pub api_key_secret_ref: Option<String>,
}

impl RigBackendConfig {
    pub fn fail_closed() -> Self {
        Self {
            mode: RigBackendMode::FailClosed,
            provider_id: None,
            api_key_secret_ref: None,
        }
    }

    pub fn from_configuration(configuration: &AgentConfiguration) -> KernelResult<Self> {
        let mode = match configuration.value("runtime.rig.backend_mode") {
            Some(AgentConfigValue::String(value)) if value == "fail_closed" => {
                RigBackendMode::FailClosed
            }
            Some(AgentConfigValue::String(value)) if value == "live" => RigBackendMode::Live,
            Some(AgentConfigValue::String(value)) => {
                return Err(KernelError::validation(format!(
                    "unsupported Rig backend mode: {value}"
                )));
            }
            Some(_) => {
                return Err(KernelError::validation(
                    "runtime.rig.backend_mode must be a string",
                ));
            }
            None => RigBackendMode::FailClosed,
        };

        let provider_id = match configuration.value("llm.rig.provider_id") {
            Some(AgentConfigValue::String(value)) if !value.trim().is_empty() => {
                Some(value.clone())
            }
            Some(AgentConfigValue::String(_)) | None => None,
            Some(_) => {
                return Err(KernelError::validation(
                    "llm.rig.provider_id must be a string",
                ));
            }
        };

        let api_key_secret_ref = match configuration.value("llm.rig.api_key") {
            Some(AgentConfigValue::SecretRef(value)) => Some(value.clone()),
            Some(_) => {
                return Err(KernelError::validation(
                    "llm.rig.api_key must be a secret reference",
                ));
            }
            None => None,
        };

        if mode == RigBackendMode::Live && api_key_secret_ref.is_none() {
            return Err(KernelError::validation(
                "live Rig backend mode requires llm.rig.api_key secret reference",
            ));
        }

        Ok(Self {
            mode,
            provider_id,
            api_key_secret_ref,
        })
    }

    pub fn execution_status(&self) -> RigBackendExecutionStatus {
        execution_status_for_mode(self.mode)
    }

    pub fn bootstrap_plan(&self) -> RigBackendBootstrapPlan {
        let execution_status = self.execution_status();
        let mut required_secret_refs = Vec::new();
        let mut secret_refs = Vec::new();
        let mut policy_categories = vec![PolicyCategory::ModelInvoke.as_str().to_string()];

        if self.mode == RigBackendMode::Live {
            required_secret_refs.push("llm.rig.api_key".to_string());
            policy_categories.push(PolicyCategory::HostSecretsRead.as_str().to_string());
            if let Some(api_key_secret_ref) = self.api_key_secret_ref.clone() {
                secret_refs.push(("llm.rig.api_key".to_string(), api_key_secret_ref));
            }
        }

        RigBackendBootstrapPlan {
            backend_mode: self.mode,
            state: bootstrap_state_for_mode(self.mode),
            provider_id: self.provider_id.clone(),
            required_secret_refs,
            secret_refs,
            policy_categories,
            fail_closed: execution_status.fail_closed,
            safe_summary: safe_bootstrap_summary(self.mode),
        }
    }
}

impl RigBackend {
    pub fn fail_closed() -> Self {
        let config = RigBackendConfig::fail_closed();
        Self {
            mode: config.mode,
            config,
        }
    }

    pub fn from_config(config: RigBackendConfig) -> Self {
        Self {
            mode: config.mode,
            config,
        }
    }

    pub fn execution_status(&self) -> RigBackendExecutionStatus {
        execution_status_for_mode(self.mode)
    }

    pub fn bootstrap_plan(&self) -> RigBackendBootstrapPlan {
        self.config.bootstrap_plan()
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

fn execution_status_for_mode(mode: RigBackendMode) -> RigBackendExecutionStatus {
    match mode {
        RigBackendMode::FailClosed => RigBackendExecutionStatus {
            mode,
            state: RigBackendExecutionState::FailClosed,
            fail_closed: true,
            safe_reason: "Rig backend is fail-closed; no live model or tool execution adapter is connected"
                .to_string(),
        },
        RigBackendMode::Live => RigBackendExecutionStatus {
            mode,
            state: RigBackendExecutionState::LivePending,
            fail_closed: true,
            safe_reason: "Rig backend live mode is configured but the upstream execution adapter is not connected"
                .to_string(),
        },
    }
}

fn bootstrap_state_for_mode(mode: RigBackendMode) -> RigBackendBootstrapState {
    match mode {
        RigBackendMode::FailClosed => RigBackendBootstrapState::FailClosed,
        RigBackendMode::Live => RigBackendBootstrapState::LivePending,
    }
}

fn safe_bootstrap_summary(mode: RigBackendMode) -> String {
    match mode {
        RigBackendMode::FailClosed => {
            "Rig backend bootstrap plan stays fail-closed without live adapter configuration"
                .to_string()
        }
        RigBackendMode::Live => {
            "Rig backend bootstrap plan is live-pending and remains fail-closed until the upstream adapter is connected"
                .to_string()
        }
    }
}
