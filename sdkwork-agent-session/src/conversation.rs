use crate::types::MessageConfig;
use sdkwork_agent_database::{
    EventRow, MessageQuery, MessageRepository, MessageRow, RuntimeSessionWrites,
};
use std::sync::Arc;

/// Manages conversation history for a session
pub struct ConversationManager<M>
where
    M: MessageRepository + RuntimeSessionWrites,
{
    message_repo: M,
    event_listener: Option<Arc<dyn Fn(EventRow) + Send + Sync>>,
}

impl<M> ConversationManager<M>
where
    M: MessageRepository + RuntimeSessionWrites,
{
    pub fn new(message_repo: M) -> Self {
        Self {
            message_repo,
            event_listener: None,
        }
    }

    pub fn set_event_listener(&mut self, listener: Arc<dyn Fn(EventRow) + Send + Sync>) {
        self.event_listener = Some(listener);
    }

    /// Send a message and store it
    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        let now = sdkwork_agent_database::runtime_now_timestamp();
        let message_id = format!("msg.{}", generate_id());

        let row = MessageRow {
            message_id,
            session_id: session_id.to_string(),
            role: config.role,
            content: config.content,
            created_at: now,
            metadata_json: config.metadata.map(|v| v.to_string()),
        };

        let event = EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: "message.sent".to_string(),
            severity: "info".to_string(),
            payload: Some(format!("role={}", row.role)),
            created_at: sdkwork_agent_database::runtime_now_timestamp(),
        };
        self.message_repo
            .append_message_with_event(&row, &event)
            .map_err(|e| format!("failed to append message: {e}"))?;
        if let Some(listener) = &self.event_listener {
            listener(event);
        }

        Ok(row)
    }

    /// Get conversation history
    pub fn get_history(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRow>, String> {
        let query = MessageQuery {
            limit,
            offset: None,
            ..Default::default()
        };
        self.message_repo
            .load_messages(session_id, &query)
            .map_err(|e| format!("failed to load messages: {}", e))
    }

    /// Get message count
    pub fn message_count(&self, session_id: &str) -> Result<i64, String> {
        self.message_repo
            .message_count(session_id)
            .map_err(|e| format!("failed to count messages: {}", e))
    }

    /// Clear conversation history
    pub fn clear_history(&self, session_id: &str) -> Result<(), String> {
        let updated_at = sdkwork_agent_database::runtime_now_timestamp();
        let event = EventRow {
            event_id: format!("evt.{}", generate_id()),
            session_id: Some(session_id.to_string()),
            event_type: "session.updated".to_string(),
            severity: "info".to_string(),
            payload: Some("messages_cleared=true".to_string()),
            created_at: updated_at.clone(),
        };
        self.message_repo
            .delete_messages_and_reset_count_with_event(session_id, &updated_at, &event)
            .map_err(|e| format!("failed to delete messages: {e}"))?;
        if let Some(listener) = &self.event_listener {
            listener(event);
        }
        Ok(())
    }
}

/// Generate a collision-resistant runtime ID.
fn generate_id() -> String {
    sdkwork_utils_rust::uuid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_database::{
        EventQuery, EventRepository, InMemoryDatabase, SessionRepository, SessionRow,
    };

    fn database_with_session() -> InMemoryDatabase {
        let db = InMemoryDatabase::new();
        db.save_session(&SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "test".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-07-15T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("session");
        db
    }

    #[test]
    fn send_and_get_messages() {
        let db = database_with_session();
        let manager = ConversationManager::new(db.clone());

        manager
            .send_message("session.1", MessageConfig::user("Hello"))
            .expect("sent");
        manager
            .send_message("session.1", MessageConfig::assistant("Hi there!"))
            .expect("sent");

        let messages = manager.get_history("session.1", None).expect("loaded");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "Hi there!");
        assert_eq!(
            db.load_session("session.1")
                .expect("load")
                .expect("session")
                .message_count,
            2
        );
        assert_eq!(
            db.load_events("session.1", &EventQuery::default())
                .expect("events")
                .len(),
            2
        );
    }

    #[test]
    fn message_count() {
        let db = database_with_session();
        let manager = ConversationManager::new(db);

        manager
            .send_message("session.1", MessageConfig::user("Hello"))
            .expect("sent");
        assert_eq!(manager.message_count("session.1").expect("count"), 1);
    }

    #[test]
    fn clear_history() {
        let db = database_with_session();
        let manager = ConversationManager::new(db.clone());

        manager
            .send_message("session.1", MessageConfig::user("Hello"))
            .expect("sent");
        manager.clear_history("session.1").expect("cleared");
        assert_eq!(manager.message_count("session.1").expect("count"), 0);
        assert_eq!(
            db.load_session("session.1")
                .expect("load")
                .expect("session")
                .message_count,
            0
        );
        assert!(db
            .load_events("session.1", &EventQuery::default())
            .expect("events")
            .iter()
            .any(|event| {
                event.event_type == "session.updated"
                    && event.payload.as_deref() == Some("messages_cleared=true")
            }));
    }

    #[test]
    fn send_rejects_missing_and_terminal_sessions() {
        let db = database_with_session();
        let manager = ConversationManager::new(db.clone());
        assert!(manager
            .send_message("session.missing", MessageConfig::user("missing"))
            .is_err());

        let mut session = db
            .load_session("session.1")
            .expect("load")
            .expect("session");
        session.state = "closed".to_string();
        db.update_session(&session).expect("close");
        assert!(manager
            .send_message("session.1", MessageConfig::user("late"))
            .is_err());
    }
}
