use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use sdkwork_agent_database::SessionRow;
use sdkwork_agent_session::SessionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::access::stamp_session_ownership;
use crate::agent_registry::{apply_hosted_agent_defaults, validate_hosted_agent_id};
use crate::api::kernel::{ensure_session_access, KernelApiState};
use crate::middleware::RequestContext;

/// Create session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: String,
    pub model: Option<String>,
    pub title: Option<String>,
    pub instructions: Option<String>,
    pub source: Option<String>,
    pub kind: Option<String>,
}

/// Session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub agent_id: String,
    pub model: Option<String>,
    pub title: Option<String>,
    pub state: String,
    pub kind: String,
    pub source: String,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub message_count: u32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// List sessions query
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub agent_id: Option<String>,
    pub state: Option<String>,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub limit: Option<u32>,
}

fn session_row_to_response(row: SessionRow) -> SessionResponse {
    SessionResponse {
        session_id: row.session_id,
        agent_id: row.agent_id,
        model: row.model,
        title: row.title,
        state: row.state,
        kind: row.kind,
        source: row.source,
        provider_id: row.provider_id,
        bridge_id: row.bridge_id,
        message_count: row.message_count.max(0) as u32,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_persistence_error(error: String) -> StatusCode {
    if error.contains("not found") {
        StatusCode::NOT_FOUND
    } else if error.contains("closed") {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
    let registered = validate_hosted_agent_id(&request.agent_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut metadata_map = HashMap::new();
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
    Ok((StatusCode::CREATED, Json(session_row_to_response(row))))
}

/// Get a session by ID
pub async fn get_session(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    Ok(Json(session_row_to_response(row)))
}

/// List sessions
pub async fn list_sessions(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let query = sdkwork_agent_session::SessionQuery {
        agent_id: query.agent_id,
        state: query.state,
        kind: None,
        provider_id: query.provider_id,
        bridge_id: query.bridge_id,
        limit: Some(query.limit.unwrap_or(100).into()),
        offset: None,
    };
    let rows = state
        .persist(move |persistence| persistence.list_sessions(query))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut sessions = Vec::new();
    for row in rows {
        if ensure_session_access(&state, &ctx, &row).is_ok() {
            sessions.push(session_row_to_response(row));
        }
    }
    Ok(Json(sessions))
}

/// Close a session
pub async fn close_session(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;
    let row = state
        .persist(move |persistence| persistence.close_session(&session_id))
        .await
        .map(session_row_to_response)
        .map(Json)
        .map_err(map_persistence_error)?;
    Ok(row)
}

/// Delete a session
pub async fn delete_session(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> StatusCode {
    let session_key = session_id.clone();
    let access = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await;
    match access {
        Ok(row) => {
            if ensure_session_access(&state, &ctx, &row).is_err() {
                return StatusCode::FORBIDDEN;
            }
        }
        Err(error) => return map_persistence_error(error),
    }

    match state
        .persist(move |persistence| persistence.delete_session(&session_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) => map_persistence_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;

    fn test_state() -> Arc<KernelApiState> {
        Arc::new(KernelApiState::new(
            Arc::new(crate::persistence::PersistenceState::memory().expect("persistence")),
            Arc::new(ServerConfig::default()),
        ))
    }

    fn test_context() -> Extension<RequestContext> {
        Extension(RequestContext {
            request_id: "req.test".to_string(),
            tenant_id: None,
            user_id: None,
            subject_id: None,
        })
    }

    #[tokio::test]
    async fn create_and_get_session() {
        let state = test_state();
        let request = CreateSessionRequest {
            agent_id: "agent.1".to_string(),
            model: Some("gpt-4".to_string()),
            title: Some("Test".to_string()),
            instructions: None,
            source: None,
            kind: None,
        };

        let (status, Json(session)) =
            create_session(State(state.clone()), test_context(), Json(request))
                .await
                .expect("created");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(session.agent_id, "agent.1");
        assert_eq!(session.state, "active");

        let result = get_session(
            State(state.clone()),
            test_context(),
            Path(session.session_id.clone()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_sessions_test() {
        let state = test_state();
        let request1 = CreateSessionRequest {
            agent_id: "agent.1".to_string(),
            model: None,
            title: None,
            instructions: None,
            source: None,
            kind: None,
        };
        let request2 = CreateSessionRequest {
            agent_id: "agent.2".to_string(),
            model: None,
            title: None,
            instructions: None,
            source: None,
            kind: None,
        };

        let _ = create_session(State(state.clone()), test_context(), Json(request1))
            .await
            .expect("created");
        let _ = create_session(State(state.clone()), test_context(), Json(request2))
            .await
            .expect("created");

        let Json(sessions) = list_sessions(
            State(state.clone()),
            test_context(),
            Query(ListSessionsQuery {
                agent_id: None,
                state: None,
                provider_id: None,
                bridge_id: None,
                limit: None,
            }),
        )
        .await
        .expect("listed");

        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn close_session_test() {
        let state = test_state();
        let request = CreateSessionRequest {
            agent_id: "agent.1".to_string(),
            model: None,
            title: None,
            instructions: None,
            source: None,
            kind: None,
        };

        let (_, Json(session)) =
            create_session(State(state.clone()), test_context(), Json(request))
                .await
                .expect("created");
        let Json(closed) = close_session(
            State(state.clone()),
            test_context(),
            Path(session.session_id.clone()),
        )
        .await
        .expect("closed");
        assert_eq!(closed.state, "closed");
    }
}
