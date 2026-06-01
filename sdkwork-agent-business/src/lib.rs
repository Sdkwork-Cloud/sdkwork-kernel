mod api;
mod application;
mod domain;
mod dto;
mod infrastructure;
mod persistence;
mod ports;
#[cfg(feature = "http-axum")]
mod http;

pub use api::{
    ApiOperation, AGENT_BACKEND_API_OPERATIONS, AGENT_BACKEND_API_PREFIX, AGENT_APP_API_OPERATIONS,
    AGENT_APP_API_PREFIX,
};
pub use application::{
    AgentBusinessService, ChangeAgentStatusCommand, CreateAgentCommand, DeleteAgentCommand,
    GetAgentCommand, ListAgentsCommand, RestoreAgentCommand, UpdateAgentCommand,
};
pub use domain::{
    AgentAuditAction, AgentBusinessRecord, AgentBusinessStatus, AgentVisibility,
    DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY,
};
pub use dto::{
    AgentListResponseDto, AgentRecordDto, AgentResponseDto, CreateAgentRequestDto,
    DeleteAgentRequestDto, GetAgentRequestDto, ListAgentsRequestDto, RestoreAgentRequestDto,
    UpdateAgentRequestDto, UpdateAgentStatusRequestDto,
};
pub use infrastructure::{
    AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository, PolicyMode,
};
pub use persistence::{
    AgentAuditEventRow, AgentBusinessRow, PostgresAgentAuditSink, PostgresAgentRepository,
    PostgresAgentRepositoryAdapter, SQL_INSERT_AGENT_BUSINESS, SQL_INSERT_AUDIT_EVENT,
    SQL_LIST_AGENT_BUSINESS, SQL_NEXT_AGENT_BUSINESS_ID, SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID,
    SQL_UPDATE_AGENT_BUSINESS,
};
#[cfg(feature = "postgres-sync")]
pub use persistence::SyncPostgresAdapter;
pub use ports::{AgentAuditSink, AgentListQuery, AgentRepository};
#[cfg(feature = "http-axum")]
pub use http::{build_app_router, build_backend_router, build_combined_router, AgentHttpState};
