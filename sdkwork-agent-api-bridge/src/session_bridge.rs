use crate::types::{generate_id, BridgeSessionConfig};
use sdkwork_agent_kernel::{
    AgentMessage, AgentSession, EventRecorder, KernelError, KernelResult, SessionKind,
    SessionSource,
};
use std::collections::HashMap;

/// Manages session lifecycle and message history
pub struct SessionBridge {
    sessions: HashMap<String, AgentSession>,
    histories: HashMap<String, Vec<AgentMessage>>,
}

impl SessionBridge {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            histories: HashMap::new(),
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

        self.sessions.insert(session_id.to_string(), session.clone());
        self.histories.insert(session_id.to_string(), Vec::new());

        Ok(session)
    }

    /// Create a new session
    pub fn create_session(&mut self, config: BridgeSessionConfig) -> KernelResult<AgentSession> {
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
        self.histories.insert(session_id, Vec::new());

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

    /// Append a message to session history
    pub fn append_message(&mut self, session_id: &str, message: AgentMessage) -> KernelResult<()> {
        let history = self
            .histories
            .get_mut(session_id)
            .ok_or_else(|| KernelError::validation(format!("session not found: {}", session_id)))?;

        history.push(message);

        // Update session message count
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.record_message_received();
        }

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

/// Simple chrono-free timestamp generator
fn chrono_now() -> String {
    "2026-01-01T00:00:00Z".to_string()
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
