use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::kernel::{ensure_session_access, KernelApiState};
use crate::message_dispatch::assistant_content_from_bridge;
use crate::middleware::RequestContext;

/// Chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub content: String,
    pub model: Option<String>,
    pub stream: bool,
}

/// Chat message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message_id: String,
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub usage: Option<UsageResponse>,
}

/// Usage response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Send a chat message through the runtime bridge.
pub async fn send_chat(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    let session_key = request.session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let (_, bridge_response) = crate::message_dispatch::dispatch_user_message(
        &state,
        &request.session_id,
        &request.content,
        &row,
    )
    .await?;

    let content = assistant_content_from_bridge(&bridge_response);

    Ok((
        StatusCode::OK,
        Json(ChatResponse {
            message_id: bridge_response.message.message_id,
            session_id: request.session_id,
            content,
            status: "completed".to_string(),
            usage: bridge_response.model_response.as_ref().and_then(|response| {
                response.usage.as_ref().map(|usage| UsageResponse {
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    total_tokens: usage.input_tokens + usage.output_tokens,
                })
            }),
        }),
    ))
}

/// Get chat history for a session from persistence storage.
pub async fn get_chat_history(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ChatMessageResponse>>, StatusCode> {
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let rows = state
        .persist(move |persistence| persistence.get_messages(&session_id, Some(200)))
        .await
        .map_err(map_persistence_error)?;

    Ok(Json(
        rows.into_iter()
            .map(|row| ChatMessageResponse {
                message_id: row.message_id,
                session_id: row.session_id,
                role: row.role,
                content: row.content,
                status: "completed".to_string(),
                created_at: row.created_at,
            })
            .collect(),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::persistence::PersistenceState;
    use crate::api::kernel::KernelApiState;
    use sdkwork_agent_session::SessionConfig;
    use std::sync::Arc;

    fn test_state() -> Arc<KernelApiState> {
        let config = Arc::new(ServerConfig::default());
        let persistence = Arc::new(
            PersistenceState::memory().expect("in-memory persistence should initialize for tests"),
        );
        Arc::new(KernelApiState::new(persistence, config))
    }

    #[tokio::test]
    async fn send_chat_message() {
        let state = test_state();
        let session = state
            .persistence
            .create_session(SessionConfig::new("agent.1"))
            .expect("session should be created");

        let request = ChatRequest {
            session_id: session.session_id.clone(),
            content: "Hello".to_string(),
            model: Some("gpt-4".to_string()),
            stream: false,
        };

        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            tenant_id: None,
            user_id: None,
            subject_id: None,
        };

        let (status, Json(response)) = send_chat(State(state.clone()), Extension(ctx), Json(request))
            .await
            .expect("chat should succeed");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, "completed");
        assert!(!response.content.is_empty());

        let history = get_chat_history(
            State(state),
            Extension(RequestContext {
                request_id: "req.2".to_string(),
                tenant_id: None,
                user_id: None,
                subject_id: None,
            }),
            axum::extract::Path(session.session_id),
        )
        .await
        .expect("history should load")
        .0;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[1].role, "assistant");
    }
}
