use sdkwork_agent_api_bridge::BridgeMessageResponse;
use sdkwork_agent_database::{MessageRow, SessionRow};
use sdkwork_agent_kernel::ModelStreamChunk;
use sdkwork_agent_session::MessageConfig;

use crate::api::internal_runtime::{bridge_config_from_row, InternalRuntimeApiState};
use crate::http_response::ApiError;

/// Extract assistant-visible text from a bridge turn response.
pub fn assistant_content_from_bridge(response: &BridgeMessageResponse) -> String {
    response
        .model_response
        .as_ref()
        .and_then(|model| model.messages.first().cloned())
        .or_else(|| {
            response
                .message
                .parts
                .first()
                .and_then(|part| part.text.clone())
        })
        .unwrap_or_default()
}

/// Persist the user message, run the runtime bridge turn, then persist the assistant reply.
pub async fn dispatch_user_message(
    state: &InternalRuntimeApiState,
    session_id: &str,
    content: &str,
    row: &SessionRow,
    trace_id: &str,
) -> Result<(MessageRow, BridgeMessageResponse), ApiError> {
    let session_key = session_id.to_string();
    let user_content = content.to_string();
    let user_row = state
        .persist(move |persistence| {
            persistence.send_message(&session_key, MessageConfig::user(user_content))
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, trace_id))?;

    state
        .runtime
        .register_session(session_id, bridge_config_from_row(row))
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let runtime = state.runtime.clone();
    let session_key = session_id.to_string();
    let content_owned = content.to_string();
    let bridge_response =
        tokio::task::spawn_blocking(move || runtime.send_message(&session_key, &content_owned))
            .await
            .map_err(|error| ApiError::internal(error.to_string(), trace_id))?
            .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let assistant_content = assistant_content_from_bridge(&bridge_response);
    if !assistant_content.is_empty() {
        let session_key = session_id.to_string();
        state
            .persist(move |persistence| {
                persistence.send_message(&session_key, MessageConfig::assistant(assistant_content))
            })
            .await
            .map_err(|error| ApiError::from_persistence(error, trace_id))?;
    }

    emit_turn_completed(state, session_id, &user_row.message_id, trace_id).await?;

    Ok((user_row, bridge_response))
}

/// Persist the user message, stream a runtime bridge turn, then persist the assistant reply.
pub async fn dispatch_user_message_stream(
    state: &InternalRuntimeApiState,
    session_id: &str,
    content: &str,
    row: &SessionRow,
    model_override: Option<&str>,
    trace_id: &str,
) -> Result<(MessageRow, String, Vec<ModelStreamChunk>), ApiError> {
    let session_key = session_id.to_string();
    let user_content = content.to_string();
    let user_row = state
        .persist(move |persistence| {
            persistence.send_message(&session_key, MessageConfig::user(user_content))
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, trace_id))?;

    state
        .runtime
        .register_session(session_id, bridge_config_from_row(row))
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let runtime = state.runtime.clone();
    let session_key = session_id.to_string();
    let content_owned = content.to_string();
    let model_override_owned = model_override.map(str::to_string);
    let (assistant_message_id, chunks) = tokio::task::spawn_blocking(move || {
        runtime.stream_message(
            &session_key,
            &content_owned,
            model_override_owned.as_deref(),
        )
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string(), trace_id))?
    .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let assistant_content: String = chunks.iter().map(|chunk| chunk.content.as_str()).collect();
    if !assistant_content.is_empty() {
        let session_key = session_id.to_string();
        state
            .persist(move |persistence| {
                persistence.send_message(&session_key, MessageConfig::assistant(assistant_content))
            })
            .await
            .map_err(|error| ApiError::from_persistence(error, trace_id))?;
    }

    emit_turn_completed(state, session_id, &user_row.message_id, trace_id).await?;

    Ok((user_row, assistant_message_id, chunks))
}

async fn emit_turn_completed(
    state: &InternalRuntimeApiState,
    session_id: &str,
    user_message_id: &str,
    trace_id: &str,
) -> Result<(), ApiError> {
    let session_key = session_id.to_string();
    let payload = format!("user_message_id={user_message_id}");
    state
        .persist(move |persistence| {
            persistence.emit_session_event(&session_key, "turn.completed", "info", Some(&payload))
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, trace_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::persistence::PersistenceState;
    use sdkwork_agent_session::SessionConfig;
    use std::sync::Arc;

    #[tokio::test]
    async fn dispatch_persists_user_and_assistant_messages() {
        let _lock = crate::testing::env::lock();
        let _plugin = crate::testing::env::VarGuard::set(
            crate::runtime_bootstrap::KERNEL_AGENT_PLUGIN_ENV,
            None,
        );
        let config = Arc::new(ServerConfig::default());
        let persistence = Arc::new(
            PersistenceState::memory().expect("in-memory persistence should initialize for tests"),
        );
        let state = InternalRuntimeApiState::new(persistence.clone(), config)
            .expect("runtime state should initialize for tests");
        let session = persistence
            .create_session(SessionConfig::new("agent.1"))
            .expect("session should be created");

        let (user_row, bridge_response) = dispatch_user_message(
            &state,
            &session.session_id,
            "Hello runtime",
            &session,
            "trace-test-dispatch",
        )
        .await
        .expect("dispatch should succeed");

        assert_eq!(user_row.role, "user");
        assert_eq!(user_row.content, "Hello runtime");
        assert!(!assistant_content_from_bridge(&bridge_response).is_empty());

        let messages = persistence
            .get_messages(&session.session_id, None, None)
            .expect("messages should load");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }
}
