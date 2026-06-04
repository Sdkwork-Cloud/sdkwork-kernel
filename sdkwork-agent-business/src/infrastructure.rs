use crate::domain::{AgentBusinessRecord, AgentDeploymentRecord, AgentProviderBindingRecord};
use crate::ports::{AgentAuditSink, AgentListQuery, AgentRepository};
use sdkwork_agent_kernel::{
    KernelError, KernelEvent, KernelResult, PolicyDecision, PolicyProvider, PolicyRequest,
};
use std::cmp::Ordering;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryAgentRepository {
    next_id: u64,
    records: Vec<AgentBusinessRecord>,
    provider_bindings: Vec<AgentProviderBindingRecord>,
    deployments: Vec<AgentDeploymentRecord>,
}

impl InMemoryAgentRepository {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            records: Vec::new(),
            provider_bindings: Vec::new(),
            deployments: Vec::new(),
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
        if self.records.iter().any(|existing| {
            existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id
        }) {
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
        let Some(index) = self.records.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id
        }) else {
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
            current != index
                && existing.tenant_id == record.tenant_id
                && existing.code == record.code
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
                record
                    .agent_id
                    .to_lowercase()
                    .contains(normalized_query.as_str())
                    || record
                        .code
                        .to_lowercase()
                        .contains(normalized_query.as_str())
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

    fn insert_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        if self.provider_bindings.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.binding_id == record.binding_id
        }) {
            return Err(KernelError::conflict(
                "agent provider binding already exists",
            ));
        }
        if record.active
            && self.provider_bindings.iter().any(|existing| {
                existing.tenant_id == record.tenant_id
                    && existing.agent_id == record.agent_id
                    && existing.active
            })
        {
            return Err(KernelError::conflict(
                "active provider binding already exists",
            ));
        }
        self.provider_bindings.push(record);
        Ok(())
    }

    fn update_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        let Some(index) = self.provider_bindings.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.binding_id == record.binding_id
        }) else {
            return Err(KernelError::validation("agent provider binding not found"));
        };
        let expected_version = self.provider_bindings[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "provider binding version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if record.active
            && self
                .provider_bindings
                .iter()
                .enumerate()
                .any(|(current, existing)| {
                    current != index
                        && existing.tenant_id == record.tenant_id
                        && existing.agent_id == record.agent_id
                        && existing.active
                })
        {
            return Err(KernelError::conflict(
                "active provider binding already exists",
            ));
        }
        self.provider_bindings[index] = record;
        Ok(())
    }

    fn get_provider_binding(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRecord> {
        self.provider_bindings
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id
                    && record.agent_id == agent_id
                    && record.binding_id == binding_id
            })
            .cloned()
    }

    fn list_provider_bindings(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRecord> {
        let mut records: Vec<AgentProviderBindingRecord> = self
            .provider_bindings
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
            .collect();
        records.sort_by(compare_provider_bindings_standard_order);
        records
    }

    fn insert_deployment(&mut self, record: AgentDeploymentRecord) -> KernelResult<()> {
        if self.deployments.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.deployment_id == record.deployment_id
        }) {
            return Err(KernelError::conflict("agent deployment already exists"));
        }
        self.deployments.push(record);
        Ok(())
    }

    fn list_deployments(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRecord> {
        let mut records: Vec<AgentDeploymentRecord> = self
            .deployments
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
            .collect();
        records.sort_by(compare_deployments_standard_order);
        records
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

        events.sort_by(compare_audit_events_desc);
        Ok(events)
    }
}

fn compare_audit_events_desc(left: &KernelEvent, right: &KernelEvent) -> Ordering {
    let left_time = parse_occurred_at(left.occurred_at.as_deref());
    let right_time = parse_occurred_at(right.occurred_at.as_deref());

    right_time
        .cmp(&left_time)
        .then_with(|| right.event_id.cmp(&left.event_id))
}

fn compare_provider_bindings_standard_order(
    left: &AgentProviderBindingRecord,
    right: &AgentProviderBindingRecord,
) -> Ordering {
    right
        .active
        .cmp(&left.active)
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| left.binding_id.cmp(&right.binding_id))
}

fn compare_deployments_standard_order(
    left: &AgentDeploymentRecord,
    right: &AgentDeploymentRecord,
) -> Ordering {
    right
        .created_at
        .cmp(&left.created_at)
        .then_with(|| left.deployment_id.cmp(&right.deployment_id))
}

fn parse_occurred_at(value: Option<&str>) -> Option<OffsetDateTime> {
    let value = value?;
    OffsetDateTime::parse(value, &Rfc3339).ok()
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
    use crate::domain::{
        AgentBusinessStatus, AgentImplementationKind, AgentProviderBindingRecord, AgentVisibility,
    };
    use sdkwork_agent_kernel::AgentManifest;
    use sdkwork_agent_kernel::KernelErrorKind;

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
            implementation_provider_id: None,
            implementation_kind: None,
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

    #[test]
    fn in_memory_repository_rejects_stale_provider_binding_update() {
        let mut repository = InMemoryAgentRepository::new();
        let record = AgentProviderBindingRecord {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            active: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        };
        repository
            .insert_provider_binding(record.clone())
            .expect("initial binding insert should succeed");

        let mut stale = record.clone();
        stale.provider_id = "provider.model.rig-alt".to_string();
        let error = repository
            .update_provider_binding(stale)
            .expect_err("stale binding version should fail");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("provider binding version mismatch"));
    }

    #[test]
    fn in_memory_repository_rejects_second_active_provider_binding() {
        let mut repository = InMemoryAgentRepository::new();
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.local".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:00:00Z".to_string(),
            })
            .expect("first active binding insert should succeed");

        let error = repository
            .insert_provider_binding(AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            })
            .expect_err("second active binding should fail");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("active provider binding already exists"));
    }

    #[test]
    fn in_memory_repository_rejects_update_that_creates_second_active_provider_binding() {
        let mut repository = InMemoryAgentRepository::new();
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.local".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:00:00Z".to_string(),
            })
            .expect("first active binding insert should succeed");
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            })
            .expect("inactive binding insert should succeed");

        let error = repository
            .update_provider_binding(AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 2,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            })
            .expect_err("update cannot create a second active binding");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("active provider binding already exists"));
    }

    #[test]
    fn in_memory_repository_lists_provider_bindings_in_standard_order() {
        let mut repository = InMemoryAgentRepository::new();
        for record in [
            AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.beta".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.beta".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
            AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.default".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            },
            AgentProviderBindingRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alpha".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alpha".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
        ] {
            repository
                .insert_provider_binding(record)
                .expect("binding insert should succeed");
        }

        let binding_ids: Vec<String> = repository
            .list_provider_bindings(1, "agent.alpha")
            .into_iter()
            .map(|record| record.binding_id)
            .collect();

        assert_eq!(
            binding_ids,
            vec![
                "binding.rig.default".to_string(),
                "binding.rig.alpha".to_string(),
                "binding.rig.beta".to_string()
            ]
        );
    }

    #[test]
    fn in_memory_repository_lists_deployments_in_standard_order() {
        let mut repository = InMemoryAgentRepository::new();
        for record in [
            AgentDeploymentRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.beta".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:02:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
            AgentDeploymentRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.latest".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:03:00Z".to_string(),
                updated_at: "2026-06-01T00:03:00Z".to_string(),
            },
            AgentDeploymentRecord {
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:02:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
        ] {
            repository
                .insert_deployment(record)
                .expect("deployment insert should succeed");
        }

        let deployment_ids: Vec<String> = repository
            .list_deployments(1, "agent.alpha")
            .into_iter()
            .map(|record| record.deployment_id)
            .collect();

        assert_eq!(
            deployment_ids,
            vec![
                "deployment.rig.latest".to_string(),
                "deployment.rig.alpha".to_string(),
                "deployment.rig.beta".to_string()
            ]
        );
    }
}
