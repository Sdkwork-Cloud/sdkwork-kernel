use crate::types::{generate_id, BridgeSessionConfig};
use sdkwork_agent_kernel::{
    AgentMessage, AgentSession, EventRecorder, KernelError, KernelResult, SessionKind,
    SessionSource,
};
use std::collections::HashMap;

/// Maximum in-bridge message history entries retained per session.
const MAX_SESSION_BRIDGE_HISTORY: usize = 512;
/// Maximum flattened message bytes retained per session.
const MAX_SESSION_BRIDGE_HISTORY_BYTES: usize = 4 * 1024 * 1024;
/// Hard bound for active transient sessions retained by one runtime process.
const MAX_SESSION_BRIDGE_SESSIONS: usize = 4096;

/// Manages session lifecycle and message history
pub struct SessionBridge {
    sessions: HashMap<String, AgentSession>,
    histories: HashMap<String, Vec<AgentMessage>>,
    history_bytes: HashMap<String, usize>,
}

impl SessionBridge {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            histories: HashMap::new(),
            history_bytes: HashMap::new(),
        }
    }

    /// Create a new session with a caller-provided session id.
    pub fn register_session(
        &mut self,
        session_id: &str,
        config: BridgeSessionConfig,
    ) -> KernelResult<AgentSession> {
        if self.sessions.contains_key(session_id) {
            return self.get_session(session_id);
        }
        if self.sessions.len() >= MAX_SESSION_BRIDGE_SESSIONS {
            return Err(KernelError::resource_exhausted(
                "active session bridge capacity exhausted",
            ));
        }

        let mut session = AgentSession::new(session_id)
            .with_agent_id(&config.agent_id)
            .with_source(SessionSource::Api)
            .with_kind(SessionKind::Main)
            .created_at(chrono_now());

        if let Some(user_ref) = &config.user_ref {
            session = session.with_user_ref(user_ref);
        }
        if let Some(model) = &config.model {
            session = session.with_model(model);
        }
        if let Some(instructions) = &config.instructions {
            session = session.with_instructions(instructions);
        }
        if let Some(cwd) = &config.cwd {
            session = session.with_cwd(cwd);
        }
        for (key, value) in config.metadata {
            session = session.with_metadata(key, value);
        }

        let mut recorder = EventRecorder::new();
        session = session.activate(&mut recorder)?;

        self.sessions
            .insert(session_id.to_string(), session.clone());
        self.histories.insert(session_id.to_string(), Vec::new());
        self.history_bytes.insert(session_id.to_string(), 0);

        Ok(session)
    }

    /// Create a new session
    pub fn create_session(&mut self, config: BridgeSessionConfig) -> KernelResult<AgentSession> {
        if self.sessions.len() >= MAX_SESSION_BRIDGE_SESSIONS {
            return Err(KernelError::resource_exhausted(
                "active session bridge capacity exhausted",
            ));
        }
        let session_id = format!("session.{}", generate_id());

        let mut session = AgentSession::new(&session_id)
            .with_agent_id(&config.agent_id)
            .with_source(SessionSource::Api)
            .with_kind(SessionKind::Main)
            .created_at(chrono_now());

        if let Some(user_ref) = &config.user_ref {
            session = session.with_user_ref(user_ref);
        }
        if let Some(model) = &config.model {
            session = session.with_model(model);
        }
        if let Some(instructions) = &config.instructions {
            session = session.with_instructions(instructions);
        }
        if let Some(cwd) = &config.cwd {
            session = session.with_cwd(cwd);
        }
        for (key, value) in config.metadata {
            session = session.with_metadata(key, value);
        }

        // Activate the session
        let mut recorder = EventRecorder::new();
        session = session.activate(&mut recorder)?;

        self.sessions.insert(session_id.clone(), session.clone());
        self.histories.insert(session_id.clone(), Vec::new());
        self.history_bytes.insert(session_id, 0);

        Ok(session)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| KernelError::validation(format!("session not found: {}", session_id)))
    }

    /// List all sessions
    pub fn list_sessions(&self) -> KernelResult<Vec<AgentSession>> {
        Ok(self.sessions.values().cloned().collect())
    }

    /// Close a session
    pub fn close_session(&mut self, session_id: &str) -> KernelResult<AgentSession> {
        let session = self.get_session(session_id)?;
        let mut recorder = EventRecorder::new();
        let closed = session.close(&mut recorder)?;
        self.sessions.insert(session_id.to_string(), closed.clone());
        Ok(closed)
    }

    /// Remove a session and its in-bridge message history from transient runtime state.
    pub fn remove_session(&mut self, session_id: &str) -> bool {
        self.histories.remove(session_id);
        self.history_bytes.remove(session_id);
        self.sessions.remove(session_id).is_some()
    }

    /// Append a message to session history
    pub fn append_message(&mut self, session_id: &str, message: AgentMessage) -> KernelResult<()> {
        let history = self
            .histories
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {}", session_id)))?;

        let message_bytes = message_size(&message);
        if message_bytes > MAX_SESSION_BRIDGE_HISTORY_BYTES {
            return Err(KernelError::resource_exhausted(
                "message exceeds session bridge byte budget",
            ));
        }
        history.push(message);
        let retained_bytes = self.history_bytes.entry(session_id.to_string()).or_insert(0);
        *retained_bytes = retained_bytes.saturating_add(message_bytes);
        while history.len() > MAX_SESSION_BRIDGE_HISTORY
            || *retained_bytes > MAX_SESSION_BRIDGE_HISTORY_BYTES
        {
            let removed = history.remove(0);
            *retained_bytes = retained_bytes.saturating_sub(message_size(&removed));
        }

        // Update session message count
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.record_message_received();
        }

        Ok(())
    }

    /// Replace transient history with a bounded persisted snapshot.
    pub fn replace_history(
        &mut self,
        session_id: &str,
        messages: Vec<AgentMessage>,
    ) -> KernelResult<()> {
        if !self.sessions.contains_key(session_id) {
            return Err(KernelError::validation(format!(
                "session not found: {session_id}"
            )));
        }

        let mut retained = Vec::new();
        let mut retained_bytes = 0usize;
        for message in messages.into_iter().rev() {
            let message_bytes = message_size(&message);
            if message_bytes > MAX_SESSION_BRIDGE_HISTORY_BYTES {
                continue;
            }
            if retained.len() >= MAX_SESSION_BRIDGE_HISTORY
                || retained_bytes.saturating_add(message_bytes)
                    > MAX_SESSION_BRIDGE_HISTORY_BYTES
            {
                break;
            }
            retained_bytes = retained_bytes.saturating_add(message_bytes);
            retained.push(message);
        }
        retained.reverse();
        self.histories.insert(session_id.to_string(), retained);
        self.history_bytes
            .insert(session_id.to_string(), retained_bytes);
        Ok(())
    }

    /// Get message history for a session
    pub fn get_history(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        self.histories
            .get(session_id)
            .cloned()
            .ok_or_else(|| KernelError::validation(format!("session not found: {}", session_id)))
    }

    /// Clear message history for a session
    pub fn clear_history(&mut self, session_id: &str) -> KernelResult<()> {
        let history = self
            .histories
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {}", session_id)))?;

        history.clear();
        self.history_bytes.insert(session_id.to_string(), 0);
        Ok(())
    }

    /// Get the number of messages in a session
    pub fn message_count(&self, session_id: &str) -> usize {
        self.histories.get(session_id).map(|h| h.len()).unwrap_or(0)
    }
}

impl Default for SessionBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// RFC3339 UTC timestamp for bridge-owned session metadata.
fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn message_size(message: &AgentMessage) -> usize {
    sdkwork_agent_kernel::flatten_message_to_text(message).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::SessionState;

    fn test_config() -> BridgeSessionConfig {
        BridgeSessionConfig {
            agent_id: "agent.test".to_string(),
            tenant_id: 100_001,
            user_ref: Some("user.1".to_string()),
            model: Some("gpt-4".to_string()),
            instructions: None,
            cwd: None,
            metadata: Vec::new(),
        }
    }

    #[test]
    fn create_and_get_session() {
        let mut bridge = SessionBridge::new();
        let session = bridge.create_session(test_config()).expect("created");
        let retrieved = bridge.get_session(&session.session_id).expect("found");
        assert_eq!(session.session_id, retrieved.session_id);
    }

    #[test]
    fn list_sessions() {
        let mut bridge = SessionBridge::new();
        bridge.create_session(test_config()).expect("created");
        bridge.create_session(test_config()).expect("created");
        let sessions = bridge.list_sessions().expect("listed");
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn close_session() {
        let mut bridge = SessionBridge::new();
        let session = bridge.create_session(test_config()).expect("created");
        let closed = bridge.close_session(&session.session_id).expect("closed");
        assert_eq!(closed.state, SessionState::Closed);
    }

    #[test]
    fn remove_session_deletes_session_and_history() {
        let mut bridge = SessionBridge::new();
        let session = bridge.create_session(test_config()).expect("created");
        let message = AgentMessage::new(
            "msg.remove",
            sdkwork_agent_kernel::AgentMessageRole::User,
            vec![sdkwork_agent_kernel::AgentPart::text(
                "part.remove",
                "remove me",
            )],
        );
        bridge
            .append_message(&session.session_id, message)
            .expect("message appended");
        assert_eq!(bridge.message_count(&session.session_id), 1);

        assert!(
            bridge.remove_session(&session.session_id),
            "existing session should be removed"
        );

        assert!(bridge.get_session(&session.session_id).is_err());
        assert!(bridge.get_history(&session.session_id).is_err());
        assert!(
            !bridge.remove_session(&session.session_id),
            "removing an absent session should be a no-op"
        );
    }

    #[test]
    fn append_and_get_history() {
        let mut bridge = SessionBridge::new();
        let session = bridge.create_session(test_config()).expect("created");

        let msg = AgentMessage::new(
            "msg.1",
            sdkwork_agent_kernel::AgentMessageRole::User,
            vec![sdkwork_agent_kernel::AgentPart::text("part.1", "Hello")],
        );

        bridge
            .append_message(&session.session_id, msg)
            .expect("appended");

        let history = bridge.get_history(&session.session_id).expect("history");
        assert_eq!(history.len(), 1);
    }
}
