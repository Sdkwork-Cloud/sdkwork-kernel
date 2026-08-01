use crate::{
    AgentExecutionRequest, AgentExecutionService, AgentMessage, AgentRuntime, AgentStreamEvent,
    AgentStreamSink, KernelError, KernelResult, PolicySubject, ProgressEvent, ProviderHealth,
    ProviderManifest, RedactionClassification, SubagentStopContext, TraceContext, TrustLevel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCard {
    pub agent_id: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub endpoint: Option<String>,
    pub capabilities: Vec<String>,
    pub input_modes: Vec<String>,
    pub output_modes: Vec<String>,
    pub provider_id: Option<String>,
    pub trust_level: TrustLevel,
    pub metadata: Vec<(String, String)>,
}

impl AgentCard {
    pub fn new(
        agent_id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            display_name: display_name.into(),
            description: description.into(),
            version: version.into(),
            endpoint: None,
            capabilities: Vec::new(),
            input_modes: Vec::new(),
            output_modes: Vec::new(),
            provider_id: None,
            trust_level: TrustLevel::UnknownUntrusted,
            metadata: Vec::new(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
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

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
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

    pub fn is_untrusted(&self) -> bool {
        matches!(
            self.trust_level,
            TrustLevel::UserSupplied
                | TrustLevel::ToolOutput
                | TrustLevel::RetrievedExternal
                | TrustLevel::AgentMessage
                | TrustLevel::UnknownUntrusted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffRequest {
    pub handoff_id: String,
    pub source_agent_id: String,
    pub target_agent_id: String,
    pub objective: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub messages: Vec<AgentMessage>,
    pub context_frame_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
    pub policy_context_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub input_filter: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl AgentHandoffRequest {
    pub fn new(
        handoff_id: impl Into<String>,
        source_agent_id: impl Into<String>,
        target_agent_id: impl Into<String>,
        objective: impl Into<String>,
    ) -> Self {
        Self {
            handoff_id: handoff_id.into(),
            source_agent_id: source_agent_id.into(),
            target_agent_id: target_agent_id.into(),
            objective: objective.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            messages: Vec::new(),
            context_frame_ids: Vec::new(),
            artifact_ids: Vec::new(),
            policy_context_id: None,
            trace_context: None,
            input_filter: None,
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

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_context_frame(mut self, context_frame_id: impl Into<String>) -> Self {
        self.context_frame_ids.push(context_frame_id.into());
        self
    }

    pub fn with_artifact(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_ids.push(artifact_id.into());
        self
    }

    pub fn with_policy_context(mut self, policy_context_id: impl Into<String>) -> Self {
        self.policy_context_id = Some(policy_context_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_input_filter(mut self, input_filter: impl Into<String>) -> Self {
        self.input_filter = Some(input_filter.into());
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
pub struct AgentDelegation {
    pub delegation_id: String,
    pub source_agent_id: String,
    pub target_agent_id: String,
    pub capability_id: String,
    pub policy_context_id: Option<String>,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl AgentDelegation {
    pub fn new(
        delegation_id: impl Into<String>,
        source_agent_id: impl Into<String>,
        target_agent_id: impl Into<String>,
        capability_id: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            source_agent_id: source_agent_id.into(),
            target_agent_id: target_agent_id.into(),
            capability_id: capability_id.into(),
            policy_context_id: None,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_policy_context(mut self, policy_context_id: impl Into<String>) -> Self {
        self.policy_context_id = Some(policy_context_id.into());
        self
    }

    pub fn with_redaction(mut self, redaction_classification: RedactionClassification) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffResult {
    pub handoff_id: String,
    pub delegation: AgentDelegation,
    pub status: String,
    pub messages: Vec<AgentMessage>,
    pub artifact_ids: Vec<String>,
    pub trace_context: Option<TraceContext>,
    pub metadata: Vec<(String, String)>,
}

impl AgentHandoffResult {
    pub fn accepted(handoff_id: impl Into<String>, delegation: AgentDelegation) -> Self {
        Self {
            handoff_id: handoff_id.into(),
            delegation,
            status: "accepted".to_string(),
            messages: Vec::new(),
            artifact_ids: Vec::new(),
            trace_context: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_artifact(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_ids.push(artifact_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDelegationRequest {
    pub delegation_id: String,
    pub source_agent_id: String,
    pub target_agent_id: String,
    pub capability_id: String,
    pub task_description: String,
    pub policy_context_id: Option<String>,
    pub timeout_ms: Option<u64>,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl AgentDelegationRequest {
    pub fn new(
        delegation_id: impl Into<String>,
        source_agent_id: impl Into<String>,
        target_agent_id: impl Into<String>,
        capability_id: impl Into<String>,
        task_description: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            source_agent_id: source_agent_id.into(),
            target_agent_id: target_agent_id.into(),
            capability_id: capability_id.into(),
            task_description: task_description.into(),
            policy_context_id: None,
            timeout_ms: None,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_policy_context(mut self, policy_context_id: impl Into<String>) -> Self {
        self.policy_context_id = Some(policy_context_id.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: RedactionClassification) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDelegationResult {
    pub delegation_id: String,
    pub delegation: AgentDelegation,
    pub status: AgentDelegationStatus,
    pub result: Option<String>,
    pub messages: Vec<AgentMessage>,
    pub artifact_ids: Vec<String>,
    pub trace_context: Option<TraceContext>,
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentDelegationStatus {
    Accepted,
    Completed,
    Failed,
    Rejected,
    TimedOut,
    Cancelled,
}

impl AgentDelegationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

impl AgentDelegationResult {
    pub fn accepted(delegation_id: impl Into<String>, delegation: AgentDelegation) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            delegation,
            status: AgentDelegationStatus::Accepted,
            result: None,
            messages: Vec::new(),
            artifact_ids: Vec::new(),
            trace_context: None,
            metadata: Vec::new(),
        }
    }

    pub fn completed(
        delegation_id: impl Into<String>,
        delegation: AgentDelegation,
        result: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            delegation,
            status: AgentDelegationStatus::Completed,
            result: Some(result.into()),
            messages: Vec::new(),
            artifact_ids: Vec::new(),
            trace_context: None,
            metadata: Vec::new(),
        }
    }

    pub fn failed(
        delegation_id: impl Into<String>,
        delegation: AgentDelegation,
        error: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            delegation,
            status: AgentDelegationStatus::Failed,
            result: Some(error.into()),
            messages: Vec::new(),
            artifact_ids: Vec::new(),
            trace_context: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_artifact(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_ids.push(artifact_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

pub trait AgentCollaborationProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn list_agents(&self) -> Vec<AgentCard>;

    fn describe_agent(&self, agent_id: &str) -> KernelResult<AgentCard> {
        self.list_agents()
            .into_iter()
            .find(|agent| agent.agent_id == agent_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: format!("agent.discover.{agent_id}"),
            })
    }

    fn handoff(&self, request: AgentHandoffRequest) -> KernelResult<AgentHandoffResult>;

    fn delegate(&self, request: AgentDelegationRequest) -> KernelResult<AgentDelegationResult>;
}

// ============================================================================
// Streaming Delegation - agent-as-tool sub-agent execution
// ============================================================================

/// Streaming delegation request: run a task on a sub-agent session and
/// relay its stream to the parent, aligning with the agent SDK delegation
/// primitives (codex `spawn_agent`, claude `Task` tool, hermes
/// `delegate_task`, rig agent-as-tool).
#[derive(Debug, Clone, PartialEq)]
pub struct AgentDelegationStreamRequest {
    pub delegation_id: String,
    /// Parent session that spawned the delegation.
    pub source_session_id: String,
    /// Parent tool call that produced this delegation; child messages link
    /// back through the parent chain (`parent_tool_use_id` semantics).
    pub tool_call_id: Option<String>,
    pub task_description: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub timeout_ms: Option<u64>,
    pub subject: Option<PolicySubject>,
    pub trace_context: Option<TraceContext>,
}

impl AgentDelegationStreamRequest {
    pub fn new(
        delegation_id: impl Into<String>,
        source_session_id: impl Into<String>,
        task_description: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            source_session_id: source_session_id.into(),
            tool_call_id: None,
            task_description: task_description.into(),
            provider_id: None,
            model_id: None,
            timeout_ms: None,
            subject: None,
            trace_context: None,
        }
    }

    pub fn from_tool_call(
        delegation_id: impl Into<String>,
        source_session_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        task_description: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: Some(tool_call_id.into()),
            ..Self::new(delegation_id, source_session_id, task_description)
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
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
}

/// Streaming delegation service. Runs the delegated task on a sub-agent
/// session through the kernel execution loop and relays the child stream to
/// the parent sink, tagging child messages with the parent tool call id.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentDelegationService;

impl AgentDelegationService {
    pub fn new() -> Self {
        Self
    }

    pub fn delegate_streaming(
        &self,
        runtime: &AgentRuntime,
        request: AgentDelegationStreamRequest,
        sink: &mut dyn AgentStreamSink,
    ) -> KernelResult<()> {
        let child_session_id = format!("session.subagent.{}", request.delegation_id);

        // Task lifecycle: spawned notice before the child runs.
        sink.push_event(
            AgentStreamEvent::Progress(
                ProgressEvent::new(
                    format!("{}.delegate.spawned", request.delegation_id),
                    "delegate.task_spawned",
                )
                .with_detail(format!(
                    "delegating to sub-agent session {child_session_id}"
                ))
                .with_stream_id(request.delegation_id.clone()),
            )
            .with_session_id(request.source_session_id.clone()),
        )?;

        let mut execution_request = AgentExecutionRequest::new(
            format!("exec.delegation.{}", request.delegation_id),
            vec![request.task_description],
        )
        .for_session(child_session_id.clone())
        .with_timeout_ms(request.timeout_ms.unwrap_or(60_000));
        if let Some(provider_id) = &request.provider_id {
            execution_request = execution_request.with_provider_id(provider_id.clone());
        }
        if let Some(model_id) = &request.model_id {
            execution_request = execution_request.with_model_id(model_id.clone());
        }
        if let Some(subject) = &request.subject {
            execution_request = execution_request.with_subject(subject.clone());
        }
        if let Some(trace_context) = &request.trace_context {
            execution_request = execution_request.with_trace_context(trace_context.clone());
        }

        let mut relay = DelegationRelaySink {
            inner: sink,
            tool_call_id: request.tool_call_id.clone(),
        };
        AgentExecutionService::new().execute_streaming(runtime, execution_request, &mut relay)?;

        // Sub-agent stop hook: observers learn the delegation outcome.
        runtime
            .hooks()
            .run_subagent_stop(&SubagentStopContext::new(
                request.delegation_id.clone(),
                child_session_id.clone(),
                "completed",
                0,
            ))?;

        // Task lifecycle: completed notice after the child stream ends.
        sink.push_event(
            AgentStreamEvent::Progress(
                ProgressEvent::new(
                    format!("{}.delegate.completed", request.delegation_id),
                    "delegate.task_completed",
                )
                .with_detail(format!("sub-agent session {child_session_id} completed"))
                .with_stream_id(request.delegation_id.clone()),
            )
            .with_session_id(request.source_session_id.clone()),
        )?;

        Ok(())
    }
}

/// Relay that rewrites child message starts with the parent tool call id so
/// the parent can reconstruct the delegation lineage.
struct DelegationRelaySink<'a> {
    inner: &'a mut dyn AgentStreamSink,
    tool_call_id: Option<String>,
}

impl AgentStreamSink for DelegationRelaySink<'_> {
    fn push_event(&mut self, event: AgentStreamEvent) -> KernelResult<()> {
        let event = match event {
            AgentStreamEvent::MessageStart(mut start) => {
                if let Some(tool_call_id) = &self.tool_call_id {
                    if start.parent_message_id.is_none() {
                        start = start.with_parent_message(tool_call_id.clone());
                    }
                }
                AgentStreamEvent::MessageStart(start)
            }
            other => other,
        };
        self.inner.push_event(event)
    }
}
