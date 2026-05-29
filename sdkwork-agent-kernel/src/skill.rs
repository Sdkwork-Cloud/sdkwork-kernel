use crate::{
    KernelError, KernelResult, ProviderHealth, ProviderManifest, SideEffectLevel, TraceContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSkillInvocationMode {
    ModelInvocable,
    ToolBacked,
    Workflow,
    HostProvided,
}

impl AgentSkillInvocationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelInvocable => "model_invocable",
            Self::ToolBacked => "tool_backed",
            Self::Workflow => "workflow",
            Self::HostProvided => "host_provided",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillDescriptor {
    pub skill_id: String,
    pub display_name: String,
    pub description: String,
    pub invocation_mode: AgentSkillInvocationMode,
    pub version: Option<String>,
    pub model_hint: Option<String>,
    pub allowed_tools: Vec<String>,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
}

impl AgentSkillDescriptor {
    pub fn new(
        skill_id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        invocation_mode: AgentSkillInvocationMode,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            display_name: display_name.into(),
            description: description.into(),
            invocation_mode,
            version: None,
            model_hint: None,
            allowed_tools: Vec::new(),
            side_effect_level: SideEffectLevel::SideEffectful,
            policy_categories: vec!["skill.invoke".to_string()],
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_model_hint(mut self, model_hint: impl Into<String>) -> Self {
        self.model_hint = Some(model_hint.into());
        self
    }

    pub fn with_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }

    pub fn with_side_effect_level(mut self, side_effect_level: SideEffectLevel) -> Self {
        self.side_effect_level = side_effect_level;
        self
    }

    pub fn with_policy_category(mut self, policy_category: impl Into<String>) -> Self {
        self.policy_categories.push(policy_category.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillRequest {
    pub skill_request_id: String,
    pub skill_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub arguments: Vec<(String, String)>,
    pub trace_context: Option<TraceContext>,
}

impl AgentSkillRequest {
    pub fn new(skill_request_id: impl Into<String>, skill_id: impl Into<String>) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            arguments: Vec::new(),
            trace_context: None,
        }
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn for_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.push((key.into(), value.into()));
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn argument_value(&self, key: &str) -> Option<&str> {
        self.arguments
            .iter()
            .find(|(argument_key, _)| argument_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSkillStatus {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    PolicyDenied,
}

impl AgentSkillStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::PolicyDenied => "policy_denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillResult {
    pub skill_request_id: String,
    pub skill_id: String,
    pub status: AgentSkillStatus,
    pub output: String,
    pub error: Option<String>,
    pub diagnostics: Vec<String>,
}

impl AgentSkillResult {
    pub fn succeeded(
        skill_request_id: impl Into<String>,
        skill_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            status: AgentSkillStatus::Succeeded,
            output: output.into(),
            error: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(
        skill_request_id: impl Into<String>,
        skill_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            status: AgentSkillStatus::Failed,
            output: String::new(),
            error: Some(error.into()),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

pub trait AgentSkillProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn list_skills(&self) -> Vec<AgentSkillDescriptor>;

    fn describe_skill(&self, skill_id: &str) -> KernelResult<AgentSkillDescriptor> {
        self.list_skills()
            .into_iter()
            .find(|skill| skill.skill_id == skill_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: skill_id.to_string(),
            })
    }

    fn invoke_skill(&self, request: AgentSkillRequest) -> KernelResult<AgentSkillResult>;

    fn cancel_skill(&self, skill_request_id: &str) -> KernelResult<AgentSkillResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: format!("skill.cancel.{skill_request_id}"),
        })
    }
}
