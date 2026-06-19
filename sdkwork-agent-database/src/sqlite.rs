use crate::error::{DatabaseError, DatabaseResult};
use crate::traits::*;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// SQLite database implementation
#[derive(Clone)]
pub struct SqliteDatabase {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl SqliteDatabase {
    /// Create a new SQLite database connection
    pub fn new(path: &str) -> DatabaseResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| DatabaseError::Connection(format!("failed to open database: {}", e)))?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .map_err(|e| DatabaseError::Connection(format!("failed to set WAL mode: {}", e)))?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys=ON").map_err(|e| {
            DatabaseError::Connection(format!("failed to enable foreign keys: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory SQLite database for testing
    pub fn memory() -> DatabaseResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            DatabaseError::Connection(format!("failed to open in-memory database: {}", e))
        })?;

        conn.execute_batch("PRAGMA foreign_keys=ON").map_err(|e| {
            DatabaseError::Connection(format!("failed to enable foreign keys: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl AgentDatabase for SqliteDatabase {
    fn execute(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let param_values: Vec<String> = params.iter().map(|p| p.as_sql_value()).collect();
        let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(sql, rusqlite_params.as_slice())
            .map_err(|e| DatabaseError::Query(format!("failed to execute: {}", e)))
    }

    fn query_many(
        &self,
        sql: &str,
        params: &[&dyn DatabaseParam],
    ) -> DatabaseResult<Vec<Box<dyn DatabaseRow>>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let param_values: Vec<String> = params.iter().map(|p| p.as_sql_value()).collect();
        let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DatabaseError::Query(format!("failed to prepare: {}", e)))?;

        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let mut rows = stmt
            .query(rusqlite_params.as_slice())
            .map_err(|e| DatabaseError::Query(format!("failed to query: {}", e)))?;

        let mut result = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| DatabaseError::Query(format!("failed to fetch row: {}", e)))?
        {
            let mut values = HashMap::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let value: rusqlite::types::Value = row
                    .get(i)
                    .map_err(|e| DatabaseError::Query(format!("failed to get value: {}", e)))?;
                values.insert(col_name.clone(), value);
            }
            result.push(Box::new(SqliteRow { values }) as Box<dyn DatabaseRow>);
        }

        Ok(result)
    }

    fn health(&self) -> DatabaseResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let result: i32 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .map_err(|e| DatabaseError::Query(format!("health check failed: {}", e)))?;

        Ok(result == 1)
    }
}

/// SQLite row wrapper
struct SqliteRow {
    values: HashMap<String, rusqlite::types::Value>,
}

impl DatabaseRow for SqliteRow {
    fn get_string(&self, column: &str) -> DatabaseResult<String> {
        let value = self
            .values
            .get(column)
            .ok_or_else(|| DatabaseError::Query(format!("column not found: {}", column)))?;

        match value {
            rusqlite::types::Value::Text(s) => Ok(s.clone()),
            rusqlite::types::Value::Integer(i) => Ok(i.to_string()),
            rusqlite::types::Value::Real(f) => Ok(f.to_string()),
            _ => Err(DatabaseError::Query(format!(
                "column '{}' is not a string",
                column
            ))),
        }
    }

    fn get_optional_string(&self, column: &str) -> DatabaseResult<Option<String>> {
        let value = self
            .values
            .get(column)
            .ok_or_else(|| DatabaseError::Query(format!("column not found: {}", column)))?;

        match value {
            rusqlite::types::Value::Null => Ok(None),
            rusqlite::types::Value::Text(s) => Ok(Some(s.clone())),
            rusqlite::types::Value::Integer(i) => Ok(Some(i.to_string())),
            rusqlite::types::Value::Real(f) => Ok(Some(f.to_string())),
            _ => Err(DatabaseError::Query(format!(
                "column '{}' is not a string",
                column
            ))),
        }
    }

    fn get_i64(&self, column: &str) -> DatabaseResult<i64> {
        let value = self
            .values
            .get(column)
            .ok_or_else(|| DatabaseError::Query(format!("column not found: {}", column)))?;

        match value {
            rusqlite::types::Value::Integer(i) => Ok(*i),
            _ => Err(DatabaseError::Query(format!(
                "column '{}' is not an integer",
                column
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_memory_works() {
        let db = SqliteDatabase::memory().expect("created");
        assert!(db.health().expect("health"));
    }

    #[test]
    fn sqlite_execute_and_query() {
        let db = SqliteDatabase::memory().expect("created");
        db.execute("CREATE TABLE test (id TEXT PRIMARY KEY, name TEXT)", &[])
            .expect("created");
        db.execute(
            "INSERT INTO test (id, name) VALUES (?1, ?2)",
            &[&"1", &"test"],
        )
        .expect("inserted");

        let rows = db
            .query_many("SELECT id, name FROM test WHERE id = ?1", &[&"1"])
            .expect("queried");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get_string("id").expect("id"), "1");
        assert_eq!(rows[0].get_string("name").expect("name"), "test");
    }
}
