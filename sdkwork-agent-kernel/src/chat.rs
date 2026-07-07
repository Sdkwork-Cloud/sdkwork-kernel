use crate::{
    agent_messages_to_text_lines, parse_chat_rpc_payload, AgentInputContract, AgentInputPolicy,
    AgentMessage, AgentRuntime, AgentTask, ContextFrame, KernelError, KernelErrorSource,
    KernelEvent, KernelEventRedaction, KernelResult, KnowledgeRetrievalMethod,
    KnowledgeSearchRequest, KnowledgeSearchResult, MemoryRecord, MemoryScope,
    ModelCancellationRequest, ModelExecutionRequest, ModelExecutionService, ModelRequest,
    ModelResponse, ModelStreamChunk, PolicyCategory, PolicyDecision, PolicyDecisionValue,
    PolicySubject, ProtocolAdapter, ProtocolAdapterAuthMode, ProtocolAdapterManifest,
    ProtocolAdapterRequest, ProtocolAdapterResponse, ProtocolAdapterStreamingSupport,
    ProtocolError, ProtocolFamily, ProtocolObjectEnvelope, ProtocolObjectKind,
    ProtocolObjectMapper, ProtocolStreamUpdate, ProtocolTransport, ProviderHealth, RuntimeState,
    SideEffectLevel, StandardProtocolObjectMapper, TraceContext,
};

const AGENT_CHAT_CREATE_OPERATION: &str = "agent.chat.create";
const MODEL_CHAT_INVOKE_OPERATION: &str = "model.chat.invoke";
const CHAT_RESPONSE_SCHEMA: &str = "sdkwork.agent.rpc.chat.response.v1";
const CHAT_PROTOCOL_VERSION: &str = "sdkwork.agent.rpc.chat.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatRequest {
    pub chat_request_id: String,
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
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatMemoryQuery {
    pub scope: MemoryScope,
    pub owner_context: String,
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatKnowledgeQuery {
    pub query: String,
    pub provider_id: Option<String>,
    pub tenant_id: Option<String>,
    pub namespace: Option<String>,
    pub top_k: Option<usize>,
    pub methods: Vec<KnowledgeRetrievalMethod>,
    pub filters: Vec<(String, String)>,
    pub include_external: bool,
}

impl AgentChatRequest {
    pub fn new(chat_request_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            chat_request_id: chat_request_id.into(),
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
            metadata: Vec::new(),
        }
    }

    pub fn with_input_messages(mut self, input_messages: Vec<AgentMessage>) -> Self {
        self.input_messages = input_messages;
        self.messages = agent_messages_to_text_lines(&self.input_messages);
        self
    }

    pub fn with_input_policy(mut self, input_policy: AgentInputPolicy) -> Self {
        self.input_policy = Some(input_policy.clone());
        self.input_contract = Some(AgentInputContract::from_legacy_policy(&input_policy));
        self
    }

    pub fn with_input_contract(mut self, input_contract: AgentInputContract) -> Self {
        self.input_policy = Some(input_contract.to_legacy_policy());
        self.input_contract = Some(input_contract);
        self
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
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
        scope: MemoryScope,
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
                AgentChatMemoryQuery::new(MemoryScope::Session, String::new())
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

    fn ensure_knowledge_query(&mut self) -> &mut AgentChatKnowledgeQuery {
        self.knowledge_query
            .get_or_insert_with(|| AgentChatKnowledgeQuery::new(String::new()))
    }
}

impl AgentChatMemoryQuery {
    pub fn new(scope: MemoryScope, owner_context: impl Into<String>) -> Self {
        Self {
            scope,
            owner_context: owner_context.into(),
            provider_id: None,
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    fn validate(&self) -> KernelResult<()> {
        if self.owner_context.trim().is_empty() {
            return Err(KernelError::validation(
                "chat memory query owner context must not be blank",
            ));
        }

        if let Some(provider_id) = &self.provider_id {
            if provider_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat memory query provider id must not be blank",
                ));
            }
        }

        Ok(())
    }
}

impl AgentChatKnowledgeQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            provider_id: None,
            tenant_id: None,
            namespace: None,
            top_k: None,
            methods: Vec::new(),
            filters: Vec::new(),
            include_external: false,
        }
    }

    fn validate(&self) -> KernelResult<()> {
        if self.query.trim().is_empty() {
            return Err(KernelError::validation(
                "chat knowledge query must not be blank",
            ));
        }

        if let Some(provider_id) = &self.provider_id {
            if provider_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat knowledge provider id must not be blank",
                ));
            }
        }

        if let Some(tenant_id) = &self.tenant_id {
            if tenant_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat knowledge tenant id must not be blank",
                ));
            }
        }

        if let Some(namespace) = &self.namespace {
            if namespace.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat knowledge namespace must not be blank",
                ));
            }
        }

        if self.top_k == Some(0) {
            return Err(KernelError::validation(
                "chat knowledge top_k must be a positive integer",
            ));
        }

        if self
            .filters
            .iter()
            .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
        {
            return Err(KernelError::validation(
                "chat knowledge filters must not contain blank keys or values",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatResponse {
    pub chat_request_id: String,
    pub provider_id: String,
    pub policy_decision: PolicyDecision,
    pub model_response: ModelResponse,
}

/// Streaming chat response containing the ordered output chunks from the model
/// provider, along with the policy decision and provider metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatStreamResponse {
    pub chat_request_id: String,
    pub provider_id: String,
    pub policy_decision: PolicyDecision,
    pub chunks: Vec<ModelStreamChunk>,
}

/// Cancellation response for an in-flight chat request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChatCancelResponse {
    pub chat_request_id: String,
    pub provider_id: String,
    pub model_response: ModelResponse,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentChatService;

impl AgentChatService {
    pub fn new() -> Self {
        Self
    }

    pub fn invoke(
        &self,
        runtime: &AgentRuntime,
        request: AgentChatRequest,
    ) -> KernelResult<AgentChatResponse> {
        request.validate()?;
        self.ensure_runtime_executable(runtime)?;

        let policy_request_id = format!("policy.{}", request.chat_request_id);
        let mut model_request = request.to_model_request(policy_request_id.clone());
        model_request = self.attach_memory_context(runtime, &request, model_request)?;
        model_request = self.attach_knowledge_context(runtime, &request, model_request)?;
        model_request = self.attach_tool_descriptors(runtime, &request, model_request)?;
        let mut model_execution_request =
            ModelExecutionRequest::new(request.chat_request_id.clone(), model_request);
        if let Some(provider_id) = &request.provider_id {
            model_execution_request = model_execution_request.with_provider_id(provider_id.clone());
        }
        if let Some(subject) = &request.subject {
            model_execution_request = model_execution_request.with_subject(subject.clone());
        }
        let model_execution =
            ModelExecutionService::new().invoke(runtime, model_execution_request)?;

        Ok(AgentChatResponse {
            chat_request_id: request.chat_request_id,
            provider_id: model_execution.model_response.provider_id.clone(),
            policy_decision: model_execution.invoke_policy_decision,
            model_response: model_execution.model_response,
        })
    }

    /// Stream a chat request, returning ordered output chunks from the model
    /// provider. This enables SSE-based real-time token streaming for chat
    /// conversations.
    ///
    /// The method performs the same policy evaluation, memory attachment,
    /// knowledge retrieval, and tool descriptor enrichment as [`invoke`],
    /// but delegates to `ModelExecutionService::stream` instead of
    /// `ModelExecutionService::invoke`.
    pub fn stream(
        &self,
        runtime: &AgentRuntime,
        request: AgentChatRequest,
    ) -> KernelResult<AgentChatStreamResponse> {
        request.validate()?;
        self.ensure_runtime_executable(runtime)?;

        let policy_request_id = format!("policy.{}", request.chat_request_id);
        let mut model_request = request.to_model_request(policy_request_id.clone());
        model_request = self.attach_memory_context(runtime, &request, model_request)?;
        model_request = self.attach_knowledge_context(runtime, &request, model_request)?;
        model_request = self.attach_tool_descriptors(runtime, &request, model_request)?;
        let mut model_execution_request =
            ModelExecutionRequest::new(request.chat_request_id.clone(), model_request);
        if let Some(provider_id) = &request.provider_id {
            model_execution_request = model_execution_request.with_provider_id(provider_id.clone());
        }
        if let Some(subject) = &request.subject {
            model_execution_request = model_execution_request.with_subject(subject.clone());
        }
        let stream_response =
            ModelExecutionService::new().stream(runtime, model_execution_request)?;

        Ok(AgentChatStreamResponse {
            chat_request_id: request.chat_request_id,
            provider_id: stream_response.provider_id,
            policy_decision: stream_response.invoke_policy_decision,
            chunks: stream_response.chunks,
        })
    }

    /// Cancel an in-flight chat request by its model request id. The model
    /// provider's `cancel` method is invoked, returning a terminal
    /// `ModelResponse` with `ModelStatus::Cancelled`.
    pub fn cancel(
        &self,
        runtime: &AgentRuntime,
        chat_request_id: &str,
        model_request_id: &str,
        provider_id: Option<&str>,
    ) -> KernelResult<AgentChatCancelResponse> {
        self.ensure_runtime_executable(runtime)?;

        let mut cancellation_request = ModelCancellationRequest::new(
            chat_request_id.to_string(),
            model_request_id.to_string(),
        );
        if let Some(provider_id) = provider_id {
            cancellation_request = cancellation_request.with_provider_id(provider_id.to_string());
        }
        let cancellation_response =
            ModelExecutionService::new().cancel(runtime, cancellation_request)?;

        Ok(AgentChatCancelResponse {
            chat_request_id: chat_request_id.to_string(),
            provider_id: cancellation_response.provider_id,
            model_response: cancellation_response.model_response,
        })
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

    fn attach_tool_descriptors(
        &self,
        runtime: &AgentRuntime,
        request: &AgentChatRequest,
        mut model_request: ModelRequest,
    ) -> KernelResult<ModelRequest> {
        if !request.include_tool_descriptors {
            return Ok(model_request);
        }

        let tool_provider = runtime.tool_provider()?;
        for descriptor in tool_provider.list_tools() {
            model_request = model_request.with_tool_descriptor(descriptor);
        }

        Ok(model_request)
    }

    fn attach_memory_context(
        &self,
        runtime: &AgentRuntime,
        request: &AgentChatRequest,
        mut model_request: ModelRequest,
    ) -> KernelResult<ModelRequest> {
        let Some(memory_query) = &request.memory_query else {
            return Ok(model_request);
        };

        memory_query.validate()?;
        self.evaluate_memory_read_policy(runtime, request, memory_query)?;

        let memory_provider = match memory_query.provider_id.as_deref() {
            Some(provider_id) => runtime.memory_provider_by_id(provider_id)?,
            None => runtime.memory_provider()?,
        };
        let memory_provider = memory_provider.lock().map_err(|_| {
            KernelError::provider_error("memory.lock_poisoned", "memory provider lock poisoned")
                .from_source(KernelErrorSource::Memory)
        })?;

        for record in memory_provider.query(memory_query.scope, &memory_query.owner_context)? {
            model_request = model_request
                .with_context_frame_payload(memory_record_to_context_frame(&record, request));
        }

        Ok(model_request)
    }

    fn attach_knowledge_context(
        &self,
        runtime: &AgentRuntime,
        request: &AgentChatRequest,
        mut model_request: ModelRequest,
    ) -> KernelResult<ModelRequest> {
        let Some(knowledge_query) = &request.knowledge_query else {
            return Ok(model_request);
        };

        knowledge_query.validate()?;
        let policy_decision =
            self.evaluate_knowledge_search_policy(runtime, request, knowledge_query)?;

        let knowledge_provider = match knowledge_query.provider_id.as_deref() {
            Some(provider_id) => runtime.knowledge_provider_by_id(provider_id)?,
            None => runtime.knowledge_provider()?,
        };
        let search_request =
            request.to_knowledge_search_request(knowledge_query, policy_decision.decision_id);

        for result in knowledge_provider.search(search_request)? {
            model_request = model_request.with_context_frame_payload(
                knowledge_search_result_to_context_frame(&result, request),
            );
        }

        Ok(model_request)
    }

    fn evaluate_memory_read_policy(
        &self,
        runtime: &AgentRuntime,
        request: &AgentChatRequest,
        memory_query: &AgentChatMemoryQuery,
    ) -> KernelResult<PolicyDecision> {
        let policy_request_id = format!("policy.{}.memory.read", request.chat_request_id);
        let policy_decision = runtime
            .policy_provider()?
            .evaluate(request.to_memory_read_policy_request(&policy_request_id, memory_query))?;

        match policy_decision.decision {
            PolicyDecisionValue::Allow => Ok(policy_decision),
            PolicyDecisionValue::Deny => Err(KernelError::PolicyDenied {
                reason_code: policy_decision.reason_code,
            }),
            PolicyDecisionValue::NeedsApproval => Err(KernelError::permission_required(
                policy_decision
                    .safe_reason
                    .clone()
                    .unwrap_or(policy_decision.reason_code),
            )),
            PolicyDecisionValue::Defer => Err(KernelError::provider_error(
                "policy.deferred",
                policy_decision.reason_code,
            )),
        }
    }

    fn evaluate_knowledge_search_policy(
        &self,
        runtime: &AgentRuntime,
        request: &AgentChatRequest,
        knowledge_query: &AgentChatKnowledgeQuery,
    ) -> KernelResult<PolicyDecision> {
        let policy_request_id = format!("policy.{}.knowledge.search", request.chat_request_id);
        let policy_decision = runtime.policy_provider()?.evaluate(
            request.to_knowledge_search_policy_request(&policy_request_id, knowledge_query),
        )?;

        match policy_decision.decision {
            PolicyDecisionValue::Allow => Ok(policy_decision),
            PolicyDecisionValue::Deny => Err(KernelError::PolicyDenied {
                reason_code: policy_decision.reason_code,
            }),
            PolicyDecisionValue::NeedsApproval => Err(KernelError::permission_required(
                policy_decision
                    .safe_reason
                    .clone()
                    .unwrap_or(policy_decision.reason_code),
            )),
            PolicyDecisionValue::Defer => Err(KernelError::provider_error(
                "policy.deferred",
                policy_decision.reason_code,
            )),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentChatRpcHandler {
    chat_service: AgentChatService,
}

impl AgentChatRpcHandler {
    pub fn new() -> Self {
        Self {
            chat_service: AgentChatService::new(),
        }
    }

    pub fn handle_request(
        &self,
        runtime: &AgentRuntime,
        request: ProtocolAdapterRequest,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        let chat_request = Self::map_request(request)?;
        match self.chat_service.invoke(runtime, chat_request.clone()) {
            Ok(response) => Self::map_response(chat_request, response),
            Err(error) => Self::map_error(chat_request, error),
        }
    }

    fn map_request(request: ProtocolAdapterRequest) -> KernelResult<AgentChatRequest> {
        if request.protocol != ProtocolFamily::Rpc {
            return Err(KernelError::validation(
                "agent chat RPC handler only accepts RPC protocol requests",
            )
            .from_source(KernelErrorSource::ProtocolAdapter));
        }

        if !matches!(
            request.operation.as_str(),
            AGENT_CHAT_CREATE_OPERATION | MODEL_CHAT_INVOKE_OPERATION
        ) {
            return Err(KernelError::validation(format!(
                "unsupported agent chat RPC operation: {}",
                request.operation
            ))
            .from_source(KernelErrorSource::ProtocolAdapter));
        }

        validate_rpc_metadata(&request.metadata)?;

        let message = request.payload.trim();
        if message.is_empty() {
            return Err(
                KernelError::validation("agent chat RPC payload must not be empty")
                    .from_source(KernelErrorSource::ProtocolAdapter),
            );
        }

        let input_messages = parse_chat_rpc_payload(&request.protocol_request_id, message)
            .map_err(|error| error.from_source(KernelErrorSource::ProtocolAdapter))?;
        let text_lines = agent_messages_to_text_lines(&input_messages);

        let timeout_ms = match request.metadata_value("sdkwork.chat.timeout_ms") {
            Some(value) => Some(value.parse::<u64>().map_err(|_| {
                KernelError::validation("sdkwork.chat.timeout_ms must be an unsigned integer")
                    .from_source(KernelErrorSource::ProtocolAdapter)
            })?),
            None => None,
        };

        let mut chat_request =
            AgentChatRequest::new(request.protocol_request_id.clone(), text_lines)
                .with_input_messages(input_messages)
                .with_metadata("sdkwork.protocol.operation", request.operation.clone())
                .with_metadata("sdkwork.protocol.version", CHAT_PROTOCOL_VERSION);

        if let Some(contract_body) = request.metadata_value("sdkwork.chat.input_contract") {
            chat_request = chat_request.with_input_contract(
                crate::parse_agent_input_contract_json(contract_body)
                    .map_err(|error| error.from_source(KernelErrorSource::ProtocolAdapter))?,
            );
        } else if let Some(policy_body) = request.metadata_value("sdkwork.chat.input_policy") {
            chat_request = chat_request.with_input_policy(
                crate::parse_agent_input_policy_json(policy_body)
                    .map_err(|error| error.from_source(KernelErrorSource::ProtocolAdapter))?,
            );
        }

        if let Some(provider_id) = request.metadata_value("sdkwork.chat.provider_id") {
            chat_request = chat_request.with_provider_id(provider_id.to_string());
        }

        if let Some(model_id) = request.metadata_value("sdkwork.chat.model_id") {
            chat_request = chat_request.with_model_id(model_id.to_string());
        }

        if let Some(session_id) = request.metadata_value("sdkwork.agent.session_id") {
            chat_request = chat_request.for_session(session_id.to_string());
        }

        if let Some(task_id) = request.metadata_value("sdkwork.agent.task_id") {
            chat_request = chat_request.for_task(task_id.to_string());
        }

        if let Some(run_id) = request.metadata_value("sdkwork.agent.run_id") {
            chat_request = chat_request.for_run(run_id.to_string());
        }

        if let Some(step_id) = request.metadata_value("sdkwork.agent.step_id") {
            chat_request = chat_request.for_step(step_id.to_string());
        }

        if let (Some(subject_id), Some(tenant_id)) = (
            request.metadata_value("sdkwork.policy.subject_id"),
            request.metadata_value("sdkwork.policy.tenant_id"),
        ) {
            let mut subject = PolicySubject::new(subject_id.to_string(), tenant_id.to_string());
            if let Some(roles) = request.metadata_value("sdkwork.policy.roles") {
                for role in roles
                    .split(',')
                    .map(str::trim)
                    .filter(|role| !role.is_empty())
                {
                    subject = subject.with_role(role.to_string());
                }
            }
            chat_request = chat_request.with_subject(subject);
        }

        if let Some(trace_context) = &request.trace_context {
            chat_request = chat_request.with_trace_context(trace_context.clone());
        }

        if let Some(timeout_ms) = timeout_ms {
            chat_request = chat_request.with_timeout_ms(timeout_ms);
        }

        if let Some(include_tools) = request
            .metadata_value("sdkwork.chat.include_tools")
            .or_else(|| request.metadata_value("sdkwork.chat.include_tool_descriptors"))
        {
            if parse_rpc_bool(include_tools)? {
                chat_request = chat_request.include_tool_descriptors();
            }
        }

        if let (Some(scope), Some(owner_context)) = (
            request.metadata_value("sdkwork.memory.scope"),
            request.metadata_value("sdkwork.memory.owner_context"),
        ) {
            chat_request =
                chat_request.with_memory_query(parse_chat_memory_scope(scope)?, owner_context);
            if let Some(provider_id) = request.metadata_value("sdkwork.memory.provider_id") {
                chat_request = chat_request.with_memory_provider_id(provider_id.to_string());
            }
        } else if request.metadata_value("sdkwork.memory.scope").is_some()
            || request
                .metadata_value("sdkwork.memory.owner_context")
                .is_some()
            || request
                .metadata_value("sdkwork.memory.provider_id")
                .is_some()
        {
            return Err(KernelError::validation(
                "agent chat RPC memory metadata requires sdkwork.memory.scope and sdkwork.memory.owner_context",
            )
            .from_source(KernelErrorSource::ProtocolAdapter));
        }

        if let Some(query) = request.metadata_value("sdkwork.knowledge.query") {
            chat_request = chat_request.with_knowledge_query(query.to_string());

            if let Some(provider_id) = request.metadata_value("sdkwork.knowledge.provider_id") {
                chat_request = chat_request.with_knowledge_provider_id(provider_id.to_string());
            }

            if let Some(tenant_id) = request.metadata_value("sdkwork.knowledge.tenant_id") {
                chat_request = chat_request.with_knowledge_tenant_id(tenant_id.to_string());
            }

            if let Some(namespace) = request.metadata_value("sdkwork.knowledge.namespace") {
                chat_request = chat_request.with_knowledge_namespace(namespace.to_string());
            }

            if let Some(top_k) = request.metadata_value("sdkwork.knowledge.top_k") {
                let top_k = top_k.parse::<usize>().map_err(|_| {
                    KernelError::validation("sdkwork.knowledge.top_k must be a positive integer")
                        .from_source(KernelErrorSource::ProtocolAdapter)
                })?;
                if top_k == 0 {
                    return Err(KernelError::validation(
                        "sdkwork.knowledge.top_k must be a positive integer",
                    )
                    .from_source(KernelErrorSource::ProtocolAdapter));
                }

                chat_request = chat_request.with_knowledge_top_k(top_k);
            }

            if let Some(methods) = request.metadata_value("sdkwork.knowledge.methods") {
                for method in methods
                    .split(',')
                    .map(str::trim)
                    .filter(|method| !method.is_empty())
                {
                    chat_request =
                        chat_request.with_knowledge_method(parse_knowledge_method(method)?);
                }
            }

            if let Some(include_external) =
                request.metadata_value("sdkwork.knowledge.include_external")
            {
                if parse_rpc_bool(include_external)? {
                    chat_request = chat_request.include_external_knowledge();
                }
            }

            for (key, value) in &request.metadata {
                if let Some(filter_key) = key.strip_prefix("sdkwork.knowledge.filter.") {
                    chat_request =
                        chat_request.with_knowledge_filter(filter_key.to_string(), value.clone());
                }
            }
        } else if request
            .metadata
            .iter()
            .any(|(key, _)| key.starts_with("sdkwork.knowledge."))
        {
            return Err(KernelError::validation(
                "agent chat RPC knowledge metadata requires sdkwork.knowledge.query",
            )
            .from_source(KernelErrorSource::ProtocolAdapter));
        }

        if let Some(external_id) = request.external_id {
            chat_request = chat_request.with_metadata("sdkwork.protocol.external_id", external_id);
        }

        for (key, value) in request.metadata {
            if model_forwardable_rpc_metadata_key(&key) {
                chat_request = chat_request.with_metadata(key, value);
            }
        }

        Ok(chat_request)
    }

    fn map_response(
        request: AgentChatRequest,
        response: AgentChatResponse,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        let mut envelope = ProtocolObjectEnvelope::new(
            ProtocolFamily::Rpc,
            ProtocolObjectKind::ExtensionObject,
            response.chat_request_id.clone(),
            chat_response_payload(&response),
        )
        .with_schema(CHAT_RESPONSE_SCHEMA)
        .with_metadata("sdkwork.agent.object_kind", "agent_chat_response")
        .with_metadata(
            "sdkwork.protocol.request_id",
            response.chat_request_id.clone(),
        )
        .with_metadata("sdkwork.chat.provider_id", response.provider_id.clone())
        .with_metadata(
            "sdkwork.chat.model_request_id",
            response.model_response.model_request_id.clone(),
        )
        .with_metadata(
            "sdkwork.policy.request_id",
            response.policy_decision.request_id.clone(),
        )
        .with_metadata(
            "sdkwork.policy.decision",
            response.policy_decision.decision.as_str(),
        )
        .with_redaction(response.model_response.redaction_classification);

        if let Some(external_id) = request.metadata_value("sdkwork.protocol.external_id") {
            envelope = envelope.with_external_id(external_id.to_string());
        }

        if let Some(model_id) = &request.model_id {
            envelope = envelope.with_metadata("sdkwork.chat.model_id", model_id.clone());
        }

        if let Some(session_id) = &request.session_id {
            envelope = envelope.with_metadata("sdkwork.agent.session_id", session_id.clone());
        }

        if let Some(task_id) = &request.task_id {
            envelope = envelope.with_metadata("sdkwork.agent.task_id", task_id.clone());
        }

        if let Some(run_id) = &request.run_id {
            envelope = envelope.with_metadata("sdkwork.agent.run_id", run_id.clone());
        }

        if let Some(step_id) = &request.step_id {
            envelope = envelope.with_metadata("sdkwork.agent.step_id", step_id.clone());
        }

        if let Some(finish_reason) = &response.model_response.finish_reason {
            envelope = envelope.with_metadata("sdkwork.chat.finish_reason", finish_reason.clone());
        }

        if let Some(trace_context) = response
            .model_response
            .trace_context
            .as_ref()
            .or(request.trace_context.as_ref())
        {
            envelope = envelope.with_trace_context(trace_context.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_error(
        request: AgentChatRequest,
        error: KernelError,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        let protocol_error = ProtocolError::from_kernel_error(error.clone());
        let mapper = StandardProtocolObjectMapper::new(ProtocolFamily::Rpc);
        let mut envelope = mapper.map_error(&error)?;
        replace_envelope_metadata(&mut envelope, "sdkwork.error.code", protocol_error.code);
        envelope = envelope
            .with_metadata("sdkwork.protocol.request_id", request.chat_request_id)
            .with_metadata("sdkwork.error.kernel_code", error.code());

        if let Some(trace_context) = error.trace_context().or(request.trace_context.as_ref()) {
            envelope = envelope.with_trace_context(trace_context.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentChatRpcAdapter {
    handler: AgentChatRpcHandler,
}

impl AgentChatRpcAdapter {
    pub fn new() -> Self {
        Self {
            handler: AgentChatRpcHandler::new(),
        }
    }

    pub fn handle_request(
        &self,
        runtime: &AgentRuntime,
        request: ProtocolAdapterRequest,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        self.handler.handle_request(runtime, request)
    }
}

impl ProtocolAdapter for AgentChatRpcAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        agent_chat_rpc_adapter_manifest()
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn handle_request(
        &self,
        runtime: &AgentRuntime,
        request: ProtocolAdapterRequest,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        self.handler.handle_request(runtime, request)
    }

    fn map_request_to_task(&self, request: ProtocolAdapterRequest) -> KernelResult<AgentTask> {
        let chat_request = AgentChatRpcHandler::map_request(request)?;
        let task_id = chat_request
            .task_id
            .clone()
            .unwrap_or_else(|| format!("task.{}", chat_request.chat_request_id));
        let session_id = chat_request
            .session_id
            .clone()
            .unwrap_or_else(|| format!("session.{}", chat_request.chat_request_id));

        Ok(AgentTask::new(
            task_id,
            session_id,
            chat_request.messages.join("\n"),
        ))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(ProtocolStreamUpdate::from_event(event, 1))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            format!("response.{}", task.task_id),
            task.task_id,
        ))
    }
}

impl AgentChatRequest {
    fn validate(&self) -> KernelResult<()> {
        if self.chat_request_id.trim().is_empty() {
            return Err(KernelError::validation("chat request id must not be empty"));
        }

        if self.messages.is_empty() && self.input_messages.is_empty() {
            return Err(KernelError::validation(
                "chat request requires at least one message",
            ));
        }

        if !self.messages.is_empty()
            && self
                .messages
                .iter()
                .any(|message| message.trim().is_empty())
        {
            return Err(KernelError::validation(
                "chat request messages must not be blank",
            ));
        }

        for message in &self.input_messages {
            message.validate()?;
        }

        if let Some(subject) = &self.subject {
            if subject.subject_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat request policy subject id must not be blank",
                ));
            }
            if subject.tenant_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "chat request policy tenant id must not be blank",
                ));
            }
            if subject.roles.iter().any(|role| role.trim().is_empty()) {
                return Err(KernelError::validation(
                    "chat request policy roles must not be blank",
                ));
            }
        }

        if let Some(knowledge_query) = &self.knowledge_query {
            knowledge_query.validate()?;
        }

        Ok(())
    }

    fn to_model_request(&self, policy_request_id: String) -> ModelRequest {
        let messages = if !self.input_messages.is_empty() {
            agent_messages_to_text_lines(&self.input_messages)
        } else {
            self.messages.clone()
        };
        let mut request = ModelRequest::new(self.chat_request_id.clone(), messages)
            .with_policy_context(policy_request_id);

        if !self.input_messages.is_empty() {
            request = request.with_input_messages(self.input_messages.clone());
        }
        if let Some(input_contract) = &self.input_contract {
            request = request.with_input_contract(input_contract.clone());
        } else if let Some(input_policy) = &self.input_policy {
            request = request.with_input_policy(input_policy.clone());
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

        if let Some(trace_context) = &self.trace_context {
            request = request.with_trace_context(trace_context.clone());
        }

        if let Some(timeout_ms) = self.timeout_ms {
            request = request.with_timeout_ms(timeout_ms);
        }

        if self.include_tool_descriptors {
            request = request.with_metadata("sdkwork.chat.include_tools", "true");
        }

        if let Some(memory_query) = &self.memory_query {
            request = request
                .with_metadata("sdkwork.memory.scope", memory_query.scope.as_chat_str())
                .with_metadata(
                    "sdkwork.memory.owner_context",
                    memory_query.owner_context.clone(),
                );
            if let Some(provider_id) = &memory_query.provider_id {
                request = request.with_metadata("sdkwork.memory.provider_id", provider_id.clone());
            }
        }

        if let Some(knowledge_query) = &self.knowledge_query {
            request =
                request.with_metadata("sdkwork.knowledge.query", knowledge_query.query.clone());

            if let Some(provider_id) = &knowledge_query.provider_id {
                request =
                    request.with_metadata("sdkwork.knowledge.provider_id", provider_id.clone());
            }

            if let Some(tenant_id) = &knowledge_query.tenant_id {
                request = request.with_metadata("sdkwork.knowledge.tenant_id", tenant_id.clone());
            }

            if let Some(namespace) = &knowledge_query.namespace {
                request = request.with_metadata("sdkwork.knowledge.namespace", namespace.clone());
            }

            if let Some(top_k) = knowledge_query.top_k {
                request = request.with_metadata("sdkwork.knowledge.top_k", top_k.to_string());
            }

            if !knowledge_query.methods.is_empty() {
                request = request.with_metadata(
                    "sdkwork.knowledge.methods",
                    knowledge_query
                        .methods
                        .iter()
                        .map(KnowledgeRetrievalMethod::as_str)
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }

            if knowledge_query.include_external {
                request = request.with_metadata("sdkwork.knowledge.include_external", "true");
            }

            for (key, value) in &knowledge_query.filters {
                request =
                    request.with_metadata(format!("sdkwork.knowledge.filter.{key}"), value.clone());
            }
        }

        for (key, value) in &self.metadata {
            request = request.with_metadata(key.clone(), value.clone());
        }

        request
    }

    fn to_knowledge_search_request(
        &self,
        knowledge_query: &AgentChatKnowledgeQuery,
        policy_decision_id: String,
    ) -> KnowledgeSearchRequest {
        let mut request = KnowledgeSearchRequest::new(knowledge_query.query.clone())
            .with_policy_context(policy_decision_id);

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

        if let Some(tenant_id) = &knowledge_query.tenant_id {
            request = request.with_tenant_id(tenant_id.clone());
        }

        if let Some(namespace) = &knowledge_query.namespace {
            request = request.with_namespace(namespace.clone());
        }

        if let Some(top_k) = knowledge_query.top_k {
            request = request.with_top_k(top_k);
        }

        for method in &knowledge_query.methods {
            request = request.with_method(*method);
        }

        for (key, value) in &knowledge_query.filters {
            request = request
                .with_filter(key.clone(), value.clone())
                .with_metadata(key.clone(), value.clone());
        }

        if knowledge_query.include_external {
            request = request.include_external();
        }

        if let Some(trace_context) = &self.trace_context {
            request = request.with_trace_context(trace_context.clone());
        }

        if let Some(timeout_ms) = self.timeout_ms {
            request = request.with_timeout_ms(timeout_ms);
        }

        request
    }

    fn to_memory_read_policy_request(
        &self,
        policy_request_id: &str,
        memory_query: &AgentChatMemoryQuery,
    ) -> crate::PolicyRequest {
        let mut request = crate::PolicyRequest::new(
            policy_request_id,
            PolicyCategory::MemoryRead.as_str(),
            format!(
                "{}:{}",
                memory_query.scope.as_chat_str(),
                memory_query.owner_context
            ),
        )
        .with_category(PolicyCategory::MemoryRead)
        .with_action("memory.query")
        .with_side_effect_level(SideEffectLevel::ReadOnly)
        .with_context("memory_scope", memory_query.scope.as_chat_str())
        .with_context("owner_context", memory_query.owner_context.clone())
        .with_redaction(KernelEventRedaction::Internal);

        if let Some(subject) = &self.subject {
            request = request.with_subject(subject.clone());
        }

        if let Some(session_id) = &self.session_id {
            request = request.with_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            request = request.with_task(task_id.clone());
        }

        if let Some(run_id) = &self.run_id {
            request = request.with_run(run_id.clone());
        }

        if let Some(step_id) = &self.step_id {
            request = request.with_context("step_id", step_id.clone());
        }

        if let Some(provider_id) = &memory_query.provider_id {
            request = request.with_context("memory_provider_id", provider_id.clone());
        }

        request
    }

    fn to_knowledge_search_policy_request(
        &self,
        policy_request_id: &str,
        knowledge_query: &AgentChatKnowledgeQuery,
    ) -> crate::PolicyRequest {
        let resource_scope = knowledge_query.namespace.as_deref().unwrap_or("knowledge");
        let mut request = crate::PolicyRequest::new(
            policy_request_id,
            PolicyCategory::KnowledgeSearch.as_str(),
            format!("{resource_scope}:{}", knowledge_query.query),
        )
        .with_category(PolicyCategory::KnowledgeSearch)
        .with_action("knowledge.search")
        .with_side_effect_level(SideEffectLevel::ReadOnly)
        .with_redaction(KernelEventRedaction::Internal);

        if let Some(subject) = &self.subject {
            request = request.with_subject(subject.clone());
        }

        if let Some(session_id) = &self.session_id {
            request = request.with_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            request = request.with_task(task_id.clone());
        }

        if let Some(run_id) = &self.run_id {
            request = request.with_run(run_id.clone());
        }

        if let Some(step_id) = &self.step_id {
            request = request.with_context("step_id", step_id.clone());
        }

        if let Some(provider_id) = &knowledge_query.provider_id {
            request = request.with_context("knowledge_provider_id", provider_id.clone());
        }

        if let Some(tenant_id) = &knowledge_query.tenant_id {
            request = request.with_context("knowledge_tenant_id", tenant_id.clone());
        }

        if let Some(namespace) = &knowledge_query.namespace {
            request = request.with_context("knowledge_namespace", namespace.clone());
        }

        if let Some(top_k) = knowledge_query.top_k {
            request = request.with_context("knowledge_top_k", top_k.to_string());
        }

        if !knowledge_query.methods.is_empty() {
            request = request.with_context(
                "knowledge_methods",
                knowledge_query
                    .methods
                    .iter()
                    .map(KnowledgeRetrievalMethod::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }

        request
    }
}

pub fn agent_chat_rpc_adapter_manifest() -> ProtocolAdapterManifest {
    ProtocolAdapterManifest::new(
        "adapter.rpc.agent-chat",
        ProtocolFamily::Rpc,
        CHAT_PROTOCOL_VERSION,
        ProtocolTransport::Rpc,
        ProtocolAdapterAuthMode::LocalTrusted,
    )
    .with_exposed_capabilities(vec![
        "model.chat".to_string(),
        "knowledge.search".to_string(),
        "protocol.map".to_string(),
        "protocol.stream".to_string(),
    ])
    .with_kernel_object_mappings(vec![
        "AgentChatRequest".to_string(),
        "AgentChatResponse".to_string(),
        "AgentChatKnowledgeQuery".to_string(),
        "ProtocolAdapterRequest".to_string(),
        "ProtocolObjectEnvelope".to_string(),
        "ModelRequest".to_string(),
        "ModelResponse".to_string(),
        "KnowledgeSearchRequest".to_string(),
        "KnowledgeSearchResult".to_string(),
        "PolicyRequest".to_string(),
        "PolicyDecision".to_string(),
    ])
    .with_streaming_support(ProtocolAdapterStreamingSupport::Ordered)
    .with_trace_support(true)
    .with_security_requirements(vec![
        "local_trusted_boundary".to_string(),
        "policy.evaluate.model.invoke".to_string(),
        "policy.evaluate.knowledge.search".to_string(),
    ])
}

fn validate_rpc_metadata(metadata: &[(String, String)]) -> KernelResult<()> {
    let has_subject_id = metadata
        .iter()
        .any(|(key, _)| key == "sdkwork.policy.subject_id");
    let has_tenant_id = metadata
        .iter()
        .any(|(key, _)| key == "sdkwork.policy.tenant_id");
    let has_roles = metadata
        .iter()
        .any(|(key, _)| key == "sdkwork.policy.roles");

    for (key, _) in metadata {
        if !key.contains('.') {
            return Err(KernelError::validation(format!(
                "agent chat RPC metadata key must be namespaced: {key}"
            ))
            .from_source(KernelErrorSource::ProtocolAdapter));
        }
    }

    if has_subject_id != has_tenant_id {
        return Err(KernelError::validation(
            "agent chat RPC policy subject metadata requires both sdkwork.policy.subject_id and sdkwork.policy.tenant_id",
        )
        .from_source(KernelErrorSource::ProtocolAdapter));
    }

    if has_roles && !(has_subject_id && has_tenant_id) {
        return Err(KernelError::validation(
            "agent chat RPC policy roles require sdkwork.policy.subject_id and sdkwork.policy.tenant_id",
        )
        .from_source(KernelErrorSource::ProtocolAdapter));
    }

    Ok(())
}

fn model_forwardable_rpc_metadata_key(key: &str) -> bool {
    !matches!(
        key,
        "sdkwork.policy.subject_id"
            | "sdkwork.policy.tenant_id"
            | "sdkwork.policy.roles"
            | "sdkwork.chat.include_tools"
            | "sdkwork.chat.include_tool_descriptors"
            | "sdkwork.memory.scope"
            | "sdkwork.memory.owner_context"
            | "sdkwork.memory.provider_id"
    ) && !key.starts_with("sdkwork.knowledge.")
}

fn parse_rpc_bool(value: &str) -> KernelResult<bool> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(KernelError::validation(
            "boolean RPC metadata must be one of true, false, 1, 0, yes, or no",
        )
        .from_source(KernelErrorSource::ProtocolAdapter)),
    }
}

fn parse_chat_memory_scope(value: &str) -> KernelResult<MemoryScope> {
    match value {
        "session" => Ok(MemoryScope::Session),
        "user" => Ok(MemoryScope::User),
        "tenant" => Ok(MemoryScope::Tenant),
        "organization" => Ok(MemoryScope::Organization),
        "agent" => Ok(MemoryScope::Agent),
        "application" => Ok(MemoryScope::Application),
        _ => Err(
            KernelError::validation(format!("unsupported chat memory scope: {value}"))
                .from_source(KernelErrorSource::ProtocolAdapter),
        ),
    }
}

fn parse_knowledge_method(value: &str) -> KernelResult<KnowledgeRetrievalMethod> {
    match value {
        "exact" => Ok(KnowledgeRetrievalMethod::Exact),
        "keyword" => Ok(KnowledgeRetrievalMethod::Keyword),
        "full_text" => Ok(KnowledgeRetrievalMethod::FullText),
        "structured" => Ok(KnowledgeRetrievalMethod::Structured),
        "graph" => Ok(KnowledgeRetrievalMethod::Graph),
        "vector" => Ok(KnowledgeRetrievalMethod::Vector),
        "hybrid" => Ok(KnowledgeRetrievalMethod::Hybrid),
        "llm_rerank" => Ok(KnowledgeRetrievalMethod::LlmRerank),
        "external" => Ok(KnowledgeRetrievalMethod::External),
        _ => Err(KernelError::validation(format!(
            "unsupported chat knowledge retrieval method: {value}"
        ))
        .from_source(KernelErrorSource::ProtocolAdapter)),
    }
}

trait AgentChatMemoryScopeExt {
    fn as_chat_str(&self) -> &'static str;
}

impl AgentChatMemoryScopeExt for MemoryScope {
    fn as_chat_str(&self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::User => "user",
            Self::Tenant => "tenant",
            Self::Organization => "organization",
            Self::Agent => "agent",
            Self::Application => "application",
        }
    }
}

fn memory_record_to_context_frame(
    record: &MemoryRecord,
    request: &AgentChatRequest,
) -> ContextFrame {
    let session_id = request
        .session_id
        .clone()
        .unwrap_or_else(|| format!("session.{}", request.chat_request_id));
    let mut frame = ContextFrame::new(
        format!("context.memory.{}", record.memory_record_id),
        session_id,
        record
            .source
            .clone()
            .unwrap_or_else(|| "memory".to_string()),
        record.content.clone(),
        record.trust_level,
        record.redaction_classification,
    )
    .with_content_type(record.content_type.clone())
    .with_provenance(format!(
        "memory_record_id={};scope={};owner_context={}",
        record.memory_record_id,
        record.scope.as_chat_str(),
        record.owner_context
    ))
    .with_metadata("sdkwork.memory.record_id", record.memory_record_id.clone())
    .with_metadata("sdkwork.memory.scope", record.scope.as_chat_str())
    .with_metadata("sdkwork.memory.owner_context", record.owner_context.clone());

    if let Some(task_id) = &request.task_id {
        frame = frame.for_task(task_id.clone());
    }

    if let Some(created_at) = &record.created_at {
        frame = frame.created_at(created_at.clone());
    }

    if let Some(policy_decision_id) = &record.policy_decision_id {
        frame = frame.with_metadata(
            "sdkwork.memory.policy_decision_id",
            policy_decision_id.clone(),
        );
    }

    for (key, value) in &record.metadata {
        frame = frame.with_metadata(format!("sdkwork.memory.record.{key}"), value.clone());
    }

    frame
}

fn knowledge_search_result_to_context_frame(
    result: &KnowledgeSearchResult,
    request: &AgentChatRequest,
) -> ContextFrame {
    let session_id = request
        .session_id
        .clone()
        .unwrap_or_else(|| format!("session.{}", request.chat_request_id));
    let content = result
        .snippet
        .clone()
        .unwrap_or_else(|| result.title.clone());
    let provenance = result
        .source_uri
        .clone()
        .unwrap_or_else(|| result.document_id.clone());
    let mut frame = ContextFrame::new(
        format!("context.knowledge.{}", result.document_id),
        session_id,
        "knowledge",
        content,
        result.trust_level,
        result.redaction_classification,
    )
    .with_content_type("text/plain")
    .with_provenance(provenance)
    .with_metadata("sdkwork.knowledge.document_id", result.document_id.clone())
    .with_metadata("sdkwork.knowledge.title", result.title.clone())
    .with_metadata("sdkwork.knowledge.kind", result.document_kind.as_str())
    .with_metadata(
        "sdkwork.knowledge.retrieval_method",
        result.retrieval_method.as_str(),
    );

    if let Some(score) = result.score {
        frame = frame.with_metadata("sdkwork.knowledge.score", score.to_string());
    }

    for (key, value) in &result.metadata {
        frame = frame.with_metadata(key.clone(), value.clone());
    }

    if let Some(task_id) = &request.task_id {
        frame = frame.for_task(task_id.clone());
    }

    frame
}

fn chat_response_payload(response: &AgentChatResponse) -> String {
    format!(
        "chat_request_id={};provider_id={};status={:?};messages={}",
        response.chat_request_id,
        response.provider_id,
        response.model_response.status,
        response.model_response.messages.join("\n")
    )
}

fn replace_envelope_metadata(
    envelope: &mut ProtocolObjectEnvelope,
    key: impl Into<String>,
    value: impl Into<String>,
) {
    let key = key.into();
    envelope
        .metadata
        .retain(|(metadata_key, _)| metadata_key != &key);
    envelope.metadata.push((key, value.into()));
}
