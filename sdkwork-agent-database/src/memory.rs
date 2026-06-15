use crate::error::{DatabaseError, DatabaseResult};
use crate::traits::*;
use crate::types::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory database implementation for testing
#[derive(Clone)]
pub struct InMemoryDatabase {
    sessions: Arc<Mutex<HashMap<String, SessionRow>>>,
    messages: Arc<Mutex<Vec<MessageRow>>>,
    tasks: Arc<Mutex<HashMap<String, TaskRow>>>,
    events: Arc<Mutex<Vec<EventRow>>>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDatabase for InMemoryDatabase {
    fn execute(&self, _sql: &str, _params: &[&dyn DatabaseParam]) -> DatabaseResult<usize> {
        Ok(0)
    }

    fn query_many(&self, _sql: &str, _params: &[&dyn DatabaseParam]) -> DatabaseResult<Vec<Box<dyn DatabaseRow>>> {
        Ok(Vec::new())
    }

    fn health(&self) -> DatabaseResult<bool> {
        Ok(true)
    }
}

impl SessionRepository for InMemoryDatabase {
    fn save_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let mut sessions = self.sessions.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<SessionRow>> {
        let sessions = self.sessions.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        Ok(sessions.get(session_id).cloned())
    }

    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<SessionRow>> {
        let sessions = self.sessions.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;

        let mut results: Vec<SessionRow> = sessions
            .values()
            .filter(|s| {
                if let Some(ref agent_id) = query.agent_id {
                    if s.agent_id != *agent_id {
                        return false;
                    }
                }
                if let Some(ref state) = query.state {
                    if s.state != *state {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }

        Ok(results)
    }

    fn update_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let mut sessions = self.sessions.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    fn delete_session(&self, session_id: &str) -> DatabaseResult<()> {
        let mut sessions = self.sessions.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        sessions.remove(session_id);
        Ok(())
    }
}

impl MessageRepository for InMemoryDatabase {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()> {
        let mut messages = self.messages.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        messages.push(message.clone());
        Ok(())
    }

    fn load_messages(&self, session_id: &str, query: &MessageQuery) -> DatabaseResult<Vec<MessageRow>> {
        let messages = self.messages.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;

        let mut results: Vec<MessageRow> = messages
            .iter()
            .filter(|m| m.session_id == session_id)
            .cloned()
            .collect();

        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }

        Ok(results)
    }

    fn message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let messages = self.messages.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        Ok(messages.iter().filter(|m| m.session_id == session_id).count() as i64)
    }

    fn delete_messages(&self, session_id: &str) -> DatabaseResult<()> {
        let mut messages = self.messages.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        messages.retain(|m| m.session_id != session_id);
        Ok(())
    }
}

impl TaskRepository for InMemoryDatabase {
    fn save_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let mut tasks = self.tasks.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        tasks.insert(task.task_id.clone(), task.clone());
        Ok(())
    }

    fn load_task(&self, task_id: &str) -> DatabaseResult<Option<TaskRow>> {
        let tasks = self.tasks.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        Ok(tasks.get(task_id).cloned())
    }

    fn load_tasks(&self, session_id: &str) -> DatabaseResult<Vec<TaskRow>> {
        let tasks = self.tasks.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        Ok(tasks.values().filter(|t| t.session_id == session_id).cloned().collect())
    }

    fn update_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let mut tasks = self.tasks.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        tasks.insert(task.task_id.clone(), task.clone());
        Ok(())
    }

    fn delete_task(&self, task_id: &str) -> DatabaseResult<()> {
        let mut tasks = self.tasks.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        tasks.remove(task_id);
        Ok(())
    }
}

impl EventRepository for InMemoryDatabase {
    fn save_event(&self, event: &EventRow) -> DatabaseResult<()> {
        let mut events = self.events.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        events.push(event.clone());
        Ok(())
    }

    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let events = self.events.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;

        let results: Vec<EventRow> = events
            .iter()
            .filter(|e| {
                e.session_id.as_deref() == Some(session_id)
                    && (query.event_type.is_none() || e.event_type == *query.event_type.as_ref().unwrap())
                    && (query.severity.is_none() || e.severity == *query.severity.as_ref().unwrap())
            })
            .cloned()
            .collect();

        Ok(results)
    }

    fn delete_events(&self, session_id: &str) -> DatabaseResult<()> {
        let mut events = self.events.lock().map_err(|e| {
            DatabaseError::Internal(format!("failed to acquire lock: {}", e))
        })?;
        events.retain(|e| e.session_id.as_deref() != Some(session_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_session() {
        let db = InMemoryDatabase::new();
        let session = SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "cli".to_string(),
            state: "active".to_string(),
            title: Some("Test".to_string()),
            model: Some("gpt-4".to_string()),
            cwd: None,
            token_usage_json: None,
            message_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        };

        db.save_session(&session).expect("saved");
        let loaded = db.load_session("session.1").expect("loaded");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().session_id, "session.1");
    }

    #[test]
    fn list_sessions_with_filter() {
        let db = InMemoryDatabase::new();
        db.save_session(&SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "cli".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            token_usage_json: None,
            message_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        }).expect("saved");

        let query = SessionQuery {
            agent_id: Some("agent.1".to_string()),
            ..Default::default()
        };
        let sessions = db.list_sessions(&query).expect("listed");
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn save_and_load_messages() {
        let db = InMemoryDatabase::new();
        let message = MessageRow {
            message_id: "msg.1".to_string(),
            session_id: "session.1".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata_json: None,
        };

        db.save_message(&message).expect("saved");
        let messages = db.load_messages("session.1", &MessageQuery::default()).expect("loaded");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }

    #[test]
    fn health_check() {
        let db = InMemoryDatabase::new();
        assert!(db.health().expect("health"));
    }
}
