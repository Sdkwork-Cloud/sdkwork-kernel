use crate::domain::AgentBusinessRecord;
use sdkwork_agent_kernel::{KernelEvent, KernelResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentListQuery {
    pub tenant_id: u64,
    pub organization_id: Option<u64>,
    pub owner_user_id: Option<u64>,
    pub include_deleted: bool,
    pub search_query: Option<String>,
}

impl AgentListQuery {
    pub fn for_tenant(tenant_id: u64) -> Self {
        Self {
            tenant_id,
            organization_id: None,
            owner_user_id: None,
            include_deleted: false,
            search_query: None,
        }
    }

    pub fn for_organization(mut self, organization_id: u64) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    pub fn for_owner(mut self, owner_user_id: u64) -> Self {
        self.owner_user_id = Some(owner_user_id);
        self
    }

    pub fn with_deleted(mut self) -> Self {
        self.include_deleted = true;
        self
    }

    pub fn with_search(mut self, query: impl Into<String>) -> Self {
        let query = query.into();
        self.search_query = if query.trim().is_empty() {
            None
        } else {
            Some(query)
        };
        self
    }
}

pub trait AgentRepository {
    fn next_id(&mut self) -> u64;

    fn insert(&mut self, record: AgentBusinessRecord) -> KernelResult<()>;

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()>;

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRecord>;

    fn list(&self, query: &AgentListQuery) -> Vec<AgentBusinessRecord>;
}

pub trait AgentAuditSink {
    fn record(&mut self, event: KernelEvent) -> KernelResult<()>;

    fn list_events(&self, _tenant_id: u64, _agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        Ok(Vec::new())
    }
}
