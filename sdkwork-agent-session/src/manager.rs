use crate::conversation::ConversationManager;
use crate::types::{MessageConfig, SessionConfig, SessionQuery};
use sdkwork_agent_database::{
    session_owner_fields_from_metadata_json, AgentDatabase, EventRepository, EventRow,
    MessageRepository, MessageRow, RuntimeSessionWrites, SessionRepository, SessionRow,
    TaskRepository, TaskRow,
};
use std::sync::Arc;

/// Unified session manager backed by one database type implementing all runtime repositories.
pub struct UnifiedSessionManager<DB>
where
    DB: AgentDatabase
        + SessionRepository
        + MessageRepository
        + TaskRepository
        + EventRepository
        + RuntimeSessionWrites
        + Clone,
{
    db: DB,
    event_listener: Option<Arc<dyn Fn(EventRow) + Send + Sync>>,
}

impl<DB> UnifiedSessionManager<DB>
where
    DB: AgentDatabase
        + SessionRepository
        + MessageRepository
        + TaskRepository
        + EventRepository
        + RuntimeSessionWrites
        + Clone,
{
    pub fn new(db: DB) -> Self {
        Self {
            db,
            event_listener: None,
        }
    }

    /// Register a listener invoked after each persisted session event.
    pub fn set_event_listener(&mut self, listener: Arc<dyn Fn(EventRow) + Send + Sync>) {
        self.event_listener = Some(listener);
    }

    /// Create a new session
    pub fn create_session(&self, config: SessionConfig) -> Result<SessionRow, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let session_id = format!("session.{}", generate_id());

        let metadata_json = config
            .metadata
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok());
        let (owner_tenant_id, owner_user_ref) =
            session_owner_fields_from_metadata_json(&metadata_json);

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
            owner_tenant_id,
            owner_user_ref,
            created_at: now.clone(),
            updated_at: Some(now),
            metadata_json,
        };

        let event = self.build_event(&session_id, "session.created", "info", None);
        self.db
            .save_session_with_event(&row, &event)
            .map_err(|e| format!("failed to create session: {e}"))?;
        self.notify_event(event);

        Ok(row)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<SessionRow, String> {
        self.db
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
            owner_tenant_id: query.owner_tenant_id,
            owner_user_ref: query.owner_user_ref,
            after_session_id: query.after_session_id,
            after_session_sort_at: query.after_session_sort_at,
            limit: query.limit,
            offset: query.offset,
        };

        self.db
            .list_sessions(&db_query)
            .map_err(|e| format!("failed to list sessions: {}", e))
    }

    /// Update a session
    pub fn update_session(&self, session: &SessionRow) -> Result<(), String> {
        self.db
            .update_session(session)
            .map_err(|e| format!("failed to update session: {}", e))
    }

    /// Close a session
    pub fn close_session(&self, session_id: &str) -> Result<SessionRow, String> {
        let mut session = self.get_session(session_id)?;
        session.state = "closed".to_string();
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());

        let event = self.build_event(session_id, "session.closed", "info", None);
        self.db
            .save_session_with_event(&session, &event)
            .map_err(|e| format!("failed to close session: {e}"))?;
        self.notify_event(event);

        Ok(session)
    }

    /// Delete a session and all associated data
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        self.db
            .delete_session_cascade(session_id)
            .map_err(|e| format!("failed to delete session: {}", e))
    }

    /// Send a message in a session
    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        let session = self.get_session(session_id)?;
        if session.state == "closed" {
            return Err(format!("session {session_id} is closed"));
        }

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

        let event = EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: "message.sent".to_string(),
            severity: "info".to_string(),
            payload: Some(format!("role={}", row.role)),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.db
            .append_message_with_event(&row, &event)
            .map_err(|e| format!("failed to append message: {}", e))?;

        if let Some(listener) = &self.event_listener {
            listener(event);
        }

        Ok(row)
    }

    /// Atomically persist a completed user turn, optional assistant reply, and lifecycle events.
    pub fn append_completed_turn(
        &self,
        session_id: &str,
        user_content: String,
        assistant_content: Option<String>,
    ) -> Result<(MessageRow, Option<MessageRow>), String> {
        let session = self.get_session(session_id)?;
        if !session.state.eq_ignore_ascii_case("active") {
            return Err(format!("session {session_id} is not active"));
        }

        let turn_started = chrono::Utc::now();
        let user_row = MessageRow {
            message_id: format!("msg.{}", generate_id()),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: user_content,
            created_at: turn_started.to_rfc3339(),
            metadata_json: None,
        };
        let assistant_row = assistant_content.map(|content| MessageRow {
            message_id: format!("msg.{}", generate_id()),
            session_id: session_id.to_string(),
            role: "assistant".to_string(),
            content,
            created_at: (turn_started + chrono::Duration::nanoseconds(1)).to_rfc3339(),
            metadata_json: None,
        });

        let mut messages = vec![user_row.clone()];
        if let Some(row) = &assistant_row {
            messages.push(row.clone());
        }
        let mut events = Vec::with_capacity(messages.len() + 1);
        for (index, message) in messages.iter().enumerate() {
            events.push(EventRow {
                event_id: format!("evt.{}", generate_id()),
                session_id: Some(session_id.to_string()),
                event_type: "message.sent".to_string(),
                severity: "info".to_string(),
                payload: Some(format!("role={}", message.role)),
                created_at: (turn_started
                    + chrono::Duration::nanoseconds(2 + i64::try_from(index).unwrap_or(0)))
                .to_rfc3339(),
            });
        }
        events.push(EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: "turn.completed".to_string(),
            severity: "info".to_string(),
            payload: Some(format!("user_message_id={}", user_row.message_id)),
            created_at: (turn_started
                + chrono::Duration::nanoseconds(2 + i64::try_from(messages.len()).unwrap_or(0)))
            .to_rfc3339(),
        });

        self.db
            .append_message_turn_with_events(&messages, &events)
            .map_err(|e| format!("failed to append completed message turn: {e}"))?;
        for event in events {
            self.notify_event(event);
        }

        Ok((user_row, assistant_row))
    }

    /// Delete all messages in a session and reset the cached message count.
    pub fn delete_messages(&self, session_id: &str) -> Result<(), String> {
        self.get_session(session_id)?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.db
            .delete_messages_and_reset_count(session_id, &updated_at)
            .map_err(|e| format!("failed to delete messages: {}", e))
    }

    /// List messages with full query parameters (offset or keyset continuation).
    pub fn list_messages(
        &self,
        session_id: &str,
        query: sdkwork_agent_database::MessageQuery,
    ) -> Result<Vec<MessageRow>, String> {
        self.db
            .load_messages(session_id, &query)
            .map_err(|e| format!("failed to load messages: {}", e))
    }

    /// Get message history for a session
    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<MessageRow>, String> {
        let query = sdkwork_agent_database::MessageQuery {
            limit,
            offset,
            ..Default::default()
        };
        self.db
            .load_messages(session_id, &query)
            .map_err(|e| format!("failed to load messages: {}", e))
    }

    /// Load the most recent bounded message window in chronological order.
    pub fn load_recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageRow>, String> {
        self.db
            .load_recent_messages(session_id, limit)
            .map_err(|e| format!("failed to load recent messages: {e}"))
    }

    /// Get message count for a session
    pub fn message_count(&self, session_id: &str) -> Result<i64, String> {
        self.db
            .message_count(session_id)
            .map_err(|e| format!("failed to count messages: {}", e))
    }

    /// Get a conversation manager for a session
    pub fn conversation(&self) -> ConversationManager<DB> {
        ConversationManager::new(self.db.clone())
    }

    /// Emit a persisted session event to storage and optional listeners.
    pub fn emit_session_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> Result<(), String> {
        self.record_event(session_id, event_type, severity, payload)
    }

    /// Record an event
    fn record_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> Result<(), String> {
        let event = self.build_event(session_id, event_type, severity, payload);

        self.db
            .save_event(&event)
            .map_err(|e| format!("failed to save event: {}", e))?;

        self.notify_event(event);

        Ok(())
    }

    fn build_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> EventRow {
        EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            payload: payload.map(str::to_string),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn notify_event(&self, event: EventRow) {
        if let Some(listener) = &self.event_listener {
            listener(event);
        }
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
        let event = self.build_event(session_id, "task.created", "info", Some(&task.task_id));
        self.db
            .save_task_with_event(&task, &event)
            .map_err(|e| format!("failed to create task: {e}"))?;
        self.notify_event(event);
        Ok(task)
    }

    /// Load a task by id.
    pub fn get_task(&self, task_id: &str) -> Result<TaskRow, String> {
        self.db
            .load_task(task_id)
            .map_err(|e| format!("failed to load task: {}", e))?
            .ok_or_else(|| format!("task not found: {}", task_id))
    }

    /// List tasks for a session.
    pub fn list_tasks(
        &self,
        session_id: &str,
        query: sdkwork_agent_database::TaskQuery,
    ) -> Result<Vec<TaskRow>, String> {
        self.get_session(session_id)?;
        self.db
            .load_tasks(session_id, &query)
            .map_err(|e| format!("failed to load tasks: {}", e))
    }

    /// Cancel a task.
    pub fn cancel_task(&self, task_id: &str) -> Result<TaskRow, String> {
        let mut task = self.get_task(task_id)?;
        task.state = "cancelled".to_string();
        task.updated_at = Some(chrono::Utc::now().to_rfc3339());
        let event = self.build_event(
            &task.session_id,
            "task.cancelled",
            "info",
            Some(&task.task_id),
        );
        self.db
            .save_task_with_event(&task, &event)
            .map_err(|e| format!("failed to cancel task: {e}"))?;
        self.notify_event(event);
        Ok(task)
    }

    /// Load recent events for a session.
    pub fn load_session_events(
        &self,
        session_id: &str,
        limit: Option<i64>,
        after_event_id: Option<&str>,
    ) -> Result<Vec<EventRow>, String> {
        self.get_session(session_id)?;
        let query = sdkwork_agent_database::EventQuery {
            event_type: None,
            severity: None,
            owner_tenant_id: None,
            owner_user_ref: None,
            after_event_id: after_event_id.map(str::to_string),
            limit,
            offset: None,
        };
        self.db
            .load_events(session_id, &query)
            .map_err(|e| format!("failed to load events: {}", e))
    }

    /// Load recent events across all sessions (newest first).
    pub fn list_recent_events(
        &self,
        query: sdkwork_agent_database::EventQuery,
    ) -> Result<Vec<EventRow>, String> {
        self.db
            .list_recent_events(&query)
            .map_err(|e| format!("failed to list recent events: {}", e))
    }

    /// Health check
    pub fn health(&self) -> Result<bool, String> {
        self.db
            .health()
            .map_err(|e| format!("health check failed: {}", e))
    }
}

/// Generate a collision-resistant runtime ID.
fn generate_id() -> String {
    sdkwork_utils_rust::uuid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_database::InMemoryDatabase;

    fn create_manager() -> UnifiedSessionManager<InMemoryDatabase> {
        UnifiedSessionManager::new(InMemoryDatabase::new())
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
            .get_messages(&session.session_id, None, None)
            .expect("loaded");
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn append_completed_turn_persists_messages_and_events_together() {
        let manager = create_manager();
        let session = manager
            .create_session(SessionConfig::new("agent.1"))
            .expect("created");

        let (user, assistant) = manager
            .append_completed_turn(
                &session.session_id,
                "Hello".to_string(),
                Some("Hi".to_string()),
            )
            .expect("completed turn");
        assert_eq!(user.role, "user");
        assert_eq!(
            assistant.as_ref().map(|row| row.role.as_str()),
            Some("assistant")
        );

        let messages = manager
            .get_messages(&session.session_id, None, None)
            .expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id, user.message_id);
        assert_eq!(
            messages[1].message_id,
            assistant.expect("assistant").message_id
        );
        assert_eq!(
            manager.message_count(&session.session_id).expect("count"),
            2
        );

        let events = manager
            .load_session_events(&session.session_id, Some(20), None)
            .expect("events");
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "message.sent")
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "turn.completed")
                .count(),
            1
        );
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
