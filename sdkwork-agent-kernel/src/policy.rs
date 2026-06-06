use crate::{
    KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource, KernelResult,
    ProviderHealth, ProviderManifest, SideEffectLevel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyCategory {
    ModelInvoke,
    ModelSendSensitiveContext,
    ToolInvoke,
    ToolExternalSend,
    MemoryRead,
    MemoryWrite,
    MemoryDelete,
    KnowledgeSearch,
    KnowledgeRead,
    KnowledgeList,
    HostFilesystemRead,
    HostFilesystemWrite,
    HostProcessExecute,
    HostNetworkConnect,
    HostSecretsRead,
    ArtifactRead,
    ArtifactWrite,
    ProtocolSend,
    AgentInstall,
    AgentUninstall,
    AgentUpgrade,
    AgentConfigure,
    ProviderRegister,
    ProviderConfigure,
    ProductSpecific(String),
}

impl PolicyCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ModelInvoke => "model.invoke",
            Self::ModelSendSensitiveContext => "model.send_sensitive_context",
            Self::ToolInvoke => "tool.invoke",
            Self::ToolExternalSend => "tool.external_send",
            Self::MemoryRead => "memory.read",
            Self::MemoryWrite => "memory.write",
            Self::MemoryDelete => "memory.delete",
            Self::KnowledgeSearch => "knowledge.search",
            Self::KnowledgeRead => "knowledge.read",
            Self::KnowledgeList => "knowledge.list",
            Self::HostFilesystemRead => "host.filesystem.read",
            Self::HostFilesystemWrite => "host.filesystem.write",
            Self::HostProcessExecute => "host.process.execute",
            Self::HostNetworkConnect => "host.network.connect",
            Self::HostSecretsRead => "host.secrets.read",
            Self::ArtifactRead => "artifact.read",
            Self::ArtifactWrite => "artifact.write",
            Self::ProtocolSend => "protocol.send",
            Self::AgentInstall => "agent.install",
            Self::AgentUninstall => "agent.uninstall",
            Self::AgentUpgrade => "agent.upgrade",
            Self::AgentConfigure => "agent.configure",
            Self::ProviderRegister => "provider.register",
            Self::ProviderConfigure => "provider.configure",
            Self::ProductSpecific(category) => category.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySubject {
    pub subject_id: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
}

impl PolicySubject {
    pub fn new(subject_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            subject_id: subject_id.into(),
            tenant_id: tenant_id.into(),
            roles: Vec::new(),
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRequest {
    pub policy_request_id: String,
    pub category: String,
    pub typed_category: Option<PolicyCategory>,
    pub subject: Option<PolicySubject>,
    pub action: Option<String>,
    pub resource: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub side_effect_level: Option<SideEffectLevel>,
    pub context: Vec<(String, String)>,
    pub redaction_classification: KernelEventRedaction,
}

impl PolicyRequest {
    pub fn new(
        policy_request_id: impl Into<String>,
        category: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            policy_request_id: policy_request_id.into(),
            category: category.into(),
            typed_category: None,
            subject: None,
            action: None,
            resource: resource.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            side_effect_level: None,
            context: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_category(mut self, category: PolicyCategory) -> Self {
        self.category = category.as_str().to_string();
        self.typed_category = Some(category);
        self
    }

    pub fn with_subject(mut self, subject: PolicySubject) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_side_effect_level(mut self, side_effect_level: SideEffectLevel) -> Self {
        self.side_effect_level = Some(side_effect_level);
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn context_value(&self, key: &str) -> Option<&str> {
        self.context
            .iter()
            .find(|(context_key, _)| context_key == key)
            .map(|(_, value)| value.as_str())
    }
}

pub trait PolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.unspecified",
            "policy",
            "policy-provider",
            "0.0.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecisionValue {
    Allow,
    Deny,
    NeedsApproval,
    Defer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    pub decision_id: String,
    pub request_id: String,
    pub decision: PolicyDecisionValue,
    pub policy_provider_id: String,
    pub reason_code: String,
    pub safe_reason: Option<String>,
    pub expires_at: Option<String>,
    pub constraints: Vec<PolicyDecisionConstraint>,
    pub audit_required: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecisionConstraint {
    pub key: String,
    pub value: String,
}

impl PolicyDecisionConstraint {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl PolicyDecision {
    pub fn allow(
        decision_id: impl Into<String>,
        request_id: impl Into<String>,
        policy_provider_id: impl Into<String>,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            request_id: request_id.into(),
            decision: PolicyDecisionValue::Allow,
            policy_provider_id: policy_provider_id.into(),
            reason_code: "allowed".to_string(),
            safe_reason: None,
            expires_at: None,
            constraints: Vec::new(),
            audit_required: false,
            created_at: None,
        }
    }

    pub fn deny(
        decision_id: impl Into<String>,
        request_id: impl Into<String>,
        policy_provider_id: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            request_id: request_id.into(),
            decision: PolicyDecisionValue::Deny,
            policy_provider_id: policy_provider_id.into(),
            reason_code: reason_code.into(),
            safe_reason: None,
            expires_at: None,
            constraints: Vec::new(),
            audit_required: false,
            created_at: None,
        }
    }

    pub fn needs_approval(
        decision_id: impl Into<String>,
        request_id: impl Into<String>,
        policy_provider_id: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            request_id: request_id.into(),
            decision: PolicyDecisionValue::NeedsApproval,
            policy_provider_id: policy_provider_id.into(),
            reason_code: reason_code.into(),
            safe_reason: None,
            expires_at: None,
            constraints: Vec::new(),
            audit_required: false,
            created_at: None,
        }
    }

    pub fn defer(
        decision_id: impl Into<String>,
        request_id: impl Into<String>,
        policy_provider_id: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            request_id: request_id.into(),
            decision: PolicyDecisionValue::Defer,
            policy_provider_id: policy_provider_id.into(),
            reason_code: reason_code.into(),
            safe_reason: None,
            expires_at: None,
            constraints: Vec::new(),
            audit_required: false,
            created_at: None,
        }
    }

    pub fn with_safe_reason(mut self, safe_reason: impl Into<String>) -> Self {
        self.safe_reason = Some(safe_reason.into());
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_expires_at(mut self, expires_at: impl Into<String>) -> Self {
        self.expires_at = Some(expires_at.into());
        self
    }

    pub fn with_constraint(mut self, constraint: PolicyDecisionConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn require_audit(mut self) -> Self {
        self.audit_required = true;
        self
    }

    pub fn is_allow(&self) -> bool {
        self.decision == PolicyDecisionValue::Allow
    }

    pub fn is_needs_approval(&self) -> bool {
        self.decision == PolicyDecisionValue::NeedsApproval
    }

    pub fn to_event(&self, event_id: impl Into<String>, request: &PolicyRequest) -> KernelEvent {
        let event_type = match self.decision {
            PolicyDecisionValue::Allow => "agent.policy.allowed",
            PolicyDecisionValue::Deny => "agent.policy.denied",
            PolicyDecisionValue::NeedsApproval => "agent.policy.needs_approval",
            PolicyDecisionValue::Defer => "agent.policy.deferred",
        };
        let severity = match self.decision {
            PolicyDecisionValue::Allow => KernelEventSeverity::Info,
            PolicyDecisionValue::Deny | PolicyDecisionValue::NeedsApproval => {
                KernelEventSeverity::Warn
            }
            PolicyDecisionValue::Defer => KernelEventSeverity::Error,
        };

        let mut event = KernelEvent::new(event_id, event_type, severity, self.event_payload())
            .from_source(KernelEventSource::Policy)
            .with_redaction(request.redaction_classification)
            .with_payload_schema("sdkwork.agent.policy.decision.v1");

        if let Some(session_id) = &request.session_id {
            event = event.for_session(session_id.clone());
        }

        if let Some(task_id) = &request.task_id {
            event = event.for_task(task_id.clone());
        }

        if let Some(run_id) = &request.run_id {
            event = event.for_run(run_id.clone());
        }

        event
    }

    fn event_payload(&self) -> String {
        format!(
            "decision_id={};request_id={};decision={};reason_code={};safe_reason={};audit_required={}",
            self.decision_id,
            self.request_id,
            self.decision.as_str(),
            self.reason_code,
            self.safe_reason.as_deref().unwrap_or(""),
            self.audit_required
        )
    }
}

impl PolicyDecisionValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::NeedsApproval => "needs_approval",
            Self::Defer => "defer",
        }
    }
}
