use crate::{
    agent_messages_to_text_lines, execute_with_retry, AgentChatKnowledgeQuery,
    AgentChatMemoryQuery, AgentChatRequest, AgentChatResponse, AgentChatService,
    AgentInputContract, AgentInputPolicy, AgentMessage, AgentMessageRole, AgentRuntime,
    AgentStreamEvent, AgentStreamSink, CancellationToken, EndedEvent, ErrorEvent, KernelError,
    KernelErrorKind, KernelErrorSource, KernelEvent, KernelEventRedaction, KernelEventSeverity,
    KernelEventSource, KernelResult, KnowledgeRetrievalMethod, McpToolExecutionRequest,
    McpToolExecutionResponse, McpToolExecutionService, MessageStartEvent, MessageStopEvent,
    ModelResponse, ModelUsage, Plan, PolicySubject, ResultEvent, RetryConfig, RuntimeState,
    SessionInitEvent, ToolCall, ToolCallStartEvent, ToolCallStopEvent, ToolExecutionRequest,
    ToolExecutionResponse, ToolExecutionService, ToolResultEvent, TraceContext, UsageEvent,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionStatus {
    Completed,
    Failed,
    PermissionRequired,
    Cancelled,
    Degraded,
}

impl AgentExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::PermissionRequired => "permission_required",
            Self::Cancelled => "cancelled",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionResumeDecision {
    Approved,
    Rejected,
}

impl AgentExecutionResumeDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionResumeRequest {
    pub resume_request_id: String,
    pub execution_id: String,
    pub decision: AgentExecutionResumeDecision,
    pub approved_by: Option<String>,
    pub comment: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub permission_error_kind: Option<String>,
    pub permission_error_code: Option<String>,
}

impl AgentExecutionResumeRequest {
    pub fn new(
        resume_request_id: impl Into<String>,
        execution_id: impl Into<String>,
        decision: AgentExecutionResumeDecision,
    ) -> KernelResult<Self> {
        let request = Self {
            resume_request_id: resume_request_id.into(),
            execution_id: execution_id.into(),
            decision,
            approved_by: None,
            comment: None,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            trace_context: None,
            permission_error_kind: None,
            permission_error_code: None,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        let comment = comment.into();
        if !comment.trim().is_empty() {
            self.comment = Some(comment);
        }
        self
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            format!("agent.execution.resume.{}", self.decision.as_str()),
            self.event_severity(),
            self.event_payload(),
        )
        .from_source(KernelEventSource::Policy)
        .with_redaction(KernelEventRedaction::Internal)
        .with_payload_schema("sdkwork.agent.execution.resume_request.v1");

        if let Some(session_id) = &self.session_id {
            event = event.for_session(session_id.clone());
        }
        if let Some(task_id) = &self.task_id {
            event = event.for_task(task_id.clone());
        }
        if let Some(run_id) = &self.run_id {
            event = event.for_run(run_id.clone());
        }
        if let Some(step_id) = &self.step_id {
            event = event.for_step(step_id.clone());
        }
        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }

    fn validate(&self) -> KernelResult<()> {
        if self.resume_request_id.trim().is_empty() {
            return Err(KernelError::validation(
                "resume request id must not be empty",
            ));
        }

        if self.execution_id.trim().is_empty() {
            return Err(KernelError::validation("execution id must not be empty"));
        }

        if self.decision == AgentExecutionResumeDecision::Approved
            && match self.approved_by.as_deref() {
                Some(approved_by) => approved_by.trim().is_empty(),
                None => true,
            }
        {
            return Err(KernelError::validation(
                "approval actor is required for approved resume",
            ));
        }

        Ok(())
    }

    fn event_severity(&self) -> KernelEventSeverity {
        match self.decision {
            AgentExecutionResumeDecision::Approved => KernelEventSeverity::Info,
            AgentExecutionResumeDecision::Rejected => KernelEventSeverity::Warn,
        }
    }

    fn event_payload(&self) -> String {
        let mut fields = vec![
            format!("resume_request_id={}", self.resume_request_id),
            format!("execution_id={}", self.execution_id),
            format!("decision={}", self.decision.as_str()),
            format!("approved_by={}", self.approved_by.as_deref().unwrap_or("")),
        ];

        if let Some(comment) = &self.comment {
            fields.push(format!("comment={comment}"));
        }
        if let Some(error_kind) = &self.permission_error_kind {
            fields.push(format!("permission_error_kind={error_kind}"));
        }
        if let Some(error_code) = &self.permission_error_code {
            fields.push(format!("permission_error_code={error_code}"));
        }

        fields.join(";")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentObservation {
    pub observation_id: String,
    pub source_family: String,
    pub action_id: Option<String>,
    pub status: String,
    pub summary: String,
    pub redaction_classification: KernelEventRedaction,
    pub metadata: Vec<(String, String)>,
}

impl AgentObservation {
    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        KernelEvent::new(
            event_id,
            format!("agent.execution.observation.{}", self.source_family),
            observation_event_severity(&self.status),
            self.event_payload(),
        )
        .from_source(observation_event_source(&self.source_family))
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.agent.execution.observation.v1")
    }

    fn event_payload(&self) -> String {
        let mut fields = vec![
            format!("observation_id={}", self.observation_id),
            format!("source_family={}", self.source_family),
            format!("status={}", self.status),
            format!("summary={}", self.summary),
        ];

        if let Some(action_id) = &self.action_id {
            fields.push(format!("action_id={action_id}"));
        }

        for (key, value) in &self.metadata {
            fields.push(format!("{key}={value}"));
        }

        fields.join(";")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentExecutionRequest {
    pub execution_id: String,
    pub messages: Vec<String>,
    pub input_messages: Vec<AgentMessage>,
    pub input_policy: Option<AgentInputPolicy>,
    pub input_contract: Option<AgentInputContract>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub subject: Option<PolicySubject>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
    pub include_tool_descriptors: bool,
    pub memory_query: Option<AgentChatMemoryQuery>,
    pub knowledge_query: Option<AgentChatKnowledgeQuery>,
    pub mcp_server_id: Option<String>,
    pub metadata: Vec<(String, String)>,
    /// Cooperative cancellation token checked at phase boundaries.
    pub cancellation_token: Option<CancellationToken>,
    /// Relative execution deadline in milliseconds, enforced at phase
    /// boundaries (before the model round and before each tool call).
    pub deadline_ms: Option<u64>,
    /// Retry policy for the model round; `None` disables retries.
    pub retry: Option<RetryConfig>,
}

impl AgentExecutionRequest {
    pub fn new(execution_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            execution_id: execution_id.into(),
            messages,
            input_messages: Vec::new(),
            input_policy: None,
            input_contract: None,
            provider_id: None,
            model_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            subject: None,
            trace_context: None,
            timeout_ms: None,
            include_tool_descriptors: false,
            memory_query: None,
            knowledge_query: None,
            mcp_server_id: None,
            metadata: Vec::new(),
            cancellation_token: None,
            deadline_ms: None,
            retry: None,
        }
    }

    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    pub fn with_deadline_ms(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_input_messages(mut self, input_messages: Vec<AgentMessage>) -> Self {
        self.input_messages = input_messages;
        self.messages = agent_messages_to_text_lines(&self.input_messages);
        self
    }

    pub fn with_input_contract(mut self, input_contract: AgentInputContract) -> Self {
        self.input_policy = Some(input_contract.to_legacy_policy());
        self.input_contract = Some(input_contract);
        self
    }

    pub fn with_input_policy(mut self, input_policy: AgentInputPolicy) -> Self {
        self.input_policy = Some(input_policy.clone());
        self.input_contract = Some(AgentInputContract::from_legacy_policy(&input_policy));
        self
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

    pub fn with_subject(mut self, subject: PolicySubject) -> Self {
        self.subject = Some(subject);
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

    pub fn include_tool_descriptors(mut self) -> Self {
        self.include_tool_descriptors = true;
        self
    }

    pub fn with_memory_query(
        mut self,
        scope: crate::MemoryScope,
        owner_context: impl Into<String>,
    ) -> Self {
        let provider_id = self
            .memory_query
            .as_ref()
            .and_then(|memory_query| memory_query.provider_id.clone());
        self.memory_query = Some(AgentChatMemoryQuery {
            scope,
            owner_context: owner_context.into(),
            provider_id,
        });
        self
    }

    pub fn with_memory_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        if let Some(memory_query) = &mut self.memory_query {
            memory_query.provider_id = Some(provider_id.into());
        } else {
            self.memory_query = Some(
                AgentChatMemoryQuery::new(crate::MemoryScope::Session, String::new())
                    .with_provider_id(provider_id),
            );
        }
        self
    }

    pub fn with_knowledge_query(mut self, query: impl Into<String>) -> Self {
        let mut next_query = self
            .knowledge_query
            .take()
            .unwrap_or_else(|| AgentChatKnowledgeQuery::new(String::new()));
        next_query.query = query.into();
        self.knowledge_query = Some(next_query);
        self
    }

    pub fn with_knowledge_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.ensure_knowledge_query().provider_id = Some(provider_id.into());
        self
    }

    pub fn with_knowledge_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.ensure_knowledge_query().tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_knowledge_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.ensure_knowledge_query().namespace = Some(namespace.into());
        self
    }

    pub fn with_knowledge_top_k(mut self, top_k: usize) -> Self {
        self.ensure_knowledge_query().top_k = Some(top_k);
        self
    }

    pub fn with_knowledge_method(mut self, method: KnowledgeRetrievalMethod) -> Self {
        let knowledge_query = self.ensure_knowledge_query();
        if !knowledge_query.methods.contains(&method) {
            knowledge_query.methods.push(method);
        }
        self
    }

    pub fn with_knowledge_filter(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.ensure_knowledge_query()
            .filters
            .push((key.into(), value.into()));
        self
    }

    pub fn include_external_knowledge(mut self) -> Self {
        self.ensure_knowledge_query().include_external = true;
        self
    }

    pub fn with_mcp_server_id(mut self, server_id: impl Into<String>) -> Self {
        self.mcp_server_id = Some(server_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    fn validate(&self) -> KernelResult<()> {
        if self.execution_id.trim().is_empty() {
            return Err(KernelError::validation("execution id must not be empty"));
        }

        if self.messages.is_empty() {
            return Err(KernelError::validation(
                "execution requires at least one message",
            ));
        }

        if self
            .messages
            .iter()
            .any(|message| message.trim().is_empty())
        {
            return Err(KernelError::validation(
                "execution messages must not be blank",
            ));
        }

        Ok(())
    }

    fn to_chat_request(&self) -> AgentChatRequest {
        let mut request = if self.input_messages.is_empty() {
            AgentChatRequest::new(self.execution_id.clone(), self.messages.clone())
        } else {
            AgentChatRequest::new(
                self.execution_id.clone(),
                agent_messages_to_text_lines(&self.input_messages),
            )
            .with_input_messages(self.input_messages.clone())
        };

        if let Some(input_contract) = &self.input_contract {
            request = request.with_input_contract(input_contract.clone());
        } else if let Some(input_policy) = &self.input_policy {
            request = request.with_input_policy(input_policy.clone());
        }

        if let Some(provider_id) = &self.provider_id {
            request = request.with_provider_id(provider_id.clone());
        }

        if let Some(model_id) = &self.model_id {
            request = request.with_model_id(model_id.clone());
        }

        if let Some(session_id) = &self.session_id {
            request = request.for_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            request = request.for_task(task_id.clone());
        }

        if let Some(run_id) = &self.run_id {
            request = request.for_run(run_id.clone());
        }

        if let Some(step_id) = &self.step_id {
            request = request.for_step(step_id.clone());
        }

        if let Some(subject) = &self.subject {
            request = request.with_subject(subject.clone());
        }

        if let Some(trace_context) = &self.trace_context {
            request = request.with_trace_context(trace_context.clone());
        }

        if let Some(timeout_ms) = self.timeout_ms {
            request = request.with_timeout_ms(timeout_ms);
        }

        if self.include_tool_descriptors {
            request = request.include_tool_descriptors();
        }

        if let Some(memory_query) = &self.memory_query {
            request =
                request.with_memory_query(memory_query.scope, memory_query.owner_context.clone());
            if let Some(provider_id) = &memory_query.provider_id {
                request = request.with_memory_provider_id(provider_id.clone());
            }
        }

        if let Some(knowledge_query) = &self.knowledge_query {
            request = request.with_knowledge_query(knowledge_query.query.clone());
            if let Some(provider_id) = &knowledge_query.provider_id {
                request = request.with_knowledge_provider_id(provider_id.clone());
            }
            if let Some(tenant_id) = &knowledge_query.tenant_id {
                request = request.with_knowledge_tenant_id(tenant_id.clone());
            }
            if let Some(namespace) = &knowledge_query.namespace {
                request = request.with_knowledge_namespace(namespace.clone());
            }
            if let Some(top_k) = knowledge_query.top_k {
                request = request.with_knowledge_top_k(top_k);
            }
            for method in &knowledge_query.methods {
                request = request.with_knowledge_method(*method);
            }
            for (key, value) in &knowledge_query.filters {
                request = request.with_knowledge_filter(key.clone(), value.clone());
            }
            if knowledge_query.include_external {
                request = request.include_external_knowledge();
            }
        }

        for (key, value) in &self.metadata {
            request = request.with_metadata(key.clone(), value.clone());
        }

        request
    }

    fn ensure_knowledge_query(&mut self) -> &mut AgentChatKnowledgeQuery {
        self.knowledge_query
            .get_or_insert_with(|| AgentChatKnowledgeQuery::new(String::new()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionReport {
    pub execution_id: String,
    pub status: AgentExecutionStatus,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub plan: Option<Plan>,
    pub model_response: Option<ModelResponse>,
    pub tool_executions: Vec<ToolExecutionResponse>,
    pub mcp_tool_executions: Vec<McpToolExecutionResponse>,
    pub observations: Vec<AgentObservation>,
    pub error: Option<KernelError>,
}

impl AgentExecutionReport {
    pub fn approval_resume_request(
        &self,
        resume_request_id: impl Into<String>,
        decision: AgentExecutionResumeDecision,
        approved_by: impl Into<String>,
    ) -> KernelResult<AgentExecutionResumeRequest> {
        if self.status != AgentExecutionStatus::PermissionRequired {
            return Err(KernelError::validation(
                "only permission-required execution reports can build approval resume requests",
            ));
        }

        let approved_by = approved_by.into();
        let mut request = AgentExecutionResumeRequest {
            resume_request_id: resume_request_id.into(),
            execution_id: self.execution_id.clone(),
            decision,
            approved_by: if approved_by.trim().is_empty() {
                None
            } else {
                Some(approved_by)
            },
            comment: None,
            session_id: self.session_id.clone(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            step_id: self.step_id.clone(),
            trace_context: self.trace_context.clone(),
            permission_error_kind: None,
            permission_error_code: None,
        };

        if let Some(error) = &self.error {
            request.permission_error_kind = Some(error.kind().as_str().to_string());
            request.permission_error_code = Some(error.code().to_string());
        }

        request.validate()?;
        Ok(request)
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let event_id = event_id.into();
        let mut event = KernelEvent::new(
            event_id,
            self.event_type(),
            self.event_severity(),
            self.event_payload(),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(self.report_redaction())
        .with_payload_schema("sdkwork.agent.execution.report.v1");

        if let Some(session_id) = &self.session_id {
            event = event.for_session(session_id.clone());
        }
        if let Some(task_id) = &self.task_id {
            event = event.for_task(task_id.clone());
        }
        if let Some(run_id) = &self.run_id {
            event = event.for_run(run_id.clone());
        }
        if let Some(step_id) = &self.step_id {
            event = event.for_step(step_id.clone());
        }
        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }

    pub fn to_events(&self, event_id_prefix: impl Into<String>) -> Vec<KernelEvent> {
        let event_id_prefix = event_id_prefix.into();
        let report_event_id = format!("{event_id_prefix}.report");
        let mut events = vec![self.to_event(report_event_id.clone())];

        for (index, observation) in self.observations.iter().enumerate() {
            let mut event =
                observation.to_event(format!("{event_id_prefix}.observation.{}", index + 1));
            event = event.caused_by(report_event_id.clone());
            if let Some(session_id) = &self.session_id {
                event = event.for_session(session_id.clone());
            }
            if let Some(task_id) = &self.task_id {
                event = event.for_task(task_id.clone());
            }
            if let Some(run_id) = &self.run_id {
                event = event.for_run(run_id.clone());
            }
            if let Some(step_id) = &self.step_id {
                event = event.for_step(step_id.clone());
            }
            if let Some(trace_context) = &self.trace_context {
                event = event.with_trace_context(trace_context.clone());
            }
            events.push(event);
        }

        events
    }

    fn event_type(&self) -> &'static str {
        match self.status {
            AgentExecutionStatus::Completed => "agent.execution.completed",
            AgentExecutionStatus::Failed => "agent.execution.failed",
            AgentExecutionStatus::PermissionRequired => "agent.execution.permission_required",
            AgentExecutionStatus::Cancelled => "agent.execution.cancelled",
            AgentExecutionStatus::Degraded => "agent.execution.degraded",
        }
    }

    fn event_severity(&self) -> KernelEventSeverity {
        match self.status {
            AgentExecutionStatus::Completed | AgentExecutionStatus::Degraded => {
                KernelEventSeverity::Info
            }
            AgentExecutionStatus::PermissionRequired | AgentExecutionStatus::Cancelled => {
                KernelEventSeverity::Warn
            }
            AgentExecutionStatus::Failed => KernelEventSeverity::Error,
        }
    }

    fn event_payload(&self) -> String {
        let mut fields = vec![
            format!("execution_id={}", self.execution_id),
            format!("status={}", self.status.as_str()),
            format!("observations={}", self.observations.len()),
            format!("tool_executions={}", self.tool_executions.len()),
            format!("mcp_tool_executions={}", self.mcp_tool_executions.len()),
        ];

        if let Some(plan) = &self.plan {
            fields.push(format!("plan_id={}", plan.plan_id));
            fields.push(format!("plan_actions={}", plan.actions.len()));
        }

        if let Some(model_response) = &self.model_response {
            fields.push(format!("model_provider_id={}", model_response.provider_id));
            fields.push(format!(
                "model_status={}",
                model_response.status.as_report_str()
            ));
        }

        if let Some(error) = &self.error {
            fields.push(format!("error_kind={}", error.kind().as_str()));
            fields.push(format!("error_source={}", error.source().as_str()));
        }

        fields.join(";")
    }

    fn report_redaction(&self) -> KernelEventRedaction {
        self.observations
            .iter()
            .map(|observation| observation.redaction_classification)
            .chain(
                self.error
                    .as_ref()
                    .map(KernelError::redaction_classification),
            )
            .fold(KernelEventRedaction::Internal, max_redaction)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentExecutionService;

impl AgentExecutionService {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        runtime: &AgentRuntime,
        request: AgentExecutionRequest,
    ) -> KernelResult<AgentExecutionReport> {
        request.validate()?;
        self.ensure_runtime_executable(runtime)?;
        let started_at = Instant::now();
        let plan = self.create_plan(runtime, &request)?;
        if let Err(error) = self.phase_check(&request, started_at) {
            let Some(status) = self.chat_error_status(&error) else {
                return Err(error);
            };
            return Ok(self.failed_before_model_report(&request, plan, status, error));
        }
        let chat_response = match self.invoke_model_round(runtime, &request, started_at) {
            Ok(chat_response) => chat_response,
            Err(error) => {
                let Some(status) = self.chat_error_status(&error) else {
                    return Err(error);
                };
                return Ok(self.failed_before_model_report(&request, plan, status, error));
            }
        };
        let model_response = chat_response.model_response;
        let mut observations = vec![AgentObservation {
            observation_id: format!("observation.{}.model", request.execution_id),
            source_family: "model".to_string(),
            action_id: None,
            status: model_response.status.as_report_str().to_string(),
            summary: format!(
                "model provider {} returned response",
                model_response.provider_id
            ),
            redaction_classification: model_response.redaction_classification,
            metadata: vec![(
                "sdkwork.model.provider_id".to_string(),
                model_response.provider_id.clone(),
            )],
        }];

        if let Some((status, error)) = self.non_success_model_report(&model_response) {
            return Ok(self.execution_report(
                &request,
                status,
                plan,
                Some(model_response),
                Vec::new(),
                Vec::new(),
                observations,
                Some(error),
            ));
        }

        let mut tool_executions = Vec::new();
        let mut mcp_tool_executions = Vec::new();

        for (index, tool_call) in model_response.tool_calls.iter().cloned().enumerate() {
            if let Err(error) = self.phase_check(&request, started_at) {
                let status = if error.kind() == KernelErrorKind::Cancelled {
                    AgentExecutionStatus::Cancelled
                } else {
                    AgentExecutionStatus::Failed
                };
                return Ok(self.execution_report(
                    &request,
                    status,
                    plan,
                    Some(model_response.clone()),
                    tool_executions,
                    mcp_tool_executions,
                    observations,
                    Some(error),
                ));
            }
            match self.execute_tool_call(runtime, &request, tool_call.clone(), index + 1) {
                Ok(ExecutedToolCall::Tool(tool_execution)) => {
                    observations.push(AgentObservation {
                        observation_id: format!(
                            "observation.{}.tool.{}",
                            request.execution_id,
                            index + 1
                        ),
                        source_family: "tool".to_string(),
                        action_id: Some(tool_execution.result.tool_call_id.clone()),
                        status: tool_execution.result.status.clone(),
                        summary: format!("tool {} executed", tool_execution.descriptor.tool_id),
                        redaction_classification: tool_execution.result.redaction_classification,
                        metadata: vec![(
                            "sdkwork.tool.provider_id".to_string(),
                            tool_execution.provider_id.clone(),
                        )],
                    });
                    let failed =
                        tool_execution.result.normalized_status != crate::ToolCallStatus::Succeeded;
                    tool_executions.push(tool_execution);
                    if failed {
                        return Ok(self.execution_report(
                            &request,
                            AgentExecutionStatus::Failed,
                            plan,
                            Some(model_response),
                            tool_executions,
                            mcp_tool_executions,
                            observations,
                            Some(
                                KernelError::provider_error(
                                    "tool.execution_failed",
                                    "tool execution failed",
                                )
                                .from_source(KernelErrorSource::Tool),
                            ),
                        ));
                    }
                }
                Ok(ExecutedToolCall::Mcp(mcp_tool_execution)) => {
                    observations.push(AgentObservation {
                        observation_id: format!(
                            "observation.{}.mcp.{}",
                            request.execution_id,
                            index + 1
                        ),
                        source_family: "mcp".to_string(),
                        action_id: Some(mcp_tool_execution.result.tool_call_id.clone()),
                        status: mcp_tool_execution.result.status.clone(),
                        summary: format!(
                            "mcp tool {} executed",
                            mcp_tool_execution.descriptor.tool_id
                        ),
                        redaction_classification: mcp_tool_execution
                            .result
                            .redaction_classification,
                        metadata: vec![
                            (
                                "sdkwork.mcp.provider_id".to_string(),
                                mcp_tool_execution.provider_id.clone(),
                            ),
                            (
                                "sdkwork.mcp.server_id".to_string(),
                                mcp_tool_execution.server_id.clone(),
                            ),
                        ],
                    });
                    let failed = mcp_tool_execution.result.normalized_status
                        != crate::ToolCallStatus::Succeeded;
                    mcp_tool_executions.push(mcp_tool_execution);
                    if failed {
                        return Ok(self.execution_report(
                            &request,
                            AgentExecutionStatus::Failed,
                            plan,
                            Some(model_response),
                            tool_executions,
                            mcp_tool_executions,
                            observations,
                            Some(
                                KernelError::provider_error(
                                    "mcp.tool.execution_failed",
                                    "MCP tool execution failed",
                                )
                                .from_source(KernelErrorSource::Tool),
                            ),
                        ));
                    }
                }
                Err(error) if error.kind() == KernelErrorKind::PermissionRequired => {
                    return Ok(self.execution_report(
                        &request,
                        AgentExecutionStatus::PermissionRequired,
                        plan,
                        Some(model_response),
                        tool_executions,
                        mcp_tool_executions,
                        observations,
                        Some(error),
                    ));
                }
                Err(error) => {
                    observations.push(AgentObservation {
                        observation_id: format!(
                            "observation.{}.tool.{}",
                            request.execution_id,
                            index + 1
                        ),
                        source_family: self
                            .failed_tool_source_family(runtime, &request, &tool_call),
                        action_id: Some(tool_call.tool_call_id.clone()),
                        status: "failed".to_string(),
                        summary: format!("tool {} failed closed", tool_call.tool_id),
                        redaction_classification: KernelEventRedaction::Internal,
                        metadata: vec![("sdkwork.tool.id".to_string(), tool_call.tool_id.clone())],
                    });

                    return Ok(self.execution_report(
                        &request,
                        AgentExecutionStatus::Failed,
                        plan,
                        Some(model_response),
                        tool_executions,
                        mcp_tool_executions,
                        observations,
                        Some(error),
                    ));
                }
            }
        }

        Ok(self.execution_report(
            &request,
            AgentExecutionStatus::Completed,
            plan,
            Some(model_response),
            tool_executions,
            mcp_tool_executions,
            observations,
            None,
        ))
    }

    /// Execute an agent turn through the unified [`AgentStreamEvent`]
    /// protocol.
    ///
    /// The event sequence mirrors the agent SDK turn lifecycle:
    /// `SessionInit -> MessageStart -> MessageDelta* -> (ToolCallStart ->
    /// ToolCallStop -> ToolResult)* -> MessageStop -> Usage -> Result ->
    /// Ended`. Tool failures and permission denials surface as
    /// `ToolResult(is_error)` events and the terminal `Result` carries the
    /// aggregate outcome, cost, and usage.
    pub fn execute_streaming(
        &self,
        runtime: &AgentRuntime,
        request: AgentExecutionRequest,
        sink: &mut dyn AgentStreamSink,
    ) -> KernelResult<()> {
        request.validate()?;
        self.ensure_runtime_executable(runtime)?;

        let started_at = Instant::now();
        let session_id = request.session_id.clone();
        let stream_id = format!("execution.{}", request.execution_id);
        // Multi-turn guard: bounded tool-use rounds per run.
        const MAX_TURNS: u32 = 10;

        if let Some(session_id) = &session_id {
            sink.push_event(
                AgentStreamEvent::SessionInit(
                    SessionInitEvent::new(format!("{}.init", request.execution_id))
                        .with_stream_id(stream_id.clone())
                        .with_model(String::new(), request.model_id.clone().unwrap_or_default()),
                )
                .with_session_id(session_id.clone()),
            )?;
        }

        let mut num_turns: u32 = 0;
        let mut history: Vec<AgentMessage> = Vec::new();
        let mut aggregate_usage: Option<ModelUsage> = None;
        let mut model_id_used: Option<String> = None;
        let mut tool_failed = false;
        let mut final_content = String::new();
        // Every loop exit path assigns the stop reason before breaking.
        let mut stop_reason: String;

        loop {
            num_turns += 1;
            if num_turns > MAX_TURNS {
                sink.push_event(
                    AgentStreamEvent::Status(
                        crate::StatusEvent::warn(
                            format!("{}.status.turns", request.execution_id),
                            format!("max turns ({MAX_TURNS}) exceeded; stopping"),
                        )
                        .with_stream_id(stream_id.clone()),
                    )
                    .with_session_id_optional(&session_id),
                )?;
                stop_reason = "max_turns".to_string();
                break;
            }

            let chat_response = match self.invoke_model_round_with_history(
                runtime,
                &request,
                started_at,
                history.clone(),
            ) {
                Ok(chat_response) => chat_response,
                Err(error) => {
                    sink.push_event(
                        AgentStreamEvent::Error(
                            ErrorEvent::new(
                                format!("{}.error", request.execution_id),
                                error.to_string(),
                            )
                            .with_code(error.kind().as_str())
                            .with_stream_id(stream_id.clone()),
                        )
                        .with_session_id_optional(&session_id),
                    )?;
                    tool_failed = true;
                    final_content = error.to_string();
                    stop_reason = error.kind().as_str().to_string();
                    break;
                }
            };

            let model_response = chat_response.model_response;
            let message_id = model_response.model_request_id.clone();
            let content = model_response.messages.join("\n");
            if model_id_used.is_none() {
                model_id_used = model_response.model_id.clone();
            }
            final_content = content.clone();
            stop_reason = model_response
                .finish_reason
                .clone()
                .unwrap_or_else(|| model_response.status.as_report_str().to_string());

            // Aggregate usage across turns for the terminal result.
            if let Some(usage) = &model_response.usage {
                aggregate_usage = Some(match aggregate_usage {
                    Some(aggregate) => ModelUsage {
                        input_tokens: aggregate.input_tokens + usage.input_tokens,
                        output_tokens: aggregate.output_tokens + usage.output_tokens,
                        cached_input_tokens: aggregate.cached_input_tokens
                            + usage.cached_input_tokens,
                        reasoning_tokens: aggregate.reasoning_tokens + usage.reasoning_tokens,
                        duration_ms: usage.duration_ms.or(aggregate.duration_ms),
                    },
                    None => usage.clone(),
                });
            }

            sink.push_event(
                AgentStreamEvent::MessageStart(
                    MessageStartEvent::new(
                        format!("{}.message.start.{}", request.execution_id, num_turns),
                        message_id.clone(),
                        AgentMessageRole::Agent,
                    )
                    .with_stream_id(stream_id.clone()),
                )
                .with_session_id_optional(&session_id),
            )?;

            if !content.is_empty() {
                sink.push_event(
                    AgentStreamEvent::MessageDelta(
                        crate::MessageDeltaEvent::text(
                            format!("{}.message.delta.{}", request.execution_id, num_turns),
                            message_id.clone(),
                            content.clone(),
                        )
                        .with_stream_id(stream_id.clone()),
                    )
                    .with_session_id_optional(&session_id),
                )?;
            }

            for tool_call in &model_response.tool_calls {
                sink.push_event(
                    AgentStreamEvent::ToolCallStart(
                        ToolCallStartEvent::new(
                            format!(
                                "{}.tool.start.{}.{}",
                                request.execution_id, num_turns, tool_call.tool_call_id
                            ),
                            tool_call.tool_call_id.clone(),
                            tool_call.tool_id.clone(),
                        )
                        .with_message(message_id.clone())
                        .with_stream_id(stream_id.clone()),
                    )
                    .with_session_id_optional(&session_id),
                )?;
                sink.push_event(
                    AgentStreamEvent::ToolCallStop(
                        ToolCallStopEvent::new(
                            format!(
                                "{}.tool.stop.{}.{}",
                                request.execution_id, num_turns, tool_call.tool_call_id
                            ),
                            tool_call.tool_call_id.clone(),
                            tool_call.tool_id.clone(),
                            tool_call.arguments.clone(),
                        )
                        .with_stream_id(stream_id.clone()),
                    )
                    .with_session_id_optional(&session_id),
                )?;
            }

            sink.push_event(
                AgentStreamEvent::MessageStop(
                    MessageStopEvent::new(
                        format!("{}.message.stop.{}", request.execution_id, num_turns),
                        message_id.clone(),
                    )
                    .with_content(content.clone())
                    .with_finish_reason(stop_reason.clone())
                    .with_stream_id(stream_id.clone()),
                )
                .with_session_id_optional(&session_id),
            )?;

            // Execute this round's tool calls and record results into the
            // history so the next model round observes them.
            let mut tool_results: Vec<(String, String)> = Vec::new();
            for (index, tool_call) in model_response.tool_calls.iter().cloned().enumerate() {
                if let Err(error) = self.phase_check(&request, started_at) {
                    tool_failed = true;
                    sink.push_event(
                        AgentStreamEvent::ToolResult(
                            ToolResultEvent::new(
                                format!("{}.tool.result.{}", request.execution_id, index + 1),
                                format!("tool.{}", index + 1),
                                tool_call.tool_id,
                                error.to_string(),
                                crate::ToolCallStatus::Cancelled,
                            )
                            .with_error(true)
                            .with_stream_id(stream_id.clone()),
                        )
                        .with_session_id_optional(&session_id),
                    )?;
                    tool_results.push((tool_call.tool_call_id, format!("cancelled: {error}")));
                    continue;
                }
                let result_event =
                    match self.execute_tool_call(runtime, &request, tool_call.clone(), index + 1) {
                        Ok(ExecutedToolCall::Tool(tool_execution)) => {
                            let result = tool_execution.result;
                            let failed =
                                result.normalized_status != crate::ToolCallStatus::Succeeded;
                            tool_failed |= failed;
                            tool_results.push((
                                result.tool_call_id.clone(),
                                result.error.clone().unwrap_or(result.output.clone()),
                            ));
                            ToolResultEvent::new(
                                format!(
                                    "{}.tool.result.{}",
                                    request.execution_id, result.tool_call_id
                                ),
                                result.tool_call_id,
                                tool_execution.descriptor.tool_id,
                                result.error.clone().unwrap_or(result.output.clone()),
                                result.normalized_status,
                            )
                            .with_error(failed)
                            .with_duration_ms(result.duration_ms.unwrap_or(0))
                        }
                        Ok(ExecutedToolCall::Mcp(mcp_tool_execution)) => {
                            let result = mcp_tool_execution.result;
                            let failed =
                                result.normalized_status != crate::ToolCallStatus::Succeeded;
                            tool_failed |= failed;
                            tool_results.push((
                                result.tool_call_id.clone(),
                                result.error.clone().unwrap_or(result.output.clone()),
                            ));
                            ToolResultEvent::new(
                                format!(
                                    "{}.tool.result.{}",
                                    request.execution_id, result.tool_call_id
                                ),
                                result.tool_call_id,
                                mcp_tool_execution.descriptor.tool_id,
                                result.error.clone().unwrap_or(result.output.clone()),
                                result.normalized_status,
                            )
                            .with_error(failed)
                            .with_duration_ms(result.duration_ms.unwrap_or(0))
                        }
                        Err(error) => {
                            tool_failed = true;
                            tool_results
                                .push((tool_call.tool_call_id.clone(), format!("failed: {error}")));
                            ToolResultEvent::new(
                                format!(
                                    "{}.tool.result.{}",
                                    request.execution_id,
                                    error.kind().as_str()
                                ),
                                tool_call.tool_call_id,
                                tool_call.tool_id,
                                error.to_string(),
                                crate::ToolCallStatus::Failed,
                            )
                            .with_error(true)
                        }
                    };
                sink.push_event(
                    AgentStreamEvent::ToolResult(result_event.with_stream_id(stream_id.clone()))
                        .with_session_id_optional(&session_id),
                )?;
            }

            // Append this round to the history for the next model round.
            if !model_response.tool_calls.is_empty() {
                let mut assistant_parts = Vec::new();
                if !content.is_empty() {
                    assistant_parts.push(crate::AgentPart::text(
                        format!("{message_id}.text"),
                        content.clone(),
                    ));
                }
                for tool_call in &model_response.tool_calls {
                    assistant_parts.push(crate::AgentPart::tool_call_ref(
                        format!("{message_id}.tool.{}", tool_call.tool_call_id),
                        tool_call.tool_call_id.clone(),
                    ));
                }
                let assistant_message =
                    AgentMessage::new(message_id.clone(), AgentMessageRole::Agent, assistant_parts)
                        .with_parent_message(
                            history
                                .last()
                                .map(|m| m.message_id.clone())
                                .unwrap_or_default(),
                        );
                history.push(assistant_message);
                for (tool_call_id, output) in tool_results {
                    history.push(
                        AgentMessage::new(
                            format!("{message_id}.tool-result.{tool_call_id}"),
                            AgentMessageRole::Tool,
                            vec![crate::AgentPart::text(
                                format!("{message_id}.tool-result.{tool_call_id}.text"),
                                output,
                            )],
                        )
                        .with_parent_message(message_id.clone()),
                    );
                }
            }

            // A round without tool calls ends the loop; a non-succeeded
            // model status also terminates.
            if model_response.tool_calls.is_empty()
                || model_response.status != crate::ModelStatus::Succeeded
            {
                break;
            }
        }

        // Terminal accounting: aggregated usage, cost, and result.
        let usage_event = aggregate_usage.clone().map(|usage| {
            UsageEvent::new(
                format!("{}.usage", request.execution_id),
                usage.input_tokens,
                usage.output_tokens,
            )
            .with_cached_input_tokens(usage.cached_input_tokens)
            .with_reasoning_tokens(usage.reasoning_tokens)
            .with_stream_id(stream_id.clone())
        });
        if let Some(usage) = &usage_event {
            sink.push_event(
                AgentStreamEvent::Usage(usage.clone()).with_session_id_optional(&session_id),
            )?;
        }

        // Cost accounting: derive cents from the runtime price table and
        // surface a CostEvent before the terminal result.
        if let (Some(model_id), Some(model_usage)) =
            (model_id_used.as_deref(), aggregate_usage.as_ref())
        {
            if let Some(estimate) = runtime.cost_calculator().estimate(model_id, model_usage) {
                sink.push_event(
                    AgentStreamEvent::Cost(
                        estimate
                            .to_cost_event(format!("{}.cost", request.execution_id))
                            .with_stream_id(stream_id.clone()),
                    )
                    .with_session_id_optional(&session_id),
                )?;
            }
        }

        let mut result_event = ResultEvent::new(format!("{}.result", request.execution_id))
            .with_run_id(request.run_id.clone().unwrap_or_default())
            // The cap check fires before the next round is invoked, so the
            // executed turn count is bounded by MAX_TURNS.
            .with_num_turns(num_turns.min(MAX_TURNS))
            .with_error(tool_failed)
            .with_result(final_content)
            .with_stop_reason(stop_reason)
            .with_stream_id(stream_id.clone());
        if let Some(usage) = &usage_event {
            result_event = result_event.with_usage(usage.clone());
        }
        sink.push_event(
            AgentStreamEvent::Result(result_event).with_session_id_optional(&session_id),
        )?;

        self.push_ended(sink, &request, &session_id, &stream_id)
    }

    fn push_ended(
        &self,
        sink: &mut dyn AgentStreamSink,
        request: &AgentExecutionRequest,
        session_id: &Option<String>,
        stream_id: &str,
    ) -> KernelResult<()> {
        sink.push_event(
            AgentStreamEvent::Ended(
                EndedEvent::new(format!("{}.ended", request.execution_id))
                    .with_stream_id(stream_id.to_string()),
            )
            .with_session_id_optional(session_id),
        )
    }

    /// Check cooperative cancellation and the relative deadline at a phase
    /// boundary. Cancellation wins over deadline when both are present.
    fn phase_check(
        &self,
        request: &AgentExecutionRequest,
        started_at: Instant,
    ) -> KernelResult<()> {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled() {
                return Err(KernelError::cancelled(format!(
                    "execution {} cancelled",
                    request.execution_id
                ))
                .from_source(KernelErrorSource::Runtime));
            }
        }
        if let Some(deadline_ms) = request.deadline_ms {
            if started_at.elapsed() >= std::time::Duration::from_millis(deadline_ms) {
                return Err(KernelError::timeout(format!(
                    "execution {} exceeded {deadline_ms}ms deadline",
                    request.execution_id
                ))
                .from_source(KernelErrorSource::Runtime));
            }
        }
        Ok(())
    }

    /// Invoke the model round with explicit message history (multi-turn
    /// tool-use loop); the history carries prior assistant and tool-result
    /// messages so the provider observes the full conversation.
    fn invoke_model_round_with_history(
        &self,
        runtime: &AgentRuntime,
        request: &AgentExecutionRequest,
        started_at: Instant,
        history: Vec<AgentMessage>,
    ) -> KernelResult<AgentChatResponse> {
        self.phase_check(request, started_at)?;
        let mut chat_request = request.to_chat_request();
        if !history.is_empty() {
            chat_request = chat_request.with_input_messages(history);
        }
        match &request.retry {
            Some(config) => {
                let result = execute_with_retry::<AgentChatResponse, KernelError, _>(
                    config.clone(),
                    None,
                    true,
                    request
                        .deadline_ms
                        .map(|ms| started_at + Duration::from_millis(ms)),
                    || AgentChatService::new().invoke(runtime, chat_request.clone()),
                )?;
                Ok(result.value)
            }
            None => AgentChatService::new().invoke(runtime, chat_request),
        }
    }

    /// Invoke the model round, optionally wrapping the chat invocation in
    /// the kernel retry engine when the request carries a retry policy.
    fn invoke_model_round(
        &self,
        runtime: &AgentRuntime,
        request: &AgentExecutionRequest,
        started_at: Instant,
    ) -> KernelResult<AgentChatResponse> {
        self.phase_check(request, started_at)?;
        let chat_request = request.to_chat_request();
        match &request.retry {
            Some(config) => {
                let result = execute_with_retry::<AgentChatResponse, KernelError, _>(
                    config.clone(),
                    None,
                    true,
                    request
                        .deadline_ms
                        .map(|ms| started_at + Duration::from_millis(ms)),
                    || AgentChatService::new().invoke(runtime, chat_request.clone()),
                )?;
                Ok(result.value)
            }
            None => AgentChatService::new().invoke(runtime, chat_request),
        }
    }

    fn failed_before_model_report(
        &self,
        request: &AgentExecutionRequest,
        plan: Option<Plan>,
        status: AgentExecutionStatus,
        error: KernelError,
    ) -> AgentExecutionReport {
        self.execution_report(
            request,
            status,
            plan,
            None,
            Vec::new(),
            Vec::new(),
            vec![AgentObservation {
                observation_id: format!("observation.{}.model", request.execution_id),
                source_family: "model".to_string(),
                action_id: None,
                status: error.kind().as_str().to_string(),
                summary: "model invocation stopped before provider execution".to_string(),
                redaction_classification: error.redaction_classification(),
                metadata: vec![
                    (
                        "sdkwork.error.kind".to_string(),
                        error.kind().as_str().to_string(),
                    ),
                    (
                        "sdkwork.error.source".to_string(),
                        error.source().as_str().to_string(),
                    ),
                ],
            }],
            Some(error),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn execution_report(
        &self,
        request: &AgentExecutionRequest,
        status: AgentExecutionStatus,
        plan: Option<Plan>,
        model_response: Option<ModelResponse>,
        tool_executions: Vec<ToolExecutionResponse>,
        mcp_tool_executions: Vec<McpToolExecutionResponse>,
        observations: Vec<AgentObservation>,
        error: Option<KernelError>,
    ) -> AgentExecutionReport {
        AgentExecutionReport {
            execution_id: request.execution_id.clone(),
            status,
            session_id: request.session_id.clone(),
            task_id: request.task_id.clone(),
            run_id: request.run_id.clone(),
            step_id: request.step_id.clone(),
            trace_context: request.trace_context.clone(),
            plan,
            model_response,
            tool_executions,
            mcp_tool_executions,
            observations,
            error,
        }
    }

    fn chat_error_status(&self, error: &KernelError) -> Option<AgentExecutionStatus> {
        match error.kind() {
            KernelErrorKind::ValidationError => None,
            KernelErrorKind::PermissionRequired => Some(AgentExecutionStatus::PermissionRequired),
            KernelErrorKind::Cancelled => Some(AgentExecutionStatus::Cancelled),
            _ => Some(AgentExecutionStatus::Failed),
        }
    }

    fn non_success_model_report(
        &self,
        model_response: &ModelResponse,
    ) -> Option<(AgentExecutionStatus, KernelError)> {
        match model_response.status {
            crate::ModelStatus::Succeeded => None,
            crate::ModelStatus::Failed => Some((
                AgentExecutionStatus::Failed,
                KernelError::provider_error("model.execution_failed", "model execution failed")
                    .from_source(KernelErrorSource::Model),
            )),
            crate::ModelStatus::Cancelled => Some((
                AgentExecutionStatus::Cancelled,
                KernelError::cancelled("model execution cancelled")
                    .from_source(KernelErrorSource::Model),
            )),
            crate::ModelStatus::TimedOut => Some((
                AgentExecutionStatus::Failed,
                KernelError::timeout("model execution timed out")
                    .from_source(KernelErrorSource::Model),
            )),
            crate::ModelStatus::PolicyDenied => Some((
                AgentExecutionStatus::Failed,
                KernelError::PolicyDenied {
                    reason_code: "model.policy_denied".to_string(),
                },
            )),
        }
    }

    fn ensure_runtime_executable(&self, runtime: &AgentRuntime) -> KernelResult<()> {
        if runtime.state() != RuntimeState::Failed {
            return Ok(());
        }

        let capability_id = runtime
            .capability_manifest()
            .missing_required_capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "runtime.failed".to_string());

        Err(KernelError::CapabilityMissing { capability_id }
            .from_source(KernelErrorSource::Runtime))
    }

    fn create_plan(
        &self,
        runtime: &AgentRuntime,
        request: &AgentExecutionRequest,
    ) -> KernelResult<Option<Plan>> {
        let planning_provider = match runtime.planning_provider() {
            Ok(provider) => provider,
            Err(error) if error.kind() == KernelErrorKind::CapabilityMissing => return Ok(None),
            Err(error) => return Err(error),
        };

        let task_id = request
            .task_id
            .clone()
            .unwrap_or_else(|| format!("task.{}", request.execution_id));
        let run_id = request
            .run_id
            .clone()
            .unwrap_or_else(|| format!("run.{}", request.execution_id));
        let summary = request.messages.join("\n");
        let plan = planning_provider.create_plan(&task_id, &run_id, &summary)?;
        planning_provider.validate_plan(&plan)?;

        Ok(Some(plan))
    }

    fn execute_tool_call(
        &self,
        runtime: &AgentRuntime,
        request: &AgentExecutionRequest,
        tool_call: ToolCall,
        sequence: usize,
    ) -> KernelResult<ExecutedToolCall> {
        let tool_call = self.enrich_tool_call(request, tool_call);

        if let Some(provider_id) = tool_call.provider_id.clone() {
            if runtime.tool_provider_by_id(&provider_id).is_ok() {
                return ToolExecutionService::new()
                    .invoke(
                        runtime,
                        ToolExecutionRequest::new(
                            format!("tool-execution.{}.{}", request.execution_id, sequence),
                            tool_call,
                        ),
                    )
                    .map(ExecutedToolCall::Tool);
            }

            if runtime.mcp_provider_by_id(&provider_id).is_ok() {
                let server_id = request.mcp_server_id.as_deref().ok_or_else(|| {
                    KernelError::validation("mcp server id is required for MCP tool calls")
                        .from_source(KernelErrorSource::Tool)
                })?;
                return McpToolExecutionService::new()
                    .invoke(
                        runtime,
                        McpToolExecutionRequest::new(
                            format!("mcp-execution.{}.{}", request.execution_id, sequence),
                            server_id,
                            tool_call,
                        )
                        .with_provider_id(provider_id),
                    )
                    .map(ExecutedToolCall::Mcp);
            }
        }

        let tool_result = ToolExecutionService::new().invoke(
            runtime,
            ToolExecutionRequest::new(
                format!("tool-execution.{}.{}", request.execution_id, sequence),
                tool_call.clone(),
            ),
        );

        match tool_result {
            Ok(tool_execution) => Ok(ExecutedToolCall::Tool(tool_execution)),
            Err(tool_error)
                if request.mcp_server_id.is_some()
                    && tool_error.kind() == KernelErrorKind::CapabilityMissing =>
            {
                let mcp_result = McpToolExecutionService::new().invoke(
                    runtime,
                    McpToolExecutionRequest::new(
                        format!("mcp-execution.{}.{}", request.execution_id, sequence),
                        request.mcp_server_id.clone().unwrap_or_default(),
                        tool_call,
                    ),
                );
                match mcp_result {
                    Ok(mcp_tool_execution) => Ok(ExecutedToolCall::Mcp(mcp_tool_execution)),
                    Err(mcp_error) if mcp_error.kind() == KernelErrorKind::CapabilityMissing => {
                        Err(tool_error)
                    }
                    Err(mcp_error) => Err(mcp_error),
                }
            }
            Err(tool_error) => Err(tool_error),
        }
    }

    fn enrich_tool_call(
        &self,
        request: &AgentExecutionRequest,
        mut tool_call: ToolCall,
    ) -> ToolCall {
        if tool_call.session_id.is_none() {
            tool_call.session_id = request.session_id.clone();
        }
        if tool_call.task_id.is_none() {
            tool_call.task_id = request.task_id.clone();
        }
        if tool_call.run_id.is_none() {
            tool_call.run_id = request.run_id.clone();
        }
        if tool_call.step_id.is_none() {
            tool_call.step_id = request.step_id.clone();
        }
        if tool_call.trace_context.is_none() {
            tool_call.trace_context = request.trace_context.clone();
        }
        if tool_call.timeout_ms.is_none() {
            tool_call.timeout_ms = request.timeout_ms;
        }

        tool_call
    }

    fn failed_tool_source_family(
        &self,
        runtime: &AgentRuntime,
        request: &AgentExecutionRequest,
        tool_call: &ToolCall,
    ) -> String {
        if tool_call
            .provider_id
            .as_deref()
            .is_some_and(|provider_id| runtime.mcp_provider_by_id(provider_id).is_ok())
            || request.mcp_server_id.is_some()
        {
            "mcp".to_string()
        } else {
            "tool".to_string()
        }
    }
}

enum ExecutedToolCall {
    Tool(ToolExecutionResponse),
    Mcp(McpToolExecutionResponse),
}

trait ModelStatusReportExt {
    fn as_report_str(&self) -> &'static str;
}

impl ModelStatusReportExt for crate::ModelStatus {
    fn as_report_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::PolicyDenied => "policy_denied",
        }
    }
}

fn observation_event_source(source_family: &str) -> KernelEventSource {
    match source_family {
        "model" => KernelEventSource::Model,
        "tool" | "mcp" => KernelEventSource::Tool,
        "memory" => KernelEventSource::Memory,
        "knowledge" | "context" => KernelEventSource::Context,
        "policy" => KernelEventSource::Policy,
        _ => KernelEventSource::Unknown,
    }
}

fn observation_event_severity(status: &str) -> KernelEventSeverity {
    match status {
        "succeeded" | "completed" => KernelEventSeverity::Info,
        "permission_required" | "cancelled" => KernelEventSeverity::Warn,
        "failed" | "timed_out" | "policy_denied" | "provider_unavailable" => {
            KernelEventSeverity::Error
        }
        _ => KernelEventSeverity::Info,
    }
}

fn max_redaction(left: KernelEventRedaction, right: KernelEventRedaction) -> KernelEventRedaction {
    if redaction_rank(left) >= redaction_rank(right) {
        left
    } else {
        right
    }
}

fn redaction_rank(redaction: KernelEventRedaction) -> u8 {
    match redaction {
        KernelEventRedaction::Public => 0,
        KernelEventRedaction::Unknown => 1,
        KernelEventRedaction::Internal => 2,
        KernelEventRedaction::TenantSensitive => 3,
        KernelEventRedaction::PersonalData => 4,
        KernelEventRedaction::Secret => 5,
        KernelEventRedaction::Regulated => 6,
    }
}
