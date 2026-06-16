use crate::application::{
    AgentBusinessService, AgentKnowledgeSyncJobCancelCommand, AgentKnowledgeSyncJobCompleteCommand,
    AgentKnowledgeSyncJobFailCommand, AgentKnowledgeSyncJobStartCommand,
    DeleteAgentMarketplaceItemCommand, GetAgentMarketplaceItemCommand,
    RestoreAgentMarketplaceItemCommand,
};
use crate::domain::{
    AgentDeploymentRecord, AgentKnowledgeBaseRecord, AgentKnowledgeBindingRecord,
    AgentKnowledgeChunkRecord, AgentKnowledgeDocumentRecord, AgentKnowledgeIndexRecord,
    AgentKnowledgeSourceRecord, AgentKnowledgeSyncJobRecord, AgentMemoryBindingRecord,
    AgentMemoryNamespaceRecord, AgentMemoryProfileRecord, AgentMemoryRecord,
    AgentMemoryRelationRecord, AgentMemoryRetrievalIndexRecord, AgentMemorySourceRecord,
    AgentMemoryStoreRecord, AgentProviderBindingRecord,
};
use crate::dto::{
    ActivateAgentProviderBindingRequestDto, AgentDeploymentRecordDto,
    AgentKnowledgeBaseCreateRequestDto, AgentKnowledgeBaseRecordDto,
    AgentKnowledgeBaseUpdateRequestDto, AgentKnowledgeBindingCreateRequestDto,
    AgentKnowledgeBindingRecordDto, AgentKnowledgeChunkCreateRequestDto,
    AgentKnowledgeChunkRecordDto, AgentKnowledgeDocumentCreateRequestDto,
    AgentKnowledgeDocumentProfileDto, AgentKnowledgeDocumentRecordDto,
    AgentKnowledgeDocumentUpdateRequestDto, AgentKnowledgeIndexRecordDto,
    AgentKnowledgeIndexUpsertRequestDto, AgentKnowledgeSearchRequestDto,
    AgentKnowledgeSearchResultDto, AgentKnowledgeSourceCreateRequestDto,
    AgentKnowledgeSourceRecordDto, AgentKnowledgeSourceUpdateRequestDto,
    AgentKnowledgeSyncJobCreateRequestDto, AgentKnowledgeSyncJobRecordDto,
    AgentManagementProfileDto, AgentMemoryBindingCreateRequestDto, AgentMemoryBindingRecordDto,
    AgentMemoryNamespaceCreateRequestDto, AgentMemoryNamespaceRecordDto,
    AgentMemoryProfileCreateRequestDto, AgentMemoryProfileRecordDto,
    AgentMemoryRecordCreateRequestDto, AgentMemoryRecordDto, AgentMemoryRelationCreateRequestDto,
    AgentMemoryRelationRecordDto, AgentMemoryRetrievalIndexRecordDto,
    AgentMemoryRetrievalIndexUpsertRequestDto, AgentMemorySourceCreateRequestDto,
    AgentMemorySourceRecordDto, AgentMemoryStoreCreateRequestDto, AgentMemoryStoreRecordDto,
    AgentMemoryStoreUpdateRequestDto, AgentPreviewResponseRequestDto,
    AgentPromptOptimizationRequestDto, AgentProviderBindingRecordDto,
    AgentProviderBindingRequestDto, AgentProviderDeploymentRequestDto, AgentRecordDto,
    AgentRuntimeExecutionRecordDto, CreateAgentRequestDto, DeleteAgentRequestDto,
    GetAgentRequestDto, ListAgentKnowledgeBasesRequestDto, ListAgentsRequestDto,
    RestoreAgentRequestDto, UpdateAgentRequestDto, UpdateAgentStatusRequestDto,
};
use crate::ports::{AgentAuditSink, AgentRepository};
use crate::validation::{
    parse_expected_version, parse_optional_rfc3339_datetime, parse_rfc3339_datetime,
    parse_tenant_id, validate_requested_at, validate_standard_id,
};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{middleware, Json, Router};
use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelErrorKind, KernelResult, PolicyDecision, PolicyProvider,
    PolicyRequest, PolicySubject, ProviderHealth,
};
use sdkwork_code_kernel::CodeTaskIntent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use time::OffsetDateTime;
#[cfg(not(feature = "postgres-sync"))]
use tokio::sync::Mutex as ServiceMutex;
#[cfg(feature = "postgres-sync")]
use std::sync::Mutex as ServiceMutex;

const HEADER_SUBJECT_ID: &str = "x-subject-id";
const HEADER_SUBJECT_TENANT_ID: &str = "x-subject-tenant-id";
const HEADER_SUBJECT_ROLES: &str = "x-subject-roles";
const HEADER_SDKWORK_USER_ID: &str = "x-sdkwork-user-id";
const HEADER_SDKWORK_ACTOR_ID: &str = "x-sdkwork-actor-id";
const HEADER_SDKWORK_TENANT_ID: &str = "x-sdkwork-tenant-id";
const HEADER_SDKWORK_PERMISSION_SCOPE: &str = "x-sdkwork-permission-scope";
const MAX_PAGE_SIZE: usize = 200;
const DEFAULT_PAGE_SIZE: usize = 20;
const ALLOWED_AUDIT_ACTIONS: &[&str] = &[
    "created",
    "updated",
    "deleted",
    "restored",
    "status_changed",
    "started",
    "completed",
    "failed",
    "cancelled",
    "runtime_executed",
    "provider_binding_changed",
    "deployment_created",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequestContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub subject_id: String,
    pub roles: Vec<String>,
}

impl AgentRequestContext {
    pub fn new(tenant_id: impl Into<String>, owner_user_id: impl Into<String>) -> Self {
        let owner_user_id = owner_user_id.into();
        Self {
            tenant_id: tenant_id.into(),
            organization_id: None,
            subject_id: owner_user_id.clone(),
            owner_user_id,
            roles: Vec::new(),
        }
    }

    pub fn with_organization_id(mut self, organization_id: impl Into<String>) -> Self {
        self.organization_id = Some(organization_id.into());
        self
    }

    pub fn with_subject_id(mut self, subject_id: impl Into<String>) -> Self {
        self.subject_id = subject_id.into();
        self
    }

    pub fn with_roles(mut self, roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }

    /// Build trusted context from gateway-injected subject headers before resource tenant reconciliation.
    pub(crate) fn from_gateway_subject_headers(headers: &HeaderMap) -> Result<Self, ApiProblem> {
        let subject_id = required_header_any(
            headers,
            &[
                HEADER_SUBJECT_ID,
                HEADER_SDKWORK_USER_ID,
                HEADER_SDKWORK_ACTOR_ID,
            ],
        )?;
        let tenant_id = optional_header_any(
            headers,
            &[HEADER_SUBJECT_TENANT_ID, HEADER_SDKWORK_TENANT_ID],
        )
        .unwrap_or_default();
        let mut roles = Vec::new();
        if let Some(roles_header) = optional_header_any(
            headers,
            &[HEADER_SUBJECT_ROLES, HEADER_SDKWORK_PERMISSION_SCOPE],
        ) {
            for role in roles_header
                .split([',', ' '])
                .map(str::trim)
                .filter(|role| !role.is_empty())
            {
                roles.push(role.to_string());
            }
        }
        Ok(Self {
            tenant_id,
            organization_id: None,
            owner_user_id: subject_id.clone(),
            subject_id,
            roles,
        })
    }

    fn subject(&self) -> PolicySubject {
        let mut subject = PolicySubject::new(self.subject_id.clone(), self.tenant_id.clone());
        for role in &self.roles {
            subject = subject.with_role(role.clone());
        }
        subject
    }
}

#[derive(Debug, Clone)]
struct RequestScope {
    tenant_id: String,
    organization_id: String,
    owner_user_id: String,
    subject: PolicySubject,
}

impl RequestScope {
    fn from_context(context: AgentRequestContext) -> Self {
        let subject = context.subject();
        Self {
            tenant_id: context.tenant_id.clone(),
            organization_id: context.organization_id.unwrap_or_else(|| "0".to_string()),
            owner_user_id: context.owner_user_id.clone(),
            subject,
        }
    }

    fn from_trusted_extension(
        mut context: AgentRequestContext,
        resource_tenant_id: String,
        organization_id: Option<String>,
        owner_user_id: Option<String>,
    ) -> Result<Self, ApiProblem> {
        let header_tenant = if context.tenant_id.is_empty() {
            None
        } else {
            Some(context.tenant_id.clone())
        };
        let tenant_id = reconcile_resource_tenant_with_subject_header(
            resource_tenant_id.as_str(),
            header_tenant,
        )?;
        context.tenant_id = tenant_id;
        if let Some(organization_id) = organization_id {
            context.organization_id = Some(organization_id);
        }
        if let Some(owner_user_id) = owner_user_id {
            context.owner_user_id = owner_user_id;
        }
        Ok(Self::from_context(context))
    }

    fn tenant_id_u64(&self) -> Result<u64, ApiProblem> {
        parse_tenant_id(self.tenant_id.as_str()).map_err(ApiProblem::from_kernel_error)
    }
}

struct DynAgentRepository(Box<dyn AgentRepository + Send>);
struct DynAgentAuditSink(Box<dyn AgentAuditSink + Send>);
struct DynPolicyProvider(Box<dyn PolicyProvider + Send + Sync>);

impl DynAgentRepository {
    fn new<R>(repository: R) -> Self
    where
        R: AgentRepository + Send + 'static,
    {
        Self(Box::new(repository))
    }
}

impl DynAgentAuditSink {
    fn new<A>(audit_sink: A) -> Self
    where
        A: AgentAuditSink + Send + 'static,
    {
        Self(Box::new(audit_sink))
    }
}

impl DynPolicyProvider {
    fn new<P>(policy_provider: P) -> Self
    where
        P: PolicyProvider + Send + Sync + 'static,
    {
        Self(Box::new(policy_provider))
    }
}

impl AgentRepository for DynAgentRepository {
    fn next_id(&mut self) -> KernelResult<u64> {
        self.0.next_id()
    }

    fn insert(&mut self, record: crate::domain::AgentBusinessRecord) -> KernelResult<()> {
        self.0.insert(record)
    }

    fn update(&mut self, record: crate::domain::AgentBusinessRecord) -> KernelResult<()> {
        self.0.update(record)
    }

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<crate::domain::AgentBusinessRecord> {
        self.0.get(tenant_id, agent_id)
    }

    fn list(
        &self,
        query: &crate::ports::AgentListQuery,
    ) -> Vec<crate::domain::AgentBusinessRecord> {
        self.0.list(query)
    }

    fn insert_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        self.0.insert_provider_binding(record)
    }

    fn update_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        self.0.update_provider_binding(record)
    }

    fn get_provider_binding(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRecord> {
        self.0.get_provider_binding(tenant_id, agent_id, binding_id)
    }

    fn list_provider_bindings(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRecord> {
        self.0.list_provider_bindings(tenant_id, agent_id)
    }

    fn insert_deployment(&mut self, record: AgentDeploymentRecord) -> KernelResult<()> {
        self.0.insert_deployment(record)
    }

    fn list_deployments(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRecord> {
        self.0.list_deployments(tenant_id, agent_id)
    }

    fn insert_knowledge_base(&mut self, record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        self.0.insert_knowledge_base(record)
    }

    fn update_knowledge_base(&mut self, record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        self.0.update_knowledge_base(record)
    }

    fn get_knowledge_base(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Option<AgentKnowledgeBaseRecord> {
        self.0.get_knowledge_base(tenant_id, knowledge_base_id)
    }

    fn list_knowledge_bases(
        &self,
        query: &crate::ports::AgentMarketplaceListQuery,
    ) -> Vec<AgentKnowledgeBaseRecord> {
        self.0.list_knowledge_bases(query)
    }

    fn insert_knowledge_source(&mut self, record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        self.0.insert_knowledge_source(record)
    }

    fn update_knowledge_source(&mut self, record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        self.0.update_knowledge_source(record)
    }

    fn get_knowledge_source(
        &self,
        tenant_id: u64,
        knowledge_source_id: &str,
    ) -> Option<AgentKnowledgeSourceRecord> {
        self.0.get_knowledge_source(tenant_id, knowledge_source_id)
    }

    fn list_knowledge_sources(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSourceRecord> {
        self.0.list_knowledge_sources(tenant_id, knowledge_base_id)
    }

    fn insert_knowledge_document(
        &mut self,
        record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        self.0.insert_knowledge_document(record)
    }

    fn update_knowledge_document(
        &mut self,
        record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        self.0.update_knowledge_document(record)
    }

    fn get_knowledge_document(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Option<AgentKnowledgeDocumentRecord> {
        self.0
            .get_knowledge_document(tenant_id, knowledge_document_id)
    }

    fn list_knowledge_documents(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeDocumentRecord> {
        self.0
            .list_knowledge_documents(tenant_id, knowledge_base_id)
    }

    fn insert_knowledge_chunk(&mut self, record: AgentKnowledgeChunkRecord) -> KernelResult<()> {
        self.0.insert_knowledge_chunk(record)
    }

    fn get_knowledge_chunk(
        &self,
        tenant_id: u64,
        knowledge_chunk_id: &str,
    ) -> Option<AgentKnowledgeChunkRecord> {
        self.0.get_knowledge_chunk(tenant_id, knowledge_chunk_id)
    }

    fn list_knowledge_chunks(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeChunkRecord> {
        self.0
            .list_knowledge_chunks(tenant_id, knowledge_document_id)
    }

    fn upsert_knowledge_index(&mut self, record: AgentKnowledgeIndexRecord) -> KernelResult<()> {
        self.0.upsert_knowledge_index(record)
    }

    fn get_knowledge_index(
        &self,
        tenant_id: u64,
        knowledge_index_id: &str,
    ) -> Option<AgentKnowledgeIndexRecord> {
        self.0.get_knowledge_index(tenant_id, knowledge_index_id)
    }

    fn list_knowledge_indexes(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        self.0
            .list_knowledge_indexes(tenant_id, knowledge_document_id)
    }

    fn list_knowledge_indexes_by_base(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        self.0
            .list_knowledge_indexes_by_base(tenant_id, knowledge_base_id)
    }

    fn insert_knowledge_binding(
        &mut self,
        record: AgentKnowledgeBindingRecord,
    ) -> KernelResult<()> {
        self.0.insert_knowledge_binding(record)
    }

    fn get_knowledge_binding(
        &self,
        tenant_id: u64,
        knowledge_binding_id: &str,
    ) -> Option<AgentKnowledgeBindingRecord> {
        self.0
            .get_knowledge_binding(tenant_id, knowledge_binding_id)
    }

    fn list_knowledge_bindings(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeBindingRecord> {
        self.0.list_knowledge_bindings(tenant_id, knowledge_base_id)
    }

    fn insert_knowledge_sync_job(
        &mut self,
        record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        self.0.insert_knowledge_sync_job(record)
    }

    fn update_knowledge_sync_job(
        &mut self,
        record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        self.0.update_knowledge_sync_job(record)
    }

    fn get_knowledge_sync_job(
        &self,
        tenant_id: u64,
        sync_job_id: &str,
    ) -> Option<AgentKnowledgeSyncJobRecord> {
        self.0.get_knowledge_sync_job(tenant_id, sync_job_id)
    }

    fn list_knowledge_sync_jobs(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSyncJobRecord> {
        self.0
            .list_knowledge_sync_jobs(tenant_id, knowledge_base_id)
    }

    fn insert_memory_store(&mut self, record: AgentMemoryStoreRecord) -> KernelResult<()> {
        self.0.insert_memory_store(record)
    }

    fn update_memory_store(&mut self, record: AgentMemoryStoreRecord) -> KernelResult<()> {
        self.0.update_memory_store(record)
    }

    fn get_memory_store(
        &self,
        tenant_id: u64,
        memory_store_id: &str,
    ) -> Option<AgentMemoryStoreRecord> {
        self.0.get_memory_store(tenant_id, memory_store_id)
    }

    fn insert_memory_profile(&mut self, record: AgentMemoryProfileRecord) -> KernelResult<()> {
        self.0.insert_memory_profile(record)
    }

    fn get_memory_profile(
        &self,
        tenant_id: u64,
        memory_profile_id: &str,
    ) -> Option<AgentMemoryProfileRecord> {
        self.0.get_memory_profile(tenant_id, memory_profile_id)
    }

    fn insert_memory_binding(&mut self, record: AgentMemoryBindingRecord) -> KernelResult<()> {
        self.0.insert_memory_binding(record)
    }

    fn get_memory_binding(
        &self,
        tenant_id: u64,
        memory_binding_id: &str,
    ) -> Option<AgentMemoryBindingRecord> {
        self.0.get_memory_binding(tenant_id, memory_binding_id)
    }

    fn insert_memory_namespace(&mut self, record: AgentMemoryNamespaceRecord) -> KernelResult<()> {
        self.0.insert_memory_namespace(record)
    }

    fn get_memory_namespace(
        &self,
        tenant_id: u64,
        memory_namespace_id: &str,
    ) -> Option<AgentMemoryNamespaceRecord> {
        self.0.get_memory_namespace(tenant_id, memory_namespace_id)
    }

    fn insert_memory_record(&mut self, record: AgentMemoryRecord) -> KernelResult<()> {
        self.0.insert_memory_record(record)
    }

    fn update_memory_record(&mut self, record: AgentMemoryRecord) -> KernelResult<()> {
        self.0.update_memory_record(record)
    }

    fn get_memory_record(&self, tenant_id: u64, memory_id: &str) -> Option<AgentMemoryRecord> {
        self.0.get_memory_record(tenant_id, memory_id)
    }

    fn list_memory_records(
        &self,
        tenant_id: u64,
        memory_namespace_id: &str,
    ) -> Vec<AgentMemoryRecord> {
        self.0.list_memory_records(tenant_id, memory_namespace_id)
    }

    fn insert_memory_source(&mut self, record: AgentMemorySourceRecord) -> KernelResult<()> {
        self.0.insert_memory_source(record)
    }

    fn list_memory_sources(&self, tenant_id: u64, memory_id: &str) -> Vec<AgentMemorySourceRecord> {
        self.0.list_memory_sources(tenant_id, memory_id)
    }

    fn insert_memory_relation(&mut self, record: AgentMemoryRelationRecord) -> KernelResult<()> {
        self.0.insert_memory_relation(record)
    }

    fn list_memory_relations(
        &self,
        tenant_id: u64,
        memory_id: &str,
    ) -> Vec<AgentMemoryRelationRecord> {
        self.0.list_memory_relations(tenant_id, memory_id)
    }

    fn upsert_memory_retrieval_index(
        &mut self,
        record: AgentMemoryRetrievalIndexRecord,
    ) -> KernelResult<()> {
        self.0.upsert_memory_retrieval_index(record)
    }

    fn list_memory_retrieval_indexes(
        &self,
        tenant_id: u64,
        memory_id: &str,
    ) -> Vec<AgentMemoryRetrievalIndexRecord> {
        self.0.list_memory_retrieval_indexes(tenant_id, memory_id)
    }
}

impl AgentAuditSink for DynAgentAuditSink {
    fn record(&mut self, event: sdkwork_agent_kernel::KernelEvent) -> KernelResult<()> {
        self.0.record(event)
    }

    fn list_events(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::KernelEvent>> {
        self.0.list_events(tenant_id, agent_id)
    }
}

impl PolicyProvider for DynPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        self.0.evaluate(request)
    }

    fn health(&self) -> ProviderHealth {
        self.0.health()
    }
}

type HttpService = AgentBusinessService<DynAgentRepository, DynAgentAuditSink, DynPolicyProvider>;

#[derive(Clone)]
pub struct AgentHttpState {
    service: Arc<ServiceMutex<HttpService>>,
}

impl AgentHttpState {
    pub fn new<R, A, P>(repository: R, audit_sink: A, policy_provider: P) -> Self
    where
        R: AgentRepository + Send + 'static,
        A: AgentAuditSink + Send + 'static,
        P: PolicyProvider + Send + Sync + 'static,
    {
        let service = AgentBusinessService::new(
            DynAgentRepository::new(repository),
            DynAgentAuditSink::new(audit_sink),
            DynPolicyProvider::new(policy_provider),
        );
        Self {
            service: Arc::new(ServiceMutex::new(service)),
        }
    }
}

async fn inject_gateway_agent_context(mut request: Request<axum::body::Body>, next: Next) -> Response {
    match AgentRequestContext::from_gateway_subject_headers(request.headers()) {
        Ok(context) => {
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        Err(problem) => problem.into_response(),
    }
}

fn with_gateway_trusted_context(router: Router<AgentHttpState>) -> Router<AgentHttpState> {
    router.layer(middleware::from_fn(inject_gateway_agent_context))
}

pub fn build_app_router() -> Router<AgentHttpState> {
    add_app_memory_routes(
        add_app_knowledge_routes(
            Router::new()
                .route(
                    "/app/v3/api/ai/agents",
                    get(app_list_agents).post(app_create_agent),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}",
                    get(app_get_agent)
                        .patch(app_update_agent)
                        .delete(app_delete_agent),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/restore",
                    post(app_restore_agent),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/provider_bindings",
                    get(app_list_provider_bindings).post(app_add_provider_binding),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
                    post(app_activate_provider_binding),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/deployments",
                    get(app_list_deployments).post(app_create_deployment),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/preview_responses",
                    post(app_create_preview_response),
                )
                .route(
                    "/app/v3/api/ai/agents/{agentId}/prompt_optimizations",
                    post(app_create_prompt_optimization),
                ),
            "/app/v3/api",
        ),
        "/app/v3/api",
    )
}

pub fn build_open_router() -> Router<AgentHttpState> {
    with_gateway_trusted_context(add_memory_routes(
        add_knowledge_routes(
            Router::new()
                .route(
                    "/agent/v3/api/ai/agents",
                    get(backend_list_agents).post(backend_create_agent),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}",
                    get(backend_get_agent)
                        .patch(backend_update_agent)
                        .delete(open_delete_agent),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/restore",
                    post(backend_restore_agent),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/provider_bindings",
                    get(backend_list_provider_bindings).post(backend_add_provider_binding),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
                    post(backend_activate_provider_binding),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/deployments",
                    get(backend_list_deployments).post(backend_create_deployment),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/preview_responses",
                    post(open_create_preview_response),
                )
                .route(
                    "/agent/v3/api/ai/agents/{agentId}/prompt_optimizations",
                    post(open_create_prompt_optimization),
                ),
            "/agent/v3/api",
        ),
        "/agent/v3/api",
    ))
}

pub fn build_backend_router() -> Router<AgentHttpState> {
    with_gateway_trusted_context(add_memory_routes(
        add_knowledge_routes(
            Router::new()
                .route(
                    "/backend/v3/api/ai/agents",
                    get(backend_list_agents).post(backend_create_agent),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}",
                    get(backend_get_agent).patch(backend_update_agent),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/status",
                    post(backend_update_agent_status),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/restore",
                    post(backend_restore_agent),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/audit_events",
                    get(backend_list_agent_audit_events),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
                    get(backend_list_provider_bindings).post(backend_add_provider_binding),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
                    post(backend_activate_provider_binding),
                )
                .route(
                    "/backend/v3/api/ai/agents/{agentId}/deployments",
                    get(backend_list_deployments).post(backend_create_deployment),
                ),
            "/backend/v3/api",
        ),
        "/backend/v3/api",
    ))
}

fn add_knowledge_routes(router: Router<AgentHttpState>, prefix: &str) -> Router<AgentHttpState> {
    router
        .route(
            &format!("{prefix}/ai/knowledge_bases"),
            get(list_knowledge_bases).post(create_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}"),
            get(get_knowledge_base)
                .patch(update_knowledge_base)
                .delete(delete_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/restore"),
            post(restore_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sources"),
            get(list_knowledge_sources).post(create_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}"),
            get(get_knowledge_source)
                .patch(update_knowledge_source)
                .delete(delete_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}/restore"),
            post(restore_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/documents"),
            get(list_knowledge_documents).post(create_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/search"),
            post(search_knowledge),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/bindings"),
            get(list_knowledge_bindings).post(create_knowledge_binding),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sync_jobs"),
            get(list_knowledge_sync_jobs).post(create_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}"),
            get(get_knowledge_document)
                .patch(update_knowledge_document)
                .delete(delete_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/restore"),
            post(restore_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/chunks"),
            get(list_knowledge_chunks).post(create_knowledge_chunk),
        )
        .route(
            &format!("{prefix}/ai/knowledge_chunks/{{knowledgeChunkId}}"),
            get(get_knowledge_chunk),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/indexes"),
            get(list_knowledge_indexes),
        )
        .route(
            &format!("{prefix}/ai/knowledge_indexes"),
            post(upsert_knowledge_index),
        )
        .route(
            &format!("{prefix}/ai/knowledge_indexes/{{knowledgeIndexId}}"),
            get(get_knowledge_index),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bindings/{{knowledgeBindingId}}"),
            get(get_knowledge_binding),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}"),
            get(get_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/start"),
            post(start_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/complete"),
            post(complete_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/fail"),
            post(fail_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/cancel"),
            post(cancel_knowledge_sync_job),
        )
}

fn add_app_knowledge_routes(
    router: Router<AgentHttpState>,
    prefix: &str,
) -> Router<AgentHttpState> {
    router
        .route(
            &format!("{prefix}/ai/knowledge_bases"),
            get(app_list_knowledge_bases).post(app_create_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}"),
            get(app_get_knowledge_base)
                .patch(app_update_knowledge_base)
                .delete(app_delete_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/restore"),
            post(app_restore_knowledge_base),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sources"),
            get(app_list_knowledge_sources).post(app_create_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}"),
            get(app_get_knowledge_source)
                .patch(app_update_knowledge_source)
                .delete(app_delete_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}/restore"),
            post(app_restore_knowledge_source),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/documents"),
            get(app_list_knowledge_documents).post(app_create_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/search"),
            post(app_search_knowledge),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/bindings"),
            get(app_list_knowledge_bindings).post(app_create_knowledge_binding),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sync_jobs"),
            get(app_list_knowledge_sync_jobs).post(app_create_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}"),
            get(app_get_knowledge_document)
                .patch(app_update_knowledge_document)
                .delete(app_delete_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/restore"),
            post(app_restore_knowledge_document),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/chunks"),
            get(app_list_knowledge_chunks).post(app_create_knowledge_chunk),
        )
        .route(
            &format!("{prefix}/ai/knowledge_chunks/{{knowledgeChunkId}}"),
            get(app_get_knowledge_chunk),
        )
        .route(
            &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/indexes"),
            get(app_list_knowledge_indexes),
        )
        .route(
            &format!("{prefix}/ai/knowledge_indexes"),
            post(app_upsert_knowledge_index),
        )
        .route(
            &format!("{prefix}/ai/knowledge_indexes/{{knowledgeIndexId}}"),
            get(app_get_knowledge_index),
        )
        .route(
            &format!("{prefix}/ai/knowledge_bindings/{{knowledgeBindingId}}"),
            get(app_get_knowledge_binding),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}"),
            get(app_get_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/start"),
            post(app_start_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/complete"),
            post(app_complete_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/fail"),
            post(app_fail_knowledge_sync_job),
        )
        .route(
            &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/cancel"),
            post(app_cancel_knowledge_sync_job),
        )
}

fn add_memory_routes(router: Router<AgentHttpState>, prefix: &str) -> Router<AgentHttpState> {
    router
        .route(
            &format!("{prefix}/ai/memory_stores"),
            post(create_memory_store),
        )
        .route(
            &format!("{prefix}/ai/memory_stores/{{memoryStoreId}}"),
            get(get_memory_store).patch(update_memory_store),
        )
        .route(
            &format!("{prefix}/ai/memory_stores/{{memoryStoreId}}/profiles"),
            post(create_memory_profile),
        )
        .route(
            &format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}"),
            get(get_memory_profile),
        )
        .route(
            &format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}/bindings"),
            post(create_memory_binding),
        )
        .route(
            &format!("{prefix}/ai/memory_bindings/{{memoryBindingId}}"),
            get(get_memory_binding),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces"),
            post(create_memory_namespace),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}"),
            get(get_memory_namespace),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}/records"),
            get(list_memory_records).post(create_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}"),
            get(get_memory_record).delete(delete_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/restore"),
            post(restore_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/sources"),
            get(list_memory_sources).post(create_memory_source),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/relations"),
            get(list_memory_relations).post(create_memory_relation),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/retrieval_indexes"),
            get(list_memory_retrieval_indexes),
        )
        .route(
            &format!("{prefix}/ai/memory_retrieval_indexes"),
            post(upsert_memory_retrieval_index),
        )
}

fn add_app_memory_routes(router: Router<AgentHttpState>, prefix: &str) -> Router<AgentHttpState> {
    router
        .route(
            &format!("{prefix}/ai/memory_stores"),
            post(app_create_memory_store),
        )
        .route(
            &format!("{prefix}/ai/memory_stores/{{memoryStoreId}}"),
            get(app_get_memory_store).patch(app_update_memory_store),
        )
        .route(
            &format!("{prefix}/ai/memory_stores/{{memoryStoreId}}/profiles"),
            post(app_create_memory_profile),
        )
        .route(
            &format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}"),
            get(app_get_memory_profile),
        )
        .route(
            &format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}/bindings"),
            post(app_create_memory_binding),
        )
        .route(
            &format!("{prefix}/ai/memory_bindings/{{memoryBindingId}}"),
            get(app_get_memory_binding),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces"),
            post(app_create_memory_namespace),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}"),
            get(app_get_memory_namespace),
        )
        .route(
            &format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}/records"),
            get(app_list_memory_records).post(app_create_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}"),
            get(app_get_memory_record).delete(app_delete_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/restore"),
            post(app_restore_memory_record),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/sources"),
            get(app_list_memory_sources).post(app_create_memory_source),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/relations"),
            get(app_list_memory_relations).post(app_create_memory_relation),
        )
        .route(
            &format!("{prefix}/ai/memory_records/{{memoryId}}/retrieval_indexes"),
            get(app_list_memory_retrieval_indexes),
        )
        .route(
            &format!("{prefix}/ai/memory_retrieval_indexes"),
            post(app_upsert_memory_retrieval_index),
        )
}

pub fn build_combined_router(state: AgentHttpState) -> Router {
    build_open_router()
        .merge(build_app_router())
        .merge(build_backend_router())
        .with_state(state)
}

#[derive(Debug, Clone, Deserialize)]
struct ListAgentsQueryParams {
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    scope: Option<String>,
    include_deleted: Option<bool>,
    q: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppListAgentsQueryParams {
    scope: Option<String>,
    include_deleted: Option<bool>,
    q: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct TenantQueryParams {
    tenant_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeleteKnowledgeBaseQueryParams {
    tenant_id: String,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeleteKnowledgeSourceQueryParams {
    tenant_id: String,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeleteKnowledgeDocumentQueryParams {
    tenant_id: String,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeleteMemoryRecordQueryParams {
    tenant_id: String,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AppDeleteQueryParams {
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TenantListQueryParams {
    tenant_id: String,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppListQueryParams {
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct TenantAgentPathParams {
    #[serde(rename = "agentId")]
    agent_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TenantAgentBindingPathParams {
    #[serde(rename = "agentId")]
    agent_id: String,
    #[serde(rename = "bindingId")]
    binding_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditEventsQueryParams {
    tenant_id: String,
    page: Option<usize>,
    page_size: Option<usize>,
    action: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeBaseListQueryParams {
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    include_deleted: Option<bool>,
    q: Option<String>,
    status: Option<String>,
    visibility: Option<String>,
    category: Option<String>,
    tag: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppKnowledgeBaseListQueryParams {
    include_deleted: Option<bool>,
    q: Option<String>,
    status: Option<String>,
    visibility: Option<String>,
    category: Option<String>,
    tag: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeBasePathParams {
    #[serde(rename = "knowledgeBaseId")]
    knowledge_base_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeSourcePathParams {
    #[serde(rename = "knowledgeSourceId")]
    knowledge_source_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeDocumentPathParams {
    #[serde(rename = "knowledgeDocumentId")]
    knowledge_document_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeChunkPathParams {
    #[serde(rename = "knowledgeChunkId")]
    knowledge_chunk_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeIndexPathParams {
    #[serde(rename = "knowledgeIndexId")]
    knowledge_index_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeBindingPathParams {
    #[serde(rename = "knowledgeBindingId")]
    knowledge_binding_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeSyncJobPathParams {
    #[serde(rename = "syncJobId")]
    sync_job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryStorePathParams {
    #[serde(rename = "memoryStoreId")]
    memory_store_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryProfilePathParams {
    #[serde(rename = "memoryProfileId")]
    memory_profile_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryBindingPathParams {
    #[serde(rename = "memoryBindingId")]
    memory_binding_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryNamespacePathParams {
    #[serde(rename = "memoryNamespaceId")]
    memory_namespace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryRecordPathParams {
    #[serde(rename = "memoryId")]
    memory_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAgentBody {
    agent_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    code: String,
    display_name: String,
    description: Option<String>,
    manifest: Value,
    default_code_task_intent: Option<CodeTaskIntentBody>,
    management_profile: Option<AgentManagementProfileBody>,
    implementation_provider_id: Option<String>,
    implementation_kind: Option<String>,
    implementation_type: Option<String>,
    visibility: String,
    tags: Option<Vec<String>>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderBindingBody {
    binding_id: String,
    provider_id: String,
    implementation_kind: String,
    configuration_profile_id: String,
    capabilities: Option<Vec<String>>,
    make_default: Option<bool>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivateProviderBindingBody {
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentDeploymentBody {
    deployment_id: String,
    binding_id: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentPreviewResponseBody {
    execution_id: String,
    content: String,
    debug_mode: Option<bool>,
    memory_enabled: Option<bool>,
    model: Option<String>,
    temperature: Option<f32>,
    input_payload: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentPromptOptimizationBody {
    execution_id: String,
    prompt: String,
    input_payload: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAgentBody {
    display_name: Option<String>,
    description: Option<String>,
    manifest: Option<Value>,
    visibility: Option<String>,
    tags: Option<Vec<String>>,
    default_code_task_intent: Option<CodeTaskIntentBody>,
    management_profile: Option<AgentManagementProfileBody>,
    implementation_provider_id: Option<Option<String>>,
    implementation_kind: Option<Option<String>>,
    implementation_type: Option<String>,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAgentStatusBody {
    target_status: String,
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAgentBody {
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreAgentBody {
    expected_version: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseBody {
    knowledge_base_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    code: String,
    display_name: String,
    description: Option<String>,
    provider_id: String,
    base_kind: String,
    retrieval_modes: Vec<String>,
    capability_ids: Option<Vec<String>>,
    configuration_profile_id: String,
    visibility: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseUpdateBody {
    expected_version: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    provider_id: Option<String>,
    base_kind: Option<String>,
    retrieval_modes: Option<Vec<String>>,
    capability_ids: Option<Vec<String>>,
    configuration_profile_id: Option<String>,
    visibility: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceBody {
    knowledge_source_id: String,
    organization_id: Option<String>,
    source_kind: String,
    source_ref: String,
    source_hash: String,
    sync_policy: Option<Value>,
    metadata: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceUpdateBody {
    expected_version: Option<String>,
    source_kind: Option<String>,
    source_ref: Option<String>,
    source_hash: Option<String>,
    sync_policy: Option<Value>,
    metadata: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentBody {
    knowledge_document_id: String,
    organization_id: Option<String>,
    knowledge_source_id: Option<String>,
    document_kind: String,
    title: String,
    content_ref: String,
    content_hash: String,
    summary: Option<String>,
    metadata: Option<Value>,
    document_profile: Option<KnowledgeDocumentProfileBody>,
    tags: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    trust_level: Option<i16>,
    redaction_classification: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentUpdateBody {
    expected_version: Option<String>,
    knowledge_source_id: Option<String>,
    document_kind: Option<String>,
    title: Option<String>,
    content_ref: Option<String>,
    content_hash: Option<String>,
    summary: Option<String>,
    metadata: Option<Value>,
    document_profile: Option<KnowledgeDocumentProfileBody>,
    tags: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    trust_level: Option<i16>,
    redaction_classification: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeChunkBody {
    knowledge_chunk_id: String,
    organization_id: Option<String>,
    parent_chunk_id: Option<String>,
    chunk_ordinal: u32,
    heading: Option<String>,
    content_ref: String,
    content_hash: String,
    token_estimate: u32,
    summary: Option<String>,
    metadata: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeIndexBody {
    knowledge_index_id: String,
    knowledge_base_id: String,
    knowledge_document_id: Option<String>,
    knowledge_chunk_id: Option<String>,
    index_kind: String,
    index_provider_id: String,
    external_ref: String,
    embedding_model_id: Option<String>,
    vector_dimension: Option<u32>,
    content_hash: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSearchBody {
    query: String,
    top_k: Option<usize>,
    retrieval_modes: Option<Vec<String>>,
    include_external: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBindingBody {
    knowledge_binding_id: String,
    organization_id: Option<String>,
    agent_id: Option<String>,
    deployment_id: Option<String>,
    scope_kind: String,
    scope_ref: String,
    active: Option<bool>,
    default_binding: Option<bool>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobBody {
    sync_job_id: String,
    organization_id: Option<String>,
    knowledge_source_id: Option<String>,
    job_kind: String,
    input_ref: String,
    input: Option<Value>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobStartBody {
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobCompleteBody {
    output: Value,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobFailBody {
    error: Value,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobCancelBody {
    cancellation: Value,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStoreBody {
    memory_store_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    code: String,
    display_name: String,
    description: Option<String>,
    provider_id: String,
    store_kind: String,
    retrieval_modes: Vec<String>,
    capability_ids: Option<Vec<String>>,
    configuration_profile_id: String,
    visibility: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStoreUpdateBody {
    expected_version: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    provider_id: Option<String>,
    store_kind: Option<String>,
    retrieval_modes: Option<Vec<String>>,
    capability_ids: Option<Vec<String>>,
    configuration_profile_id: Option<String>,
    visibility: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryProfileBody {
    memory_profile_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
    code: String,
    display_name: String,
    description: Option<String>,
    write_policy: Option<Value>,
    retrieval_policy: Option<Value>,
    compaction_policy: Option<Value>,
    retention_policy: Option<Value>,
    privacy_policy: Option<Value>,
    visibility: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryBindingBody {
    memory_binding_id: String,
    organization_id: Option<String>,
    agent_id: Option<String>,
    deployment_id: Option<String>,
    scope_kind: String,
    scope_ref: String,
    active: Option<bool>,
    default_binding: Option<bool>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryNamespaceBody {
    memory_namespace_id: String,
    organization_id: Option<String>,
    agent_id: Option<String>,
    user_ref: Option<String>,
    session_ref: Option<String>,
    thread_ref: Option<String>,
    namespace_kind: String,
    visibility: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecordBody {
    memory_id: String,
    organization_id: Option<String>,
    agent_id: Option<String>,
    memory_kind: String,
    content_format: String,
    content: Value,
    summary: Option<String>,
    salience_score: f32,
    confidence_score: f32,
    freshness_score: f32,
    sensitivity_level: i16,
    effective_at: Option<String>,
    expires_at: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemorySourceBody {
    memory_source_id: String,
    source_kind: String,
    source_ref: String,
    source_hash: String,
    evidence: Option<Value>,
    captured_at: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRelationBody {
    memory_relation_id: String,
    from_memory_id: String,
    to_memory_id: String,
    relation_kind: String,
    weight: f32,
    valid_from: Option<String>,
    valid_until: Option<String>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRetrievalIndexBody {
    memory_index_id: String,
    memory_id: String,
    index_kind: String,
    index_provider_id: String,
    external_ref: String,
    embedding_model_id: Option<String>,
    vector_dimension: Option<u32>,
    content_hash: String,
    requested_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PageInfoResponse {
    page: usize,
    page_size: usize,
    total_items: String,
    total_pages: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentListDataResponse {
    items: Vec<AgentRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentListResponse {
    data: AgentListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentResponse {
    data: AgentRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderBindingResponse {
    data: AgentProviderBindingRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderBindingListResponse {
    data: AgentProviderBindingListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderBindingListDataResponse {
    items: Vec<AgentProviderBindingRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderBindingRecordResponse {
    tenant_id: String,
    agent_id: String,
    binding_id: String,
    provider_id: String,
    implementation_kind: String,
    configuration_profile_id: String,
    capabilities: Vec<String>,
    active: bool,
    version: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentDeploymentResponse {
    data: AgentDeploymentRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentDeploymentListResponse {
    data: AgentDeploymentListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentDeploymentListDataResponse {
    items: Vec<AgentDeploymentRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentDeploymentRecordResponse {
    tenant_id: String,
    agent_id: String,
    deployment_id: String,
    binding_id: String,
    provider_id_snapshot: String,
    implementation_kind_snapshot: String,
    configuration_profile_id_snapshot: String,
    capabilities_snapshot: Vec<String>,
    status: String,
    version: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRuntimeExecutionResponse {
    data: AgentRuntimeExecutionRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRuntimeExecutionRecordResponse {
    tenant_id: String,
    agent_id: String,
    execution_id: String,
    operation: String,
    status: String,
    input_payload: Value,
    output_payload: Value,
    requested_at: String,
    completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRecordResponse {
    id: String,
    agent_id: String,
    tenant_id: String,
    organization_id: String,
    owner_user_id: String,
    code: String,
    display_name: String,
    description: Option<String>,
    manifest: Value,
    default_code_task_intent: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    management_profile: Option<AgentManagementProfileResponse>,
    implementation_provider_id: Option<String>,
    implementation_kind: Option<String>,
    implementation_type: String,
    status: String,
    visibility: String,
    tags: Vec<String>,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentManagementProfileResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_mode: Option<bool>,
    knowledge_base_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    skill_ids: Vec<String>,
    suggested_prompts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    tool_ids: Vec<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    users: Option<String>,
    voice_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    welcome_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentAuditEventResponse {
    event_id: String,
    event_type: String,
    severity: String,
    payload: String,
    occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentAuditEventsListResponse {
    data: AgentAuditEventsData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentAuditEventsData {
    items: Vec<AgentAuditEventResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseResponse {
    data: KnowledgeBaseRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseListResponse {
    data: KnowledgeBaseListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseListDataResponse {
    items: Vec<KnowledgeBaseRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBaseRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    owner_user_id: String,
    knowledge_base_id: String,
    code: String,
    display_name: String,
    description: Option<String>,
    provider_id: String,
    base_kind: String,
    retrieval_modes: Vec<String>,
    capability_ids: Vec<String>,
    configuration_profile_id: String,
    document_count: u32,
    status: String,
    visibility: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceResponse {
    data: KnowledgeSourceRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceListResponse {
    data: KnowledgeSourceListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceListDataResponse {
    items: Vec<KnowledgeSourceRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSourceRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    knowledge_source_id: String,
    knowledge_base_id: String,
    source_kind: String,
    source_ref: String,
    source_hash: String,
    sync_policy: Value,
    metadata: Value,
    status: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentResponse {
    data: KnowledgeDocumentRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentListResponse {
    data: KnowledgeDocumentListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentListDataResponse {
    items: Vec<KnowledgeDocumentRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    knowledge_document_id: String,
    knowledge_base_id: String,
    knowledge_source_id: Option<String>,
    document_kind: String,
    title: String,
    content_ref: String,
    content_hash: String,
    summary: Option<String>,
    metadata: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_profile: Option<KnowledgeDocumentProfileResponse>,
    tags: Vec<String>,
    categories: Vec<String>,
    trust_level: i16,
    redaction_classification: String,
    chunk_count: u32,
    status: String,
    visibility: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentProfileResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    document_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drive_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeChunkResponse {
    data: KnowledgeChunkRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeChunkListResponse {
    data: KnowledgeChunkListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeChunkListDataResponse {
    items: Vec<KnowledgeChunkRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeChunkRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    knowledge_chunk_id: String,
    knowledge_document_id: String,
    parent_chunk_id: Option<String>,
    chunk_ordinal: u32,
    heading: Option<String>,
    content_ref: String,
    content_hash: String,
    token_estimate: u32,
    summary: Option<String>,
    metadata: Value,
    status: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeIndexResponse {
    data: KnowledgeIndexRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeIndexListResponse {
    data: KnowledgeIndexListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeIndexListDataResponse {
    items: Vec<KnowledgeIndexRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeIndexRecordResponse {
    id: String,
    tenant_id: String,
    knowledge_index_id: String,
    knowledge_base_id: String,
    knowledge_document_id: Option<String>,
    knowledge_chunk_id: Option<String>,
    index_kind: String,
    index_provider_id: String,
    external_ref: String,
    embedding_model_id: Option<String>,
    vector_dimension: Option<u32>,
    content_hash: String,
    indexed_at: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSearchResponse {
    data: KnowledgeSearchDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSearchDataResponse {
    items: Vec<KnowledgeSearchResultResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSearchResultResponse {
    tenant_id: String,
    knowledge_base_id: String,
    provider_id: String,
    knowledge_index_id: String,
    index_provider_id: String,
    retrieval_method: String,
    knowledge_document_id: Option<String>,
    document_kind: Option<String>,
    knowledge_chunk_id: Option<String>,
    title: String,
    snippet: Option<String>,
    score: Option<f32>,
    source_ref: Option<String>,
    content_ref: Option<String>,
    external_ref: Option<String>,
    trust_level: i16,
    redaction_classification: String,
    metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBindingResponse {
    data: KnowledgeBindingRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBindingListResponse {
    data: KnowledgeBindingListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBindingListDataResponse {
    items: Vec<KnowledgeBindingRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBindingRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    knowledge_binding_id: String,
    knowledge_base_id: String,
    agent_id: Option<String>,
    deployment_id: Option<String>,
    scope_kind: String,
    scope_ref: String,
    active: bool,
    default_binding: bool,
    version: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobResponse {
    data: KnowledgeSyncJobRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobListResponse {
    data: KnowledgeSyncJobListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobListDataResponse {
    items: Vec<KnowledgeSyncJobRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeSyncJobRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    sync_job_id: String,
    knowledge_base_id: String,
    knowledge_source_id: Option<String>,
    job_kind: String,
    status: String,
    input_ref: String,
    input: Value,
    output: Option<Value>,
    error: Option<Value>,
    requested_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStoreResponse {
    data: MemoryStoreRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStoreRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    owner_user_id: String,
    memory_store_id: String,
    code: String,
    display_name: String,
    description: Option<String>,
    provider_id: String,
    store_kind: String,
    retrieval_modes: Vec<String>,
    capability_ids: Vec<String>,
    configuration_profile_id: String,
    status: String,
    visibility: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryProfileResponse {
    data: MemoryProfileRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryProfileRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    owner_user_id: String,
    memory_profile_id: String,
    memory_store_id: String,
    code: String,
    display_name: String,
    description: Option<String>,
    write_policy: Value,
    retrieval_policy: Value,
    compaction_policy: Value,
    retention_policy: Value,
    privacy_policy: Value,
    status: String,
    visibility: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryBindingResponse {
    data: MemoryBindingRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryBindingRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    memory_binding_id: String,
    memory_profile_id: String,
    agent_id: Option<String>,
    deployment_id: Option<String>,
    scope_kind: String,
    scope_ref: String,
    active: bool,
    default_binding: bool,
    version: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryNamespaceResponse {
    data: MemoryNamespaceRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryNamespaceRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    memory_namespace_id: String,
    agent_id: Option<String>,
    user_ref: Option<String>,
    session_ref: Option<String>,
    thread_ref: Option<String>,
    namespace_kind: String,
    status: String,
    visibility: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecordResponse {
    data: MemoryRecordRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecordListResponse {
    data: MemoryRecordListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecordListDataResponse {
    items: Vec<MemoryRecordRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecordRecordResponse {
    id: String,
    tenant_id: String,
    organization_id: String,
    memory_id: String,
    memory_namespace_id: String,
    agent_id: Option<String>,
    memory_kind: String,
    content_format: String,
    content: Value,
    summary: Option<String>,
    salience_score: f32,
    confidence_score: f32,
    freshness_score: f32,
    sensitivity_level: i16,
    source_count: u32,
    effective_at: Option<String>,
    expires_at: Option<String>,
    last_used_at: Option<String>,
    use_count: String,
    status: String,
    version: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
    redacted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySourceResponse {
    data: MemorySourceRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySourceListResponse {
    data: MemorySourceListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySourceListDataResponse {
    items: Vec<MemorySourceRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySourceRecordResponse {
    id: String,
    tenant_id: String,
    memory_source_id: String,
    memory_id: String,
    source_kind: String,
    source_ref: String,
    source_hash: String,
    evidence: Value,
    captured_at: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRelationResponse {
    data: MemoryRelationRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRelationListResponse {
    data: MemoryRelationListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRelationListDataResponse {
    items: Vec<MemoryRelationRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRelationRecordResponse {
    id: String,
    tenant_id: String,
    memory_relation_id: String,
    from_memory_id: String,
    to_memory_id: String,
    relation_kind: String,
    weight: f32,
    valid_from: Option<String>,
    valid_until: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRetrievalIndexResponse {
    data: MemoryRetrievalIndexRecordResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRetrievalIndexListResponse {
    data: MemoryRetrievalIndexListDataResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRetrievalIndexListDataResponse {
    items: Vec<MemoryRetrievalIndexRecordResponse>,
    page_info: PageInfoResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRetrievalIndexRecordResponse {
    id: String,
    tenant_id: String,
    memory_index_id: String,
    memory_id: String,
    index_kind: String,
    index_provider_id: String,
    external_ref: String,
    embedding_model_id: Option<String>,
    vector_dimension: Option<u32>,
    content_hash: String,
    indexed_at: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodeTaskIntentBody {
    prompt: String,
    context_paths: Option<Vec<String>>,
    constraints: Option<Vec<String>>,
}

impl From<CodeTaskIntentBody> for CodeTaskIntent {
    fn from(value: CodeTaskIntentBody) -> Self {
        Self {
            prompt: value.prompt,
            context_paths: value.context_paths.unwrap_or_default(),
            constraints: value.constraints.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentManagementProfileBody {
    author: Option<String>,
    avatar: Option<String>,
    category_id: Option<String>,
    color: Option<String>,
    debug_mode: Option<bool>,
    icon_name: Option<String>,
    json_mode: Option<bool>,
    knowledge_base_ids: Option<Vec<String>>,
    memory_enabled: Option<bool>,
    model: Option<String>,
    skill_ids: Option<Vec<String>>,
    suggested_prompts: Option<Vec<String>>,
    system_prompt: Option<String>,
    temperature: Option<f64>,
    tool_ids: Option<Vec<String>>,
    #[serde(rename = "type")]
    agent_type: Option<String>,
    users: Option<String>,
    voice_ids: Option<Vec<String>>,
    welcome_message: Option<String>,
}

impl From<AgentManagementProfileBody> for AgentManagementProfileDto {
    fn from(value: AgentManagementProfileBody) -> Self {
        Self {
            author: value.author,
            avatar: value.avatar,
            category_id: value.category_id,
            color: value.color,
            debug_mode: value.debug_mode,
            icon_name: value.icon_name,
            json_mode: value.json_mode,
            knowledge_base_ids: value.knowledge_base_ids.unwrap_or_default(),
            memory_enabled: value.memory_enabled,
            model: value.model,
            skill_ids: value.skill_ids.unwrap_or_default(),
            suggested_prompts: value.suggested_prompts.unwrap_or_default(),
            system_prompt: value.system_prompt,
            temperature: value.temperature,
            tool_ids: value.tool_ids.unwrap_or_default(),
            agent_type: value.agent_type,
            users: value.users,
            voice_ids: value.voice_ids.unwrap_or_default(),
            welcome_message: value.welcome_message,
        }
    }
}

impl AgentManagementProfileBody {
    fn into_validated_dto(self) -> Result<AgentManagementProfileDto, ApiProblem> {
        validate_agent_management_profile_body(&self)?;
        Ok(self.into())
    }
}

fn validate_agent_management_profile_body(
    profile: &AgentManagementProfileBody,
) -> Result<(), ApiProblem> {
    validate_optional_profile_string(profile.author.as_deref(), "managementProfile.author", 128)?;
    validate_optional_profile_string(profile.avatar.as_deref(), "managementProfile.avatar", 512)?;
    validate_optional_profile_string(
        profile.category_id.as_deref(),
        "managementProfile.categoryId",
        64,
    )?;
    validate_optional_profile_string(profile.color.as_deref(), "managementProfile.color", 32)?;
    validate_optional_profile_string(
        profile.icon_name.as_deref(),
        "managementProfile.iconName",
        64,
    )?;
    validate_profile_standard_ids(
        profile.knowledge_base_ids.as_deref().unwrap_or_default(),
        "managementProfile.knowledgeBaseIds",
        "knowledge.base.",
        128,
    )?;
    if let Some(model) = profile.model.as_deref() {
        validate_standard_id(model, "managementProfile.model", Some("model."))
            .map_err(ApiProblem::from_kernel_error)?;
    }
    validate_profile_standard_ids(
        profile.skill_ids.as_deref().unwrap_or_default(),
        "managementProfile.skillIds",
        "skill.",
        128,
    )?;
    validate_profile_suggested_prompts(profile.suggested_prompts.as_deref().unwrap_or_default())?;
    validate_optional_profile_string(
        profile.system_prompt.as_deref(),
        "managementProfile.systemPrompt",
        32768,
    )?;
    if let Some(temperature) = profile.temperature {
        if temperature < 0.0 {
            return Err(ApiProblem::validation(
                "managementProfile.temperature must be greater than or equal to 0",
            ));
        }
        if temperature > 2.0 {
            return Err(ApiProblem::validation(
                "managementProfile.temperature must be less than or equal to 2",
            ));
        }
    }
    validate_profile_standard_ids(
        profile.tool_ids.as_deref().unwrap_or_default(),
        "managementProfile.toolIds",
        "tool.",
        128,
    )?;
    if let Some(agent_type) = profile.agent_type.as_deref() {
        if !matches!(agent_type, "normal" | "independent") {
            return Err(ApiProblem::validation(
                "managementProfile.type must be one of normal, independent",
            ));
        }
    }
    validate_optional_profile_string(profile.users.as_deref(), "managementProfile.users", 128)?;
    validate_profile_standard_ids(
        profile.voice_ids.as_deref().unwrap_or_default(),
        "managementProfile.voiceIds",
        "voice.",
        16,
    )?;
    validate_optional_profile_string(
        profile.welcome_message.as_deref(),
        "managementProfile.welcomeMessage",
        4096,
    )?;
    Ok(())
}

fn validate_optional_profile_string(
    value: Option<&str>,
    field_name: &str,
    max_length: usize,
) -> Result<(), ApiProblem> {
    let Some(value) = value else {
        return Ok(());
    };
    let length = value.chars().count();
    if length == 0 {
        return Err(ApiProblem::validation(format!("{field_name} is required")));
    }
    if length > max_length {
        return Err(ApiProblem::validation(format!(
            "{field_name} must be at most {max_length} characters"
        )));
    }
    Ok(())
}

fn validate_profile_standard_ids(
    values: &[String],
    field_name: &str,
    required_prefix: &str,
    max_items: usize,
) -> Result<(), ApiProblem> {
    if values.len() > max_items {
        return Err(ApiProblem::validation(format!(
            "{field_name} must contain at most {max_items} items"
        )));
    }
    for value in values {
        validate_standard_id(
            value.as_str(),
            format!("{field_name} items").as_str(),
            Some(required_prefix),
        )
        .map_err(ApiProblem::from_kernel_error)?;
    }
    Ok(())
}

fn validate_profile_suggested_prompts(values: &[String]) -> Result<(), ApiProblem> {
    if values.len() > 12 {
        return Err(ApiProblem::validation(
            "managementProfile.suggestedPrompts must contain at most 12 items",
        ));
    }
    for value in values {
        let length = value.chars().count();
        if length == 0 {
            return Err(ApiProblem::validation(
                "managementProfile.suggestedPrompts items is required",
            ));
        }
        if length > 256 {
            return Err(ApiProblem::validation(
                "managementProfile.suggestedPrompts items must be at most 256 characters",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDocumentProfileBody {
    author: Option<String>,
    content: Option<String>,
    parent_id: Option<String>,
    #[serde(rename = "type")]
    document_type: Option<String>,
    file_name: Option<String>,
    file_size: Option<String>,
    mime_type: Option<String>,
    drive_uri: Option<String>,
}

impl From<KnowledgeDocumentProfileBody> for AgentKnowledgeDocumentProfileDto {
    fn from(value: KnowledgeDocumentProfileBody) -> Self {
        Self {
            author: value.author,
            content: value.content,
            parent_id: value.parent_id,
            document_type: value.document_type,
            file_name: value.file_name,
            file_size: value.file_size,
            mime_type: value.mime_type,
            drive_uri: value.drive_uri,
        }
    }
}

impl KnowledgeDocumentProfileBody {
    fn into_validated_dto(self) -> Result<AgentKnowledgeDocumentProfileDto, ApiProblem> {
        validate_knowledge_document_profile_body(&self)?;
        Ok(self.into())
    }
}

fn validate_knowledge_document_profile_body(
    profile: &KnowledgeDocumentProfileBody,
) -> Result<(), ApiProblem> {
    validate_optional_profile_string(profile.author.as_deref(), "documentProfile.author", 128)?;
    validate_optional_document_profile_content(profile.content.as_deref())?;
    if let Some(parent_id) = profile.parent_id.as_deref() {
        validate_standard_id(
            parent_id,
            "documentProfile.parentId",
            Some("knowledge.document."),
        )
        .map_err(ApiProblem::from_kernel_error)?;
    }
    if let Some(document_type) = profile.document_type.as_deref() {
        if !matches!(document_type, "markdown" | "file" | "folder") {
            return Err(ApiProblem::validation(
                "documentProfile.type must be one of markdown, file, folder",
            ));
        }
    }
    validate_optional_profile_string(
        profile.file_name.as_deref(),
        "documentProfile.fileName",
        512,
    )?;
    validate_optional_profile_string(profile.file_size.as_deref(), "documentProfile.fileSize", 64)?;
    validate_optional_profile_string(
        profile.mime_type.as_deref(),
        "documentProfile.mimeType",
        255,
    )?;
    validate_optional_profile_string(
        profile.drive_uri.as_deref(),
        "documentProfile.driveUri",
        1024,
    )?;
    Ok(())
}

fn validate_optional_document_profile_content(value: Option<&str>) -> Result<(), ApiProblem> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.chars().count() > 1_048_576 {
        return Err(ApiProblem::validation(
            "documentProfile.content must be at most 1048576 characters",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemDetailResponse {
    r#type: String,
    title: String,
    status: u16,
    detail: String,
    code: String,
    error_category: String,
    retryable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiProblem {
    status: StatusCode,
    response: ProblemDetailResponse,
}

#[derive(Debug, Clone, Copy)]
enum ErrorCategory {
    Validation,
    Permission,
    Business,
    Concurrency,
    Resource,
    Internal,
}

impl ErrorCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Permission => "permission",
            Self::Business => "business",
            Self::Concurrency => "concurrency",
            Self::Resource => "resource",
            Self::Internal => "internal",
        }
    }
}

impl ApiProblem {
    fn validation(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "validation_error",
            ErrorCategory::Validation,
            false,
            detail,
        )
    }

    fn permission(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "permission_required",
            ErrorCategory::Permission,
            false,
            detail,
        )
    }

    fn conflict(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "conflict",
            ErrorCategory::Business,
            false,
            detail,
        )
    }

    fn version_conflict(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "version_conflict",
            ErrorCategory::Concurrency,
            true,
            detail,
        )
    }

    fn not_found(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            ErrorCategory::Resource,
            false,
            detail,
        )
    }

    fn internal(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            ErrorCategory::Internal,
            false,
            detail,
        )
    }

    fn new(
        status: StatusCode,
        code: impl Into<String>,
        error_category: ErrorCategory,
        retryable: bool,
        detail: impl Into<String>,
    ) -> Self {
        let code = code.into();
        Self {
            status,
            response: ProblemDetailResponse {
                r#type: format!("https://sdkwork.dev/problems/{code}"),
                title: code.clone(),
                status: status.as_u16(),
                detail: detail.into(),
                code,
                error_category: error_category.as_str().to_string(),
                retryable,
            },
        }
    }

    fn from_kernel_error(error: KernelError) -> Self {
        let safe_message = error.safe_message();
        if safe_message.contains("not found") {
            return Self::not_found(safe_message);
        }
        match error.kind() {
            KernelErrorKind::ValidationError => Self::validation(error.safe_message()),
            KernelErrorKind::Conflict => {
                if safe_message.contains("version mismatch") {
                    Self::version_conflict(safe_message)
                } else {
                    Self::conflict(safe_message)
                }
            }
            KernelErrorKind::PermissionRequired | KernelErrorKind::PolicyDenied => {
                Self::permission(error.safe_message())
            }
            _ => Self::internal(error.safe_message()),
        }
    }

    fn from_json_rejection(rejection: JsonRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
            ErrorCategory::Validation,
            false,
            format!("invalid json request: {}", rejection.body_text()),
        )
    }

    fn from_query_rejection(rejection: QueryRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
            ErrorCategory::Validation,
            false,
            format!("invalid query request: {}", rejection.body_text()),
        )
    }

    fn from_path_rejection(rejection: PathRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
            ErrorCategory::Validation,
            false,
            format!("invalid path request: {}", rejection.body_text()),
        )
    }
}

impl IntoResponse for ApiProblem {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.response)).into_response();
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

async fn app_list_agents(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<AppListAgentsQueryParams>, QueryRejection>,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_context(context);
    let owner_user_id = match query.scope.as_deref() {
        Some("market" | "public" | "published") => None,
        _ => Some(scope.owner_user_id.clone()),
    };
    let query = ListAgentsQueryParams {
        tenant_id: scope.tenant_id.clone(),
        organization_id: Some(scope.organization_id.clone()),
        owner_user_id,
        scope: query.scope,
        include_deleted: query.include_deleted,
        q: query.q,
        page: query.page,
        page_size: query.page_size,
    };
    execute_list(state, query, scope).await
}

async fn backend_list_agents(
    State(state): State<AgentHttpState>,
    query: Result<Query<ListAgentsQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id.clone(),
        query.organization_id.clone(),
        query.owner_user_id.clone()
    )?;
    execute_list(state, query, scope).await
}

async fn app_create_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<CreateAgentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create(state, RequestScope::from_context(context), body).await
}

async fn backend_create_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<CreateAgentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        body.owner_user_id.clone()
    )?;
    execute_create(state, scope, body).await
}

async fn app_get_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    agent_id: Result<Path<String>, PathRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    execute_get(state, RequestScope::from_context(context), agent_id).await
}

async fn backend_get_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get(state, scope, agent_id).await
}

async fn app_update_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    agent_id: Result<Path<String>, PathRejection>,
    body: Result<Json<UpdateAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update(state, RequestScope::from_context(context), agent_id, body).await
}

async fn backend_update_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<UpdateAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_update(state, scope, agent_id, body).await
}

async fn app_delete_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    agent_id: Result<Path<String>, PathRejection>,
    body: Result<Json<DeleteAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_delete(state, RequestScope::from_context(context), agent_id, body).await
}

async fn open_delete_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<DeleteAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_delete(state, scope, agent_id, body).await
}

async fn app_restore_agent(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    agent_id: Result<Path<String>, PathRejection>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore(state, RequestScope::from_context(context), agent_id, body).await
}

async fn backend_update_agent_status(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<UpdateAgentStatusBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    let subject = scope.subject.clone();
    let command = UpdateAgentStatusRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
        expected_version: body.expected_version,
        target_status: body.target_status,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.change_status(command)).await?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn backend_restore_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_restore(state, scope, agent_id, body).await
}

async fn backend_list_agent_audit_events(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<AuditEventsQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<AgentAuditEventsListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    let subject = scope.subject.clone();
    let tenant_id = scope.tenant_id_u64()?;
    let events = with_service_mut(&state, move |service| {
        service.list_agent_audit_events(tenant_id, path.agent_id.as_str(), subject)
    })
    .await?;
    let events = filter_audit_events(events, &query)?;

    let (page, page_size) = normalized_pagination(query.page, query.page_size)?;
    let total_items = events.len();
    let total_pages = if total_items == 0 {
        0
    } else {
        total_items.div_ceil(page_size)
    };
    let paged = paginate(events, page, page_size);

    let items: Vec<AgentAuditEventResponse> = paged
        .into_iter()
        .map(|event| AgentAuditEventResponse {
            event_id: event.event_id,
            event_type: event.event_type,
            severity: kernel_event_severity(event.severity).to_string(),
            payload: event.payload,
            occurred_at: event.occurred_at.unwrap_or_default(),
        })
        .collect();

    Ok(Json(AgentAuditEventsListResponse {
        data: AgentAuditEventsData {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn app_list_provider_bindings(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<AgentProviderBindingListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_provider_bindings(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.agent_id,
    )
    .await
}

async fn backend_list_provider_bindings(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<AgentProviderBindingListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_provider_bindings(state, scope, query.page, query.page_size, path.agent_id).await
}

async fn app_add_provider_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    body: Result<Json<AgentProviderBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentProviderBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_add_provider_binding(
        state,
        RequestScope::from_context(context),
        path.agent_id,
        body,
    )
    .await
}

async fn backend_add_provider_binding(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<AgentProviderBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentProviderBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_add_provider_binding(state, scope, path.agent_id, body).await
}

async fn app_activate_provider_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentBindingPathParams>, PathRejection>,
    body: Result<Json<ActivateProviderBindingBody>, JsonRejection>,
) -> Result<Json<AgentProviderBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_activate_provider_binding(state, RequestScope::from_context(context), path, body).await
}

async fn backend_activate_provider_binding(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentBindingPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<ActivateProviderBindingBody>, JsonRejection>,
) -> Result<Json<AgentProviderBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_activate_provider_binding(state, scope, path, body).await
}

async fn app_list_deployments(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<AgentDeploymentListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_deployments(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.agent_id,
    )
    .await
}

async fn backend_list_deployments(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<AgentDeploymentListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_deployments(state, scope, query.page, query.page_size, path.agent_id).await
}

async fn app_create_deployment(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    body: Result<Json<AgentDeploymentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentDeploymentResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_deployment(
        state,
        RequestScope::from_context(context),
        path.agent_id,
        body,
    )
    .await
}

async fn backend_create_deployment(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<AgentDeploymentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentDeploymentResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_create_deployment(state, scope, path.agent_id, body).await
}

async fn app_create_preview_response(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    body: Result<Json<AgentPreviewResponseBody>, JsonRejection>,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_preview_response(
        state,
        RequestScope::from_context(context),
        path.agent_id,
        body,
    )
    .await
}

async fn app_create_prompt_optimization(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    body: Result<Json<AgentPromptOptimizationBody>, JsonRejection>,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_prompt_optimization(
        state,
        RequestScope::from_context(context),
        path.agent_id,
        body,
    )
    .await
}

async fn open_create_preview_response(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<AgentPreviewResponseBody>, JsonRejection>,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_create_preview_response(state, scope, path.agent_id, body).await
}

async fn open_create_prompt_optimization(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<AgentPromptOptimizationBody>, JsonRejection>,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_create_prompt_optimization(state, scope, path.agent_id, body).await
}

async fn app_list_knowledge_bases(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<AppKnowledgeBaseListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeBaseListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_context(context);
    let query = KnowledgeBaseListQueryParams {
        tenant_id: scope.tenant_id.clone(),
        organization_id: Some(scope.organization_id.clone()),
        owner_user_id: Some(scope.owner_user_id.clone()),
        include_deleted: query.include_deleted,
        q: query.q,
        status: query.status,
        visibility: query.visibility,
        category: query.category,
        tag: query.tag,
        page: query.page,
        page_size: query.page_size,
    };
    execute_list_knowledge_bases(state, query, scope).await
}

async fn app_create_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeBaseBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeBaseResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_base(state, RequestScope::from_context(context), body).await
}

async fn app_get_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_base(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
    )
    .await
}

async fn app_update_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeBaseUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update_knowledge_base(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_delete_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<AppDeleteQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_delete_knowledge_base(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn app_restore_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore_knowledge_base(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_list_knowledge_sources(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeSourceListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_sources(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn app_create_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeSourceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeSourceResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_source(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_get_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_source(
        state,
        RequestScope::from_context(context),
        path.knowledge_source_id,
    )
    .await
}

async fn app_update_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    body: Result<Json<KnowledgeSourceUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update_knowledge_source(
        state,
        RequestScope::from_context(context),
        path.knowledge_source_id,
        body,
    )
    .await
}

async fn app_delete_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    query: Result<Query<AppDeleteQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_delete_knowledge_source(
        state,
        RequestScope::from_context(context),
        path.knowledge_source_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn app_restore_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore_knowledge_source(
        state,
        RequestScope::from_context(context),
        path.knowledge_source_id,
        body,
    )
    .await
}

async fn app_list_knowledge_documents(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeDocumentListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_documents(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn app_create_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeDocumentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeDocumentResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_document(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_search_knowledge(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeSearchBody>, JsonRejection>,
) -> Result<Json<KnowledgeSearchResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_search_knowledge(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_get_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_document(
        state,
        RequestScope::from_context(context),
        path.knowledge_document_id,
    )
    .await
}

async fn app_update_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    body: Result<Json<KnowledgeDocumentUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update_knowledge_document(
        state,
        RequestScope::from_context(context),
        path.knowledge_document_id,
        body,
    )
    .await
}

async fn app_delete_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<AppDeleteQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_delete_knowledge_document(
        state,
        RequestScope::from_context(context),
        path.knowledge_document_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn app_restore_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore_knowledge_document(
        state,
        RequestScope::from_context(context),
        path.knowledge_document_id,
        body,
    )
    .await
}

async fn app_list_knowledge_chunks(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeChunkListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_chunks(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_document_id,
    )
    .await
}

async fn app_create_knowledge_chunk(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    body: Result<Json<KnowledgeChunkBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeChunkResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_chunk(
        state,
        RequestScope::from_context(context),
        path.knowledge_document_id,
        body,
    )
    .await
}

async fn app_get_knowledge_chunk(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeChunkPathParams>, PathRejection>,
) -> Result<Json<KnowledgeChunkResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_chunk(
        state,
        RequestScope::from_context(context),
        path.knowledge_chunk_id,
    )
    .await
}

async fn app_list_knowledge_indexes(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeIndexListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_indexes(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_document_id,
    )
    .await
}

async fn app_upsert_knowledge_index(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeIndexBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeIndexResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_upsert_knowledge_index(state, RequestScope::from_context(context), body).await
}

async fn app_get_knowledge_index(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeIndexPathParams>, PathRejection>,
) -> Result<Json<KnowledgeIndexResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_index(
        state,
        RequestScope::from_context(context),
        path.knowledge_index_id,
    )
    .await
}

async fn app_list_knowledge_bindings(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeBindingListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_bindings(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn app_create_knowledge_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_binding(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_get_knowledge_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBindingPathParams>, PathRejection>,
) -> Result<Json<KnowledgeBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_binding(
        state,
        RequestScope::from_context(context),
        path.knowledge_binding_id,
    )
    .await
}

async fn app_list_knowledge_sync_jobs(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeSyncJobListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_knowledge_sync_jobs(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn app_create_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    body: Result<Json<KnowledgeSyncJobBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeSyncJobResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_knowledge_sync_job(
        state,
        RequestScope::from_context(context),
        path.knowledge_base_id,
        body,
    )
    .await
}

async fn app_get_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_knowledge_sync_job(state, RequestScope::from_context(context), path.sync_job_id)
        .await
}

async fn app_start_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    body: Result<Json<KnowledgeSyncJobStartBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_start_knowledge_sync_job(
        state,
        RequestScope::from_context(context),
        path.sync_job_id,
        body,
    )
    .await
}

async fn app_complete_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    body: Result<Json<KnowledgeSyncJobCompleteBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_complete_knowledge_sync_job(
        state,
        RequestScope::from_context(context),
        path.sync_job_id,
        body,
    )
    .await
}

async fn app_fail_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    body: Result<Json<KnowledgeSyncJobFailBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_fail_knowledge_sync_job(
        state,
        RequestScope::from_context(context),
        path.sync_job_id,
        body,
    )
    .await
}

async fn app_cancel_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    body: Result<Json<KnowledgeSyncJobCancelBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_cancel_knowledge_sync_job(
        state,
        RequestScope::from_context(context),
        path.sync_job_id,
        body,
    )
    .await
}

async fn list_knowledge_bases(
    State(state): State<AgentHttpState>,
    query: Result<Query<KnowledgeBaseListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeBaseListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id.clone(),
        query.organization_id.clone(),
        query.owner_user_id.clone()
    )?;
    execute_list_knowledge_bases(state, query, scope).await
}

async fn create_knowledge_base(
    State(state): State<AgentHttpState>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeBaseBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeBaseResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        body.owner_user_id.clone()
    )?;
    execute_create_knowledge_base(state, scope, body).await
}

async fn get_knowledge_base(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_base(state, scope, path.knowledge_base_id).await
}

async fn update_knowledge_base(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeBaseUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_update_knowledge_base(state, scope, path.knowledge_base_id, body).await
}

async fn delete_knowledge_base(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<DeleteKnowledgeBaseQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_delete_knowledge_base(
        state,
        scope,
        path.knowledge_base_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn restore_knowledge_base(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_restore_knowledge_base(state, scope, path.knowledge_base_id, body).await
}

async fn list_knowledge_sources(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeSourceListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_sources(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn create_knowledge_source(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeSourceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeSourceResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_knowledge_source(state, scope, path.knowledge_base_id, body).await
}

async fn get_knowledge_source(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_source(state, scope, path.knowledge_source_id).await
}

async fn update_knowledge_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSourceUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_update_knowledge_source(state, scope, path.knowledge_source_id, body).await
}

async fn delete_knowledge_source(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    query: Result<Query<DeleteKnowledgeSourceQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_delete_knowledge_source(
        state,
        scope,
        path.knowledge_source_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn restore_knowledge_source(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeSourcePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_restore_knowledge_source(state, scope, path.knowledge_source_id, body).await
}

async fn list_knowledge_documents(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeDocumentListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_documents(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn create_knowledge_document(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeDocumentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeDocumentResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_knowledge_document(state, scope, path.knowledge_base_id, body).await
}

async fn search_knowledge(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSearchBody>, JsonRejection>,
) -> Result<Json<KnowledgeSearchResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_search_knowledge(state, scope, path.knowledge_base_id, body).await
}

async fn get_knowledge_document(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_document(state, scope, path.knowledge_document_id).await
}

async fn update_knowledge_document(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeDocumentUpdateBody>, JsonRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_update_knowledge_document(state, scope, path.knowledge_document_id, body).await
}

async fn delete_knowledge_document(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<DeleteKnowledgeDocumentQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_delete_knowledge_document(
        state,
        scope,
        path.knowledge_document_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn restore_knowledge_document(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_restore_knowledge_document(state, scope, path.knowledge_document_id, body).await
}

async fn list_knowledge_chunks(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeChunkListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_chunks(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_document_id,
    )
    .await
}

async fn create_knowledge_chunk(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeChunkBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeChunkResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_knowledge_chunk(state, scope, path.knowledge_document_id, body).await
}

async fn get_knowledge_chunk(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeChunkPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeChunkResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_chunk(state, scope, path.knowledge_chunk_id).await
}

async fn list_knowledge_indexes(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeDocumentPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeIndexListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_indexes(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_document_id,
    )
    .await
}

async fn upsert_knowledge_index(
    State(state): State<AgentHttpState>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeIndexBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeIndexResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_upsert_knowledge_index(state, scope, body).await
}

async fn get_knowledge_index(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeIndexPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeIndexResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_index(state, scope, path.knowledge_index_id).await
}

async fn list_knowledge_bindings(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeBindingListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_bindings(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn create_knowledge_binding(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_knowledge_binding(state, scope, path.knowledge_base_id, body).await
}

async fn get_knowledge_binding(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBindingPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_binding(state, scope, path.knowledge_binding_id).await
}

async fn list_knowledge_sync_jobs(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
) -> Result<Json<KnowledgeSyncJobListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_knowledge_sync_jobs(
        state,
        scope,
        query.page,
        query.page_size,
        path.knowledge_base_id,
    )
    .await
}

async fn create_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeBasePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<KnowledgeSyncJobBody>, JsonRejection>,
) -> Result<(StatusCode, Json<KnowledgeSyncJobResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_knowledge_sync_job(state, scope, path.knowledge_base_id, body).await
}

async fn get_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_knowledge_sync_job(state, scope, path.sync_job_id).await
}

async fn start_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSyncJobStartBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_start_knowledge_sync_job(state, scope, path.sync_job_id, body).await
}

async fn complete_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSyncJobCompleteBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_complete_knowledge_sync_job(state, scope, path.sync_job_id, body).await
}

async fn fail_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSyncJobFailBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_fail_knowledge_sync_job(state, scope, path.sync_job_id, body).await
}

async fn cancel_knowledge_sync_job(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<KnowledgeSyncJobPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<KnowledgeSyncJobCancelBody>, JsonRejection>,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_cancel_knowledge_sync_job(state, scope, path.sync_job_id, body).await
}

async fn app_create_memory_store(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<MemoryStoreBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryStoreResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_store(state, RequestScope::from_context(context), body).await
}

async fn app_get_memory_store(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_memory_store(
        state,
        RequestScope::from_context(context),
        path.memory_store_id,
    )
    .await
}

async fn app_update_memory_store(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
    body: Result<Json<MemoryStoreUpdateBody>, JsonRejection>,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update_memory_store(
        state,
        RequestScope::from_context(context),
        path.memory_store_id,
        body,
    )
    .await
}

async fn app_create_memory_profile(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
    body: Result<Json<MemoryProfileBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryProfileResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_profile(
        state,
        RequestScope::from_context(context),
        path.memory_store_id,
        body,
    )
    .await
}

async fn app_get_memory_profile(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryProfilePathParams>, PathRejection>,
) -> Result<Json<MemoryProfileResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_memory_profile(
        state,
        RequestScope::from_context(context),
        path.memory_profile_id,
    )
    .await
}

async fn app_create_memory_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryProfilePathParams>, PathRejection>,
    body: Result<Json<MemoryBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_binding(
        state,
        RequestScope::from_context(context),
        path.memory_profile_id,
        body,
    )
    .await
}

async fn app_get_memory_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryBindingPathParams>, PathRejection>,
) -> Result<Json<MemoryBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_memory_binding(
        state,
        RequestScope::from_context(context),
        path.memory_binding_id,
    )
    .await
}

async fn app_create_memory_namespace(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<MemoryNamespaceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryNamespaceResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_namespace(state, RequestScope::from_context(context), body).await
}

async fn app_get_memory_namespace(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
) -> Result<Json<MemoryNamespaceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_memory_namespace(
        state,
        RequestScope::from_context(context),
        path.memory_namespace_id,
    )
    .await
}

async fn app_list_memory_records(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRecordListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_memory_records(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.memory_namespace_id,
    )
    .await
}

async fn app_create_memory_record(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
    body: Result<Json<MemoryRecordBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRecordResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_record(
        state,
        RequestScope::from_context(context),
        path.memory_namespace_id,
        body,
    )
    .await
}

async fn app_get_memory_record(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    execute_get_memory_record(state, RequestScope::from_context(context), path.memory_id).await
}

async fn app_delete_memory_record(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<AppDeleteQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_delete_memory_record(
        state,
        RequestScope::from_context(context),
        path.memory_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn app_restore_memory_record(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore_memory_record(
        state,
        RequestScope::from_context(context),
        path.memory_id,
        body,
    )
    .await
}

async fn app_list_memory_sources(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<MemorySourceListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_memory_sources(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.memory_id,
    )
    .await
}

async fn app_create_memory_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    body: Result<Json<MemorySourceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemorySourceResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_source(
        state,
        RequestScope::from_context(context),
        path.memory_id,
        body,
    )
    .await
}

async fn app_list_memory_relations(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRelationListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_memory_relations(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.memory_id,
    )
    .await
}

async fn app_create_memory_relation(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    body: Result<Json<MemoryRelationBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRelationResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create_memory_relation(
        state,
        RequestScope::from_context(context),
        path.memory_id,
        body,
    )
    .await
}

async fn app_list_memory_retrieval_indexes(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<AppListQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRetrievalIndexListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list_memory_retrieval_indexes(
        state,
        RequestScope::from_context(context),
        query.page,
        query.page_size,
        path.memory_id,
    )
    .await
}

async fn app_upsert_memory_retrieval_index(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<MemoryRetrievalIndexBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRetrievalIndexResponse>), ApiProblem> {
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_upsert_memory_retrieval_index(state, RequestScope::from_context(context), body).await
}

async fn create_memory_store(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryStoreBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryStoreResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        body.owner_user_id.clone()
    )?;
    execute_create_memory_store(state, scope, body).await
}

async fn get_memory_store(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_memory_store(state, scope, path.memory_store_id).await
}

async fn update_memory_store(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryStoreUpdateBody>, JsonRejection>,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_update_memory_store(state, scope, path.memory_store_id, body).await
}

async fn create_memory_profile(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryStorePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<MemoryProfileBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryProfileResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        body.owner_user_id.clone()
    )?;
    execute_create_memory_profile(state, scope, path.memory_store_id, body).await
}

async fn get_memory_profile(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryProfilePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryProfileResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_memory_profile(state, scope, path.memory_profile_id).await
}

async fn create_memory_binding(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryProfilePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryBindingBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryBindingResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_memory_binding(state, scope, path.memory_profile_id, body).await
}

async fn get_memory_binding(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryBindingPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryBindingResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_memory_binding(state, scope, path.memory_binding_id).await
}

async fn create_memory_namespace(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryNamespaceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryNamespaceResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_memory_namespace(state, scope, body).await
}

async fn get_memory_namespace(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryNamespaceResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_memory_namespace(state, scope, path.memory_namespace_id).await
}

async fn list_memory_records(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRecordListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_memory_records(
        state,
        scope,
        query.page,
        query.page_size,
        path.memory_namespace_id,
    )
    .await
}

async fn create_memory_record(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryNamespacePathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<MemoryRecordBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRecordResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, 
        query.tenant_id,
        body.organization_id.clone(),
        None
    )?;
    execute_create_memory_record(state, scope, path.memory_namespace_id, body).await
}

async fn get_memory_record(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_get_memory_record(state, scope, path.memory_id).await
}

async fn delete_memory_record(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<DeleteMemoryRecordQueryParams>, QueryRejection>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_delete_memory_record(
        state,
        scope,
        path.memory_id,
        query.expected_version,
        query.requested_at,
    )
    .await
}

async fn restore_memory_record(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_restore_memory_record(state, scope, path.memory_id, body).await
}

async fn list_memory_sources(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemorySourceListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_memory_sources(state, scope, query.page, query.page_size, path.memory_id).await
}

async fn create_memory_source(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemorySourceBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemorySourceResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_create_memory_source(state, scope, path.memory_id, body).await
}

async fn list_memory_relations(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryRelationListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_memory_relations(state, scope, query.page, query.page_size, path.memory_id).await
}

async fn create_memory_relation(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryRelationBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRelationResponse>), ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_create_memory_relation(state, scope, path.memory_id, body).await
}

async fn list_memory_retrieval_indexes(
    State(state): State<AgentHttpState>,
    path: Result<Path<MemoryRecordPathParams>, PathRejection>,
    query: Result<Query<TenantListQueryParams>, QueryRejection>,
    Extension(context): Extension<AgentRequestContext>,
) -> Result<Json<MemoryRetrievalIndexListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_list_memory_retrieval_indexes(state, scope, query.page, query.page_size, path.memory_id)
        .await
}

async fn upsert_memory_retrieval_index(
    State(state): State<AgentHttpState>,
    Extension(context): Extension<AgentRequestContext>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    body: Result<Json<MemoryRetrievalIndexBody>, JsonRejection>,
) -> Result<(StatusCode, Json<MemoryRetrievalIndexResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let scope = RequestScope::from_trusted_extension(context, query.tenant_id.clone(), None, None)?;
    execute_upsert_memory_retrieval_index(state, scope, body).await
}

#[cfg(not(feature = "postgres-sync"))]
async fn with_service_mut<T>(
    state: &AgentHttpState,
    action: impl FnOnce(&mut HttpService) -> KernelResult<T>,
) -> Result<T, ApiProblem> {
    let mut service = state.service.lock().await;
    action(&mut *service).map_err(ApiProblem::from_kernel_error)
}

#[cfg(feature = "postgres-sync")]
async fn with_service_mut<T, F>(state: &AgentHttpState, action: F) -> Result<T, ApiProblem>
where
    F: FnOnce(&mut HttpService) -> KernelResult<T> + Send + 'static,
    T: Send + 'static,
{
    let service = Arc::clone(&state.service);
    let result = tokio::task::spawn_blocking(move || {
        let mut guard = service
            .lock()
            .map_err(|_| KernelError::Internal {
                message: "agent business service lock poisoned".to_string(),
            })?;
        action(&mut *guard)
    })
    .await
    .map_err(|_| {
        ApiProblem::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            ErrorCategory::Internal,
            false,
            "agent business service worker failed",
        )
    })?;
    result.map_err(ApiProblem::from_kernel_error)
}

fn knowledge_base_document_count(
    service: &mut HttpService,
    tenant_id: u64,
    knowledge_base_id: &str,
    subject: PolicySubject,
) -> KernelResult<u32> {
    let documents = service.list_knowledge_documents(tenant_id, knowledge_base_id, subject)?;
    u32::try_from(documents.len()).map_err(|_| KernelError::Internal {
        message: "knowledge base document count exceeds u32 range".to_string(),
    })
}

async fn execute_list_knowledge_bases(
    state: AgentHttpState,
    query: KnowledgeBaseListQueryParams,
    scope: RequestScope,
) -> Result<Json<KnowledgeBaseListResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    let request_dto = ListAgentKnowledgeBasesRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: query.organization_id,
        owner_user_id: query.owner_user_id,
        include_deleted: query.include_deleted.unwrap_or(false),
        search_query: query.q,
        status: query.status,
        visibility: query.visibility,
        category: query.category,
        tag: query.tag,
    };
    let list_query = request_dto
        .into_query()
        .map_err(ApiProblem::from_kernel_error)?;

    let list_subject = subject.clone();
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_bases(list_query, list_subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(query.page, query.page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = with_service_mut(&state, move |service| {
        paged
            .iter()
            .map(|record| {
                let document_count = if record.is_deleted() {
                    0
                } else {
                    knowledge_base_document_count(
                        service,
                        record.tenant_id,
                        record.knowledge_base_id.as_str(),
                        subject.clone(),
                    )?
                };
                Ok(map_knowledge_base_record(
                    &AgentKnowledgeBaseRecordDto::from_record(record),
                    document_count,
                ))
            })
            .collect::<KernelResult<Vec<_>>>()
    })
    .await?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeBaseListResponse {
        data: KnowledgeBaseListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_base(
    state: AgentHttpState,
    scope: RequestScope,
    body: KnowledgeBaseBody,
) -> Result<(StatusCode, Json<KnowledgeBaseResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentKnowledgeBaseCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        owner_user_id: scope.owner_user_id,
        knowledge_base_id: body.knowledge_base_id,
        code: body.code,
        display_name: body.display_name,
        description: body.description,
        provider_id: body.provider_id,
        base_kind: body.base_kind,
        retrieval_modes: body.retrieval_modes,
        capability_ids: body.capability_ids.unwrap_or_default(),
        configuration_profile_id: body.configuration_profile_id,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject.clone())
    .map_err(ApiProblem::from_kernel_error)?;

    let (record, document_count) = with_service_mut(&state, move |service| {
        let record = service.create_knowledge_base(command)?;
        let document_count = knowledge_base_document_count(
            service,
            record.tenant_id,
            record.knowledge_base_id.as_str(),
            subject.clone(),
        )?;
        Ok((record, document_count))
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeBaseResponse {
            data: map_knowledge_base_record(
                &AgentKnowledgeBaseRecordDto::from_record(&record),
                document_count,
            ),
        }),
    ))
}

async fn execute_get_knowledge_base(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_base_id,
        requested_by: subject.clone(),
    };
    let (record, document_count) = with_service_mut(&state, move |service| {
        let record = service.get_knowledge_base(command)?;
        let document_count = knowledge_base_document_count(
            service,
            record.tenant_id,
            record.knowledge_base_id.as_str(),
            subject.clone(),
        )?;
        Ok((record, document_count))
    })
    .await?;
    Ok(Json(KnowledgeBaseResponse {
        data: map_knowledge_base_record(
            &AgentKnowledgeBaseRecordDto::from_record(&record),
            document_count,
        ),
    }))
}

async fn execute_update_knowledge_base(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeBaseUpdateBody,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentKnowledgeBaseUpdateRequestDto {
        tenant_id: scope.tenant_id,
        knowledge_base_id,
        expected_version: body.expected_version,
        display_name: body.display_name,
        description: body.description,
        provider_id: body.provider_id,
        base_kind: body.base_kind,
        retrieval_modes: body.retrieval_modes,
        capability_ids: body.capability_ids,
        configuration_profile_id: body.configuration_profile_id,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject.clone())
    .map_err(ApiProblem::from_kernel_error)?;
    let (record, document_count) = with_service_mut(&state, move |service| {
        let record = service.update_knowledge_base(command)?;
        let document_count = knowledge_base_document_count(
            service,
            record.tenant_id,
            record.knowledge_base_id.as_str(),
            subject.clone(),
        )?;
        Ok((record, document_count))
    })
    .await?;
    Ok(Json(KnowledgeBaseResponse {
        data: map_knowledge_base_record(
            &AgentKnowledgeBaseRecordDto::from_record(&record),
            document_count,
        ),
    }))
}

async fn execute_delete_knowledge_base(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    expected_version: Option<String>,
    requested_at: String,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    validate_requested_at(requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = DeleteAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_base_id,
        expected_version,
        requested_by: scope.subject,
        requested_at,
    };
    let record = with_service_mut(&state, move |service| service.delete_knowledge_base(command)).await?;
    Ok(Json(KnowledgeBaseResponse {
        data: map_knowledge_base_record(&AgentKnowledgeBaseRecordDto::from_record(&record), 0),
    }))
}

async fn execute_restore_knowledge_base(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: RestoreAgentBody,
) -> Result<Json<KnowledgeBaseResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = body
        .expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = RestoreAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_base_id,
        expected_version,
        requested_by: subject.clone(),
        requested_at: body.requested_at,
    };
    let (record, document_count) = with_service_mut(&state, move |service| {
        let record = service.restore_knowledge_base(command)?;
        let document_count = knowledge_base_document_count(
            service,
            record.tenant_id,
            record.knowledge_base_id.as_str(),
            subject.clone(),
        )?;
        Ok((record, document_count))
    })
    .await?;
    Ok(Json(KnowledgeBaseResponse {
        data: map_knowledge_base_record(
            &AgentKnowledgeBaseRecordDto::from_record(&record),
            document_count,
        ),
    }))
}

async fn execute_list_knowledge_sources(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_base_id: String,
) -> Result<Json<KnowledgeSourceListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_sources(tenant_id, knowledge_base_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(record))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeSourceListResponse {
        data: KnowledgeSourceListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_source(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeSourceBody,
) -> Result<(StatusCode, Json<KnowledgeSourceResponse>), ApiProblem> {
    let sync_policy_json =
        json_value_to_string(body.sync_policy.unwrap_or_else(|| json!({})), "syncPolicy")?;
    let metadata_json =
        json_value_to_string(body.metadata.unwrap_or_else(|| json!({})), "metadata")?;
    let command = AgentKnowledgeSourceCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        knowledge_source_id: body.knowledge_source_id,
        knowledge_base_id,
        source_kind: body.source_kind,
        source_ref: body.source_ref,
        source_hash: body.source_hash,
        sync_policy_json,
        metadata_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_knowledge_source(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeSourceResponse {
            data: map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(
                &record,
            ))?,
        }),
    ))
}

async fn execute_get_knowledge_source(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_source_id: String,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_source_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_knowledge_source(command)).await?;
    Ok(Json(KnowledgeSourceResponse {
        data: map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(&record))?,
    }))
}

async fn execute_update_knowledge_source(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_source_id: String,
    body: KnowledgeSourceUpdateBody,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    let sync_policy_json = body
        .sync_policy
        .map(|value| json_value_to_string(value, "syncPolicy"))
        .transpose()?;
    let metadata_json = body
        .metadata
        .map(|value| json_value_to_string(value, "metadata"))
        .transpose()?;
    let command = AgentKnowledgeSourceUpdateRequestDto {
        tenant_id: scope.tenant_id,
        knowledge_source_id,
        expected_version: body.expected_version,
        source_kind: body.source_kind,
        source_ref: body.source_ref,
        source_hash: body.source_hash,
        sync_policy_json,
        metadata_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;
    let record =
        with_service_mut(&state, move |service| service.update_knowledge_source(command)).await?;
    Ok(Json(KnowledgeSourceResponse {
        data: map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(&record))?,
    }))
}

async fn execute_delete_knowledge_source(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_source_id: String,
    expected_version: Option<String>,
    requested_at: String,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    validate_requested_at(requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = DeleteAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_source_id,
        expected_version,
        requested_by: scope.subject,
        requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.delete_knowledge_source(command)).await?;
    Ok(Json(KnowledgeSourceResponse {
        data: map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(&record))?,
    }))
}

async fn execute_restore_knowledge_source(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_source_id: String,
    body: RestoreAgentBody,
) -> Result<Json<KnowledgeSourceResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = body
        .expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = RestoreAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_source_id,
        expected_version,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.restore_knowledge_source(command)).await?;
    Ok(Json(KnowledgeSourceResponse {
        data: map_knowledge_source_record(&AgentKnowledgeSourceRecordDto::from_record(&record))?,
    }))
}

async fn execute_list_knowledge_documents(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_base_id: String,
) -> Result<Json<KnowledgeDocumentListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_documents(tenant_id, knowledge_base_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(record))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeDocumentListResponse {
        data: KnowledgeDocumentListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_document(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeDocumentBody,
) -> Result<(StatusCode, Json<KnowledgeDocumentResponse>), ApiProblem> {
    let mut metadata_json =
        json_value_to_string(body.metadata.unwrap_or_else(|| json!({})), "metadata")?;
    if let Some(document_profile) = body.document_profile {
        metadata_json = document_profile
            .into_validated_dto()?
            .merge_into_metadata_json(Some(metadata_json))
            .map_err(ApiProblem::from_kernel_error)?;
    }
    let command = AgentKnowledgeDocumentCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        knowledge_document_id: body.knowledge_document_id,
        knowledge_base_id,
        knowledge_source_id: body.knowledge_source_id,
        document_kind: body.document_kind,
        title: body.title,
        content_ref: body.content_ref,
        content_hash: body.content_hash,
        summary: body.summary,
        metadata_json,
        tags: body.tags.unwrap_or_default(),
        categories: body.categories.unwrap_or_default(),
        trust_level: body.trust_level.unwrap_or(0),
        redaction_classification: body
            .redaction_classification
            .unwrap_or_else(|| "public".to_string()),
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_knowledge_document(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeDocumentResponse {
            data: map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(
                &record,
            ))?,
        }),
    ))
}

async fn execute_search_knowledge(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeSearchBody,
) -> Result<Json<KnowledgeSearchResponse>, ApiProblem> {
    let command = AgentKnowledgeSearchRequestDto {
        tenant_id: scope.tenant_id,
        knowledge_base_id,
        query: body.query,
        top_k: body.top_k.unwrap_or(10),
        retrieval_modes: body.retrieval_modes.unwrap_or_default(),
        include_external: body.include_external.unwrap_or(false),
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let results = with_service_mut(&state, move |service| service.search_knowledge(command)).await?;
    let items = results
        .iter()
        .map(|record| {
            map_knowledge_search_result(&AgentKnowledgeSearchResultDto::from_record(record))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(KnowledgeSearchResponse {
        data: KnowledgeSearchDataResponse { items },
    }))
}

async fn execute_get_knowledge_document(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_document_id: String,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_document_id,
        requested_by: scope.subject,
    };
    let record =
        with_service_mut(&state, move |service| service.get_knowledge_document(command)).await?;
    Ok(Json(KnowledgeDocumentResponse {
        data: map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(
            &record,
        ))?,
    }))
}

async fn execute_update_knowledge_document(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_document_id: String,
    body: KnowledgeDocumentUpdateBody,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    let mut metadata_json = body
        .metadata
        .map(|value| json_value_to_string(value, "metadata"))
        .transpose()?;
    if let Some(document_profile) = body.document_profile {
        let base_metadata_json = match metadata_json.take() {
            Some(metadata_json) => metadata_json,
            None => {
                let command = GetAgentMarketplaceItemCommand {
                    tenant_id: scope.tenant_id_u64()?,
                    item_id: knowledge_document_id.clone(),
                    requested_by: subject.clone(),
                };
                let current =
                    with_service_mut(&state, move |service| service.get_knowledge_document(command))
                        .await?;
                current.metadata_json
            }
        };
        metadata_json = Some(
            document_profile
                .into_validated_dto()?
                .merge_into_metadata_json(Some(base_metadata_json))
                .map_err(ApiProblem::from_kernel_error)?,
        );
    }
    let command = AgentKnowledgeDocumentUpdateRequestDto {
        tenant_id: scope.tenant_id,
        knowledge_document_id,
        expected_version: body.expected_version,
        knowledge_source_id: body.knowledge_source_id,
        document_kind: body.document_kind,
        title: body.title,
        content_ref: body.content_ref,
        content_hash: body.content_hash,
        summary: body.summary,
        metadata_json,
        tags: body.tags,
        categories: body.categories,
        trust_level: body.trust_level,
        redaction_classification: body.redaction_classification,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;
    let record =
        with_service_mut(&state, move |service| service.update_knowledge_document(command)).await?;
    Ok(Json(KnowledgeDocumentResponse {
        data: map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(
            &record,
        ))?,
    }))
}

async fn execute_delete_knowledge_document(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_document_id: String,
    expected_version: Option<String>,
    requested_at: String,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    validate_requested_at(requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = DeleteAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_document_id,
        expected_version,
        requested_by: scope.subject,
        requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.delete_knowledge_document(command)).await?;
    Ok(Json(KnowledgeDocumentResponse {
        data: map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(
            &record,
        ))?,
    }))
}

async fn execute_restore_knowledge_document(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_document_id: String,
    body: RestoreAgentBody,
) -> Result<Json<KnowledgeDocumentResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = body
        .expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = RestoreAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_document_id,
        expected_version,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record = with_service_mut(&state, move |service| {
        service.restore_knowledge_document(command)
    })
    .await?;
    Ok(Json(KnowledgeDocumentResponse {
        data: map_knowledge_document_record(&AgentKnowledgeDocumentRecordDto::from_record(
            &record,
        ))?,
    }))
}

async fn execute_list_knowledge_chunks(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_document_id: String,
) -> Result<Json<KnowledgeChunkListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_chunks(tenant_id, knowledge_document_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_chunk_record(&AgentKnowledgeChunkRecordDto::from_record(record))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeChunkListResponse {
        data: KnowledgeChunkListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_chunk(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_document_id: String,
    body: KnowledgeChunkBody,
) -> Result<(StatusCode, Json<KnowledgeChunkResponse>), ApiProblem> {
    let metadata_json =
        json_value_to_string(body.metadata.unwrap_or_else(|| json!({})), "metadata")?;
    let command = AgentKnowledgeChunkCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        knowledge_chunk_id: body.knowledge_chunk_id,
        knowledge_document_id,
        parent_chunk_id: body.parent_chunk_id,
        chunk_ordinal: body.chunk_ordinal,
        heading: body.heading,
        content_ref: body.content_ref,
        content_hash: body.content_hash,
        token_estimate: body.token_estimate,
        summary: body.summary,
        metadata_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_knowledge_chunk(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeChunkResponse {
            data: map_knowledge_chunk_record(&AgentKnowledgeChunkRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_get_knowledge_chunk(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_chunk_id: String,
) -> Result<Json<KnowledgeChunkResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_chunk_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_knowledge_chunk(command)).await?;
    Ok(Json(KnowledgeChunkResponse {
        data: map_knowledge_chunk_record(&AgentKnowledgeChunkRecordDto::from_record(&record))?,
    }))
}

async fn execute_list_knowledge_indexes(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_document_id: String,
) -> Result<Json<KnowledgeIndexListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_indexes(tenant_id, knowledge_document_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_index_record(&AgentKnowledgeIndexRecordDto::from_record(record))
        })
        .collect();
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeIndexListResponse {
        data: KnowledgeIndexListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_upsert_knowledge_index(
    state: AgentHttpState,
    scope: RequestScope,
    body: KnowledgeIndexBody,
) -> Result<(StatusCode, Json<KnowledgeIndexResponse>), ApiProblem> {
    let command = AgentKnowledgeIndexUpsertRequestDto {
        tenant_id: scope.tenant_id,
        knowledge_index_id: body.knowledge_index_id,
        knowledge_base_id: body.knowledge_base_id,
        knowledge_document_id: body.knowledge_document_id,
        knowledge_chunk_id: body.knowledge_chunk_id,
        index_kind: body.index_kind,
        index_provider_id: body.index_provider_id,
        external_ref: body.external_ref,
        embedding_model_id: body.embedding_model_id,
        vector_dimension: body.vector_dimension,
        content_hash: body.content_hash,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.upsert_knowledge_index(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeIndexResponse {
            data: map_knowledge_index_record(&AgentKnowledgeIndexRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_get_knowledge_index(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_index_id: String,
) -> Result<Json<KnowledgeIndexResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_index_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_knowledge_index(command)).await?;
    Ok(Json(KnowledgeIndexResponse {
        data: map_knowledge_index_record(&AgentKnowledgeIndexRecordDto::from_record(&record)),
    }))
}

async fn execute_list_knowledge_bindings(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_base_id: String,
) -> Result<Json<KnowledgeBindingListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_bindings(tenant_id, knowledge_base_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_binding_record(&AgentKnowledgeBindingRecordDto::from_record(record))
        })
        .collect();
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeBindingListResponse {
        data: KnowledgeBindingListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_binding(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeBindingBody,
) -> Result<(StatusCode, Json<KnowledgeBindingResponse>), ApiProblem> {
    let command = AgentKnowledgeBindingCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        knowledge_binding_id: body.knowledge_binding_id,
        knowledge_base_id,
        agent_id: body.agent_id,
        deployment_id: body.deployment_id,
        scope_kind: body.scope_kind,
        scope_ref: body.scope_ref,
        active: body.active.unwrap_or(true),
        default_binding: body.default_binding.unwrap_or(false),
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_knowledge_binding(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeBindingResponse {
            data: map_knowledge_binding_record(&AgentKnowledgeBindingRecordDto::from_record(
                &record,
            )),
        }),
    ))
}

async fn execute_get_knowledge_binding(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_binding_id: String,
) -> Result<Json<KnowledgeBindingResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: knowledge_binding_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_knowledge_binding(command)).await?;
    Ok(Json(KnowledgeBindingResponse {
        data: map_knowledge_binding_record(&AgentKnowledgeBindingRecordDto::from_record(&record)),
    }))
}

async fn execute_list_knowledge_sync_jobs(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    knowledge_base_id: String,
) -> Result<Json<KnowledgeSyncJobListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let records = with_service_mut(&state, move |service| {
        service.list_knowledge_sync_jobs(tenant_id, knowledge_base_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(record))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(KnowledgeSyncJobListResponse {
        data: KnowledgeSyncJobListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    knowledge_base_id: String,
    body: KnowledgeSyncJobBody,
) -> Result<(StatusCode, Json<KnowledgeSyncJobResponse>), ApiProblem> {
    let input_json = json_value_to_string(body.input.unwrap_or_else(|| json!({})), "input")?;
    let command = AgentKnowledgeSyncJobCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        sync_job_id: body.sync_job_id,
        knowledge_base_id,
        knowledge_source_id: body.knowledge_source_id,
        job_kind: body.job_kind,
        input_ref: body.input_ref,
        input_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_knowledge_sync_job(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(KnowledgeSyncJobResponse {
            data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(
                &record,
            ))?,
        }),
    ))
}

async fn execute_get_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    sync_job_id: String,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: sync_job_id,
        requested_by: scope.subject,
    };
    let record =
        with_service_mut(&state, move |service| service.get_knowledge_sync_job(command)).await?;
    Ok(Json(KnowledgeSyncJobResponse {
        data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(&record))?,
    }))
}

async fn execute_start_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    sync_job_id: String,
    body: KnowledgeSyncJobStartBody,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let command = AgentKnowledgeSyncJobStartCommand {
        tenant_id: scope.tenant_id_u64()?,
        sync_job_id,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.start_knowledge_sync_job(command)).await?;
    Ok(Json(KnowledgeSyncJobResponse {
        data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(&record))?,
    }))
}

async fn execute_complete_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    sync_job_id: String,
    body: KnowledgeSyncJobCompleteBody,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let output_json = json_value_to_string(body.output, "output")?;
    let command = AgentKnowledgeSyncJobCompleteCommand {
        tenant_id: scope.tenant_id_u64()?,
        sync_job_id,
        output_json,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record = with_service_mut(&state, move |service| {
        service.complete_knowledge_sync_job(command)
    })
    .await?;
    Ok(Json(KnowledgeSyncJobResponse {
        data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(&record))?,
    }))
}

async fn execute_fail_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    sync_job_id: String,
    body: KnowledgeSyncJobFailBody,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let error_json = json_value_to_string(body.error, "error")?;
    let command = AgentKnowledgeSyncJobFailCommand {
        tenant_id: scope.tenant_id_u64()?,
        sync_job_id,
        error_json,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.fail_knowledge_sync_job(command)).await?;
    Ok(Json(KnowledgeSyncJobResponse {
        data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(&record))?,
    }))
}

async fn execute_cancel_knowledge_sync_job(
    state: AgentHttpState,
    scope: RequestScope,
    sync_job_id: String,
    body: KnowledgeSyncJobCancelBody,
) -> Result<Json<KnowledgeSyncJobResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let cancellation_json = json_value_to_string(body.cancellation, "cancellation")?;
    let command = AgentKnowledgeSyncJobCancelCommand {
        tenant_id: scope.tenant_id_u64()?,
        sync_job_id,
        cancellation_json,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record =
        with_service_mut(&state, move |service| service.cancel_knowledge_sync_job(command)).await?;
    Ok(Json(KnowledgeSyncJobResponse {
        data: map_knowledge_sync_job_record(&AgentKnowledgeSyncJobRecordDto::from_record(&record))?,
    }))
}

async fn execute_create_memory_store(
    state: AgentHttpState,
    scope: RequestScope,
    body: MemoryStoreBody,
) -> Result<(StatusCode, Json<MemoryStoreResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryStoreCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        owner_user_id: scope.owner_user_id,
        memory_store_id: body.memory_store_id,
        code: body.code,
        display_name: body.display_name,
        description: body.description,
        provider_id: body.provider_id,
        store_kind: body.store_kind,
        retrieval_modes: body.retrieval_modes,
        capability_ids: body.capability_ids.unwrap_or_default(),
        configuration_profile_id: body.configuration_profile_id,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_memory_store(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryStoreResponse {
            data: map_memory_store_record(&AgentMemoryStoreRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_get_memory_store(
    state: AgentHttpState,
    scope: RequestScope,
    memory_store_id: String,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_store_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_memory_store(command)).await?;
    Ok(Json(MemoryStoreResponse {
        data: map_memory_store_record(&AgentMemoryStoreRecordDto::from_record(&record)),
    }))
}

async fn execute_update_memory_store(
    state: AgentHttpState,
    scope: RequestScope,
    memory_store_id: String,
    body: MemoryStoreUpdateBody,
) -> Result<Json<MemoryStoreResponse>, ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryStoreUpdateRequestDto {
        tenant_id: scope.tenant_id,
        memory_store_id,
        expected_version: body.expected_version,
        display_name: body.display_name,
        description: body.description,
        provider_id: body.provider_id,
        store_kind: body.store_kind,
        retrieval_modes: body.retrieval_modes,
        capability_ids: body.capability_ids,
        configuration_profile_id: body.configuration_profile_id,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.update_memory_store(command)).await?;
    Ok(Json(MemoryStoreResponse {
        data: map_memory_store_record(&AgentMemoryStoreRecordDto::from_record(&record)),
    }))
}

async fn execute_create_memory_profile(
    state: AgentHttpState,
    scope: RequestScope,
    memory_store_id: String,
    body: MemoryProfileBody,
) -> Result<(StatusCode, Json<MemoryProfileResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryProfileCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        owner_user_id: scope.owner_user_id,
        memory_profile_id: body.memory_profile_id,
        memory_store_id,
        code: body.code,
        display_name: body.display_name,
        description: body.description,
        write_policy_json: json_value_to_string(
            body.write_policy.unwrap_or_else(|| json!({})),
            "writePolicy",
        )?,
        retrieval_policy_json: json_value_to_string(
            body.retrieval_policy.unwrap_or_else(|| json!({})),
            "retrievalPolicy",
        )?,
        compaction_policy_json: json_value_to_string(
            body.compaction_policy.unwrap_or_else(|| json!({})),
            "compactionPolicy",
        )?,
        retention_policy_json: json_value_to_string(
            body.retention_policy.unwrap_or_else(|| json!({})),
            "retentionPolicy",
        )?,
        privacy_policy_json: json_value_to_string(
            body.privacy_policy.unwrap_or_else(|| json!({})),
            "privacyPolicy",
        )?,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_memory_profile(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryProfileResponse {
            data: map_memory_profile_record(&AgentMemoryProfileRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_get_memory_profile(
    state: AgentHttpState,
    scope: RequestScope,
    memory_profile_id: String,
) -> Result<Json<MemoryProfileResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_profile_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_memory_profile(command)).await?;
    Ok(Json(MemoryProfileResponse {
        data: map_memory_profile_record(&AgentMemoryProfileRecordDto::from_record(&record))?,
    }))
}

async fn execute_create_memory_binding(
    state: AgentHttpState,
    scope: RequestScope,
    memory_profile_id: String,
    body: MemoryBindingBody,
) -> Result<(StatusCode, Json<MemoryBindingResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryBindingCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        memory_binding_id: body.memory_binding_id,
        memory_profile_id,
        agent_id: body.agent_id,
        deployment_id: body.deployment_id,
        scope_kind: body.scope_kind,
        scope_ref: body.scope_ref,
        active: body.active.unwrap_or(true),
        default_binding: body.default_binding.unwrap_or(false),
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_memory_binding(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryBindingResponse {
            data: map_memory_binding_record(&AgentMemoryBindingRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_get_memory_binding(
    state: AgentHttpState,
    scope: RequestScope,
    memory_binding_id: String,
) -> Result<Json<MemoryBindingResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_binding_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_memory_binding(command)).await?;
    Ok(Json(MemoryBindingResponse {
        data: map_memory_binding_record(&AgentMemoryBindingRecordDto::from_record(&record)),
    }))
}

async fn execute_create_memory_namespace(
    state: AgentHttpState,
    scope: RequestScope,
    body: MemoryNamespaceBody,
) -> Result<(StatusCode, Json<MemoryNamespaceResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryNamespaceCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        memory_namespace_id: body.memory_namespace_id,
        agent_id: body.agent_id,
        user_ref: body.user_ref,
        session_ref: body.session_ref,
        thread_ref: body.thread_ref,
        namespace_kind: body.namespace_kind,
        visibility: body.visibility,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_memory_namespace(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryNamespaceResponse {
            data: map_memory_namespace_record(&AgentMemoryNamespaceRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_get_memory_namespace(
    state: AgentHttpState,
    scope: RequestScope,
    memory_namespace_id: String,
) -> Result<Json<MemoryNamespaceResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_namespace_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_memory_namespace(command)).await?;
    Ok(Json(MemoryNamespaceResponse {
        data: map_memory_namespace_record(&AgentMemoryNamespaceRecordDto::from_record(&record)),
    }))
}

async fn execute_list_memory_records(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    memory_namespace_id: String,
) -> Result<Json<MemoryRecordListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let subject = scope.subject;
    let records = with_service_mut(&state, move |service| {
        service.list_memory_records(tenant_id, memory_namespace_id.as_str(), subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| map_memory_record(&AgentMemoryRecordDto::from_record(record)))
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(MemoryRecordListResponse {
        data: MemoryRecordListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_memory_record(
    state: AgentHttpState,
    scope: RequestScope,
    memory_namespace_id: String,
    body: MemoryRecordBody,
) -> Result<(StatusCode, Json<MemoryRecordResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let content_json = json_value_to_string(body.content, "content")?;
    let command = AgentMemoryRecordCreateRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        memory_id: body.memory_id,
        memory_namespace_id,
        agent_id: body.agent_id,
        memory_kind: body.memory_kind,
        content_format: body.content_format,
        content_json,
        summary: body.summary,
        salience_score: body.salience_score,
        confidence_score: body.confidence_score,
        freshness_score: body.freshness_score,
        sensitivity_level: body.sensitivity_level,
        effective_at: body.effective_at,
        expires_at: body.expires_at,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_memory_record(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryRecordResponse {
            data: map_memory_record(&AgentMemoryRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_get_memory_record(
    state: AgentHttpState,
    scope: RequestScope,
    memory_id: String,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    let command = GetAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_id,
        requested_by: scope.subject,
    };
    let record = with_service_mut(&state, move |service| service.get_memory_record(command)).await?;
    Ok(Json(MemoryRecordResponse {
        data: map_memory_record(&AgentMemoryRecordDto::from_record(&record))?,
    }))
}

async fn execute_delete_memory_record(
    state: AgentHttpState,
    scope: RequestScope,
    memory_id: String,
    expected_version: Option<String>,
    requested_at: String,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    validate_requested_at(requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = DeleteAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_id,
        expected_version,
        requested_by: scope.subject,
        requested_at,
    };
    let record = with_service_mut(&state, move |service| service.delete_memory_record(command)).await?;
    Ok(Json(MemoryRecordResponse {
        data: map_memory_record(&AgentMemoryRecordDto::from_record(&record))?,
    }))
}

async fn execute_restore_memory_record(
    state: AgentHttpState,
    scope: RequestScope,
    memory_id: String,
    body: RestoreAgentBody,
) -> Result<Json<MemoryRecordResponse>, ApiProblem> {
    validate_requested_at(body.requested_at.as_str()).map_err(ApiProblem::from_kernel_error)?;
    let expected_version = body
        .expected_version
        .as_deref()
        .map(parse_expected_version)
        .transpose()
        .map_err(ApiProblem::from_kernel_error)?;
    let command = RestoreAgentMarketplaceItemCommand {
        tenant_id: scope.tenant_id_u64()?,
        item_id: memory_id,
        expected_version,
        requested_by: scope.subject,
        requested_at: body.requested_at,
    };
    let record = with_service_mut(&state, move |service| service.restore_memory_record(command)).await?;
    Ok(Json(MemoryRecordResponse {
        data: map_memory_record(&AgentMemoryRecordDto::from_record(&record))?,
    }))
}

async fn execute_list_memory_sources(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    memory_id: String,
) -> Result<Json<MemorySourceListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let subject = scope.subject;
    let records = with_service_mut(&state, move |service| {
        service.list_memory_sources(tenant_id, memory_id.as_str(), subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| map_memory_source_record(&AgentMemorySourceRecordDto::from_record(record)))
        .collect::<Result<Vec<_>, _>>()?;
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(MemorySourceListResponse {
        data: MemorySourceListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_memory_source(
    state: AgentHttpState,
    scope: RequestScope,
    memory_id: String,
    body: MemorySourceBody,
) -> Result<(StatusCode, Json<MemorySourceResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let evidence_json =
        json_value_to_string(body.evidence.unwrap_or_else(|| json!({})), "evidence")?;
    let command = AgentMemorySourceCreateRequestDto {
        tenant_id: scope.tenant_id,
        memory_source_id: body.memory_source_id,
        memory_id,
        source_kind: body.source_kind,
        source_ref: body.source_ref,
        source_hash: body.source_hash,
        evidence_json,
        captured_at: body.captured_at,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_memory_source(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemorySourceResponse {
            data: map_memory_source_record(&AgentMemorySourceRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_list_memory_relations(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    memory_id: String,
) -> Result<Json<MemoryRelationListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let subject = scope.subject;
    let records = with_service_mut(&state, move |service| {
        service.list_memory_relations(tenant_id, memory_id.as_str(), subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_memory_relation_record(&AgentMemoryRelationRecordDto::from_record(record))
        })
        .collect();
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(MemoryRelationListResponse {
        data: MemoryRelationListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_memory_relation(
    state: AgentHttpState,
    scope: RequestScope,
    memory_id: String,
    body: MemoryRelationBody,
) -> Result<(StatusCode, Json<MemoryRelationResponse>), ApiProblem> {
    if body.from_memory_id != memory_id {
        return Err(ApiProblem::validation(
            "fromMemoryId must match the memoryId path parameter",
        ));
    }
    let subject = scope.subject.clone();
    let command = AgentMemoryRelationCreateRequestDto {
        tenant_id: scope.tenant_id,
        memory_relation_id: body.memory_relation_id,
        from_memory_id: body.from_memory_id,
        to_memory_id: body.to_memory_id,
        relation_kind: body.relation_kind,
        weight: body.weight,
        valid_from: body.valid_from,
        valid_until: body.valid_until,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_memory_relation(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryRelationResponse {
            data: map_memory_relation_record(&AgentMemoryRelationRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_list_memory_retrieval_indexes(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    memory_id: String,
) -> Result<Json<MemoryRetrievalIndexListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;
    let subject = scope.subject;
    let records = with_service_mut(&state, move |service| {
        service.list_memory_retrieval_indexes(tenant_id, memory_id.as_str(), subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);
    let items = paged
        .iter()
        .map(|record| {
            map_memory_retrieval_index_record(&AgentMemoryRetrievalIndexRecordDto::from_record(
                record,
            ))
        })
        .collect();
    let total_pages = total_pages(total_items, page_size);

    Ok(Json(MemoryRetrievalIndexListResponse {
        data: MemoryRetrievalIndexListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_upsert_memory_retrieval_index(
    state: AgentHttpState,
    scope: RequestScope,
    body: MemoryRetrievalIndexBody,
) -> Result<(StatusCode, Json<MemoryRetrievalIndexResponse>), ApiProblem> {
    let subject = scope.subject.clone();
    let command = AgentMemoryRetrievalIndexUpsertRequestDto {
        tenant_id: scope.tenant_id,
        memory_index_id: body.memory_index_id,
        memory_id: body.memory_id,
        index_kind: body.index_kind,
        index_provider_id: body.index_provider_id,
        external_ref: body.external_ref,
        embedding_model_id: body.embedding_model_id,
        vector_dimension: body.vector_dimension,
        content_hash: body.content_hash,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| {
        service.upsert_memory_retrieval_index(command)
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryRetrievalIndexResponse {
            data: map_memory_retrieval_index_record(
                &AgentMemoryRetrievalIndexRecordDto::from_record(&record),
            ),
        }),
    ))
}

async fn execute_list(
    state: AgentHttpState,
    query: ListAgentsQueryParams,
    scope: RequestScope,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let include_deleted = query.include_deleted.unwrap_or(false);
    let request_dto = ListAgentsRequestDto {
        tenant_id: scope.tenant_id,
        organization_id: query.organization_id,
        owner_user_id: query.owner_user_id,
        include_deleted,
        search_query: query.q,
    };
    let command = request_dto
        .into_command(scope.subject)
        .map_err(ApiProblem::from_kernel_error)?;

    let mut records = with_service_mut(&state, move |service| service.list_agents(command)).await?;
    if matches!(
        query.scope.as_deref(),
        Some("market" | "public" | "published")
    ) {
        records.retain(|record| record.visibility.as_str() == "public");
    }
    let (page, page_size) = normalized_pagination(query.page, query.page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);

    let items: Vec<AgentRecordResponse> = paged
        .iter()
        .map(|record| map_agent_record(&AgentRecordDto::from_record(record)))
        .collect::<Result<Vec<_>, _>>()?;

    let total_pages = if total_items == 0 {
        0
    } else {
        total_items.div_ceil(page_size)
    };

    Ok(Json(AgentListResponse {
        data: AgentListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create(
    state: AgentHttpState,
    scope: RequestScope,
    body: CreateAgentBody,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let manifest = parse_manifest(body.manifest)?;
    let mut default_code_task_intent = body.default_code_task_intent.map(Into::into);
    if let Some(management_profile) = body.management_profile {
        default_code_task_intent = management_profile
            .into_validated_dto()?
            .merge_into_default_code_task_intent(default_code_task_intent)
            .map_err(ApiProblem::from_kernel_error)?;
    }

    let command = CreateAgentRequestDto {
        agent_id: body.agent_id,
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        owner_user_id: scope.owner_user_id,
        code: body.code,
        display_name: body.display_name,
        description: body.description,
        manifest,
        visibility: body.visibility,
        tags: body.tags.unwrap_or_default(),
        default_code_task_intent,
        implementation_provider_id: body.implementation_provider_id,
        implementation_kind: body.implementation_kind,
        implementation_type: body.implementation_type,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_agent(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(AgentResponse {
            data: map_agent_record(&AgentRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_get(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let command = GetAgentRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.get_agent(command)).await?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_update(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: UpdateAgentBody,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let mut default_code_task_intent = body.default_code_task_intent.map(Into::into);
    if let Some(management_profile) = body.management_profile {
        let base_intent = match default_code_task_intent.take() {
            Some(intent) => Some(intent),
            None => {
                let command = GetAgentRequestDto {
                    tenant_id: scope.tenant_id.clone(),
                    agent_id: agent_id.clone(),
                }
                .into_command(scope.subject.clone())
                .map_err(ApiProblem::from_kernel_error)?;
                let current =
                    with_service_mut(&state, move |service| service.get_agent(command)).await?;
                current.default_code_task_intent
            }
        };
        default_code_task_intent = management_profile
            .into_validated_dto()?
            .merge_into_default_code_task_intent(base_intent)
            .map_err(ApiProblem::from_kernel_error)?;
    }
    let command = UpdateAgentRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        expected_version: body.expected_version,
        display_name: body.display_name,
        description: body.description,
        manifest: body.manifest.map(parse_manifest).transpose()?,
        visibility: body.visibility,
        tags: body.tags,
        default_code_task_intent,
        implementation_provider_id: body.implementation_provider_id,
        implementation_kind: body.implementation_kind,
        implementation_type: body.implementation_type,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.update_agent(command)).await?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_delete(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: DeleteAgentBody,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let command = DeleteAgentRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        expected_version: body.expected_version,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.delete_agent(command)).await?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_restore(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: RestoreAgentBody,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let command = RestoreAgentRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        expected_version: body.expected_version,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.restore_agent(command)).await?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_list_provider_bindings(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    agent_id: String,
) -> Result<Json<AgentProviderBindingListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;

    let records = with_service_mut(&state, move |service| {
        service.list_provider_bindings(tenant_id, agent_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);

    let items = paged
        .iter()
        .map(|record| {
            map_provider_binding_record(&AgentProviderBindingRecordDto::from_record(record))
        })
        .collect();
    let total_pages = if total_items == 0 {
        0
    } else {
        total_items.div_ceil(page_size)
    };

    Ok(Json(AgentProviderBindingListResponse {
        data: AgentProviderBindingListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_add_provider_binding(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: AgentProviderBindingBody,
) -> Result<(StatusCode, Json<AgentProviderBindingResponse>), ApiProblem> {
    let command = AgentProviderBindingRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        binding_id: body.binding_id,
        provider_id: body.provider_id,
        implementation_kind: body.implementation_kind,
        configuration_profile_id: body.configuration_profile_id,
        capabilities: body.capabilities.unwrap_or_default(),
        make_default: body.make_default.unwrap_or(false),
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.add_provider_binding(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(AgentProviderBindingResponse {
            data: map_provider_binding_record(&AgentProviderBindingRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_activate_provider_binding(
    state: AgentHttpState,
    scope: RequestScope,
    path: TenantAgentBindingPathParams,
    body: ActivateProviderBindingBody,
) -> Result<Json<AgentProviderBindingResponse>, ApiProblem> {
    let command = ActivateAgentProviderBindingRequestDto {
        tenant_id: scope.tenant_id,
        agent_id: path.agent_id,
        binding_id: path.binding_id,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.activate_provider_binding(command)).await?;
    Ok(Json(AgentProviderBindingResponse {
        data: map_provider_binding_record(&AgentProviderBindingRecordDto::from_record(&record)),
    }))
}

async fn execute_list_deployments(
    state: AgentHttpState,
    scope: RequestScope,
    page: Option<usize>,
    page_size: Option<usize>,
    agent_id: String,
) -> Result<Json<AgentDeploymentListResponse>, ApiProblem> {
    let tenant_id = scope.tenant_id_u64()?;

    let records = with_service_mut(&state, move |service| {
        service.list_deployments(tenant_id, agent_id.as_str(), scope.subject)
    })
    .await?;
    let (page, page_size) = normalized_pagination(page, page_size)?;
    let total_items = records.len();
    let paged = paginate(records, page, page_size);

    let items = paged
        .iter()
        .map(|record| map_deployment_record(&AgentDeploymentRecordDto::from_record(record)))
        .collect();
    let total_pages = if total_items == 0 {
        0
    } else {
        total_items.div_ceil(page_size)
    };

    Ok(Json(AgentDeploymentListResponse {
        data: AgentDeploymentListDataResponse {
            items,
            page_info: PageInfoResponse {
                page,
                page_size,
                total_items: total_items.to_string(),
                total_pages,
            },
        },
    }))
}

async fn execute_create_deployment(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: AgentDeploymentBody,
) -> Result<(StatusCode, Json<AgentDeploymentResponse>), ApiProblem> {
    let command = AgentProviderDeploymentRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        deployment_id: body.deployment_id,
        binding_id: body.binding_id,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| service.create_deployment(command)).await?;
    Ok((
        StatusCode::CREATED,
        Json(AgentDeploymentResponse {
            data: map_deployment_record(&AgentDeploymentRecordDto::from_record(&record)),
        }),
    ))
}

async fn execute_create_preview_response(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: AgentPreviewResponseBody,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let input_payload_json = json_value_to_string(
        body.input_payload
            .unwrap_or_else(|| json!({ "content": body.content })),
        "inputPayload",
    )?;
    let command = AgentPreviewResponseRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        execution_id: body.execution_id,
        content: body.content,
        debug_mode: body.debug_mode.unwrap_or(false),
        memory_enabled: body.memory_enabled.unwrap_or(false),
        model: body.model,
        temperature: body.temperature,
        input_payload_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record =
        with_service_mut(&state, move |service| service.create_preview_response(command)).await?;
    Ok(Json(AgentRuntimeExecutionResponse {
        data: map_runtime_execution_record(&AgentRuntimeExecutionRecordDto::from_record(&record))?,
    }))
}

async fn execute_create_prompt_optimization(
    state: AgentHttpState,
    scope: RequestScope,
    agent_id: String,
    body: AgentPromptOptimizationBody,
) -> Result<Json<AgentRuntimeExecutionResponse>, ApiProblem> {
    let input_payload_json = json_value_to_string(
        body.input_payload
            .unwrap_or_else(|| json!({ "prompt": body.prompt })),
        "inputPayload",
    )?;
    let command = AgentPromptOptimizationRequestDto {
        tenant_id: scope.tenant_id,
        agent_id,
        execution_id: body.execution_id,
        prompt: body.prompt,
        input_payload_json,
        requested_at: body.requested_at,
    }
    .into_command(scope.subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, move |service| {
        service.create_prompt_optimization(command)
    })
    .await?;
    Ok(Json(AgentRuntimeExecutionResponse {
        data: map_runtime_execution_record(&AgentRuntimeExecutionRecordDto::from_record(&record))?,
    }))
}

fn parse_manifest(value: Value) -> Result<AgentManifest, ApiProblem> {
    let json_string = serde_json::to_string(&value)
        .map_err(|error| ApiProblem::validation(format!("manifest json encode failed: {error}")))?;
    AgentManifest::from_json(json_string.as_str()).map_err(ApiProblem::from_kernel_error)
}

fn map_agent_record(record: &AgentRecordDto) -> Result<AgentRecordResponse, ApiProblem> {
    let manifest_value = manifest_to_value(&record.manifest)?;
    let default_code_task_intent = record
        .default_code_task_intent
        .as_ref()
        .map(intent_to_value);

    Ok(AgentRecordResponse {
        id: record.id.clone(),
        agent_id: record.agent_id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        owner_user_id: record.owner_user_id.clone(),
        code: record.code.clone(),
        display_name: record.display_name.clone(),
        description: record.description.clone(),
        manifest: manifest_value,
        default_code_task_intent,
        management_profile: record
            .management_profile
            .as_ref()
            .map(map_agent_management_profile),
        implementation_provider_id: record.implementation_provider_id.clone(),
        implementation_kind: record.implementation_kind.clone(),
        implementation_type: record.implementation_type.clone(),
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        tags: record.tags.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    })
}

fn map_agent_management_profile(
    profile: &AgentManagementProfileDto,
) -> AgentManagementProfileResponse {
    AgentManagementProfileResponse {
        author: profile.author.clone(),
        avatar: profile.avatar.clone(),
        category_id: profile.category_id.clone(),
        color: profile.color.clone(),
        debug_mode: profile.debug_mode,
        icon_name: profile.icon_name.clone(),
        json_mode: profile.json_mode,
        knowledge_base_ids: profile.knowledge_base_ids.clone(),
        memory_enabled: profile.memory_enabled,
        model: profile.model.clone(),
        skill_ids: profile.skill_ids.clone(),
        suggested_prompts: profile.suggested_prompts.clone(),
        system_prompt: profile.system_prompt.clone(),
        temperature: profile.temperature,
        tool_ids: profile.tool_ids.clone(),
        agent_type: profile.agent_type.clone(),
        users: profile.users.clone(),
        voice_ids: profile.voice_ids.clone(),
        welcome_message: profile.welcome_message.clone(),
    }
}

fn map_provider_binding_record(
    record: &AgentProviderBindingRecordDto,
) -> AgentProviderBindingRecordResponse {
    AgentProviderBindingRecordResponse {
        tenant_id: record.tenant_id.clone(),
        agent_id: record.agent_id.clone(),
        binding_id: record.binding_id.clone(),
        provider_id: record.provider_id.clone(),
        implementation_kind: record.implementation_kind.clone(),
        configuration_profile_id: record.configuration_profile_id.clone(),
        capabilities: record.capabilities.clone(),
        active: record.active,
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

fn map_deployment_record(record: &AgentDeploymentRecordDto) -> AgentDeploymentRecordResponse {
    AgentDeploymentRecordResponse {
        tenant_id: record.tenant_id.clone(),
        agent_id: record.agent_id.clone(),
        deployment_id: record.deployment_id.clone(),
        binding_id: record.binding_id.clone(),
        provider_id_snapshot: record.provider_id_snapshot.clone(),
        implementation_kind_snapshot: record.implementation_kind_snapshot.clone(),
        configuration_profile_id_snapshot: record.configuration_profile_id_snapshot.clone(),
        capabilities_snapshot: record.capabilities_snapshot.clone(),
        status: record.status.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

fn map_runtime_execution_record(
    record: &AgentRuntimeExecutionRecordDto,
) -> Result<AgentRuntimeExecutionRecordResponse, ApiProblem> {
    Ok(AgentRuntimeExecutionRecordResponse {
        tenant_id: record.tenant_id.clone(),
        agent_id: record.agent_id.clone(),
        execution_id: record.execution_id.clone(),
        operation: record.operation.clone(),
        status: record.status.clone(),
        input_payload: json_string_to_value(record.input_payload_json.as_str(), "inputPayload")?,
        output_payload: json_string_to_value(record.output_payload_json.as_str(), "outputPayload")?,
        requested_at: record.requested_at.clone(),
        completed_at: record.completed_at.clone(),
    })
}

fn map_knowledge_base_record(
    record: &AgentKnowledgeBaseRecordDto,
    document_count: u32,
) -> KnowledgeBaseRecordResponse {
    KnowledgeBaseRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        owner_user_id: record.owner_user_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        code: record.code.clone(),
        display_name: record.display_name.clone(),
        description: record.description.clone(),
        provider_id: record.provider_id.clone(),
        base_kind: record.base_kind.clone(),
        retrieval_modes: record.retrieval_modes.clone(),
        capability_ids: record.capability_ids.clone(),
        configuration_profile_id: record.configuration_profile_id.clone(),
        document_count,
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    }
}

fn map_knowledge_source_record(
    record: &AgentKnowledgeSourceRecordDto,
) -> Result<KnowledgeSourceRecordResponse, ApiProblem> {
    Ok(KnowledgeSourceRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        knowledge_source_id: record.knowledge_source_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        source_kind: record.source_kind.clone(),
        source_ref: record.source_ref.clone(),
        source_hash: record.source_hash.clone(),
        sync_policy: json_string_to_value(record.sync_policy_json.as_str(), "syncPolicy")?,
        metadata: json_string_to_value(record.metadata_json.as_str(), "metadata")?,
        status: record.status.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    })
}

fn map_knowledge_document_record(
    record: &AgentKnowledgeDocumentRecordDto,
) -> Result<KnowledgeDocumentRecordResponse, ApiProblem> {
    Ok(KnowledgeDocumentRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        knowledge_document_id: record.knowledge_document_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        knowledge_source_id: record.knowledge_source_id.clone(),
        document_kind: record.document_kind.clone(),
        title: record.title.clone(),
        content_ref: record.content_ref.clone(),
        content_hash: record.content_hash.clone(),
        summary: record.summary.clone(),
        metadata: json_string_to_value(record.metadata_json.as_str(), "metadata")?,
        document_profile: record
            .document_profile
            .as_ref()
            .map(map_knowledge_document_profile),
        tags: record.tags.clone(),
        categories: record.categories.clone(),
        trust_level: record.trust_level,
        redaction_classification: record.redaction_classification.clone(),
        chunk_count: record.chunk_count,
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    })
}

fn map_knowledge_document_profile(
    profile: &AgentKnowledgeDocumentProfileDto,
) -> KnowledgeDocumentProfileResponse {
    KnowledgeDocumentProfileResponse {
        author: profile.author.clone(),
        content: profile.content.clone(),
        parent_id: profile.parent_id.clone(),
        document_type: profile.document_type.clone(),
        file_name: profile.file_name.clone(),
        file_size: profile.file_size.clone(),
        mime_type: profile.mime_type.clone(),
        drive_uri: profile.drive_uri.clone(),
    }
}

fn map_knowledge_chunk_record(
    record: &AgentKnowledgeChunkRecordDto,
) -> Result<KnowledgeChunkRecordResponse, ApiProblem> {
    Ok(KnowledgeChunkRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        knowledge_chunk_id: record.knowledge_chunk_id.clone(),
        knowledge_document_id: record.knowledge_document_id.clone(),
        parent_chunk_id: record.parent_chunk_id.clone(),
        chunk_ordinal: record.chunk_ordinal,
        heading: record.heading.clone(),
        content_ref: record.content_ref.clone(),
        content_hash: record.content_hash.clone(),
        token_estimate: record.token_estimate,
        summary: record.summary.clone(),
        metadata: json_string_to_value(record.metadata_json.as_str(), "metadata")?,
        status: record.status.clone(),
        created_at: record.created_at.clone(),
    })
}

fn map_knowledge_index_record(
    record: &AgentKnowledgeIndexRecordDto,
) -> KnowledgeIndexRecordResponse {
    KnowledgeIndexRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        knowledge_index_id: record.knowledge_index_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        knowledge_document_id: record.knowledge_document_id.clone(),
        knowledge_chunk_id: record.knowledge_chunk_id.clone(),
        index_kind: record.index_kind.clone(),
        index_provider_id: record.index_provider_id.clone(),
        external_ref: record.external_ref.clone(),
        embedding_model_id: record.embedding_model_id.clone(),
        vector_dimension: record.vector_dimension,
        content_hash: record.content_hash.clone(),
        indexed_at: record.indexed_at.clone(),
        status: record.status.clone(),
    }
}

fn map_knowledge_search_result(
    record: &AgentKnowledgeSearchResultDto,
) -> Result<KnowledgeSearchResultResponse, ApiProblem> {
    Ok(KnowledgeSearchResultResponse {
        tenant_id: record.tenant_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        provider_id: record.provider_id.clone(),
        knowledge_index_id: record.knowledge_index_id.clone(),
        index_provider_id: record.index_provider_id.clone(),
        retrieval_method: record.retrieval_method.clone(),
        knowledge_document_id: record.knowledge_document_id.clone(),
        document_kind: record.document_kind.clone(),
        knowledge_chunk_id: record.knowledge_chunk_id.clone(),
        title: record.title.clone(),
        snippet: record.snippet.clone(),
        score: record.score,
        source_ref: record.source_ref.clone(),
        content_ref: record.content_ref.clone(),
        external_ref: record.external_ref.clone(),
        trust_level: record.trust_level,
        redaction_classification: record.redaction_classification.clone(),
        metadata: json_string_to_value(record.metadata_json.as_str(), "metadata")?,
    })
}

fn map_knowledge_binding_record(
    record: &AgentKnowledgeBindingRecordDto,
) -> KnowledgeBindingRecordResponse {
    KnowledgeBindingRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        knowledge_binding_id: record.knowledge_binding_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        agent_id: record.agent_id.clone(),
        deployment_id: record.deployment_id.clone(),
        scope_kind: record.scope_kind.clone(),
        scope_ref: record.scope_ref.clone(),
        active: record.active,
        default_binding: record.default_binding,
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

fn map_knowledge_sync_job_record(
    record: &AgentKnowledgeSyncJobRecordDto,
) -> Result<KnowledgeSyncJobRecordResponse, ApiProblem> {
    Ok(KnowledgeSyncJobRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        sync_job_id: record.sync_job_id.clone(),
        knowledge_base_id: record.knowledge_base_id.clone(),
        knowledge_source_id: record.knowledge_source_id.clone(),
        job_kind: record.job_kind.clone(),
        status: record.status.clone(),
        input_ref: record.input_ref.clone(),
        input: json_string_to_value(record.input_json.as_str(), "input")?,
        output: record
            .output_json
            .as_deref()
            .map(|value| json_string_to_value(value, "output"))
            .transpose()?,
        error: record
            .error_json
            .as_deref()
            .map(|value| json_string_to_value(value, "error"))
            .transpose()?,
        requested_at: record.requested_at.clone(),
        started_at: record.started_at.clone(),
        completed_at: record.completed_at.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    })
}

fn map_memory_store_record(record: &AgentMemoryStoreRecordDto) -> MemoryStoreRecordResponse {
    MemoryStoreRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        owner_user_id: record.owner_user_id.clone(),
        memory_store_id: record.memory_store_id.clone(),
        code: record.code.clone(),
        display_name: record.display_name.clone(),
        description: record.description.clone(),
        provider_id: record.provider_id.clone(),
        store_kind: record.store_kind.clone(),
        retrieval_modes: record.retrieval_modes.clone(),
        capability_ids: record.capability_ids.clone(),
        configuration_profile_id: record.configuration_profile_id.clone(),
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    }
}

fn map_memory_profile_record(
    record: &AgentMemoryProfileRecordDto,
) -> Result<MemoryProfileRecordResponse, ApiProblem> {
    Ok(MemoryProfileRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        owner_user_id: record.owner_user_id.clone(),
        memory_profile_id: record.memory_profile_id.clone(),
        memory_store_id: record.memory_store_id.clone(),
        code: record.code.clone(),
        display_name: record.display_name.clone(),
        description: record.description.clone(),
        write_policy: json_string_to_value(record.write_policy_json.as_str(), "writePolicy")?,
        retrieval_policy: json_string_to_value(
            record.retrieval_policy_json.as_str(),
            "retrievalPolicy",
        )?,
        compaction_policy: json_string_to_value(
            record.compaction_policy_json.as_str(),
            "compactionPolicy",
        )?,
        retention_policy: json_string_to_value(
            record.retention_policy_json.as_str(),
            "retentionPolicy",
        )?,
        privacy_policy: json_string_to_value(record.privacy_policy_json.as_str(), "privacyPolicy")?,
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    })
}

fn map_memory_binding_record(record: &AgentMemoryBindingRecordDto) -> MemoryBindingRecordResponse {
    MemoryBindingRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        memory_binding_id: record.memory_binding_id.clone(),
        memory_profile_id: record.memory_profile_id.clone(),
        agent_id: record.agent_id.clone(),
        deployment_id: record.deployment_id.clone(),
        scope_kind: record.scope_kind.clone(),
        scope_ref: record.scope_ref.clone(),
        active: record.active,
        default_binding: record.default_binding,
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

fn map_memory_namespace_record(
    record: &AgentMemoryNamespaceRecordDto,
) -> MemoryNamespaceRecordResponse {
    MemoryNamespaceRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        memory_namespace_id: record.memory_namespace_id.clone(),
        agent_id: record.agent_id.clone(),
        user_ref: record.user_ref.clone(),
        session_ref: record.session_ref.clone(),
        thread_ref: record.thread_ref.clone(),
        namespace_kind: record.namespace_kind.clone(),
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    }
}

fn map_memory_record(
    record: &AgentMemoryRecordDto,
) -> Result<MemoryRecordRecordResponse, ApiProblem> {
    Ok(MemoryRecordRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        memory_id: record.memory_id.clone(),
        memory_namespace_id: record.memory_namespace_id.clone(),
        agent_id: record.agent_id.clone(),
        memory_kind: record.memory_kind.clone(),
        content_format: record.content_format.clone(),
        content: json_string_to_value(record.content_json.as_str(), "content")?,
        summary: record.summary.clone(),
        salience_score: record.salience_score,
        confidence_score: record.confidence_score,
        freshness_score: record.freshness_score,
        sensitivity_level: record.sensitivity_level,
        source_count: record.source_count,
        effective_at: record.effective_at.clone(),
        expires_at: record.expires_at.clone(),
        last_used_at: record.last_used_at.clone(),
        use_count: record.use_count.clone(),
        status: record.status.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
        redacted_at: record.redacted_at.clone(),
    })
}

fn map_memory_source_record(
    record: &AgentMemorySourceRecordDto,
) -> Result<MemorySourceRecordResponse, ApiProblem> {
    Ok(MemorySourceRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        memory_source_id: record.memory_source_id.clone(),
        memory_id: record.memory_id.clone(),
        source_kind: record.source_kind.clone(),
        source_ref: record.source_ref.clone(),
        source_hash: record.source_hash.clone(),
        evidence: json_string_to_value(record.evidence_json.as_str(), "evidence")?,
        captured_at: record.captured_at.clone(),
        created_at: record.created_at.clone(),
    })
}

fn map_memory_relation_record(
    record: &AgentMemoryRelationRecordDto,
) -> MemoryRelationRecordResponse {
    MemoryRelationRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        memory_relation_id: record.memory_relation_id.clone(),
        from_memory_id: record.from_memory_id.clone(),
        to_memory_id: record.to_memory_id.clone(),
        relation_kind: record.relation_kind.clone(),
        weight: record.weight,
        valid_from: record.valid_from.clone(),
        valid_until: record.valid_until.clone(),
        created_at: record.created_at.clone(),
    }
}

fn map_memory_retrieval_index_record(
    record: &AgentMemoryRetrievalIndexRecordDto,
) -> MemoryRetrievalIndexRecordResponse {
    MemoryRetrievalIndexRecordResponse {
        id: record.id.clone(),
        tenant_id: record.tenant_id.clone(),
        memory_index_id: record.memory_index_id.clone(),
        memory_id: record.memory_id.clone(),
        index_kind: record.index_kind.clone(),
        index_provider_id: record.index_provider_id.clone(),
        external_ref: record.external_ref.clone(),
        embedding_model_id: record.embedding_model_id.clone(),
        vector_dimension: record.vector_dimension,
        content_hash: record.content_hash.clone(),
        indexed_at: record.indexed_at.clone(),
        status: record.status.clone(),
    }
}

fn manifest_to_value(manifest: &AgentManifest) -> Result<Value, ApiProblem> {
    let value = json!({
        "schema_version": manifest.schema_version,
        "manifest_type": manifest.manifest_type,
        "agent_id": manifest.agent_id,
        "name": manifest.name,
        "display_name": manifest.display_name,
        "description": manifest.description,
        "version": manifest.version,
        "domain": manifest.domain,
        "required_capabilities": manifest.required_capabilities,
        "optional_capabilities": manifest.optional_capabilities,
        "event_families": manifest.event_families,
        "owner": {
            "name": manifest.owner_name,
        },
        "status": manifest.status,
    });
    serde_json::from_value(value)
        .map_err(|error| ApiProblem::internal(format!("manifest json decode failed: {error}")))
}

fn intent_to_value(intent: &CodeTaskIntent) -> Value {
    json!({
        "prompt": intent.prompt,
        "contextPaths": intent.context_paths,
        "constraints": intent.constraints,
    })
}

fn json_value_to_string(value: Value, field_name: &str) -> Result<String, ApiProblem> {
    serde_json::to_string(&value).map_err(|error| {
        ApiProblem::validation(format!("{field_name} json encode failed: {error}"))
    })
}

fn json_string_to_value(value: &str, field_name: &str) -> Result<Value, ApiProblem> {
    serde_json::from_str(value)
        .map_err(|error| ApiProblem::internal(format!("{field_name} json decode failed: {error}")))
}

fn kernel_event_severity(severity: sdkwork_agent_kernel::KernelEventSeverity) -> &'static str {
    match severity {
        sdkwork_agent_kernel::KernelEventSeverity::Debug => "debug",
        sdkwork_agent_kernel::KernelEventSeverity::Info => "info",
        sdkwork_agent_kernel::KernelEventSeverity::Warn => "warn",
        sdkwork_agent_kernel::KernelEventSeverity::Error => "error",
    }
}

fn filter_audit_events(
    events: Vec<sdkwork_agent_kernel::KernelEvent>,
    query: &AuditEventsQueryParams,
) -> Result<Vec<sdkwork_agent_kernel::KernelEvent>, ApiProblem> {
    if let Some(action) = query.action.as_ref() {
        if !ALLOWED_AUDIT_ACTIONS.contains(&action.as_str()) {
            return Err(ApiProblem::validation(format!(
                "action must be one of {}",
                ALLOWED_AUDIT_ACTIONS.join(", ")
            )));
        }
    }

    let from = parse_optional_query_datetime("from", query.from.as_deref())?;
    let to = parse_optional_query_datetime("to", query.to.as_deref())?;
    if let (Some(from_value), Some(to_value)) = (from.as_ref(), to.as_ref()) {
        if from_value > to_value {
            return Err(ApiProblem::validation(
                "from must be less than or equal to to",
            ));
        }
    }

    let mut filtered = Vec::new();
    for event in events {
        let action_ok = query
            .action
            .as_ref()
            .map(|action| action == audit_event_action(event.event_type.as_str()))
            .unwrap_or(true);
        if !action_ok {
            continue;
        }

        let occurred_at_raw = event
            .occurred_at
            .as_deref()
            .ok_or_else(|| ApiProblem::internal("audit event occurred_at is missing"))?;
        let occurred_at = parse_rfc3339_datetime(occurred_at_raw, "audit event occurred_at")
            .map_err(|error| ApiProblem::internal(error.safe_message()))?;

        let from_ok = from
            .as_ref()
            .map(|from_value| occurred_at >= *from_value)
            .unwrap_or(true);
        let to_ok = to
            .as_ref()
            .map(|to_value| occurred_at <= *to_value)
            .unwrap_or(true);
        if from_ok && to_ok {
            filtered.push(event);
        }
    }
    Ok(filtered)
}

fn audit_event_action(event_type: &str) -> &str {
    event_type.rsplit('.').next().unwrap_or(event_type)
}

fn parse_optional_query_datetime(
    field_name: &str,
    value: Option<&str>,
) -> Result<Option<OffsetDateTime>, ApiProblem> {
    parse_optional_rfc3339_datetime(value, field_name).map_err(ApiProblem::from_kernel_error)
}

fn reconcile_resource_tenant_with_subject_header(
    resource_tenant_id: &str,
    header_tenant_id: Option<String>,
) -> Result<String, ApiProblem> {
    let resource_tenant =
        parse_tenant_id(resource_tenant_id).map_err(ApiProblem::from_kernel_error)?;
    let Some(header_tenant_id) = header_tenant_id else {
        return Ok(resource_tenant_id.to_string());
    };
    let header_tenant = parse_tenant_id(header_tenant_id.as_str())
        .map_err(|_| ApiProblem::permission("subject tenant does not match resource tenant"))?;
    if header_tenant != resource_tenant {
        return Err(ApiProblem::permission(
            "subject tenant does not match resource tenant",
        ));
    }
    Ok(resource_tenant_id.to_string())
}

fn required_header_any(headers: &HeaderMap, keys: &[&str]) -> Result<String, ApiProblem> {
    optional_header_any(headers, keys).ok_or_else(|| {
        ApiProblem::validation(format!("required header missing: {}", keys.join(" or ")))
    })
}

fn optional_header_any(headers: &HeaderMap, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| optional_header(headers, key))
}

fn optional_header(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn normalized_pagination(
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<(usize, usize), ApiProblem> {
    let page = page.unwrap_or(1);
    if page == 0 {
        return Err(ApiProblem::validation(
            "page must be greater than or equal to 1",
        ));
    }

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page_size == 0 {
        return Err(ApiProblem::validation(
            "page_size must be greater than or equal to 1",
        ));
    }
    if page_size > MAX_PAGE_SIZE {
        return Err(ApiProblem::validation(format!(
            "page_size must be less than or equal to {MAX_PAGE_SIZE}"
        )));
    }

    Ok((page, page_size))
}

fn total_pages(total_items: usize, page_size: usize) -> usize {
    if total_items == 0 {
        0
    } else {
        total_items.div_ceil(page_size)
    }
}

fn paginate<T: Clone>(items: Vec<T>, page: usize, page_size: usize) -> Vec<T> {
    let start = (page - 1).saturating_mul(page_size);
    items.into_iter().skip(start).take(page_size).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{
        AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository,
    };
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::Extension;
    use tower::ServiceExt;

    fn test_manifest() -> Value {
        json!({
            "schema_version": "1.0.0",
            "manifest_type": "agent",
            "agent_id": "agent.alpha",
            "name": "sample-agent",
            "display_name": "Sample Agent",
            "description": "sample",
            "version": "0.1.0",
            "domain": "intelligence",
            "required_capabilities": ["model.chat"],
            "optional_capabilities": ["tool.invoke"],
            "event_families": ["agent.lifecycle"],
            "owner": { "name": "sdkwork" },
            "status": "active"
        })
    }

    fn auth_headers(mut request: Request<Body>) -> Request<Body> {
        let headers = request.headers_mut();
        headers.insert("x-subject-id", HeaderValue::from_static("u-1"));
        headers.insert("x-subject-tenant-id", HeaderValue::from_static("1"));
        request
    }

    fn test_agent_context() -> AgentRequestContext {
        AgentRequestContext::new("1", "100")
            .with_organization_id("10")
            .with_subject_id("u-1")
            .with_roles(["agent.write", "agent.read"])
    }

    #[tokio::test]
    async fn app_create_and_retrieve_agent_should_work() {
        let state = AgentHttpState::new(
            InMemoryAgentRepository::new(),
            InMemoryAgentAuditSink::default(),
            AllowAllPolicyProvider::allow("policy.memory"),
        );
        let app = build_combined_router(state).layer(Extension(test_agent_context()));

        let create_body = json!({
            "agentId": "agent.alpha",
            "code": "alpha",
            "displayName": "Alpha",
            "description": "first",
            "manifest": test_manifest(),
            "defaultCodeTaskIntent": {
                "prompt": "Refactor runtime",
                "contextPaths": ["src/lib.rs"],
                "constraints": ["safe"]
            },
            "visibility": "organization",
            "tags": ["starter"],
            "requestedAt": "2026-06-01T00:00:00Z"
        });

        let request = Request::builder()
            .method("POST")
            .uri("/app/v3/api/ai/agents")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(create_body.to_string()))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("create request should succeed");
        assert_eq!(response.status(), StatusCode::CREATED);

        let request = Request::builder()
            .method("GET")
            .uri("/app/v3/api/ai/agents/agent.alpha")
            .body(Body::empty())
            .expect("request should be built");
        let response = app
            .oneshot(auth_headers(request))
            .await
            .expect("get request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(body_json["data"]["agentId"], "agent.alpha");
    }

    #[tokio::test]
    async fn open_api_create_and_retrieve_agent_should_work() {
        let state = AgentHttpState::new(
            InMemoryAgentRepository::new(),
            InMemoryAgentAuditSink::default(),
            AllowAllPolicyProvider::allow("policy.memory"),
        );
        let app = build_combined_router(state);

        let create_body = json!({
            "agentId": "agent.open",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "open",
            "displayName": "Open Agent",
            "description": "developer api",
            "manifest": test_manifest(),
            "visibility": "organization",
            "tags": ["developer"],
            "requestedAt": "2026-06-01T00:00:00Z"
        });

        let request = Request::builder()
            .method("POST")
            .uri("/agent/v3/api/ai/agents?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(create_body.to_string()))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("create request should succeed");
        assert_eq!(response.status(), StatusCode::CREATED);

        let request = Request::builder()
            .method("GET")
            .uri("/agent/v3/api/ai/agents/agent.open?tenant_id=1")
            .body(Body::empty())
            .expect("request should be built");
        let response = app
            .oneshot(auth_headers(request))
            .await
            .expect("get request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn backend_status_update_should_work() {
        let state = AgentHttpState::new(
            InMemoryAgentRepository::new(),
            InMemoryAgentAuditSink::default(),
            AllowAllPolicyProvider::allow("policy.memory"),
        );
        let app = build_combined_router(state);

        let create_body = json!({
            "agentId": "agent.beta",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "beta",
            "displayName": "Beta",
            "description": null,
            "manifest": {
                "schema_version": "1.0.0",
                "manifest_type": "agent",
                "agent_id": "agent.beta",
                "name": "sample-agent",
                "display_name": "Sample Agent",
                "description": "sample",
                "version": "0.1.0",
                "domain": "intelligence",
                "required_capabilities": ["model.chat"],
                "optional_capabilities": ["tool.invoke"],
                "event_families": ["agent.lifecycle"],
                "owner": { "name": "sdkwork" },
                "status": "active"
            },
            "visibility": "private",
            "requestedAt": "2026-06-01T01:00:00Z"
        });

        let create_request = Request::builder()
            .method("POST")
            .uri("/backend/v3/api/ai/agents?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(create_body.to_string()))
            .expect("request should be built");
        let create_response = app
            .clone()
            .oneshot(auth_headers(create_request))
            .await
            .expect("create request should succeed");
        assert_eq!(create_response.status(), StatusCode::CREATED);

        let update_status_body = json!({
            "targetStatus": "active",
            "expectedVersion": "1",
            "requestedAt": "2026-06-01T01:05:00Z"
        });

        let status_request = Request::builder()
            .method("POST")
            .uri("/backend/v3/api/ai/agents/agent.beta/status?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(update_status_body.to_string()))
            .expect("request should be built");
        let status_response = app
            .oneshot(auth_headers(status_request))
            .await
            .expect("status request should succeed");

        assert_eq!(status_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(status_response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(body_json["data"]["status"], "active");
    }
}
