use crate::types::MessageConfig;
use sdkwork_agent_database::{MessageQuery, MessageRepository, MessageRow};

/// Manages conversation history for a session
pub struct ConversationManager<M: MessageRepository> {
    message_repo: M,
}

impl<M: MessageRepository> ConversationManager<M> {
    pub fn new(message_repo: M) -> Self {
        Self { message_repo }
    }

    /// Send a message and store it
    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let message_id = format!("msg.{}", generate_id());

        let row = MessageRow {
            message_id,
            session_id: session_id.to_string(),
            role: config.role,
            content: config.content,
            created_at: now,
            metadata_json: config.metadata.map(|v| v.to_string()),
        };

        self.message_repo
            .save_message(&row)
            .map_err(|e| format!("failed to save message: {}", e))?;

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
        self.message_repo
            .delete_messages(session_id)
            .map_err(|e| format!("failed to delete messages: {}", e))
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

    #[test]
    fn send_and_get_messages() {
        let db = InMemoryDatabase::new();
        let manager = ConversationManager::new(db);

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
    }

    #[test]
    fn message_count() {
        let db = InMemoryDatabase::new();
        let manager = ConversationManager::new(db);

        manager
            .send_message("session.1", MessageConfig::user("Hello"))
            .expect("sent");
        assert_eq!(manager.message_count("session.1").expect("count"), 1);
    }

    #[test]
    fn clear_history() {
        let db = InMemoryDatabase::new();
        let manager = ConversationManager::new(db);

        manager
            .send_message("session.1", MessageConfig::user("Hello"))
            .expect("sent");
        manager.clear_history("session.1").expect("cleared");
        assert_eq!(manager.message_count("session.1").expect("count"), 0);
    }
}
