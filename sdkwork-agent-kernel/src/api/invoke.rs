//! High-level agent invoke request — single canonical input path for runtime services.

use super::message::AgentConversation;
use crate::{
    agent_messages_to_text_lines, AgentChatRequest, AgentExecutionRequest, AgentInputContract,
    AgentInteractionContract, AgentMessage, KernelResult, ModelRequest, PolicySubject,
    TraceContext,
};

/// Unified invoke surface aligned with industry "chat completion" ergonomics.
///
/// Providers and protocol adapters receive a normalized `ModelRequest` derived
/// from this type — callers should not set legacy `messages` and structured
/// `input_messages` independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInvokeRequest {
    pub request_id: String,
    pub conversation: Vec<AgentMessage>,
    pub interaction: Option<AgentInteractionContract>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub subject: Option<PolicySubject>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
}

impl AgentInvokeRequest {
    pub fn builder(request_id: impl Into<String>) -> AgentInvokeRequestBuilder {
        AgentInvokeRequestBuilder::new(request_id)
    }

    pub fn from_conversation(
        request_id: impl Into<String>,
        conversation: AgentConversation,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            conversation: conversation.into_messages(),
            interaction: None,
            provider_id: None,
            model_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            subject: None,
            trace_context: None,
            timeout_ms: None,
        }
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.conversation.is_empty() {
            return Err(crate::KernelError::validation(
                "invoke request requires at least one message",
            ));
        }
        for message in &self.conversation {
            message.validate()?;
        }
        if let Some(interaction) = &self.interaction {
            interaction.validate()?;
        }
        Ok(())
    }

    pub fn to_model_request(
        &self,
        policy_request_id: impl Into<String>,
    ) -> KernelResult<ModelRequest> {
        self.validate()?;
        let projection = agent_messages_to_text_lines(&self.conversation);
        let mut request = ModelRequest::new(self.request_id.clone(), projection)
            .with_input_messages(self.conversation.clone())
            .with_policy_context(policy_request_id.into());

        if let Some(interaction) = &self.interaction {
            request = request.with_input_contract(interaction.input.clone());
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
        Ok(request)
    }

    pub fn to_chat_request(&self) -> KernelResult<AgentChatRequest> {
        self.validate()?;
        let projection = agent_messages_to_text_lines(&self.conversation);
        let mut request = AgentChatRequest::new(self.request_id.clone(), projection)
            .with_input_messages(self.conversation.clone());

        if let Some(interaction) = &self.interaction {
            request = request.with_input_contract(interaction.input.clone());
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
        Ok(request)
    }

    pub fn to_execution_request(&self) -> KernelResult<AgentExecutionRequest> {
        self.validate()?;
        let projection = agent_messages_to_text_lines(&self.conversation);
        let mut request = AgentExecutionRequest::new(self.request_id.clone(), projection)
            .with_input_messages(self.conversation.clone());

        if let Some(interaction) = &self.interaction {
            request = request.with_input_contract(interaction.input.clone());
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
        Ok(request)
    }
}

/// Fluent builder for [`AgentInvokeRequest`].
#[derive(Debug, Clone)]
pub struct AgentInvokeRequestBuilder {
    request: AgentInvokeRequest,
}

impl AgentInvokeRequestBuilder {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request: AgentInvokeRequest {
                request_id: request_id.into(),
                conversation: Vec::new(),
                interaction: None,
                provider_id: None,
                model_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                step_id: None,
                subject: None,
                trace_context: None,
                timeout_ms: None,
            },
        }
    }

    pub fn conversation(mut self, conversation: AgentConversation) -> Self {
        self.request.conversation = conversation.into_messages();
        self
    }

    pub fn message(mut self, message: AgentMessage) -> KernelResult<Self> {
        message.validate()?;
        self.request.conversation.push(message);
        Ok(self)
    }

    pub fn user_text(
        mut self,
        message_id: impl Into<String>,
        text: impl Into<String>,
    ) -> KernelResult<Self> {
        self.request.conversation.push(
            super::message::MessageBuilder::user()
                .text(text)
                .build(message_id)?,
        );
        Ok(self)
    }

    pub fn interaction(mut self, interaction: AgentInteractionContract) -> Self {
        self.request.interaction = Some(interaction);
        self
    }

    pub fn input_contract(mut self, input_contract: AgentInputContract) -> Self {
        self.request.interaction = Some(AgentInteractionContract {
            schema_version: "1.0.0".to_string(),
            input: input_contract,
            output: crate::AgentOutputContract::text_json(),
        });
        self
    }

    pub fn model_id(mut self, model_id: impl Into<String>) -> Self {
        self.request.model_id = Some(model_id.into());
        self
    }

    pub fn provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.request.provider_id = Some(provider_id.into());
        self
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.request.session_id = Some(session_id.into());
        self
    }

    pub fn build(self) -> KernelResult<AgentInvokeRequest> {
        self.request.validate()?;
        Ok(self.request)
    }
}
