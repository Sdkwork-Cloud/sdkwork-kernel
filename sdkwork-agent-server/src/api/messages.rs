use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use sdkwork_agent_database::MessageRow;
use sdkwork_agent_session::MessageConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::kernel::{ensure_session_access, KernelApiState};
use crate::middleware::RequestContext;

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

fn map_persistence_error(error: String) -> StatusCode {
    if error.contains("not found") {
        StatusCode::NOT_FOUND
    } else if error.contains("closed") {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn load_session_for_access(
    state: &KernelApiState,
    ctx: &RequestContext,
    session_id: &str,
) -> Result<(), StatusCode> {
    let session_key = session_id.to_string();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(state, ctx, &row)
}

/// Send a message
pub async fn send_message(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let role = request.role.unwrap_or_else(|| "user".to_string());
    if role == "user" {
        let (user_row, _) = crate::message_dispatch::dispatch_user_message(
            &state,
            &session_id,
            &request.content,
            &row,
        )
        .await?;
        return Ok((StatusCode::CREATED, Json(message_row_to_response(user_row))));
    }

    let config = MessageConfig {
        role,
        content: request.content,
        metadata: None,
    };
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.send_message(&session_key, config))
        .await
        .map_err(map_persistence_error)?;
    Ok((StatusCode::CREATED, Json(message_row_to_response(row))))
}

/// Get messages for a session
pub async fn get_messages(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    load_session_for_access(&state, &ctx, &session_id).await?;
    let limit = query.limit.map(i64::from);
    let session_key = session_id.clone();
    let rows = state
        .persist(move |persistence| persistence.get_messages(&session_key, limit))
        .await
        .map_err(map_persistence_error)?;
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
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    load_session_for_access(&state, &ctx, &session_id).await?;
    let session_key = session_id.clone();
    let count = state
        .persist(move |persistence| persistence.message_count(&session_key))
        .await
        .map_err(map_persistence_error)?;
    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "count": count
    })))
}

/// Delete messages for a session
pub async fn delete_messages(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> StatusCode {
    if let Err(status) = load_session_for_access(&state, &ctx, &session_id).await {
        return status;
    }

    match state
        .persist(move |persistence| persistence.delete_messages(&session_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(error) => map_persistence_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sessions::{create_session, CreateSessionRequest};
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
    async fn send_and_get_messages() {
        let state = test_state();
        let (_, Json(session)) = create_session(
            State(state.clone()),
            test_context(),
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
            test_context(),
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
            test_context(),
            Path(session.session_id),
            Query(ListMessagesQuery {
                limit: None,
                offset: None,
            }),
        )
        .await
        .expect("loaded");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn message_count_test() {
        let state = test_state();
        let (_, Json(session)) = create_session(
            State(state.clone()),
            test_context(),
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

        let _ = send_message(
            State(state.clone()),
            test_context(),
            Path(session.session_id.clone()),
            Json(SendMessageRequest {
                content: "Hello".to_string(),
                role: None,
            }),
        )
        .await
        .expect("sent");

        let Json(result) = message_count(
            State(state.clone()),
            test_context(),
            Path(session.session_id),
        )
        .await
        .expect("count");
        assert_eq!(result["count"], 2);
    }
}
