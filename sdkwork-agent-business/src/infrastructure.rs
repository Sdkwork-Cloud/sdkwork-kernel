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
        let expected_version = self.records[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
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
            .filter(|record| {
                let Some(search_query) = query.search_query.as_ref() else {
                    return true;
                };
                let normalized_query = search_query.trim().to_lowercase();
                if normalized_query.is_empty() {
                    return true;
                }

                let description = record.description.as_deref().unwrap_or("");
                record.agent_id.to_lowercase().contains(normalized_query.as_str())
                    || record.code.to_lowercase().contains(normalized_query.as_str())
                    || record
                        .display_name
                        .to_lowercase()
                        .contains(normalized_query.as_str())
                    || description
                        .to_lowercase()
                        .contains(normalized_query.as_str())
            })
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

    fn list_events(&self, tenant_id: u64, agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        let tenant_pattern = format!("tenant_id={tenant_id};");
        let agent_pattern = format!("agent_id={agent_id};");
        let mut events: Vec<KernelEvent> = self
            .events
            .iter()
            .filter(|event| {
                event.payload.contains(tenant_pattern.as_str())
                    && event.payload.contains(agent_pattern.as_str())
            })
            .cloned()
            .collect();

        events.sort_by(|left, right| right.occurred_at.cmp(&left.occurred_at));
        Ok(events)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentBusinessStatus, AgentVisibility};
    use sdkwork_agent_kernel::AgentManifest;

    fn sample_manifest(agent_id: &str) -> AgentManifest {
        AgentManifest {
            schema_version: "1.0.0".to_string(),
            manifest_type: "agent".to_string(),
            agent_id: agent_id.to_string(),
            name: "sample-agent".to_string(),
            display_name: "Sample Agent".to_string(),
            description: "sample".to_string(),
            version: "0.1.0".to_string(),
            domain: "intelligence".to_string(),
            required_capabilities: vec!["model.chat".to_string()],
            optional_capabilities: vec!["tool.invoke".to_string()],
            required_capability_requirements: vec![],
            optional_capability_requirements: vec![],
            event_families: vec!["agent.lifecycle".to_string()],
            owner_name: "sdkwork".to_string(),
            status: "active".to_string(),
        }
    }

    #[test]
    fn in_memory_repository_rejects_stale_record_version_update() {
        let mut repository = InMemoryAgentRepository::new();
        let record = AgentBusinessRecord {
            id: 1,
            agent_id: "agent.alpha".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: None,
            manifest: sample_manifest("agent.alpha"),
            default_code_task_intent: None,
            status: AgentBusinessStatus::Draft,
            visibility: AgentVisibility::Organization,
            tags: vec!["starter".to_string()],
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
        };
        repository
            .insert(record.clone())
            .expect("initial insert should succeed");

        let mut stale = record.clone();
        stale.display_name = "Alpha stale".to_string();
        let error = repository
            .update(stale)
            .expect_err("stale version should fail");
        match error {
            KernelError::Structured { info } => {
                assert_eq!(info.kind.as_str(), "conflict");
                assert!(info.message.contains("version mismatch"));
            }
            _ => panic!("expected structured conflict"),
        }
    }
}
