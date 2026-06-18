use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sdkwork_agent_database::MessageRow;
use sdkwork_agent_session::MessageConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::persistence::PersistenceState;

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub role: Option<String>,
}

/// Message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// List messages query
#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

fn message_row_to_response(row: MessageRow) -> MessageResponse {
    MessageResponse {
        message_id: row.message_id,
        session_id: row.session_id,
        role: row.role,
        content: row.content,
        created_at: row.created_at,
    }
}

/// Send a message
pub async fn send_message(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), StatusCode> {
    let role = request.role.unwrap_or_else(|| "user".to_string());
    let config = MessageConfig {
        role,
        content: request.content,
        metadata: None,
    };
    let row = state.send_message(&session_id, config).map_err(|error| {
        if error.contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok((StatusCode::CREATED, Json(message_row_to_response(row))))
}

/// Get messages for a session
pub async fn get_messages(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let rows = state
        .get_messages(&session_id, query.limit.map(i64::from))
        .map_err(|error| {
            if error.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    let mut results: Vec<MessageResponse> = rows.into_iter().map(message_row_to_response).collect();
    if let Some(offset) = query.offset {
        let offset = offset as usize;
        if offset < results.len() {
            results = results[offset..].to_vec();
        } else {
            results.clear();
        }
    }
    Ok(Json(results))
}

/// Get message count for a session
pub async fn message_count(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let count = state.message_count(&session_id).map_err(|error| {
        if error.contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "count": count
    })))
}

/// Delete messages for a session
pub async fn delete_messages(
    State(state): State<Arc<PersistenceState>>,
    Path(session_id): Path<String>,
) -> StatusCode {
    match state.delete_messages(&session_id) {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) if error.contains("not found") => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sessions::{create_session, CreateSessionRequest};
    use crate::persistence::PersistenceState;

    fn test_state() -> Arc<PersistenceState> {
        Arc::new(PersistenceState::memory().expect("persistence"))
    }

    #[tokio::test]
    async fn send_and_get_messages() {
        let state = test_state();
        let (_, Json(session)) = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                agent_id: "agent.1".to_string(),
                model: None,
                title: None,
                instructions: None,
                source: None,
                kind: None,
            }),
        )
        .await
        .expect("created");

        let request = SendMessageRequest {
            content: "Hello".to_string(),
            role: Some("user".to_string()),
        };

        let (status, Json(msg)) = send_message(
            State(state.clone()),
            Path(session.session_id.clone()),
            Json(request),
        )
        .await
        .expect("sent");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let Json(messages) = get_messages(
            State(state.clone()),
            Path(session.session_id),
            Query(ListMessagesQuery {
                limit: None,
                offset: None,
            }),
        )
        .await
        .expect("loaded");

        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn message_count_test() {
        let state = test_state();
        let (_, Json(session)) = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                agent_id: "agent.1".to_string(),
                model: None,
                title: None,
                instructions: None,
                source: None,
                kind: None,
            }),
        )
        .await
        .expect("created");

        send_message(
            State(state.clone()),
            Path(session.session_id.clone()),
            Json(SendMessageRequest {
                content: "Hello".to_string(),
                role: None,
            }),
        )
        .await
        .expect("sent");

        let Json(result) = message_count(State(state.clone()), Path(session.session_id))
            .await
            .expect("count");
        assert_eq!(result["count"], 1);
    }
}
