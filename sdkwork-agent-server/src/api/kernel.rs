use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::{self, Stream};
use sdkwork_agent_database::{EventRow, MessageRow, SessionRow, TaskRow};
use sdkwork_agent_session::{MessageConfig, SessionConfig, SessionQuery};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use crate::persistence::PersistenceState;

/// Shared kernel UI API state.
#[derive(Clone)]
pub struct KernelApiState {
    pub persistence: Arc<PersistenceState>,
    permissions: Arc<Mutex<HashMap<String, PermissionRecord>>>,
}

#[derive(Debug, Clone)]
struct PermissionRecord {
    view: PermissionRequestJson,
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

impl KernelApiState {
    pub fn new(persistence: Arc<PersistenceState>) -> Self {
        Self {
            persistence,
            permissions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn build_snapshot(&self) -> Result<KernelUiSnapshotJson, String> {
        let db_healthy = self.persistence.health().unwrap_or(false);
        let runtime_health = if db_healthy { "healthy" } else { "degraded" };
        let runtime_state = if db_healthy { "ready" } else { "degraded" };

        let permissions = self
            .permissions
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?
            .values()
            .map(|record| record.view.clone())
            .collect();

        Ok(KernelUiSnapshotJson {
            runtime: KernelRuntimeJson {
                runtime_id: "runtime.local".to_string(),
                agent_id: "agent.intelligence.general".to_string(),
                kernel_version: env!("CARGO_PKG_VERSION").to_string(),
                state: runtime_state.to_string(),
                health: runtime_health.to_string(),
                capabilities: vec![
                    CapabilityJson {
                        capability_id: "model.chat".to_string(),
                        provider_id: "provider.model.local".to_string(),
                        status: "available".to_string(),
                        required: true,
                    },
                    CapabilityJson {
                        capability_id: "policy.evaluate".to_string(),
                        provider_id: "provider.policy.local".to_string(),
                        status: "available".to_string(),
                        required: true,
                    },
                ],
                missing_required_capabilities: Vec::new(),
                degraded_capabilities: if db_healthy {
                    Vec::new()
                } else {
                    vec!["persistence.sqlite".to_string()]
                },
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

fn event_row_to_stream(row: &EventRow, sequence: u32) -> StreamEventJson {
    StreamEventJson {
        event_id: row.event_id.clone(),
        event_type: row.event_type.clone(),
        sequence,
        payload: row.payload.clone().unwrap_or_default(),
        timestamp: Some(row.created_at.clone()),
    }
}

fn parse_metadata_map(raw: Option<&str>) -> HashMap<String, String> {
    raw.and_then(|value| serde_json::from_str::<HashMap<String, String>>(value).ok())
        .unwrap_or_default()
}

fn build_session_metadata(request: &CreateKernelSessionRequest) -> Option<serde_json::Value> {
    let mut metadata = request.metadata.clone().unwrap_or_default();
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
    if metadata.is_empty() {
        None
    } else {
        serde_json::to_value(metadata).ok()
    }
}

pub async fn load_snapshot(
    State(state): State<Arc<KernelApiState>>,
) -> Result<Json<KernelUiSnapshotJson>, StatusCode> {
    state
        .build_snapshot()
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn decide_permission(
    State(state): State<Arc<KernelApiState>>,
    Path(permission_request_id): Path<String>,
    Json(body): Json<PermissionDecisionBody>,
) -> Result<Json<PermissionRequestJson>, StatusCode> {
    let mut permissions = state
        .permissions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let record = permissions
        .get_mut(&permission_request_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    record.view.status = body.decision;
    Ok(Json(record.view.clone()))
}

pub async fn create_session(
    State(state): State<Arc<KernelApiState>>,
    Json(request): Json<CreateKernelSessionRequest>,
) -> Result<(StatusCode, Json<SessionViewJson>), StatusCode> {
    let metadata = build_session_metadata(&request);
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
    config.metadata = metadata;

    let row = state
        .persistence
        .create_session(config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(session_row_to_view(row))))
}

pub async fn get_session(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionViewJson>, StatusCode> {
    state
        .persistence
        .get_session(&session_id)
        .map(session_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn list_sessions(
    State(state): State<Arc<KernelApiState>>,
) -> Result<Json<Vec<SessionViewJson>>, StatusCode> {
    let rows = state
        .persistence
        .list_sessions(SessionQuery::default())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter().map(session_row_to_view).collect(),
    ))
}

pub async fn close_session(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionViewJson>, StatusCode> {
    state
        .persistence
        .close_session(&session_id)
        .map(session_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn delete_session(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> StatusCode {
    match state.persistence.delete_session(&session_id) {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) => map_persistence_error(error),
    }
}

pub async fn send_message(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SendKernelMessageRequest>,
) -> Result<(StatusCode, Json<MessageViewJson>), StatusCode> {
    let row = state
        .persistence
        .send_message(&session_id, MessageConfig::user(request.content))
        .map_err(map_persistence_error)?;
    Ok((StatusCode::CREATED, Json(message_row_to_view(row))))
}

pub async fn get_messages(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageViewJson>>, StatusCode> {
    let mut rows = state
        .persistence
        .get_messages(&session_id, query.limit.map(i64::from))
        .map_err(map_persistence_error)?;
    if let Some(offset) = query.offset {
        let offset = offset as usize;
        if offset < rows.len() {
            rows = rows[offset..].to_vec();
        } else {
            rows.clear();
        }
    }
    Ok(Json(rows.into_iter().map(message_row_to_view).collect()))
}

pub async fn submit_task(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SubmitTaskRequest>,
) -> Result<(StatusCode, Json<TaskViewJson>), StatusCode> {
    let row = state
        .persistence
        .create_task(&session_id, &request.instruction)
        .map_err(map_persistence_error)?;
    Ok((StatusCode::CREATED, Json(task_row_to_view(row))))
}

pub async fn get_task(
    State(state): State<Arc<KernelApiState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskViewJson>, StatusCode> {
    state
        .persistence
        .get_task(&task_id)
        .map(task_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn list_tasks(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<TaskViewJson>>, StatusCode> {
    let rows = state
        .persistence
        .list_tasks(&session_id)
        .map_err(map_persistence_error)?;
    Ok(Json(rows.into_iter().map(task_row_to_view).collect()))
}

pub async fn cancel_task(
    State(state): State<Arc<KernelApiState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskViewJson>, StatusCode> {
    state
        .persistence
        .cancel_task(&task_id)
        .map(task_row_to_view)
        .map(Json)
        .map_err(map_persistence_error)
}

pub async fn list_models() -> Json<Vec<ModelDescriptorJson>> {
    Json(vec![ModelDescriptorJson {
        model_id: "model.local.default".to_string(),
        provider_id: "provider.model.local".to_string(),
        display_name: "Local Default Model".to_string(),
        family: "local".to_string(),
        capabilities: vec!["chat".to_string()],
    }])
}

pub async fn invoke_model(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
    Json(request): Json<InvokeModelRequest>,
) -> Result<Json<ModelResponseJson>, StatusCode> {
    let _session = state
        .persistence
        .get_session(&session_id)
        .map_err(map_persistence_error)?;
    let model_id = request
        .model_id
        .unwrap_or_else(|| "model.local.default".to_string());
    Ok(Json(ModelResponseJson {
        model_request_id: format!("model-req.{}", generate_id()),
        provider_id: "provider.model.local".to_string(),
        status: "succeeded".to_string(),
        messages: vec![format!("Model {model_id} acknowledged session {session_id}")],
        tool_calls: Vec::new(),
    }))
}

pub async fn list_tools(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let _session = state
        .persistence
        .get_session(&session_id)
        .map_err(map_persistence_error)?;
    Ok(Json(Vec::new()))
}

pub async fn execute_tool(
    State(state): State<Arc<KernelApiState>>,
    Path((session_id, tool_name)): Path<(String, String)>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ToolCallJson>, StatusCode> {
    let _session = state
        .persistence
        .get_session(&session_id)
        .map_err(map_persistence_error)?;
    Ok(Json(ToolCallJson {
        tool_call_id: format!("tool-call.{}", generate_id()),
        tool_id: tool_name,
        input: request.input,
        status: "succeeded".to_string(),
        output: Some("{\"ok\":true}".to_string()),
    }))
}

pub async fn stream_session_events(
    State(state): State<Arc<KernelApiState>>,
    Path(session_id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let events = state
        .persistence
        .load_session_events(&session_id, Some(100))
        .map_err(map_persistence_error)?;
    let stream = stream::iter(events.into_iter().enumerate().map(|(index, row)| {
        let payload = event_row_to_stream(&row, index as u32);
        let event = Event::default().data(serde_json::to_string(&payload).unwrap_or_default());
        Ok(event)
    }));
    Ok(Sse::new(stream))
}

fn map_persistence_error(error: String) -> StatusCode {
    if error.contains("not found") {
        StatusCode::NOT_FOUND
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
    use axum::extract::{Path, State};

    fn test_state() -> Arc<KernelApiState> {
        Arc::new(KernelApiState::new(Arc::new(
            PersistenceState::memory().expect("persistence"),
        )))
    }

    #[tokio::test]
    async fn kernel_snapshot_and_session_roundtrip() {
        let state = test_state();
        let snapshot = load_snapshot(State(state.clone()))
            .await
            .expect("snapshot")
            .0;
        assert_eq!(snapshot.runtime.health, "healthy");

        let (status, Json(session)) = create_session(
            State(state.clone()),
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

        let Json(loaded) = get_session(State(state.clone()), Path(session.session_id.clone()))
            .await
            .expect("loaded");
        assert_eq!(loaded.tenant_id, Some("tenant.1".to_string()));
    }
}
