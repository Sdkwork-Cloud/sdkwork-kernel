use crate::conversation::ConversationManager;
use crate::types::{MessageConfig, SessionConfig, SessionQuery};
use sdkwork_agent_database::{
    AgentDatabase, EventRepository, EventRow, MessageRepository, MessageRow, SessionRepository,
    SessionRow, TaskRepository, TaskRow,
};

/// Unified session manager that integrates database persistence
pub struct UnifiedSessionManager<D, S, M, T, E>
where
    D: AgentDatabase,
    S: SessionRepository,
    M: MessageRepository,
    T: TaskRepository,
    E: EventRepository,
{
    db: D,
    session_repo: S,
    message_repo: M,
    task_repo: T,
    event_repo: E,
}

impl<D, S, M, T, E> UnifiedSessionManager<D, S, M, T, E>
where
    D: AgentDatabase,
    S: SessionRepository,
    M: MessageRepository + Clone,
    T: TaskRepository,
    E: EventRepository,
{
    pub fn new(db: D, session_repo: S, message_repo: M, task_repo: T, event_repo: E) -> Self {
        Self {
            db,
            session_repo,
            message_repo,
            task_repo,
            event_repo,
        }
    }

    /// Create a new session
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionRow, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let session_id = format!("session.{}", generate_id());

        let row = SessionRow {
            session_id: session_id.clone(),
            agent_id: config.agent_id,
            kind: config.kind.unwrap_or_else(|| "main".to_string()),
            source: config.source.unwrap_or_else(|| "api".to_string()),
            state: "active".to_string(),
            title: config.title,
            model: config.model,
            cwd: config.cwd,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            created_at: now.clone(),
            updated_at: Some(now),
            metadata_json: config
                .metadata
                .as_ref()
                .and_then(|value| serde_json::to_string(value).ok()),
        };

        self.session_repo
            .save_session(&row)
            .map_err(|e| format!("failed to save session: {}", e))?;

        // Record session creation event
        self.record_event(&session_id, "session.created", "info", None)?;

        Ok(row)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<SessionRow, String> {
        self.session_repo
            .load_session(session_id)
            .map_err(|e| format!("failed to load session: {}", e))?
            .ok_or_else(|| format!("session not found: {}", session_id))
    }

    /// List sessions with optional filters
    pub fn list_sessions(&self, query: SessionQuery) -> Result<Vec<SessionRow>, String> {
        let db_query = sdkwork_agent_database::SessionQuery {
            agent_id: query.agent_id,
            state: query.state,
            kind: query.kind,
            provider_id: query.provider_id,
            bridge_id: query.bridge_id,
            limit: query.limit,
            offset: query.offset,
        };

        self.session_repo
            .list_sessions(&db_query)
            .map_err(|e| format!("failed to list sessions: {}", e))
    }

    /// Update a session
    pub fn update_session(&self, session: &SessionRow) -> Result<(), String> {
        self.session_repo
            .update_session(session)
            .map_err(|e| format!("failed to update session: {}", e))
    }

    /// Close a session
    pub fn close_session(&self, session_id: &str) -> Result<SessionRow, String> {
        let mut session = self.get_session(session_id)?;
        session.state = "closed".to_string();
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());

        self.session_repo
            .update_session(&session)
            .map_err(|e| format!("failed to close session: {}", e))?;

        // Record session closure event
        self.record_event(session_id, "session.closed", "info", None)?;

        Ok(session)
    }

    /// Delete a session and all associated data
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        // Delete associated data first
        self.event_repo
            .delete_events(session_id)
            .map_err(|e| format!("failed to delete events: {}", e))?;

        self.message_repo
            .delete_messages(session_id)
            .map_err(|e| format!("failed to delete messages: {}", e))?;

        // Delete tasks
        let tasks = self
            .task_repo
            .load_tasks(session_id)
            .map_err(|e| format!("failed to load tasks: {}", e))?;
        for task in tasks {
            self.task_repo
                .delete_task(&task.task_id)
                .map_err(|e| format!("failed to delete task: {}", e))?;
        }

        // Delete session
        self.session_repo
            .delete_session(session_id)
            .map_err(|e| format!("failed to delete session: {}", e))?;

        Ok(())
    }

    /// Send a message in a session
    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        // Verify session exists
        let _session = self.get_session(session_id)?;

        let now = chrono::Utc::now().to_rfc3339();
        let message_id = format!("msg.{}", generate_id());

        let row = MessageRow {
            message_id,
            session_id: session_id.to_string(),
            role: config.role,
            content: config.content,
            created_at: now,
            metadata_json: config
                .metadata
                .as_ref()
                .and_then(|value| serde_json::to_string(value).ok()),
        };

        self.message_repo
            .save_message(&row)
            .map_err(|e| format!("failed to save message: {}", e))?;

        // Update session message count
        let mut session = self.get_session(session_id)?;
        session.message_count += 1;
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());
        self.update_session(&session)?;

        // Record message event
        self.record_event(
            session_id,
            "message.sent",
            "info",
            Some(&format!("role={}", row.role)),
        )?;

        Ok(row)
    }

    /// Get message history for a session
    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRow>, String> {
        let query = sdkwork_agent_database::MessageQuery {
            limit,
            offset: None,
        };
        self.message_repo
            .load_messages(session_id, &query)
            .map_err(|e| format!("failed to load messages: {}", e))
    }

    /// Get message count for a session
    pub fn message_count(&self, session_id: &str) -> Result<i64, String> {
        self.message_repo
            .message_count(session_id)
            .map_err(|e| format!("failed to count messages: {}", e))
    }

    /// Get a conversation manager for a session
    pub fn conversation(&self) -> ConversationManager<M> {
        ConversationManager::new(self.message_repo.clone())
    }

    /// Record an event
    fn record_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> Result<(), String> {
        let event = EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            payload: payload.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.event_repo
            .save_event(&event)
            .map_err(|e| format!("failed to save event: {}", e))?;

        Ok(())
    }

    /// Create a task in a session.
    pub fn create_task(&self, session_id: &str, instruction: &str) -> Result<TaskRow, String> {
        let _session = self.get_session(session_id)?;
        let now = chrono::Utc::now().to_rfc3339();
        let task = TaskRow {
            task_id: format!("task.{}", generate_id()),
            session_id: session_id.to_string(),
            instruction: instruction.to_string(),
            state: "created".to_string(),
            created_at: now.clone(),
            updated_at: Some(now),
        };
        self.task_repo
            .save_task(&task)
            .map_err(|e| format!("failed to save task: {}", e))?;
        self.record_event(session_id, "task.created", "info", Some(&task.task_id))?;
        Ok(task)
    }

    /// Load a task by id.
    pub fn get_task(&self, task_id: &str) -> Result<TaskRow, String> {
        self.task_repo
            .load_task(task_id)
            .map_err(|e| format!("failed to load task: {}", e))?
            .ok_or_else(|| format!("task not found: {}", task_id))
    }

    /// List tasks for a session.
    pub fn list_tasks(&self, session_id: &str) -> Result<Vec<TaskRow>, String> {
        self.get_session(session_id)?;
        self.task_repo
            .load_tasks(session_id)
            .map_err(|e| format!("failed to load tasks: {}", e))
    }

    /// Cancel a task.
    pub fn cancel_task(&self, task_id: &str) -> Result<TaskRow, String> {
        let mut task = self.get_task(task_id)?;
        task.state = "cancelled".to_string();
        task.updated_at = Some(chrono::Utc::now().to_rfc3339());
        self.task_repo
            .update_task(&task)
            .map_err(|e| format!("failed to cancel task: {}", e))?;
        self.record_event(
            &task.session_id,
            "task.cancelled",
            "info",
            Some(&task.task_id),
        )?;
        Ok(task)
    }

    /// Load recent events for a session.
    pub fn load_session_events(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<EventRow>, String> {
        self.get_session(session_id)?;
        let query = sdkwork_agent_database::EventQuery {
            event_type: None,
            severity: None,
            limit,
            offset: None,
        };
        self.event_repo
            .load_events(session_id, &query)
            .map_err(|e| format!("failed to load events: {}", e))
    }

    /// Health check
    pub fn health(&self) -> Result<bool, String> {
        self.db
            .health()
            .map_err(|e| format!("health check failed: {}", e))
    }
}

/// Generate a simple unique ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_database::InMemoryDatabase;

    fn create_manager() -> UnifiedSessionManager<
        InMemoryDatabase,
        InMemoryDatabase,
        InMemoryDatabase,
        InMemoryDatabase,
        InMemoryDatabase,
    > {
        let db = InMemoryDatabase::new();
        UnifiedSessionManager::new(db.clone(), db.clone(), db.clone(), db.clone(), db)
    }

    #[test]
    fn create_and_get_session() {
        let manager = create_manager();
        let config = SessionConfig::new("agent.1").with_title("Test Session");

        let session = manager.create_session(config).expect("created");
        assert_eq!(session.agent_id, "agent.1");
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert_eq!(session.state, "active");

        let loaded = manager.get_session(&session.session_id).expect("loaded");
        assert_eq!(loaded.session_id, session.session_id);
    }

    #[test]
    fn list_sessions() {
        let manager = create_manager();
        manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");
        manager
            .create_session(SessionConfig::new("agent.2"))
            .expect("created");

        let sessions = manager
            .list_sessions(SessionQuery::default())
            .expect("listed");
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn close_session() {
        let manager = create_manager();
        let session = manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");
        let closed = manager.close_session(&session.session_id).expect("closed");
        assert_eq!(closed.state, "closed");
    }

    #[test]
    fn send_message() {
        let manager = create_manager();
        let session = manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");

        let msg = manager
            .send_message(&session.session_id, MessageConfig::user("Hello"))
            .expect("sent");

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let messages = manager
            .get_messages(&session.session_id, None)
            .expect("loaded");
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn delete_session() {
        let manager = create_manager();
        let session = manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");
        manager
            .send_message(&session.session_id, MessageConfig::user("Hello"))
            .expect("sent");

        manager
            .delete_session(&session.session_id)
            .expect("deleted");

        let result = manager.get_session(&session.session_id);
        assert!(result.is_err());
    }

    #[test]
    fn health_check() {
        let manager = create_manager();
        assert!(manager.health().expect("health"));
    }

    #[test]
    fn create_and_cancel_task() {
        let manager = create_manager();
        let session = manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");
        let task = manager
            .create_task(&session.session_id, "run tests")
            .expect("task");
        assert_eq!(task.state, "created");
        let cancelled = manager.cancel_task(&task.task_id).expect("cancelled");
        assert_eq!(cancelled.state, "cancelled");
    }
}
