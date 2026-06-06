use crate::{
    KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, ProviderManifest, TraceContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectLevel {
    ReadOnly,
    SideEffectful,
    Destructive,
    ExternalSend,
    Privileged,
}

impl SideEffectLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::SideEffectful => "side_effectful",
            Self::Destructive => "destructive",
            Self::ExternalSend => "external_send",
            Self::Privileged => "privileged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub tool_id: String,
    pub provider_id: String,
    pub name: Option<String>,
    pub display_name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub input_schema: Option<ToolSchema>,
    pub output_schema: Option<ToolSchema>,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub cancellation_supported: bool,
    pub audit_required: bool,
}

impl ToolDescriptor {
    pub fn new(
        tool_id: impl Into<String>,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        side_effect_level: SideEffectLevel,
    ) -> Self {
        Self {
            tool_id: tool_id.into(),
            provider_id: provider_id.into(),
            name: None,
            display_name: display_name.into(),
            version: None,
            description: None,
            input_schema: None,
            output_schema: None,
            side_effect_level,
            policy_categories: Vec::new(),
            timeout_ms: None,
            cancellation_supported: false,
            audit_required: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
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

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
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

    pub fn requires_policy(&self) -> bool {
        self.side_effect_level != SideEffectLevel::ReadOnly || !self.policy_categories.is_empty()
    }

    pub fn policy_request(
        &self,
        policy_request_id: impl Into<String>,
        call: &ToolCall,
    ) -> PolicyRequest {
        let category = self
            .policy_categories
            .first()
            .cloned()
            .unwrap_or_else(|| "tool.invoke".to_string());
        let mut request =
            PolicyRequest::new(policy_request_id, category.clone(), self.tool_id.clone())
                .with_category(PolicyCategory::ProductSpecific(category))
                .with_action("invoke")
                .with_side_effect_level(self.side_effect_level)
                .with_redaction(KernelEventRedaction::Internal);

        if let Some(session_id) = &call.session_id {
            request = request.with_session(session_id.clone());
        }

        if let Some(task_id) = &call.task_id {
            request = request.with_task(task_id.clone());
        }

        if let Some(run_id) = &call.run_id {
            request = request.with_run(run_id.clone());
        }

        request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSchema {
    pub schema_id: String,
}

impl ToolSchema {
    pub fn json_schema(schema_id: impl Into<String>) -> Self {
        Self {
            schema_id: schema_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub tool_id: String,
    pub provider_id: Option<String>,
    pub arguments: String,
    pub trace_context: Option<TraceContext>,
    pub policy_decision_id: Option<String>,
    pub timeout_ms: Option<u64>,
    pub created_at: Option<String>,
}

impl ToolCall {
    pub fn new(
        tool_call_id: impl Into<String>,
        tool_id: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            tool_id: tool_id.into(),
            provider_id: None,
            arguments: arguments.into(),
            trace_context: None,
            policy_decision_id: None,
            timeout_ms: None,
            created_at: None,
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

    pub fn with_provider(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_policy_decision(mut self, policy_decision_id: impl Into<String>) -> Self {
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

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub status: String,
    pub normalized_status: ToolCallStatus,
    pub output: String,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub audit_refs: Vec<String>,
}

impl ToolResult {
    pub fn succeeded(tool_call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            status: "succeeded".to_string(),
            normalized_status: ToolCallStatus::Succeeded,
            output: output.into(),
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
        }
    }

    pub fn failed(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            status: "failed".to_string(),
            normalized_status: ToolCallStatus::Failed,
            output: String::new(),
            error: Some(error.into()),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: ToolCallStatus) -> Self {
        self.status = status.as_str().to_string();
        self.normalized_status = status;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Denied,
    InvalidInput,
}

impl ToolCallStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Denied => "denied",
            Self::InvalidInput => "invalid_input",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStreamChunk {
    pub tool_call_id: String,
    pub sequence: u64,
    pub content: String,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
}

impl ToolStreamChunk {
    pub fn output(
        tool_call_id: impl Into<String>,
        sequence: u64,
        content: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            sequence,
            content: content.into(),
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            "agent.tool.call.output_streamed",
            KernelEventSeverity::Info,
            format!(
                "tool_call_id={};sequence={};chunk={}",
                self.tool_call_id, self.sequence, self.content
            ),
        )
        .from_source(KernelEventSource::Tool)
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.agent.tool.stream_chunk.v1");

        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }
}

pub trait ToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        let provider_id = self
            .list_tools()
            .first()
            .map(|descriptor| descriptor.provider_id.clone())
            .unwrap_or_else(|| "provider.tool.unspecified".to_string());

        ProviderManifest::new(
            provider_id,
            "tool",
            "tool-provider",
            "0.0.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor>;

    fn health(&self) -> ProviderHealth;

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult>;

    fn describe_tool(&self, tool_id: &str) -> KernelResult<ToolDescriptor> {
        self.list_tools()
            .into_iter()
            .find(|descriptor| descriptor.tool_id == tool_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: tool_id.to_string(),
            })
    }

    fn authorize_tool_call(
        &self,
        descriptor: &ToolDescriptor,
        call: &ToolCall,
    ) -> KernelResult<PolicyRequest> {
        Ok(descriptor.policy_request(format!("policy-request.{}", call.tool_call_id), call))
    }

    fn stream_tool_call(&self, _call: ToolCall) -> KernelResult<Vec<ToolStreamChunk>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "tool.streaming".to_string(),
        })
    }

    fn cancel_tool_call(&self, _tool_call_id: &str) -> KernelResult<ToolResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "tool.cancellation".to_string(),
        })
    }
}
