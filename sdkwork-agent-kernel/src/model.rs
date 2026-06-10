use crate::{
    AgentRuntime, ContextFrame, KernelError, KernelEvent, KernelEventRedaction,
    KernelEventSeverity, KernelEventSource, KernelResult, PolicyCategory, PolicyDecision,
    PolicyDecisionValue, PolicyRequest, PolicySubject, ProviderHealth, ProviderManifest,
    RedactionClassification, SideEffectLevel, ToolCall, ToolDescriptor, TraceContext,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelResponseFormat {
    Text,
    Json,
    JsonSchema(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDescriptor {
    pub model_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub family: String,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub input_modes: Vec<String>,
    pub output_modes: Vec<String>,
    pub response_formats: Vec<ModelResponseFormat>,
    pub tool_capabilities: Vec<String>,
    pub policy_categories: Vec<String>,
    pub metadata: Vec<(String, String)>,
}

impl ModelDescriptor {
    pub fn new(
        model_id: impl Into<String>,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        family: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            family: family.into(),
            version: None,
            capabilities: Vec::new(),
            context_window_tokens: None,
            max_output_tokens: None,
            input_modes: Vec::new(),
            output_modes: Vec::new(),
            response_formats: Vec::new(),
            tool_capabilities: Vec::new(),
            policy_categories: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    pub fn with_context_window_tokens(mut self, context_window_tokens: u32) -> Self {
        self.context_window_tokens = Some(context_window_tokens);
        self
    }

    pub fn with_max_output_tokens(mut self, max_output_tokens: u32) -> Self {
        self.max_output_tokens = Some(max_output_tokens);
        self
    }

    pub fn with_input_mode(mut self, input_mode: impl Into<String>) -> Self {
        self.input_modes.push(input_mode.into());
        self
    }

    pub fn with_output_mode(mut self, output_mode: impl Into<String>) -> Self {
        self.output_modes.push(output_mode.into());
        self
    }

    pub fn with_response_format(mut self, response_format: ModelResponseFormat) -> Self {
        self.response_formats.push(response_format);
        self
    }

    pub fn with_tool_capability(mut self, tool_capability: impl Into<String>) -> Self {
        self.tool_capabilities.push(tool_capability.into());
        self
    }

    pub fn with_policy_category(mut self, policy_category: impl Into<String>) -> Self {
        self.policy_categories.push(policy_category.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|registered_capability| registered_capability == capability)
    }

    pub fn supports_response_format(&self, response_format: &ModelResponseFormat) -> bool {
        self.response_formats
            .iter()
            .any(|registered_format| registered_format == response_format)
    }

    pub fn requires_policy_for_sensitive_context(&self) -> bool {
        self.policy_categories
            .iter()
            .any(|category| category == "model.send_sensitive_context")
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
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
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub messages: Vec<String>,
    pub context_frame_ids: Vec<String>,
    pub context_frames: Vec<ContextFrame>,
    pub tool_descriptors: Vec<ToolDescriptor>,
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
            model_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            messages,
            context_frame_ids: Vec::new(),
            context_frames: Vec::new(),
            tool_descriptors: Vec::new(),
            response_format: None,
            policy_request_id: None,
            trace_context: None,
            timeout_ms: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
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

    pub fn with_context_frame(mut self, context_frame_id: impl Into<String>) -> Self {
        self.context_frame_ids.push(context_frame_id.into());
        self
    }

    pub fn with_context_frame_payload(mut self, context_frame: ContextFrame) -> Self {
        if !self
            .context_frame_ids
            .iter()
            .any(|frame_id| frame_id == &context_frame.context_frame_id)
        {
            self.context_frame_ids
                .push(context_frame.context_frame_id.clone());
        }
        self.context_frames.push(context_frame);
        self
    }

    pub fn with_tool_descriptor(mut self, tool_descriptor: ToolDescriptor) -> Self {
        self.tool_descriptors.push(tool_descriptor);
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

    pub fn with_model_parameter(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_metadata(format!("model.{}", key.into()), value)
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

    fn list_models(&self) -> Vec<ModelDescriptor> {
        Vec::new()
    }

    fn describe_model(&self, model_id: &str) -> KernelResult<ModelDescriptor> {
        self.list_models()
            .into_iter()
            .find(|model| model.model_id == model_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: model_id.to_string(),
            })
    }

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

    fn validate_structured_output(
        &self,
        _request: &ModelRequest,
        _response: &ModelResponse,
    ) -> KernelResult<ModelStructuredOutputValidation> {
        Err(KernelError::CapabilityMissing {
            capability_id: "model.structured_output".to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStructuredOutputValidation {
    pub model_request_id: String,
    pub schema_id: String,
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ModelStructuredOutputValidation {
    pub fn valid(model_request_id: impl Into<String>, schema_id: impl Into<String>) -> Self {
        Self {
            model_request_id: model_request_id.into(),
            schema_id: schema_id.into(),
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(
        model_request_id: impl Into<String>,
        schema_id: impl Into<String>,
        errors: Vec<String>,
    ) -> Self {
        Self {
            model_request_id: model_request_id.into(),
            schema_id: schema_id.into(),
            valid: false,
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExecutionRequest {
    pub model_execution_id: String,
    pub provider_id: Option<String>,
    pub subject: Option<PolicySubject>,
    pub model_request: ModelRequest,
}

impl ModelExecutionRequest {
    pub fn new(model_execution_id: impl Into<String>, model_request: ModelRequest) -> Self {
        Self {
            model_execution_id: model_execution_id.into(),
            provider_id: None,
            subject: None,
            model_request,
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_subject(mut self, subject: PolicySubject) -> Self {
        self.subject = Some(subject);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExecutionResponse {
    pub model_execution_id: String,
    pub provider_id: String,
    pub model_descriptor: Option<ModelDescriptor>,
    pub invoke_policy_decision: PolicyDecision,
    pub sensitive_context_policy_decision: Option<PolicyDecision>,
    pub structured_output_validation: Option<ModelStructuredOutputValidation>,
    pub model_response: ModelResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStreamExecutionResponse {
    pub model_execution_id: String,
    pub provider_id: String,
    pub model_descriptor: Option<ModelDescriptor>,
    pub invoke_policy_decision: PolicyDecision,
    pub sensitive_context_policy_decision: Option<PolicyDecision>,
    pub chunks: Vec<ModelStreamChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCancellationRequest {
    pub model_cancellation_id: String,
    pub provider_id: Option<String>,
    pub model_request_id: String,
}

impl ModelCancellationRequest {
    pub fn new(
        model_cancellation_id: impl Into<String>,
        model_request_id: impl Into<String>,
    ) -> Self {
        Self {
            model_cancellation_id: model_cancellation_id.into(),
            provider_id: None,
            model_request_id: model_request_id.into(),
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCancellationResponse {
    pub model_cancellation_id: String,
    pub provider_id: String,
    pub model_response: ModelResponse,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModelExecutionService;

impl ModelExecutionService {
    pub fn new() -> Self {
        Self
    }

    pub fn invoke(
        &self,
        runtime: &AgentRuntime,
        request: ModelExecutionRequest,
    ) -> KernelResult<ModelExecutionResponse> {
        let provider = self.select_provider(runtime, request.provider_id.as_deref())?;
        let provider_id = provider.provider_manifest().provider_id;
        let model_descriptor =
            self.resolve_model_descriptor(provider, request.model_request.model_id.as_deref())?;
        let (invoke_policy_decision, sensitive_context_policy_decision) = self
            .evaluate_model_policies(runtime, &request, &provider_id, model_descriptor.as_ref())?;
        let mut model_request = request.model_request;
        model_request = self.with_policy_metadata(
            model_request,
            &provider_id,
            &invoke_policy_decision,
            sensitive_context_policy_decision.as_ref(),
        );
        let validation_request = model_request.clone();
        let model_response = provider.invoke(model_request)?;
        let structured_output_validation =
            self.validate_structured_output(provider, &validation_request, &model_response)?;

        Ok(ModelExecutionResponse {
            model_execution_id: request.model_execution_id,
            provider_id,
            model_descriptor,
            invoke_policy_decision,
            sensitive_context_policy_decision,
            structured_output_validation,
            model_response,
        })
    }

    pub fn stream(
        &self,
        runtime: &AgentRuntime,
        request: ModelExecutionRequest,
    ) -> KernelResult<ModelStreamExecutionResponse> {
        let provider = self.select_provider(runtime, request.provider_id.as_deref())?;
        let provider_id = provider.provider_manifest().provider_id;
        let model_descriptor =
            self.resolve_model_descriptor(provider, request.model_request.model_id.as_deref())?;
        let (invoke_policy_decision, sensitive_context_policy_decision) = self
            .evaluate_model_policies(runtime, &request, &provider_id, model_descriptor.as_ref())?;
        let model_request = self.with_policy_metadata(
            request.model_request,
            &provider_id,
            &invoke_policy_decision,
            sensitive_context_policy_decision.as_ref(),
        );
        let chunks = provider.stream(model_request)?;

        Ok(ModelStreamExecutionResponse {
            model_execution_id: request.model_execution_id,
            provider_id,
            model_descriptor,
            invoke_policy_decision,
            sensitive_context_policy_decision,
            chunks,
        })
    }

    pub fn cancel(
        &self,
        runtime: &AgentRuntime,
        request: ModelCancellationRequest,
    ) -> KernelResult<ModelCancellationResponse> {
        let provider = self.select_provider(runtime, request.provider_id.as_deref())?;
        let provider_id = provider.provider_manifest().provider_id;
        let model_response = provider.cancel(&request.model_request_id)?;

        Ok(ModelCancellationResponse {
            model_cancellation_id: request.model_cancellation_id,
            provider_id,
            model_response,
        })
    }

    fn select_provider<'a>(
        &self,
        runtime: &'a AgentRuntime,
        provider_id: Option<&str>,
    ) -> KernelResult<&'a (dyn ModelProvider + Send + Sync)> {
        match provider_id {
            Some(provider_id) => runtime.model_provider_by_id(provider_id),
            None => runtime.model_provider(),
        }
    }

    fn evaluate_model_policies(
        &self,
        runtime: &AgentRuntime,
        request: &ModelExecutionRequest,
        provider_id: &str,
        model_descriptor: Option<&ModelDescriptor>,
    ) -> KernelResult<(PolicyDecision, Option<PolicyDecision>)> {
        let invoke_policy_request =
            self.invoke_policy_request(request, provider_id, model_descriptor);
        let invoke_policy_decision = runtime.policy_provider()?.evaluate(invoke_policy_request)?;
        self.ensure_allowed(&invoke_policy_decision)?;

        let sensitive_context_policy_decision =
            if self.requires_sensitive_context_policy(&request.model_request) {
                let sensitive_context_policy_request =
                    self.sensitive_context_policy_request(request, provider_id, model_descriptor);
                let decision = runtime
                    .policy_provider()?
                    .evaluate(sensitive_context_policy_request)?;
                self.ensure_allowed(&decision)?;
                Some(decision)
            } else {
                None
            };

        Ok((invoke_policy_decision, sensitive_context_policy_decision))
    }

    fn with_policy_metadata(
        &self,
        mut model_request: ModelRequest,
        provider_id: &str,
        invoke_policy_decision: &PolicyDecision,
        sensitive_context_policy_decision: Option<&PolicyDecision>,
    ) -> ModelRequest {
        model_request = model_request
            .with_metadata("sdkwork.model.provider_id", provider_id)
            .with_metadata(
                "sdkwork.model.policy_decision_id",
                invoke_policy_decision.decision_id.clone(),
            );
        if let Some(decision) = sensitive_context_policy_decision {
            model_request = model_request.with_metadata(
                "sdkwork.model.sensitive_context_policy_decision_id",
                decision.decision_id.clone(),
            );
        }

        model_request
    }

    fn validate_structured_output(
        &self,
        provider: &(dyn ModelProvider + Send + Sync),
        request: &ModelRequest,
        response: &ModelResponse,
    ) -> KernelResult<Option<ModelStructuredOutputValidation>> {
        let Some(ModelResponseFormat::JsonSchema(schema_id)) = &request.response_format else {
            return Ok(None);
        };

        let validation = provider.validate_structured_output(request, response)?;
        if validation.valid {
            Ok(Some(validation))
        } else {
            Err(KernelError::validation(format!(
                "structured model output did not match {schema_id}: {}",
                validation.errors.join("; ")
            )))
        }
    }

    fn resolve_model_descriptor(
        &self,
        provider: &(dyn ModelProvider + Send + Sync),
        model_id: Option<&str>,
    ) -> KernelResult<Option<ModelDescriptor>> {
        let catalog = provider.list_models();
        if catalog.is_empty() {
            return Ok(None);
        }

        match model_id {
            Some(model_id) => catalog
                .into_iter()
                .find(|descriptor| descriptor.model_id == model_id)
                .map(Some)
                .ok_or_else(|| KernelError::CapabilityMissing {
                    capability_id: model_id.to_string(),
                }),
            None => Ok(catalog.into_iter().next()),
        }
    }

    fn invoke_policy_request(
        &self,
        request: &ModelExecutionRequest,
        provider_id: &str,
        model_descriptor: Option<&ModelDescriptor>,
    ) -> PolicyRequest {
        let policy_request_id = request
            .model_request
            .policy_request_id
            .clone()
            .unwrap_or_else(|| format!("policy.{}.model.invoke", request.model_execution_id));
        let resource = model_descriptor
            .map(|descriptor| descriptor.model_id.as_str())
            .or(request.model_request.model_id.as_deref())
            .unwrap_or("model.default");
        self.base_policy_request(
            policy_request_id,
            PolicyCategory::ModelInvoke,
            "model.invoke",
            resource,
            request,
            provider_id,
        )
    }

    fn sensitive_context_policy_request(
        &self,
        request: &ModelExecutionRequest,
        provider_id: &str,
        model_descriptor: Option<&ModelDescriptor>,
    ) -> PolicyRequest {
        let policy_request_id = request
            .model_request
            .policy_request_id
            .clone()
            .map(|request_id| format!("{request_id}.sensitive_context"))
            .unwrap_or_else(|| {
                format!(
                    "policy.{}.model.send_sensitive_context",
                    request.model_execution_id
                )
            });
        let resource = model_descriptor
            .map(|descriptor| descriptor.model_id.as_str())
            .or(request.model_request.model_id.as_deref())
            .unwrap_or("model.default");
        self.base_policy_request(
            policy_request_id,
            PolicyCategory::ModelSendSensitiveContext,
            "model.send_sensitive_context",
            resource,
            request,
            provider_id,
        )
        .with_redaction(self.sensitive_context_redaction(&request.model_request))
    }

    fn base_policy_request(
        &self,
        policy_request_id: impl Into<String>,
        category: PolicyCategory,
        action: &str,
        resource: &str,
        request: &ModelExecutionRequest,
        provider_id: &str,
    ) -> PolicyRequest {
        let mut policy_request = PolicyRequest::new(policy_request_id, category.as_str(), resource)
            .with_category(category)
            .with_action(action)
            .with_side_effect_level(SideEffectLevel::ExternalSend)
            .with_context("provider_id", provider_id.to_string());

        if let Some(subject) = &request.subject {
            policy_request = policy_request.with_subject(subject.clone());
        }

        if let Some(session_id) = &request.model_request.session_id {
            policy_request = policy_request.with_session(session_id.clone());
        }

        if let Some(task_id) = &request.model_request.task_id {
            policy_request = policy_request.with_task(task_id.clone());
        }

        if let Some(run_id) = &request.model_request.run_id {
            policy_request = policy_request.with_run(run_id.clone());
        }

        if let Some(step_id) = &request.model_request.step_id {
            policy_request = policy_request.with_context("step_id", step_id.clone());
        }

        if let Some(model_id) = &request.model_request.model_id {
            policy_request = policy_request.with_context("model_id", model_id.clone());
        }

        policy_request
    }

    fn requires_sensitive_context_policy(&self, request: &ModelRequest) -> bool {
        request
            .context_frames
            .iter()
            .any(|frame| frame.redaction_classification.requires_redaction())
    }

    fn sensitive_context_redaction(&self, request: &ModelRequest) -> KernelEventRedaction {
        if request
            .context_frames
            .iter()
            .any(|frame| frame.redaction_classification == RedactionClassification::Secret)
        {
            KernelEventRedaction::Secret
        } else if request
            .context_frames
            .iter()
            .any(|frame| frame.redaction_classification == RedactionClassification::Regulated)
        {
            KernelEventRedaction::Regulated
        } else if request
            .context_frames
            .iter()
            .any(|frame| frame.redaction_classification == RedactionClassification::PersonalData)
        {
            KernelEventRedaction::PersonalData
        } else if request
            .context_frames
            .iter()
            .any(|frame| frame.redaction_classification == RedactionClassification::TenantSensitive)
        {
            KernelEventRedaction::TenantSensitive
        } else {
            KernelEventRedaction::Internal
        }
    }

    fn ensure_allowed(&self, policy_decision: &PolicyDecision) -> KernelResult<()> {
        match policy_decision.decision {
            PolicyDecisionValue::Allow => Ok(()),
            PolicyDecisionValue::Deny => Err(KernelError::PolicyDenied {
                reason_code: policy_decision.reason_code.clone(),
            }),
            PolicyDecisionValue::NeedsApproval => Err(KernelError::permission_required(
                policy_decision
                    .safe_reason
                    .clone()
                    .unwrap_or_else(|| policy_decision.reason_code.clone()),
            )),
            PolicyDecisionValue::Defer => Err(KernelError::provider_error(
                "policy.deferred",
                policy_decision.reason_code.clone(),
            )),
        }
    }
}
