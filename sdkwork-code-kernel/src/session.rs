use crate::{CodeTask, Workspace};
use sdkwork_agent_kernel::{KernelError, KernelResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSession {
    pub session_id: String,
    pub workspace: Workspace,
    pub state: CodeSessionState,
    pub tasks: Vec<CodeTask>,
    pub provider_bindings: Vec<CodeProviderBinding>,
}

impl CodeSession {
    pub fn new(session_id: impl Into<String>, workspace: Workspace) -> Self {
        Self {
            session_id: session_id.into(),
            workspace,
            state: CodeSessionState::Created,
            tasks: Vec::new(),
            provider_bindings: Vec::new(),
        }
    }

    pub fn add_task(mut self, task: CodeTask) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn with_provider_binding(mut self, provider_binding: CodeProviderBinding) -> Self {
        self.provider_bindings.push(provider_binding);
        self
    }

    pub fn transition(mut self, next_state: CodeSessionState) -> KernelResult<Self> {
        if !is_valid_code_session_transition(self.state, next_state) {
            return Err(KernelError::validation("invalid code session transition"));
        }

        self.state = next_state;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeSessionState {
    Created,
    Active,
    Paused,
    Closing,
    Closed,
    Failed,
}

impl CodeSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Closing => "closing",
            Self::Closed => "closed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeProviderBinding {
    pub provider_family: String,
    pub provider_id: String,
    pub capabilities: Vec<String>,
}

impl CodeProviderBinding {
    pub fn new(provider_family: impl Into<String>, provider_id: impl Into<String>) -> Self {
        Self {
            provider_family: provider_family.into(),
            provider_id: provider_id.into(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.capabilities.push(capability_id.into());
        self
    }
}

fn is_valid_code_session_transition(current: CodeSessionState, next: CodeSessionState) -> bool {
    use CodeSessionState::{Active, Closed, Closing, Created, Failed, Paused};

    matches!(
        (current, next),
        (Created, Active | Failed | Closed)
            | (Active, Paused | Closing | Failed)
            | (Paused, Active | Closing | Failed)
            | (Closing, Closed | Failed)
    )
}
