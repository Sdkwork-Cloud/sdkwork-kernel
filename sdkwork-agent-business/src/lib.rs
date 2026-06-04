mod api;
mod application;
mod domain;
mod dto;
#[cfg(feature = "http-axum")]
mod http;
mod infrastructure;
mod persistence;
mod ports;
mod validation;

pub use api::{
    ApiOperation, AGENT_APP_API_OPERATIONS, AGENT_APP_API_PREFIX, AGENT_BACKEND_API_OPERATIONS,
    AGENT_BACKEND_API_PREFIX,
};
pub use application::{
    ActivateAgentProviderBindingCommand, AgentBusinessService, AgentProviderBindingCommand,
    AgentProviderDeploymentCommand, ChangeAgentStatusCommand, CreateAgentCommand,
    DeleteAgentCommand, GetAgentCommand, ListAgentsCommand, RestoreAgentCommand,
    UpdateAgentCommand,
};
pub use domain::{
    AgentAuditAction, AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord,
    AgentDeploymentStatus, AgentImplementationKind, AgentProviderBindingRecord, AgentVisibility,
    DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY,
};
pub use dto::{
    ActivateAgentProviderBindingRequestDto, AgentDeploymentListResponseDto,
    AgentDeploymentRecordDto, AgentDeploymentResponseDto, AgentListResponseDto,
    AgentProviderBindingListResponseDto, AgentProviderBindingRecordDto,
    AgentProviderBindingRequestDto, AgentProviderBindingResponseDto,
    AgentProviderDeploymentRequestDto, AgentRecordDto, AgentResponseDto, CreateAgentRequestDto,
    DeleteAgentRequestDto, GetAgentRequestDto, ListAgentsRequestDto, RestoreAgentRequestDto,
    UpdateAgentRequestDto, UpdateAgentStatusRequestDto,
};
#[cfg(feature = "http-axum")]
pub use http::{build_app_router, build_backend_router, build_combined_router, AgentHttpState};
pub use infrastructure::{
    AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository, PolicyMode,
};
#[cfg(feature = "postgres-sync")]
pub use persistence::SyncPostgresAdapter;
pub use persistence::{
    AgentAuditEventRow, AgentBusinessRow, AgentDeploymentRow, AgentProviderBindingRow,
    PostgresAgentAuditSink, PostgresAgentRepository, PostgresAgentRepositoryAdapter,
    SQL_INSERT_AGENT_BUSINESS, SQL_INSERT_AGENT_DEPLOYMENT, SQL_INSERT_AGENT_PROVIDER_BINDING,
    SQL_INSERT_AUDIT_EVENT, SQL_LIST_AGENT_BUSINESS, SQL_LIST_AGENT_DEPLOYMENTS,
    SQL_LIST_AGENT_PROVIDER_BINDINGS, SQL_NEXT_AGENT_BUSINESS_ID,
    SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID, SQL_SELECT_AGENT_PROVIDER_BINDING,
    SQL_UPDATE_AGENT_BUSINESS, SQL_UPDATE_AGENT_PROVIDER_BINDING,
};
pub use ports::{AgentAuditSink, AgentListQuery, AgentRepository};
