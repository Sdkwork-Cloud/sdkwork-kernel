use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::StreamExt;

use crate::api::kernel::{ensure_session_access, KernelApiState};
use crate::message_dispatch::dispatch_user_message_stream;
use crate::middleware::RequestContext;

/// SSE chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseChatRequest {
    pub session_id: String,
    pub content: String,
    pub model: Option<String>,
}

/// SSE event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEventData {
    pub event_type: String,
    pub message_id: String,
    pub content: String,
    pub sequence: u32,
}

/// Stream chat response via SSE
pub async fn stream_chat(
    State(state): State<Arc<KernelApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(request): Json<SseChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let session_key = request.session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(map_persistence_error)?;
    ensure_session_access(&state, &ctx, &row)?;

    let model_override = request.model.as_deref();
    let (_user_row, assistant_message_id, chunks) = dispatch_user_message_stream(
        &state,
        &request.session_id,
        &request.content,
        &row,
        model_override,
    )
    .await?;

    let sequence_base = {
        let mut counter = state.sse_event_counter.lock().await;
        let base = *counter;
        *counter += chunks.len() as u64 + 1;
        base as u32
    };

    let chunk_count = chunks.len() as u32;
    let done_message_id = assistant_message_id.clone();
    let chunk_stream = stream::iter(chunks.into_iter().enumerate()).map(
        move |(index, chunk)| {
            let data = SseEventData {
                event_type: "chunk".to_string(),
                message_id: assistant_message_id.clone(),
                content: chunk.content,
                sequence: sequence_base + index as u32,
            };

            let event = Event::default()
                .event("chunk")
                .data(serde_json::to_string(&data).unwrap_or_default());

            Ok(event)
        },
    );

    let done_stream = stream::once(async move {
        let data = SseEventData {
            event_type: "done".to_string(),
            message_id: done_message_id,
            content: String::new(),
            sequence: sequence_base + chunk_count,
        };

        let event = Event::default()
            .event("done")
            .data(serde_json::to_string(&data).unwrap_or_default());

        Ok(event)
    });

    Ok(Sse::new(chunk_stream.chain(done_stream)))
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
