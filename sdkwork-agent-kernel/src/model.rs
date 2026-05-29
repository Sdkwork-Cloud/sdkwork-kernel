use crate::{
    KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    KernelResult, ProviderHealth, ProviderManifest, ToolCall, TraceContext,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelResponseFormat {
    Text,
    Json,
    JsonSchema(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    PolicyDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl ModelUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub model_request_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub messages: Vec<String>,
    pub response_format: Option<ModelResponseFormat>,
    pub policy_request_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
    pub metadata: Vec<(String, String)>,
}

impl ModelRequest {
    pub fn new(model_request_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            model_request_id: model_request_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            messages,
            response_format: None,
            policy_request_id: None,
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

    pub fn with_response_format(mut self, response_format: ModelResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn with_policy_context(mut self, policy_request_id: impl Into<String>) -> Self {
        self.policy_request_id = Some(policy_request_id.into());
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

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub model_request_id: String,
    pub provider_id: String,
    pub status: ModelStatus,
    pub messages: Vec<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<ModelUsage>,
    pub finish_reason: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub diagnostics: Vec<String>,
}

impl ModelResponse {
    pub fn text(
        model_request_id: impl Into<String>,
        provider_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            model_request_id: model_request_id.into(),
            provider_id: provider_id.into(),
            status: ModelStatus::Succeeded,
            messages: vec![message.into()],
            tool_calls: Vec::new(),
            usage: None,
            finish_reason: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            diagnostics: Vec::new(),
        }
    }

    pub fn cancelled(model_request_id: impl Into<String>, provider_id: impl Into<String>) -> Self {
        Self {
            model_request_id: model_request_id.into(),
            provider_id: provider_id.into(),
            status: ModelStatus::Cancelled,
            messages: Vec::new(),
            tool_calls: Vec::new(),
            usage: None,
            finish_reason: Some("cancelled".to_string()),
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: ModelStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_usage(mut self, usage: ModelUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_tool_call(mut self, tool_call: ToolCall) -> Self {
        self.tool_calls.push(tool_call);
        self
    }

    pub fn with_finish_reason(mut self, finish_reason: impl Into<String>) -> Self {
        self.finish_reason = Some(finish_reason.into());
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

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStreamChunk {
    pub model_request_id: String,
    pub sequence: u64,
    pub content: String,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
}

impl ModelStreamChunk {
    pub fn output(
        model_request_id: impl Into<String>,
        sequence: u64,
        content: impl Into<String>,
    ) -> Self {
        Self {
            model_request_id: model_request_id.into(),
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
            "agent.model.output.streamed",
            KernelEventSeverity::Info,
            format!(
                "model_request_id={};sequence={};chunk={}",
                self.model_request_id, self.sequence, self.content
            ),
        )
        .from_source(KernelEventSource::Model)
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.agent.model.stream_chunk.v1");

        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }
}

pub trait ModelProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse>;

    fn stream(&self, _request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "model.streaming".to_string(),
        })
    }

    fn cancel(&self, _model_request_id: &str) -> KernelResult<ModelResponse> {
        Err(KernelError::CapabilityMissing {
            capability_id: "model.cancellation".to_string(),
        })
    }
}
