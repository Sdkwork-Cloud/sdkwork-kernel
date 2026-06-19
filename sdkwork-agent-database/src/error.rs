use thiserror::Error;

/// Database error type
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("connection error: {0}")]
    Connection(String),

    #[error("query error: {0}")]
    Query(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Database result type
pub type DatabaseResult<T> = Result<T, DatabaseError>;

impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::Serialization(err.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::QueryReturnedNoRows => {
                DatabaseError::NotFound("row not found".to_string())
            }
            rusqlite::Error::SqliteFailure(_, Some(msg)) => DatabaseError::Query(msg),
            _ => DatabaseError::Internal(err.to_string()),
        }
    }
}
