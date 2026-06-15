use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Session state shared across handlers
#[derive(Debug, Clone)]
pub struct SessionState {
    pub sessions: Arc<tokio::sync::Mutex<Vec<SessionResponse>>>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub message_count: u32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// List sessions query
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub agent_id: Option<String>,
    pub state: Option<String>,
    pub limit: Option<u32>,
}

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<SessionState>>,
    Json(request): Json<CreateSessionRequest>,
) -> (StatusCode, Json<SessionResponse>) {
    let now = chrono::Utc::now().to_rfc3339();
    let session_id = format!("session.{}", generate_id());

    let response = SessionResponse {
        session_id: session_id.clone(),
        agent_id: request.agent_id,
        model: request.model,
        title: request.title,
        state: "active".to_string(),
        kind: request.kind.unwrap_or_else(|| "main".to_string()),
        source: request.source.unwrap_or_else(|| "api".to_string()),
        message_count: 0,
        created_at: now.clone(),
        updated_at: None,
    };

    let mut sessions = state.sessions.lock().await;
    sessions.push(response.clone());

    (StatusCode::CREATED, Json(response))
}

/// Get a session by ID
pub async fn get_session(
    State(state): State<Arc<SessionState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let sessions = state.sessions.lock().await;
    sessions
        .iter()
        .find(|s| s.session_id == session_id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// List sessions
pub async fn list_sessions(
    State(state): State<Arc<SessionState>>,
    Query(query): Query<ListSessionsQuery>,
) -> Json<Vec<SessionResponse>> {
    let sessions = state.sessions.lock().await;
    let mut results: Vec<SessionResponse> = sessions
        .iter()
        .filter(|s| {
            if let Some(ref agent_id) = query.agent_id {
                if s.agent_id != *agent_id {
                    return false;
                }
            }
            if let Some(ref session_state) = query.state {
                if s.state != *session_state {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    if let Some(limit) = query.limit {
        results.truncate(limit as usize);
    }

    Json(results)
}

/// Close a session
pub async fn close_session(
    State(state): State<Arc<SessionState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.iter_mut().find(|s| s.session_id == session_id) {
        session.state = "closed".to_string();
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Json(session.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Delete a session
pub async fn delete_session(
    State(state): State<Arc<SessionState>>,
    Path(session_id): Path<String>,
) -> StatusCode {
    let mut sessions = state.sessions.lock().await;
    let len_before = sessions.len();
    sessions.retain(|s| s.session_id != session_id);
    if sessions.len() < len_before {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
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

    #[tokio::test]
    async fn create_and_get_session() {
        let state = Arc::new(SessionState::new());
        let request = CreateSessionRequest {
            agent_id: "agent.1".to_string(),
            model: Some("gpt-4".to_string()),
            title: Some("Test".to_string()),
            instructions: None,
            source: None,
            kind: None,
        };

        let (status, Json(session)) = create_session(State(state.clone()), Json(request)).await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(session.agent_id, "agent.1");
        assert_eq!(session.state, "active");

        let result = get_session(State(state.clone()), Path(session.session_id.clone())).await;

        assert!(result.is_ok());
        let Json(found) = result.unwrap();
        assert_eq!(found.session_id, session.session_id);
    }

    #[tokio::test]
    async fn list_sessions_test() {
        let state = Arc::new(SessionState::new());

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

        create_session(State(state.clone()), Json(request1)).await;
        create_session(State(state.clone()), Json(request2)).await;

        let Json(sessions) = list_sessions(
            State(state.clone()),
            Query(ListSessionsQuery {
                agent_id: None,
                state: None,
                limit: None,
            }),
        )
        .await;

        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn close_session_test() {
        let state = Arc::new(SessionState::new());
        let request = CreateSessionRequest {
            agent_id: "agent.1".to_string(),
            model: None,
            title: None,
            instructions: None,
            source: None,
            kind: None,
        };

        let (_, Json(session)) = create_session(State(state.clone()), Json(request)).await;

        let result = close_session(State(state.clone()), Path(session.session_id.clone())).await;

        assert!(result.is_ok());
        let Json(closed) = result.unwrap();
        assert_eq!(closed.state, "closed");
    }
}
