use crate::error::{DatabaseError, DatabaseResult};
use crate::types::EventRow;

pub(crate) fn ensure_event_session(
    event: &EventRow,
    session_id: &str,
    operation: &str,
) -> DatabaseResult<()> {
    if event.session_id.as_deref() != Some(session_id) {
        return Err(DatabaseError::ConstraintViolation(format!(
            "{operation} event must belong to session {session_id}"
        )));
    }
    Ok(())
}

pub(crate) fn ensure_event_retry_matches(
    existing: &EventRow,
    incoming: &EventRow,
) -> DatabaseResult<()> {
    if existing.session_id != incoming.session_id {
        return Err(DatabaseError::ConstraintViolation(format!(
            "event {} already belongs to a different session",
            incoming.event_id
        )));
    }
    if !event_rows_match(existing, incoming) {
        return Err(DatabaseError::ConstraintViolation(format!(
            "event {} already exists with different payload",
            incoming.event_id
        )));
    }
    Ok(())
}

fn event_rows_match(existing: &EventRow, incoming: &EventRow) -> bool {
    existing.event_id == incoming.event_id
        && existing.session_id == incoming.session_id
        && existing.event_type == incoming.event_type
        && existing.severity == incoming.severity
        && existing.payload == incoming.payload
        && existing.created_at == incoming.created_at
}
