use serde::{Deserialize, Serialize};

/// Configuration for creating a new session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub agent_id: String,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub instructions: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl SessionConfig {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            kind: None,
            source: None,
            title: None,
            model: None,
            cwd: None,
            instructions: None,
            metadata: None,
        }
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Query parameters for listing sessions
#[derive(Debug, Clone, Default)]
pub struct SessionQuery {
    pub agent_id: Option<String>,
    pub state: Option<String>,
    pub kind: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Configuration for creating a new message
#[derive(Debug, Clone)]
pub struct MessageConfig {
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

impl MessageConfig {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            metadata: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            metadata: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}
