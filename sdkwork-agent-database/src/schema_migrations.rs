//! Shared agent-runtime schema migration authority.

use crate::error::{DatabaseError, DatabaseResult};
use crate::traits::AgentDatabase;

pub const SQLITE_MIGRATION_SQL: &str = include_str!("../migrations/agent_runtime.sqlite.sql");

#[cfg(any(feature = "postgres-sync", test))]
pub const POSTGRES_MIGRATION_SQL: &str = include_str!("../migrations/agent_runtime.postgres.sql");

/// Apply SQLite migrations using a native rusqlite connection.
pub fn apply_sqlite_connection(conn: &rusqlite::Connection) -> DatabaseResult<()> {
    for statement in sqlite_migration_statements() {
        if let Err(error) = conn.execute(&statement, []) {
            let message = error.to_string().to_lowercase();
            if should_ignore_sqlite_migration_error(&statement, &message) {
                continue;
            }
            return Err(DatabaseError::Migration(format!(
                "statement failed ({message}): {}",
                statement.lines().next().unwrap_or(&statement)
            )));
        }
    }
    Ok(())
}

/// Apply SQLite migrations statement-by-statement through the generic database trait.
pub fn migrate_sqlite(db: &dyn AgentDatabase) -> DatabaseResult<()> {
    for statement in sqlite_migration_statements() {
        if let Err(error) = db.execute(&statement, &[]) {
            let message = error.to_string().to_lowercase();
            if should_ignore_sqlite_migration_error(&statement, &message) {
                continue;
            }
            return Err(error);
        }
    }
    Ok(())
}

fn should_ignore_sqlite_migration_error(statement: &str, message: &str) -> bool {
    if !statement
        .trim_start()
        .to_ascii_uppercase()
        .starts_with("ALTER TABLE")
    {
        return false;
    }
    message.contains("duplicate column") || message.contains("not an error")
}

pub fn sqlite_migration_statements() -> impl Iterator<Item = String> + Clone {
    SQLITE_MIGRATION_SQL
        .split(';')
        .map(strip_sql_line_comments)
        .filter(|statement| !statement.is_empty())
}

fn strip_sql_line_comments(chunk: &str) -> String {
    chunk
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED_SCHEMA_OBJECTS: &[&str] = &[
        "sessions",
        "messages",
        "tasks",
        "events",
        "permissions",
        "idx_messages_session_created_at",
        "idx_events_session_created_at",
        "idx_events_created_at",
        "idx_permissions_status_created_at",
        "ON DELETE CASCADE",
    ];

    #[test]
    fn migration_sql_contains_required_objects_on_both_backends() {
        for object in REQUIRED_SCHEMA_OBJECTS {
            assert!(
                SQLITE_MIGRATION_SQL.contains(object),
                "sqlite migration missing {object}"
            );
            assert!(
                POSTGRES_MIGRATION_SQL.contains(object),
                "postgres migration missing {object}"
            );
        }
    }

    #[test]
    fn sqlite_migration_statements_are_non_empty() {
        let count = sqlite_migration_statements().count();
        assert!(
            count >= 10,
            "expected multiple sqlite migration statements, got {count}"
        );
    }

    #[test]
    fn sqlite_migration_applies_on_in_memory_database() {
        let conn = rusqlite::Connection::open_in_memory().expect("memory db");
        conn.execute_batch("PRAGMA foreign_keys=ON")
            .expect("foreign keys");
        apply_sqlite_connection(&conn).expect("sqlite migration should apply");
    }
}
