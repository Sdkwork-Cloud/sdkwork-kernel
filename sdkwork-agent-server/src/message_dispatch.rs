use sdkwork_agent_api_bridge::{collect_model_stream_output, BridgeMessageResponse};
use sdkwork_agent_database::{MessageRow, SessionRow};
use sdkwork_agent_kernel::ModelStreamChunk;

use crate::api::internal_runtime::InternalRuntimeApiState;
use crate::http_response::ApiError;

/// Extract assistant-visible text from a bridge turn response.
pub fn assistant_content_from_bridge(response: &BridgeMessageResponse) -> String {
    response
        .model_response
        .as_ref()
        .map(|model| model.messages.concat())
        .filter(|content| !content.is_empty())
        .or_else(|| {
            response
                .message
                .parts
                .first()
                .and_then(|part| part.text.clone())
        })
        .unwrap_or_default()
}

/// Run the runtime bridge turn, then atomically persist the completed message turn.
pub async fn dispatch_user_message(
    state: &InternalRuntimeApiState,
    session_id: &str,
    content: &str,
    row: &SessionRow,
    trace_id: &str,
) -> Result<(MessageRow, Option<MessageRow>, BridgeMessageResponse), ApiError> {
    let lease = state
        .runtime
        .acquire_provider_admission()
        .await
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;
    state
        .register_persisted_session(&lease, row, trace_id)
        .await?;
    let runtime = state.runtime.clone();
    let session_key = session_id.to_string();
    let content_owned = content.to_string();
    let bridge_response = runtime
        .run_provider_admitted(lease, move |runtime| {
            runtime.send_message(&session_key, &content_owned)
        })
        .await
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let assistant_content = assistant_content_from_bridge(&bridge_response);
    let assistant_content = if assistant_content.is_empty() {
        None
    } else {
        Some(assistant_content)
    };
    let session_key = session_id.to_string();
    let user_content = content.to_string();
    let (user_row, assistant_row) = state
        .persist(move |persistence| {
            persistence.append_completed_turn(&session_key, user_content, assistant_content)
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, trace_id))?;

    Ok((user_row, assistant_row, bridge_response))
}

/// Run the streaming bridge turn, then atomically persist the completed message turn.
pub async fn dispatch_user_message_stream(
    state: &InternalRuntimeApiState,
    session_id: &str,
    content: &str,
    row: &SessionRow,
    model_override: Option<&str>,
    trace_id: &str,
) -> Result<(MessageRow, String, Vec<ModelStreamChunk>), ApiError> {
    let lease = state
        .runtime
        .acquire_provider_admission()
        .await
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;
    state
        .register_persisted_session(&lease, row, trace_id)
        .await?;
    let runtime = state.runtime.clone();
    let session_key = session_id.to_string();
    let content_owned = content.to_string();
    let model_override_owned = model_override.map(str::to_string);
    let (assistant_message_id, chunks) = runtime
        .run_provider_admitted(lease, move |runtime| {
            runtime.stream_message(
                &session_key,
                &content_owned,
                model_override_owned.as_deref(),
            )
        })
        .await
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;

    let assistant_content = collect_model_stream_output(&chunks)
        .map_err(|error| ApiError::from_kernel(error, trace_id))?;
    let assistant_content = if assistant_content.is_empty() {
        None
    } else {
        Some(assistant_content)
    };
    let session_key = session_id.to_string();
    let user_content = content.to_string();
    let (user_row, _) = state
        .persist(move |persistence| {
            persistence.append_completed_turn(&session_key, user_content, assistant_content)
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, trace_id))?;

    Ok((user_row, assistant_message_id, chunks))
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
        let (persistence, state) = {
            let _lock = crate::testing::env::lock();
            let _plugin = crate::testing::env::VarGuard::set(
                crate::runtime_bootstrap::KERNEL_AGENT_PLUGIN_ENV,
                None,
            );
            let config = Arc::new(ServerConfig::default());
            let persistence = Arc::new(
                PersistenceState::memory()
                    .expect("in-memory persistence should initialize for tests"),
            );
            let state = InternalRuntimeApiState::new(persistence.clone(), config)
                .expect("runtime state should initialize for tests");
            (persistence, state)
        };
        let session = persistence
            .create_session(SessionConfig::new("agent.1"))
            .expect("session should be created");

        let (user_row, assistant_row, bridge_response) = dispatch_user_message(
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
        assert_eq!(
            assistant_row.as_ref().map(|row| row.role.as_str()),
            Some("assistant")
        );
        assert!(!assistant_content_from_bridge(&bridge_response).is_empty());

        let messages = persistence
            .get_messages(&session.session_id, None, None)
            .expect("messages should load");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }
}
