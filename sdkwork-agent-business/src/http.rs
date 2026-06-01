use crate::application::AgentBusinessService;
use crate::dto::{
    AgentRecordDto, CreateAgentRequestDto, DeleteAgentRequestDto, GetAgentRequestDto,
    ListAgentsRequestDto, RestoreAgentRequestDto, UpdateAgentRequestDto,
    UpdateAgentStatusRequestDto,
};
use crate::ports::{AgentAuditSink, AgentRepository};
use crate::validation::{
    parse_int64_string_field, parse_optional_rfc3339_datetime, parse_rfc3339_datetime,
};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelErrorKind, KernelResult, PolicyDecision, PolicyProvider,
    PolicyRequest, PolicySubject, ProviderHealth,
};
use sdkwork_code_kernel::CodeTaskIntent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

const HEADER_SUBJECT_ID: &str = "x-subject-id";
const HEADER_SUBJECT_TENANT_ID: &str = "x-subject-tenant-id";
const HEADER_SUBJECT_ROLES: &str = "x-subject-roles";
const MAX_PAGE_SIZE: usize = 200;
const DEFAULT_PAGE_SIZE: usize = 20;
const ALLOWED_AUDIT_ACTIONS: &[&str] = &[
    "created",
    "updated",
    "deleted",
    "restored",
    "status_changed",
];

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
    fn next_id(&mut self) -> u64 {
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

    fn list(&self, query: &crate::ports::AgentListQuery) -> Vec<crate::domain::AgentBusinessRecord> {
        self.0.list(query)
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
    service: Arc<Mutex<HttpService>>,
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
            service: Arc::new(Mutex::new(service)),
        }
    }
}

pub fn build_app_router() -> Router<AgentHttpState> {
    Router::new()
        .route("/app/v3/api/ai/agents", get(app_list_agents).post(app_create_agent))
        .route(
            "/app/v3/api/ai/agents/{agentId}",
            get(app_get_agent).patch(app_update_agent).delete(app_delete_agent),
        )
        .route(
            "/app/v3/api/ai/agents/{agentId}/restore",
            post(app_restore_agent),
        )
}

pub fn build_backend_router() -> Router<AgentHttpState> {
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
}

pub fn build_combined_router(state: AgentHttpState) -> Router {
    build_app_router()
        .merge(build_backend_router())
        .with_state(state)
}

#[derive(Debug, Clone, Deserialize)]
struct ListAgentsQueryParams {
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: Option<String>,
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
struct TenantAgentPathParams {
    #[serde(rename = "agentId")]
    agent_id: String,
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
#[serde(rename_all = "camelCase")]
struct CreateAgentBody {
    agent_id: String,
    organization_id: String,
    owner_user_id: String,
    code: String,
    display_name: String,
    description: Option<String>,
    manifest: Value,
    default_code_task_intent: Option<CodeTaskIntentBody>,
    visibility: String,
    tags: Option<Vec<String>>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAgentBody {
    display_name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    tags: Option<Vec<String>>,
    default_code_task_intent: Option<CodeTaskIntentBody>,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAgentStatusBody {
    target_status: String,
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAgentBody {
    requested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreAgentBody {
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemDetailResponse {
    r#type: String,
    title: String,
    status: u16,
    detail: String,
    code: String,
}

#[derive(Debug, Clone)]
struct ApiProblem {
    status: StatusCode,
    response: ProblemDetailResponse,
}

impl ApiProblem {
    fn validation(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "validation_error", detail)
    }

    fn permission(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "permission_required", detail)
    }

    fn conflict(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", detail)
    }

    fn not_found(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", detail)
    }

    fn internal(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", detail)
    }

    fn new(status: StatusCode, code: impl Into<String>, detail: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            status,
            response: ProblemDetailResponse {
                r#type: format!("https://sdkwork.dev/problems/{code}"),
                title: code.clone(),
                status: status.as_u16(),
                detail: detail.into(),
                code,
            },
        }
    }

    fn from_kernel_error(error: KernelError) -> Self {
        match error.kind() {
            KernelErrorKind::ValidationError => Self::validation(error.safe_message()),
            KernelErrorKind::Conflict => Self::conflict(error.safe_message()),
            KernelErrorKind::PermissionRequired | KernelErrorKind::PolicyDenied => {
                Self::permission(error.safe_message())
            }
            _ => {
                if error.safe_message().contains("not found") {
                    Self::not_found(error.safe_message())
                } else {
                    Self::internal(error.safe_message())
                }
            }
        }
    }

    fn from_json_rejection(rejection: JsonRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
            format!("invalid json request: {}", rejection.body_text()),
        )
    }

    fn from_query_rejection(rejection: QueryRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
            format!("invalid query request: {}", rejection.body_text()),
        )
    }

    fn from_path_rejection(rejection: PathRejection) -> Self {
        Self::new(
            rejection.status(),
            "validation_error",
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
    query: Result<Query<ListAgentsQueryParams>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list(state, query, headers).await
}

async fn backend_list_agents(
    State(state): State<AgentHttpState>,
    query: Result<Query<ListAgentsQueryParams>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_list(state, query, headers).await
}

async fn app_create_agent(
    State(state): State<AgentHttpState>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<CreateAgentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create(state, query, headers, body).await
}

async fn backend_create_agent(
    State(state): State<AgentHttpState>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<CreateAgentBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_create(state, query, headers, body).await
}

async fn app_get_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_get(state, query, agent_id, headers).await
}

async fn backend_get_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    execute_get(state, query, agent_id, headers).await
}

async fn app_update_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update(state, query, agent_id, headers, body).await
}

async fn backend_update_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_update(state, query, agent_id, headers, body).await
}

async fn app_delete_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<DeleteAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let command = DeleteAgentRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.delete_agent(command))?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn app_restore_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore(state, query, agent_id, headers, body).await
}

async fn backend_update_agent_status(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateAgentStatusBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let command = UpdateAgentStatusRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
        target_status: body.target_status,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.change_status(command))?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn backend_restore_agent(
    State(state): State<AgentHttpState>,
    agent_id: Result<Path<String>, PathRejection>,
    query: Result<Query<TenantQueryParams>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<RestoreAgentBody>, JsonRejection>,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let Path(agent_id) = agent_id.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let Json(body) = body.map_err(ApiProblem::from_json_rejection)?;
    execute_restore(state, query, agent_id, headers, body).await
}

async fn backend_list_agent_audit_events(
    State(state): State<AgentHttpState>,
    path: Result<Path<TenantAgentPathParams>, PathRejection>,
    query: Result<Query<AuditEventsQueryParams>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AgentAuditEventsListResponse>, ApiProblem> {
    let Path(path) = path.map_err(ApiProblem::from_path_rejection)?;
    let Query(query) = query.map_err(ApiProblem::from_query_rejection)?;
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let tenant_id =
        parse_int64_string_field(query.tenant_id.as_str(), "tenant_id").map_err(ApiProblem::from_kernel_error)?;
    let events = with_service_mut(&state, |service| {
        service.list_agent_audit_events(tenant_id, path.agent_id.as_str(), subject)
    })?;
    let events = filter_audit_events(events, &query)?;

    let (page, page_size) = normalized_pagination(query.page, query.page_size);
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

fn with_service_mut<T>(
    state: &AgentHttpState,
    action: impl FnOnce(&mut HttpService) -> KernelResult<T>,
) -> Result<T, ApiProblem> {
    let mut service = state
        .service
        .lock()
        .map_err(|_| ApiProblem::internal("agent business service mutex poisoned"))?;
    action(&mut service).map_err(ApiProblem::from_kernel_error)
}

async fn execute_list(
    state: AgentHttpState,
    query: ListAgentsQueryParams,
    headers: HeaderMap,
) -> Result<Json<AgentListResponse>, ApiProblem> {
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let include_deleted = query.include_deleted.unwrap_or(false);
    let request_dto = ListAgentsRequestDto {
        tenant_id: query.tenant_id,
        organization_id: query.organization_id,
        owner_user_id: query.owner_user_id,
        include_deleted,
        search_query: query.q,
    };
    let command = request_dto
        .into_command(subject)
        .map_err(ApiProblem::from_kernel_error)?;

    let records = with_service_mut(&state, |service| service.list_agents(command))?;
    let (page, page_size) = normalized_pagination(query.page, query.page_size);
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
    query: TenantQueryParams,
    headers: HeaderMap,
    body: CreateAgentBody,
) -> Result<(StatusCode, Json<AgentResponse>), ApiProblem> {
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let manifest = parse_manifest(body.manifest)?;

    let command = CreateAgentRequestDto {
        agent_id: body.agent_id,
        tenant_id: query.tenant_id,
        organization_id: body.organization_id,
        owner_user_id: body.owner_user_id,
        code: body.code,
        display_name: body.display_name,
        description: body.description,
        manifest,
        visibility: body.visibility,
        tags: body.tags.unwrap_or_default(),
        default_code_task_intent: body.default_code_task_intent.map(Into::into),
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.create_agent(command))?;
    Ok((
        StatusCode::CREATED,
        Json(AgentResponse {
            data: map_agent_record(&AgentRecordDto::from_record(&record))?,
        }),
    ))
}

async fn execute_get(
    state: AgentHttpState,
    query: TenantQueryParams,
    agent_id: String,
    headers: HeaderMap,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let command = GetAgentRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.get_agent(command))?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_update(
    state: AgentHttpState,
    query: TenantQueryParams,
    agent_id: String,
    headers: HeaderMap,
    body: UpdateAgentBody,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let command = UpdateAgentRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
        display_name: body.display_name,
        description: body.description,
        visibility: body.visibility,
        tags: body.tags,
        default_code_task_intent: body.default_code_task_intent.map(Into::into),
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.update_agent(command))?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
    }))
}

async fn execute_restore(
    state: AgentHttpState,
    query: TenantQueryParams,
    agent_id: String,
    headers: HeaderMap,
    body: RestoreAgentBody,
) -> Result<Json<AgentResponse>, ApiProblem> {
    let subject = extract_policy_subject(headers, query.tenant_id.as_str())?;
    let command = RestoreAgentRequestDto {
        tenant_id: query.tenant_id,
        agent_id,
        requested_at: body.requested_at,
    }
    .into_command(subject)
    .map_err(ApiProblem::from_kernel_error)?;

    let record = with_service_mut(&state, |service| service.restore_agent(command))?;
    Ok(Json(AgentResponse {
        data: map_agent_record(&AgentRecordDto::from_record(&record))?,
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
        status: record.status.clone(),
        visibility: record.visibility.clone(),
        tags: record.tags.clone(),
        version: record.version.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    })
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
            return Err(ApiProblem::validation("from must be less than or equal to to"));
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

fn extract_policy_subject(headers: HeaderMap, tenant_id: &str) -> Result<PolicySubject, ApiProblem> {
    let subject_id = required_header(&headers, HEADER_SUBJECT_ID)?;
    let subject_tenant_id = optional_header(&headers, HEADER_SUBJECT_TENANT_ID)
        .unwrap_or_else(|| tenant_id.to_string());
    let mut subject = PolicySubject::new(subject_id, subject_tenant_id);
    if let Some(roles) = optional_header(&headers, HEADER_SUBJECT_ROLES) {
        for role in roles.split(',').map(str::trim).filter(|role| !role.is_empty()) {
            subject = subject.with_role(role.to_string());
        }
    }
    Ok(subject)
}

fn required_header(headers: &HeaderMap, key: &str) -> Result<String, ApiProblem> {
    optional_header(headers, key)
        .ok_or_else(|| ApiProblem::validation(format!("required header missing: {key}")))
}

fn optional_header(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn normalized_pagination(page: Option<usize>, page_size: Option<usize>) -> (usize, usize) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    (page, page_size)
}

fn paginate<T: Clone>(items: Vec<T>, page: usize, page_size: usize) -> Vec<T> {
    let start = (page - 1).saturating_mul(page_size);
    items.into_iter().skip(start).take(page_size).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
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
        headers.insert("x-subject-tenant-id", HeaderValue::from_static("t-1"));
        request
    }

    #[tokio::test]
    async fn app_create_and_retrieve_agent_should_work() {
        let state = AgentHttpState::new(
            InMemoryAgentRepository::new(),
            InMemoryAgentAuditSink::default(),
            AllowAllPolicyProvider::allow("policy.memory"),
        );
        let app = build_combined_router(state);

        let create_body = json!({
            "agentId": "agent.alpha",
            "organizationId": "10",
            "ownerUserId": "100",
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
            .uri("/app/v3/api/ai/agents?tenant_id=1")
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
            .uri("/app/v3/api/ai/agents/agent.alpha?tenant_id=1")
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
