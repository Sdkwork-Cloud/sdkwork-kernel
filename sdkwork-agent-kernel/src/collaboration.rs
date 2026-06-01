use crate::{
    AgentMessage, KernelError, KernelResult, ProviderHealth, ProviderManifest,
    RedactionClassification, TraceContext, TrustLevel,
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

pub trait AgentCollaborationProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

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
}
