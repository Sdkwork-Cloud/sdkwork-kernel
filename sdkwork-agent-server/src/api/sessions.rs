use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sdkwork_agent_database::SessionRow;
use sdkwork_agent_session::SessionConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::persistence::PersistenceState;

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

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<PersistenceState>>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
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

    let row = state
        .create_session(config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(session_row_to_response(row))))
}

/// Get a session by ID
pub async fn get_session(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    state
        .get_session(&session_id)
        .map(session_row_to_response)
        .map(Json)
        .map_err(|error| {
            if error.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

/// List sessions
pub async fn list_sessions(
    State(state): State<Arc<PersistenceState>>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let rows = state
        .list_sessions(sdkwork_agent_session::SessionQuery {
            agent_id: query.agent_id,
            state: query.state,
            kind: None,
            provider_id: query.provider_id,
            bridge_id: query.bridge_id,
            limit: query.limit.map(i64::from),
            offset: None,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter().map(session_row_to_response).collect(),
    ))
}

/// Close a session
pub async fn close_session(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    state
        .close_session(&session_id)
        .map(session_row_to_response)
        .map(Json)
        .map_err(|error| {
            if error.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

/// Delete a session
pub async fn delete_session(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
) -> StatusCode {
    match state.delete_session(&session_id) {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) if error.contains("not found") => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> Arc<PersistenceState> {
        Arc::new(PersistenceState::memory().expect("persistence"))
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

        let (status, Json(session)) = create_session(State(state.clone()), Json(request))
            .await
            .expect("created");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(session.agent_id, "agent.1");
        assert_eq!(session.state, "active");

        let result = get_session(State(state.clone()), Path(session.session_id.clone())).await;
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

        create_session(State(state.clone()), Json(request1))
            .await
            .expect("created");
        create_session(State(state.clone()), Json(request2))
            .await
            .expect("created");

        let Json(sessions) = list_sessions(
            State(state.clone()),
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

        let (_, Json(session)) = create_session(State(state.clone()), Json(request))
            .await
            .expect("created");
        let Json(closed) = close_session(State(state.clone()), Path(session.session_id.clone()))
            .await
            .expect("closed");
        assert_eq!(closed.state, "closed");
    }
}
