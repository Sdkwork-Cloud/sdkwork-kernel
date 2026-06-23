use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::{self, Stream, StreamExt};
use sdkwork_agent_api_bridge::BridgeSessionConfig;
use sdkwork_agent_database::{EventRow, MessageRow, SessionRow, TaskRow};
use sdkwork_agent_kernel::ModelRequest;
use sdkwork_agent_session::{SessionConfig, SessionQuery};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

type SessionEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

use crate::access::{assert_permission_access, assert_session_access, stamp_session_ownership, AccessPolicy};
use crate::agent_registry::{apply_hosted_agent_defaults, validate_hosted_agent_id};
use crate::config::ServerConfig;
use crate::metrics::MetricsRegistry;
use crate::middleware::RequestContext;
use crate::persistence::PersistenceState;
use crate::runtime::RuntimeState;
use crate::tenant_token_quota::TenantTokenQuotaState;

/// Shared internal-api runtime HTTP handler state.
#[derive(Clone)]
pub struct InternalRuntimeApiState {
    pub persistence: Arc<PersistenceState>,
    pub config: Arc<ServerConfig>,
    pub access_policy: AccessPolicy,
    pub runtime: RuntimeState,
    pub tenant_token_quota: Arc<TenantTokenQuotaState>,
    pub sse_event_counter: Arc<tokio::sync::Mutex<u64>>,
    permissions: Arc<Mutex<HashMap<String, PermissionRecord>>>,
}

#[derive(Debug, Clone)]
struct PermissionRecord {
    view: PermissionRequestJson,
    owner_tenant_id: Option<String>,
    owner_user_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestJson {
    pub permission_request_id: String,
    pub category: String,
    pub resource: String,
    pub side_effect_level: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDecisionBody {
    pub decision: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelUiSnapshotJson {
    pub runtime: KernelRuntimeJson,
    pub events: Vec<KernelEventJson>,
    pub permissions: Vec<PermissionRequestJson>,
    pub workspace: WorkspaceJson,
    pub patches: Vec<serde_json::Value>,
    pub verification_reports: Vec<serde_json::Value>,
    pub terminal_commands: Vec<serde_json::Value>,
    pub terminal_output: Vec<serde_json::Value>,
    pub review_findings: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeJson {
    pub runtime_id: String,
    pub agent_id: String,
    pub kernel_version: String,
    pub state: String,
    pub health: String,
    pub capabilities: Vec<CapabilityJson>,
    pub missing_required_capabilities: Vec<String>,
    pub degraded_capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityJson {
    pub capability_id: String,
    pub provider_id: String,
    pub status: String,
    pub required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelEventJson {
    pub event_id: String,
    pub event_type: String,
    pub severity: String,
    pub summary: String,
    pub sequence: u32,
    pub trace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceJson {
    pub workspace_id: String,
    pub root: String,
    pub branch: String,
    pub dirty: bool,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKernelSessionRequest {
    pub agent_id: String,
    pub tenant_id: Option<String>,
    pub user_ref: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub title: Option<String>,
    pub goal: Option<String>,
    pub instructions: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Option<Vec<String>>,
    pub source: Option<String>,
    pub kind: Option<String>,
    pub timeout_ms: Option<u64>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionViewJson {
    pub session_id: String,
    pub source: String,
    pub kind: String,
    pub agent_id: String,
    pub user_ref: Option<String>,
    pub tenant_id: Option<String>,
    pub title: Option<String>,
    pub goal: Option<String>,
    pub state: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Vec<String>,
    pub instructions: Option<String>,
    pub token_usage: TokenUsageJson,
    pub message_count: u32,
    pub tool_call_count: u32,
    pub compression_count: u32,
    pub change_summary: ChangeSummaryJson,
    pub child_session_ids: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageJson {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSummaryJson {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

#[derive(Debug, Deserialize)]
pub struct SendKernelMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageViewJson {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub parts: Vec<MessagePartJson>,
    pub created_at: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePartJson {
    pub part_id: String,
    pub kind: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTaskRequest {
    pub instruction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskViewJson {
    pub task_id: String,
    pub session_id: String,
    pub instruction: String,
    pub state: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptorJson {
    pub model_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub family: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponseJson {
    pub items: Vec<SessionViewJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageListResponseJson {
    pub items: Vec<MessageViewJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponseJson {
    pub items: Vec<TaskViewJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResponseJson {
    pub items: Vec<ModelDescriptorJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListResponseJson {
    pub items: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeModelRequest {
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelResponseJson {
    pub model_request_id: String,
    pub provider_id: String,
    pub status: String,
    pub messages: Vec<String>,
    pub tool_calls: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteToolRequest {
    pub input: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallJson {
    pub tool_call_id: String,
    pub tool_id: String,
    pub input: String,
    pub status: String,
    pub output: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEventJson {
    pub event_id: String,
    pub event_type: String,
    pub sequence: u32,
    pub payload: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEventsQuery {
    pub last_event_id: Option<String>,
    pub live: Option<bool>,
}

impl InternalRuntimeApiState {
    pub fn new(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
    ) -> Result<Self, sdkwork_agent_kernel::KernelError> {
        Ok(Self {
            persistence,
            config: config.clone(),
            access_policy: AccessPolicy::from_config(&config),
            runtime: RuntimeState::try_for_config(&config)?,
            tenant_token_quota: Arc::new(TenantTokenQuotaState::from_config(&config)),
            sse_event_counter: Arc::new(tokio::sync::Mutex::new(0)),
            permissions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub(crate) async fn persist<F, T>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce(&PersistenceState) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        self.persistence.run(operation).await
    }

    async fn build_snapshot(&self) -> Result<KernelUiSnapshotJson, String> {
        let db_healthy = matches!(
            self.persist(|persistence| persistence.health()).await,
            Ok(true)
        );
        let diagnostics = self.runtime.agent_runtime().diagnostics();
        let manifest = self.runtime.agent_runtime().capability_manifest();
        let allow_mock = self.runtime.allow_mock_fallback();

        let runtime_state = diagnostics.state.clone();
        let runtime_health = if db_healthy && (runtime_state == "ready" || allow_mock) {
            "healthy"
        } else {
            "degraded"
        };

        let mut degraded_capabilities = manifest.degraded_capabilities.clone();
        if !db_healthy {
            degraded_capabilities.push("persistence.sqlite".to_string());
        }

        let capabilities = manifest
            .capabilities
            .iter()
            .map(|capability| CapabilityJson {
                capability_id: capability.capability_id.clone(),
                provider_id: capability.provider_id.clone(),
                status: capability.status.clone(),
                required: capability.required,
            })
            .collect();

        let permissions = self
            .permissions
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?
            .values()
            .map(|record| record.view.clone())
            .collect();

        Ok(KernelUiSnapshotJson {
            runtime: KernelRuntimeJson {
                runtime_id: diagnostics.runtime_id.clone(),
                agent_id: diagnostics.agent_id.clone(),
                kernel_version: env!("CARGO_PKG_VERSION").to_string(),
                state: runtime_state,
                health: runtime_health.to_string(),
                capabilities,
                missing_required_capabilities: manifest
                    .missing_required_capabilities
                    .clone(),
                degraded_capabilities,
            },
            events: Vec::new(),
            permissions,
            workspace: WorkspaceJson {
                workspace_id: "workspace.local".to_string(),
                root: ".".to_string(),
                branch: "main".to_string(),
                dirty: false,
                changed_files: Vec::new(),
            },
            patches: Vec::new(),
            verification_reports: Vec::new(),
            terminal_commands: Vec::new(),
            terminal_output: Vec::new(),
            review_findings: Vec::new(),
        })
    }
}

fn session_row_to_view(row: SessionRow) -> SessionViewJson {
    let metadata = parse_metadata_map(row.metadata_json.as_deref());
    SessionViewJson {
        session_id: row.session_id,
        source: row.source,
        kind: row.kind,
        agent_id: row.agent_id,
        user_ref: metadata.get("userRef").cloned(),
        tenant_id: metadata.get("tenantId").cloned(),
        title: row.title,
        goal: metadata.get("goal").cloned(),
        state: row.state,
        created_at: Some(row.created_at),
        updated_at: row.updated_at,
        model: row.model,
        model_provider: metadata.get("modelProvider").cloned(),
        cwd: row.cwd,
        workspace_roots: metadata
            .get("workspaceRoots")
            .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
            .unwrap_or_default(),
        instructions: metadata.get("instructions").cloned(),
        token_usage: TokenUsageJson {
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: 0,
        },
        message_count: row.message_count.max(0) as u32,
        tool_call_count: 0,
        compression_count: 0,
        change_summary: ChangeSummaryJson {
            additions: 0,
            deletions: 0,
            files_changed: 0,
        },
        child_session_ids: Vec::new(),
        metadata,
    }
}

fn message_row_to_view(row: MessageRow) -> MessageViewJson {
    MessageViewJson {
        message_id: row.message_id.clone(),
        session_id: row.session_id,
        role: row.role,
        parts: vec![MessagePartJson {
            part_id: format!("{}/part.0", row.message_id),
            kind: "text".to_string(),
            content: row.content,
        }],
        created_at: Some(row.created_at),
        metadata: parse_metadata_map(row.metadata_json.as_deref()),
    }
}

fn task_row_to_view(row: TaskRow) -> TaskViewJson {
    TaskViewJson {
        task_id: row.task_id,
        session_id: row.session_id,
        instruction: row.instruction,
        state: row.state,
        created_at: Some(row.created_at),
        updated_at: row.updated_at,
    }
}

fn event_row_to_sse(row: &EventRow, sequence: u32) -> Event {
    let payload = event_row_to_stream(row, sequence);
    Event::default()
        .id(row.event_id.clone())
        .data(serde_json::to_string(&payload).unwrap_or_default())
}

fn live_event_stream(
    receiver: broadcast::Receiver<EventRow>,
    session_id: String,
    sequence: u32,
) -> impl Stream<Item = Result<Event, Infallible>> {
    futures::stream::unfold(
        (receiver, session_id, sequence),
        |(mut receiver, session_id, mut sequence)| async move {
            loop {
                match receiver.recv().await {
                    Ok(row) if row.session_id.as_deref() == Some(session_id.as_str()) => {
                        let event = event_row_to_sse(&row, sequence);
                        sequence += 1;
                        return Some((Ok(event), (receiver, session_id, sequence)));
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    )
}
fn event_row_to_stream(row: &EventRow, sequence: u32) -> StreamEventJson {
    StreamEventJson {
        event_id: row.event_id.clone(),
        event_type: row.event_type.clone(),
        sequence,
        payload: row.payload.clone().unwrap_or_default(),
        timestamp: Some(row.created_at.clone()),
    }
}

fn events_after_cursor(events: Vec<EventRow>, last_event_id: Option<String>) -> Vec<EventRow> {
    let Some(last_event_id) = last_event_id.filter(|event_id| !event_id.is_empty()) else {
        return events;
    };

    match events
        .iter()
        .position(|row| row.event_id == last_event_id)
    {
        Some(index) => events.into_iter().skip(index + 1).collect(),
        None => events,
    }
}

fn last_event_id_from_request(headers: &HeaderMap, query: &StreamEventsQuery) -> Option<String> {
    headers
        .get("Last-Event-ID")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            query
                .last_event_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn parse_metadata_map(raw: Option<&str>) -> HashMap<String, String> {
    raw.and_then(|value| serde_json::from_str::<HashMap<String, String>>(value).ok())
        .unwrap_or_default()
}

fn apply_create_session_metadata(
    metadata: &mut HashMap<String, String>,
    request: &CreateKernelSessionRequest,
) {
    if let Some(tenant_id) = &request.tenant_id {
        metadata.insert("tenantId".to_string(), tenant_id.clone());
    }
    if let Some(user_ref) = &request.user_ref {
        metadata.insert("userRef".to_string(), user_ref.clone());
    }
    if let Some(goal) = &request.goal {
        metadata.insert("goal".to_string(), goal.clone());
    }
    if let Some(model_provider) = &request.model_provider {
        metadata.insert("modelProvider".to_string(), model_provider.clone());
    }
    if let Some(instructions) = &request.instructions {
        metadata.insert("instructions".to_string(), instructions.clone());
    }
    if let Some(roots) = &request.workspace_roots {
        if let Ok(encoded) = serde_json::to_string(roots) {
            metadata.insert("workspaceRoots".to_string(), encoded);
        }
    }
}

pub fn ensure_session_access(
    state: &InternalRuntimeApiState,
    ctx: &RequestContext,
    row: &SessionRow,
) -> Result<(), StatusCode> {
    assert_session_access(state.access_policy, ctx, row)
}

pub fn bridge_config_from_row(row: &SessionRow) -> BridgeSessionConfig {
    let metadata = parse_metadata_map(row.metadata_json.as_deref());
    BridgeSessionConfig {
        agent_id: row.agent_id.clone(),
        tenant_id: metadata
            .get("tenantId")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        user_ref: metadata.get("userRef").cloned(),
        model: row.model.clone(),
        instructions: metadata.get("instructions").cloned(),
        cwd: row.cwd.clone(),
        metadata: metadata.into_iter().collect(),
    }
}

pub fn map_runtime_error(error: sdkwork_agent_kernel::KernelError) -> StatusCode {
    match error {
        sdkwork_agent_kernel::KernelError::Validation { .. } => StatusCode::BAD_REQUEST,
        sdkwork_agent_kernel::KernelError::CapabilityMissing { .. }
        | sdkwork_agent_kernel::KernelError::ProviderUnavailable { .. } => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        sdkwork_agent_kernel::KernelError::PolicyDenied { .. } => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn load_snapshot(
    State(state): State<Arc<InternalRuntimeApiState>>,
) -> Result<Json<KernelUiSnapshotJson>, StatusCode> {
    state
        .build_snapshot()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn decide_permission(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(permission_request_id): Path<String>,
    Json(body): Json<PermissionDecisionBody>,
) -> Result<Json<PermissionRequestJson>, StatusCode> {
    if state.access_policy.enforce_session_scope {
        if ctx.tenant_id.as_deref().is_none_or(str::is_empty)
            || ctx
                .user_id
                .as_deref()
                .or_else(|| ctx.subject_id.as_deref())
                .is_none_or(str::is_empty)
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    if !matches!(body.decision.as_str(), "allow" | "deny") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut permissions = state
        .permissions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let record = permissions
        .get(&permission_request_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    assert_permission_access(
        state.access_policy,
        &ctx,
        record.owner_tenant_id.as_deref(),
        record.owner_user_ref.as_deref(),
        &permission_request_id,
    )?;
    let record = permissions
        .get_mut(&permission_request_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    record.view.status = body.decision;
    Ok(Json(record.view.clone()))
}

pub async fn create_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(request): Json<CreateKernelSessionRequest>,
) -> Result<(StatusCode, Json<SessionViewJson>), StatusCode> {
    let registered = validate_hosted_agent_id(&request.agent_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut metadata_map = request.metadata.clone().unwrap_or_default();
    apply_create_session_metadata(&mut metadata_map, &request);
    apply_hosted_agent_defaults(&mut metadata_map, registered);
    stamp_session_ownership(&mut metadata_map, &ctx, &state.config)?;

    let mut config = SessionConfig::new(request.agent_id);
    if let Some(title) = request.title {
        config = config.with_title(title);
    }
    if let Some(model) = request.model {
        config = config.with_model(model);
    }
    if let Some(source) = request.source {
        config = config.with_source(source);
    }
    if let Some(kind) = request.kind {
        config = config.with_kind(kind);
    }
    if let Some(cwd) = request.cwd {
        config = config.with_cwd(cwd);
    }
    if let Some(instructions) = request.instructions {
        config = config.with_instructions(instructions);
    }
    config.metadata = if metadata_map.is_empty() {
        None
    } else {
        serde_json::to_value(metadata_map).ok()
    };

    let row = state
        .persist(move |persistence| persistence.create_session(config))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state
        .runtime
        .register_session(&row.session_id, bridge_config_from_row(&row));

    Ok((StatusCode::CREATED, Json(session_row_to_view(row))))
}

pub async fn get_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionViewJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    Ok(Json(session_row_to_view(row)))
}

pub async fn list_sessions(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Json<SessionListResponseJson>, StatusCode> {
    let query = SessionQuery {
        limit: Some(100),
        ..SessionQuery::default()
    };
    let rows = state
        .persist(move |persistence| persistence.list_sessions(query))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut views = Vec::new();
    for row in rows {
        if ensure_session_access(&state, &ctx, &row).is_ok() {
            views.push(session_row_to_view(row));
        }
    }
    Ok(Json(SessionListResponseJson { items: views }))
}

pub async fn close_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionViewJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    state
        .persist(move |persistence| persistence.close_session(&session_id))
        .await
        .map(session_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn delete_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> StatusCode {
    let session_key = session_id.clone();
    let row = match state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
    {
        Ok(row) => row,
        Err(error) => return map_persistence_error(error),
    };
    if ensure_session_access(&state, &ctx, &row).is_err() {
        return StatusCode::FORBIDDEN;
    }
    match state
        .persist(move |persistence| persistence.delete_session(&session_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) => map_persistence_error(error),
    }
}

pub async fn send_message(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<SendKernelMessageRequest>,
) -> Result<(StatusCode, Json<MessageViewJson>), StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    let content = request.content.clone();
    let (row, bridge_response) = crate::message_dispatch::dispatch_user_message(
        &state,
        &session_id,
        &content,
        &row,
    )
    .await?;
    let _ = bridge_response;
    Ok((StatusCode::CREATED, Json(message_row_to_view(row))))
}

pub async fn get_messages(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<MessageListResponseJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);
    let fetch_limit = i64::from(limit) + i64::from(offset);
    let rows = state
        .persist(move |persistence| persistence.get_messages(&session_id, Some(fetch_limit)))
        .await
        .map_err(map_persistence_error)?;
    let rows = rows
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(message_row_to_view)
        .collect();
    Ok(Json(MessageListResponseJson { items: rows }))
}

pub async fn submit_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<SubmitTaskRequest>,
) -> Result<(StatusCode, Json<TaskViewJson>), StatusCode> {
    let session_key = session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &session)?;
    let instruction = request.instruction.clone();
    let row = state
        .persist(move |persistence| persistence.create_task(&session_id, &instruction))
        .await
        .map_err(map_persistence_error)?;
    Ok((StatusCode::CREATED, Json(task_row_to_view(row))))
}

pub async fn get_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskViewJson>, StatusCode> {
    let task_key = task_id.clone();
    let task = state
        .persist(move |persistence| persistence.get_task(&task_key))
        .await
        .map_err(map_persistence_error)?;
    let session_key = task.session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &session)?;
    Ok(Json(task_row_to_view(task)))
}

pub async fn list_tasks(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<TaskListResponseJson>, StatusCode> {
    let session_key = session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &session)?;
    let rows = state
        .persist(move |persistence| persistence.list_tasks(&session_id))
        .await
        .map_err(map_persistence_error)?;
    Ok(Json(TaskListResponseJson {
        items: rows.into_iter().map(task_row_to_view).collect(),
    }))
}

pub async fn cancel_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskViewJson>, StatusCode> {
    let task_key = task_id.clone();
    let task = state
        .persist(move |persistence| persistence.get_task(&task_key))
        .await
        .map_err(map_persistence_error)?;
    let session_key = task.session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &session)?;
    state
        .persist(move |persistence| persistence.cancel_task(&task_id))
        .await
        .map(task_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn list_models(
    State(state): State<Arc<InternalRuntimeApiState>>,
) -> Result<Json<ModelListResponseJson>, StatusCode> {
    let models = state
        .runtime
        .list_models()
        .map_err(map_runtime_error)?;
    Ok(Json(ModelListResponseJson {
        items: models
            .into_iter()
            .map(|model| ModelDescriptorJson {
                model_id: model.model_id,
                provider_id: model.provider_id,
                display_name: model.display_name,
                family: model.family,
                capabilities: model.capabilities,
            })
            .collect(),
    }))
}

pub async fn invoke_model(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Extension(metrics): Extension<Arc<MetricsRegistry>>,
    Path(session_id): Path<String>,
    Json(request): Json<InvokeModelRequest>,
) -> Result<Json<ModelResponseJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    let _ = state
        .runtime
        .register_session(&session_id, bridge_config_from_row(&row));

    if let Some(tenant_id) = ctx.tenant_id.as_deref() {
        if let Err(status) = state.tenant_token_quota.check_allowed(tenant_id).await {
            crate::security_audit::log_auth_failure(
                "quota.token_rejected",
                Some(&ctx.request_id),
                "/internal/v3/api/intelligence/runtime/sessions/{session_id}/model/invoke",
                Some(tenant_id),
                ctx.user_id.as_deref(),
                "tenant daily model token quota exhausted",
            );
            metrics.record_tenant_token_quota_rejection();
            return Err(status);
        }
    }

    let model_id = request
        .model_id
        .unwrap_or_else(|| "model.local.default".to_string());
    let model_request = ModelRequest {
        model_request_id: format!("model-req.{}", generate_id()),
        model_id: Some(model_id),
        session_id: Some(session_id.clone()),
        task_id: None,
        run_id: None,
        step_id: None,
        messages: Vec::new(),
        context_frame_ids: Vec::new(),
        context_frames: Vec::new(),
        tool_descriptors: Vec::new(),
        response_format: None,
        policy_request_id: None,
        trace_context: None,
        timeout_ms: None,
        metadata: Vec::new(),
    };

    let result = state
        .runtime
        .invoke_model(model_request)
        .map_err(map_runtime_error)?;

    let status_label = format!("{:?}", result.response.status).to_lowercase();
    metrics.record_model_invocation(&result.response.provider_id, &status_label);
    if let Some(usage) = result.response.usage.as_ref() {
        crate::usage_meter::record_model_token_usage(
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            &session_id,
            &result.response.provider_id,
            usage,
        );
        metrics.record_model_token_usage(&result.response.provider_id, "input", u64::from(usage.input_tokens));
        metrics.record_model_token_usage(&result.response.provider_id, "output", u64::from(usage.output_tokens));
        if let Some(tenant_id) = ctx.tenant_id.as_deref() {
            state
                .tenant_token_quota
                .record_usage(tenant_id, u64::from(usage.total_tokens()))
                .await;
        }
    }

    Ok(Json(ModelResponseJson {
        model_request_id: result.response.model_request_id,
        provider_id: result.response.provider_id,
        status: format!("{:?}", result.response.status).to_lowercase(),
        messages: result.response.messages,
        tool_calls: result
            .response
            .tool_calls
            .into_iter()
            .map(|call| {
                serde_json::json!({
                    "toolCallId": call.tool_call_id,
                    "toolId": call.tool_id,
                    "input": call.arguments,
                })
            })
            .collect(),
    }))
}

pub async fn list_tools(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<ToolListResponseJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let tools = state
        .runtime
        .list_tools()
        .map_err(map_runtime_error)?;
    Ok(Json(ToolListResponseJson {
        items: tools
            .into_iter()
            .map(|tool| {
                serde_json::json!({
                    "toolId": tool.tool_id,
                    "providerId": tool.provider_id,
                    "name": tool.name,
                    "displayName": tool.display_name,
                    "description": tool.description,
                    "sideEffectLevel": tool.side_effect_level.as_str(),
                    "policyCategories": tool.policy_categories,
                    "timeoutMs": tool.timeout_ms,
                })
            })
            .collect(),
    }))
}

pub async fn execute_tool(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path((session_id, tool_name)): Path<(String, String)>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ToolCallJson>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    let _ = state
        .runtime
        .register_session(&session_id, bridge_config_from_row(&row));

    let result = state
        .runtime
        .execute_tool(&session_id, &tool_name, &request.input)
        .map_err(map_runtime_error)?;

    Ok(Json(ToolCallJson {
        tool_call_id: result.call_id,
        tool_id: tool_name,
        input: request.input,
        status: result.result.status,
        output: Some(result.result.output),
    }))
}

pub async fn stream_session_events(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<StreamEventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let last_event_id = last_event_id_from_request(&headers, &query);
    let live = query.live.unwrap_or(true);
    let session_for_events = session_id.clone();
    let events = state
        .persist(move |persistence| persistence.load_session_events(&session_for_events, Some(100)))
        .await
        .map_err(map_persistence_error)?;
    let events = events_after_cursor(events, last_event_id);
    let replay_count = events.len();
    let replay = stream::iter(events.into_iter().enumerate().map(|(index, row)| {
        Ok(event_row_to_sse(&row, index as u32))
    }));

    let stream: SessionEventStream = if live {
        let live_stream = live_event_stream(
            state.persistence.event_bus().subscribe(),
            session_id,
            replay_count as u32,
        );
        Box::pin(replay.chain(live_stream))
    } else {
        Box::pin(replay)
    };

    Ok(
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        ),
    )
}

fn map_persistence_error(error: String) -> StatusCode {
    if error.contains("not found") {
        StatusCode::NOT_FOUND
    } else if error.contains("is closed") {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Extension, Path, State};
    use crate::config::ServerConfig;
    use crate::middleware::RequestContext;

    fn test_state() -> Arc<InternalRuntimeApiState> {
        Arc::new(
            InternalRuntimeApiState::new(
                Arc::new(PersistenceState::memory().expect("persistence")),
                Arc::new(ServerConfig::default()),
            )
            .expect("runtime state should initialize for tests"),
        )
    }

    fn test_context() -> Extension<RequestContext> {
        Extension(RequestContext {
            request_id: "req.test".to_string(),
            trace_id: None,
            tenant_id: None,
            user_id: None,
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        })
    }

    #[test]
    fn events_after_cursor_skips_prior_events() {
        let events = vec![
            EventRow {
                event_id: "evt.1".to_string(),
                session_id: Some("session.1".to_string()),
                event_type: "session.created".to_string(),
                severity: "info".to_string(),
                payload: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            EventRow {
                event_id: "evt.2".to_string(),
                session_id: Some("session.1".to_string()),
                event_type: "session.closed".to_string(),
                severity: "info".to_string(),
                payload: None,
                created_at: "2026-01-01T00:00:01Z".to_string(),
            },
        ];
        let filtered = events_after_cursor(events, Some("evt.1".to_string()));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_id, "evt.2");
    }

    #[tokio::test]
    async fn internal_runtime_snapshot_and_session_roundtrip() {
        let state = test_state();
        let snapshot = load_snapshot(State(state.clone()))
            .await
            .expect("snapshot")
            .0;
        assert_eq!(snapshot.runtime.health, "healthy");

        let (status, Json(session)) = create_session(
            State(state.clone()),
            test_context(),
            Json(CreateKernelSessionRequest {
                agent_id: "agent.1".to_string(),
                tenant_id: Some("tenant.1".to_string()),
                user_ref: None,
                model: None,
                model_provider: None,
                title: Some("Kernel UI".to_string()),
                goal: None,
                instructions: None,
                cwd: None,
                workspace_roots: None,
                source: Some("web".to_string()),
                kind: None,
                timeout_ms: None,
                metadata: None,
            }),
        )
        .await
        .expect("created");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(session.agent_id, "agent.1");

        let Json(loaded) = get_session(
            State(state.clone()),
            test_context(),
            Path(session.session_id.clone()),
        )
        .await
        .expect("loaded");
        assert_eq!(loaded.tenant_id, Some("tenant.1".to_string()));
    }
}
