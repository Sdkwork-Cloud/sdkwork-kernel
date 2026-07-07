use crate::error::{DatabaseError, DatabaseResult};
use crate::types::MessageRow;

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
