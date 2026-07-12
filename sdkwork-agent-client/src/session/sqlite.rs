use crate::session::query::{sort_bridge_sessions, BridgeSessionQuery};
use crate::types::*;
use sdkwork_agent_database::{
    MessageQuery, MessageRepository, SessionQuery, SessionRepository, SqliteDatabase,
};
use uuid::Uuid;

pub fn default_client_database_path() -> String {
    std::env::var("SDKWORK_CLIENT_DATABASE_PATH")
        .unwrap_or_else(|_| ".sdkwork/agent-client.sqlite".to_string())
}

pub fn session_info_from_row(row: &sdkwork_agent_database::SessionRow) -> SessionInfo {
    SessionInfo {
        session_id: row.session_id.clone(),
        agent_id: row.agent_id.clone(),
        provider_id: row.provider_id.clone().unwrap_or_default(),
        bridge_id: row.bridge_id.clone().unwrap_or_default(),
        model: row.model.clone(),
        title: row.title.clone(),
        state: row.state.clone(),
        message_count: row.message_count.max(0) as u32,
        created_at: row.created_at.clone(),
        updated_at: row
            .updated_at
            .clone()
            .unwrap_or_else(|| row.created_at.clone()),
    }
}

pub fn message_from_row(row: &sdkwork_agent_database::MessageRow) -> ChatMessage {
    ChatMessage {
        id: row.message_id.clone(),
        role: match row.role.as_str() {
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            "tool" => MessageRole::Tool,
            _ => MessageRole::User,
        },
        content: row.content.clone(),
        timestamp: row.created_at.clone(),
        metadata: None,
    }
}

pub fn session_row_from_info(info: &SessionInfo) -> sdkwork_agent_database::SessionRow {
    sdkwork_agent_database::SessionRow {
        session_id: info.session_id.clone(),
        agent_id: info.agent_id.clone(),
        kind: "main".to_string(),
        source: "bridge".to_string(),
        state: info.state.clone(),
        title: info.title.clone(),
        model: info.model.clone(),
        cwd: None,
        provider_id: Some(info.provider_id.clone()),
        bridge_id: Some(info.bridge_id.clone()),
        token_usage_json: None,
        message_count: info.message_count as i64,
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: info.created_at.clone(),
        updated_at: Some(info.updated_at.clone()),
        metadata_json: None,
    }
}

/// SQLite-backed bridge session store for client runtimes.
#[derive(Clone)]
pub struct SqliteBridgeSessionStore {
    db: SqliteDatabase,
    provider_id: String,
    bridge_id: String,
}

impl SqliteBridgeSessionStore {
    pub fn open(
        database_path: impl AsRef<std::path::Path>,
        provider_id: impl Into<String>,
        bridge_id: impl Into<String>,
    ) -> Result<Self, String> {
        let path = database_path.as_ref().to_string_lossy().to_string();
        let db = SqliteDatabase::open_migrated(&path)
            .map_err(|error| format!("failed to open sqlite database: {error}"))?;
        Ok(Self {
            db,
            provider_id: provider_id.into(),
            bridge_id: bridge_id.into(),
        })
    }

    pub fn open_default(
        provider_id: impl Into<String>,
        bridge_id: impl Into<String>,
    ) -> Result<Self, String> {
        Self::open(default_client_database_path(), provider_id, bridge_id)
    }

    pub fn memory(
        provider_id: impl Into<String>,
        bridge_id: impl Into<String>,
    ) -> Result<Self, String> {
        let db = SqliteDatabase::memory_migrated()
            .map_err(|error| format!("failed to open sqlite memory database: {error}"))?;
        Ok(Self {
            db,
            provider_id: provider_id.into(),
            bridge_id: bridge_id.into(),
        })
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let session = SessionInfo {
            session_id: format!("{}.{}", self.provider_id, Uuid::new_v4()),
            agent_id: config.agent_id,
            provider_id: self.provider_id.clone(),
            bridge_id: self.bridge_id.clone(),
            model: config.model,
            title: config.title,
            state: "active".to_string(),
            message_count: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        self.db
            .save_session(&session_row_from_info(&session))
            .map_err(|error| format!("failed to save session: {error}"))?;
        Ok(session)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        let mut session = self
            .db
            .load_session(session_id)
            .map_err(|error| format!("failed to load session: {error}"))?
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        session.state = "closed".to_string();
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());
        self.db
            .update_session(&session)
            .map_err(|error| format!("failed to close session: {error}"))
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        if self
            .db
            .load_session(session_id)
            .map_err(|error| format!("failed to load session: {error}"))?
            .is_none()
        {
            return Err(format!("session not found: {session_id}"));
        }
        let mut messages = self
            .db
            .load_messages(
                session_id,
                &MessageQuery {
                    after_message_id: None,
                    after_message_created_at: None,
                    limit: limit.map(i64::from),
                    offset: None,
                },
            )
            .map_err(|error| format!("failed to load messages: {error}"))?;
        if let Some(limit) = limit {
            let keep = limit as usize;
            if messages.len() > keep {
                messages = messages.split_off(messages.len() - keep);
            }
        }
        Ok(messages
            .into_iter()
            .map(|row| message_from_row(&row))
            .collect())
    }

    pub fn append_message(&self, session_id: &str, message: ChatMessage) -> Result<(), String> {
        let mut session = self
            .db
            .load_session(session_id)
            .map_err(|error| format!("failed to load session: {error}"))?
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        self.db
            .save_message(&sdkwork_agent_database::MessageRow {
                message_id: message.id.clone(),
                session_id: session_id.to_string(),
                role: match message.role {
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                    MessageRole::User => "user".to_string(),
                },
                content: message.content,
                created_at: message.timestamp,
                metadata_json: None,
            })
            .map_err(|error| format!("failed to save message: {error}"))?;
        session.message_count = session.message_count.saturating_add(1);
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());
        self.db
            .update_session(&session)
            .map_err(|error| format!("failed to update session: {error}"))
    }

    pub fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        let mut sessions = self
            .db
            .list_sessions(&SessionQuery {
                agent_id: query.agent_id.clone(),
                state: if query.active_only {
                    Some("active".to_string())
                } else {
                    None
                },
                kind: None,
                provider_id: query
                    .provider_id
                    .clone()
                    .or_else(|| Some(self.provider_id.clone())),
                bridge_id: query
                    .bridge_id
                    .clone()
                    .or_else(|| Some(self.bridge_id.clone())),
                owner_tenant_id: None,
                owner_user_ref: None,
                after_session_id: None,
                after_session_sort_at: None,
                limit: query.limit.map(i64::from),
                offset: None,
            })
            .map_err(|error| format!("failed to list sessions: {error}"))?;
        let mut mapped: Vec<SessionInfo> = sessions
            .drain(..)
            .map(|row| session_info_from_row(&row))
            .collect();
        sort_bridge_sessions(&mut mapped);
        Ok(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_store_roundtrip() {
        let store = SqliteBridgeSessionStore::memory("codex", "bridge.codex").expect("store");
        let session = store
            .create_session(SessionConfig::new("agent.1").with_title("Codex"))
            .expect("created");
        store
            .append_message(
                &session.session_id,
                ChatMessage {
                    id: "msg.1".to_string(),
                    role: MessageRole::User,
                    content: "hello".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    metadata: None,
                },
            )
            .expect("append");
        let messages = store
            .get_messages(&session.session_id, None)
            .expect("messages");
        assert_eq!(messages.len(), 1);
        let listed = store
            .list_sessions(&BridgeSessionQuery::default())
            .expect("listed");
        assert_eq!(listed.len(), 1);
    }
}
