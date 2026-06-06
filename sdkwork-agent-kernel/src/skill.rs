use crate::{
    KernelError, KernelEventRedaction, KernelResult, ProviderHealth, ProviderManifest,
    SideEffectLevel, ToolSchema, TraceContext,
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
    pub provider_id: String,
    pub display_name: String,
    pub description: String,
    pub invocation_mode: AgentSkillInvocationMode,
    pub version: Option<String>,
    pub model_hint: Option<String>,
    pub allowed_tools: Vec<String>,
    pub input_schema: Option<ToolSchema>,
    pub output_schema: Option<ToolSchema>,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub cancellation_supported: bool,
    pub audit_required: bool,
    pub metadata: Vec<(String, String)>,
}

impl AgentSkillDescriptor {
    pub fn new(
        skill_id: impl Into<String>,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        invocation_mode: AgentSkillInvocationMode,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            description: description.into(),
            invocation_mode,
            version: None,
            model_hint: None,
            allowed_tools: Vec::new(),
            input_schema: None,
            output_schema: None,
            side_effect_level: SideEffectLevel::SideEffectful,
            policy_categories: vec!["skill.invoke".to_string()],
            timeout_ms: None,
            cancellation_supported: false,
            audit_required: false,
            metadata: Vec::new(),
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

    pub fn with_input_schema(mut self, input_schema: ToolSchema) -> Self {
        self.input_schema = Some(input_schema);
        self
    }

    pub fn with_output_schema(mut self, output_schema: ToolSchema) -> Self {
        self.output_schema = Some(output_schema);
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

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn supports_cancellation(mut self, cancellation_supported: bool) -> Self {
        self.cancellation_supported = cancellation_supported;
        self
    }

    pub fn require_audit(mut self) -> Self {
        self.audit_required = true;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn requires_policy(&self) -> bool {
        self.side_effect_level != SideEffectLevel::ReadOnly || !self.policy_categories.is_empty()
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillRequest {
    pub skill_request_id: String,
    pub skill_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub arguments: Vec<(String, String)>,
    pub policy_decision_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
    pub metadata: Vec<(String, String)>,
}

impl AgentSkillRequest {
    pub fn new(skill_request_id: impl Into<String>, skill_id: impl Into<String>) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            arguments: Vec::new(),
            policy_decision_id: None,
            trace_context: None,
            timeout_ms: None,
            metadata: Vec::new(),
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

    pub fn for_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.push((key.into(), value.into()));
        self
    }

    pub fn with_policy_context(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn argument_value(&self, key: &str) -> Option<&str> {
        self.arguments
            .iter()
            .find(|(argument_key, _)| argument_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
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
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub audit_refs: Vec<String>,
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
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
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
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: AgentSkillStatus) -> Self {
        self.status = status;
        self
    }

    pub fn started_at(mut self, started_at: impl Into<String>) -> Self {
        self.started_at = Some(started_at.into());
        self
    }

    pub fn completed_at(mut self, completed_at: impl Into<String>) -> Self {
        self.completed_at = Some(completed_at.into());
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_audit_ref(mut self, audit_ref: impl Into<String>) -> Self {
        self.audit_refs.push(audit_ref.into());
        self
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
