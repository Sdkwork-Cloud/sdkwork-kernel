use crate::domain::AgentBusinessRecord;
use crate::ports::{AgentAuditSink, AgentListQuery, AgentRepository};
use sdkwork_agent_kernel::{
    KernelError, KernelEvent, KernelResult, PolicyDecision, PolicyProvider, PolicyRequest,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryAgentRepository {
    next_id: u64,
    records: Vec<AgentBusinessRecord>,
}

impl InMemoryAgentRepository {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            records: Vec::new(),
        }
    }

    pub fn records(&self) -> &[AgentBusinessRecord] {
        &self.records
    }
}

impl AgentRepository for InMemoryAgentRepository {
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    fn insert(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        if self
            .records
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id)
        {
            return Err(KernelError::conflict("agent already exists"));
        }
        if self
            .records
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict("agent code already exists"));
        }
        self.records.push(record);
        Ok(())
    }

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        let Some(index) = self
            .records
            .iter()
            .position(|existing| existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id)
        else {
            return Err(KernelError::validation("agent not found"));
        };
        if self.records.iter().enumerate().any(|(current, existing)| {
            current != index && existing.tenant_id == record.tenant_id && existing.code == record.code
        }) {
            return Err(KernelError::conflict("agent code already exists"));
        }
        self.records[index] = record;
        Ok(())
    }

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRecord> {
        self.records
            .iter()
            .find(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
    }

    fn list(&self, query: &AgentListQuery) -> Vec<AgentBusinessRecord> {
        self.records
            .iter()
            .filter(|record| record.tenant_id == query.tenant_id)
            .filter(|record| {
                if let Some(organization_id) = query.organization_id {
                    record.organization_id == organization_id
                } else {
                    true
                }
            })
            .filter(|record| {
                if let Some(owner_user_id) = query.owner_user_id {
                    record.owner_user_id == owner_user_id
                } else {
                    true
                }
            })
            .filter(|record| query.include_deleted || !record.is_deleted())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryAgentAuditSink {
    events: Vec<KernelEvent>,
}

impl InMemoryAgentAuditSink {
    pub fn events(&self) -> &[KernelEvent] {
        &self.events
    }
}

impl AgentAuditSink for InMemoryAgentAuditSink {
    fn record(&mut self, event: KernelEvent) -> KernelResult<()> {
        self.events.push(event);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyMode {
    Allow,
    Deny(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowAllPolicyProvider {
    pub provider_id: String,
    pub mode: PolicyMode,
}

impl AllowAllPolicyProvider {
    pub fn allow(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            mode: PolicyMode::Allow,
        }
    }

    pub fn deny(provider_id: impl Into<String>, reason_code: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            mode: PolicyMode::Deny(reason_code.into()),
        }
    }
}

impl PolicyProvider for AllowAllPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision_{}", request.policy_request_id);
        match &self.mode {
            PolicyMode::Allow => Ok(PolicyDecision::allow(
                decision_id,
                request.policy_request_id,
                self.provider_id.clone(),
            )),
            PolicyMode::Deny(reason) => Ok(PolicyDecision::deny(
                decision_id,
                request.policy_request_id,
                self.provider_id.clone(),
                reason.clone(),
            )),
        }
    }
}
