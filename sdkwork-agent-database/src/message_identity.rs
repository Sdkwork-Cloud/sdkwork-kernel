use crate::error::{DatabaseError, DatabaseResult};
use crate::types::{EventRow, MessageRow};
use std::collections::HashSet;

pub(crate) fn ensure_message_retry_matches(
    existing: &MessageRow,
    incoming: &MessageRow,
) -> DatabaseResult<()> {
    if existing.session_id != incoming.session_id {
        return Err(DatabaseError::ConstraintViolation(format!(
            "message {} already belongs to session {}",
            incoming.message_id, existing.session_id
        )));
    }
    if !message_rows_match(existing, incoming) {
        return Err(DatabaseError::ConstraintViolation(format!(
            "message {} already exists with different payload",
            incoming.message_id
        )));
    }
    Ok(())
}

fn message_rows_match(existing: &MessageRow, incoming: &MessageRow) -> bool {
    existing.message_id == incoming.message_id
        && existing.session_id == incoming.session_id
        && existing.role == incoming.role
        && existing.content == incoming.content
        && existing.created_at == incoming.created_at
        && existing.metadata_json == incoming.metadata_json
}

pub(crate) fn validate_message_turn<'a>(
    messages: &'a [MessageRow],
    events: &[EventRow],
) -> DatabaseResult<&'a str> {
    if messages.is_empty() || messages.len() > 2 {
        return Err(DatabaseError::ConstraintViolation(
            "a completed message turn must contain one user message and at most one assistant message"
                .to_string(),
        ));
    }
    if messages[0].role != "user"
        || messages
            .get(1)
            .is_some_and(|message| message.role != "assistant")
    {
        return Err(DatabaseError::ConstraintViolation(
            "a completed message turn must be ordered as user then assistant".to_string(),
        ));
    }

    let session_id = messages[0].session_id.as_str();
    let mut message_ids = HashSet::with_capacity(messages.len());
    for message in messages {
        if message.session_id != session_id {
            return Err(DatabaseError::ConstraintViolation(
                "all messages in a completed turn must belong to the same session".to_string(),
            ));
        }
        if !message_ids.insert(message.message_id.as_str()) {
            return Err(DatabaseError::ConstraintViolation(format!(
                "duplicate message id in completed turn: {}",
                message.message_id
            )));
        }
    }

    if events.len() != messages.len() + 1
        || events
            .iter()
            .filter(|event| event.event_type == "message.sent")
            .count()
            != messages.len()
        || events
            .iter()
            .filter(|event| event.event_type == "turn.completed")
            .count()
            != 1
    {
        return Err(DatabaseError::ConstraintViolation(
            "a completed turn must persist one message.sent event per message and one turn.completed event"
                .to_string(),
        ));
    }
    let mut event_ids = HashSet::with_capacity(events.len());
    for event in events {
        if event.session_id.as_deref() != Some(session_id) {
            return Err(DatabaseError::ConstraintViolation(
                "all completed-turn events must belong to the message session".to_string(),
            ));
        }
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(DatabaseError::ConstraintViolation(format!(
                "duplicate event id in completed turn: {}",
                event.event_id
            )));
        }
    }
    Ok(session_id)
}
