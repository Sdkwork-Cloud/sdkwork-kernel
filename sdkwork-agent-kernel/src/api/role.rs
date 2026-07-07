//! Industry-aligned conversation roles (OpenAI / Anthropic / Gemini / A2A).
//!
//! Kernel roles `agent` and `model` map to wire `assistant` at the adapter layer.

use crate::AgentMessageRole;

/// Wire-friendly message role used by major chat/completions APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
    Tool,
}

impl ConversationRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }

    /// OpenAI Chat Completions `role` field.
    pub fn as_openai_role(self) -> &'static str {
        self.as_str()
    }

    /// Anthropic Messages API `role` field.
    pub fn as_anthropic_role(self) -> &'static str {
        self.as_str()
    }

    /// Google Gemini `role` field (`model` is used for assistant turns).
    pub fn as_gemini_role(self) -> &'static str {
        match self {
            Self::Assistant => "model",
            other => other.as_str(),
        }
    }

    /// A2A message role (user / agent).
    pub fn as_a2a_role(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "agent",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }

    pub fn to_kernel_role(self) -> AgentMessageRole {
        match self {
            Self::User => AgentMessageRole::User,
            Self::Assistant => AgentMessageRole::Agent,
            Self::System => AgentMessageRole::System,
            Self::Tool => AgentMessageRole::Tool,
        }
    }

    pub fn from_kernel_role(role: AgentMessageRole) -> Option<Self> {
        match role {
            AgentMessageRole::User => Some(Self::User),
            AgentMessageRole::Agent | AgentMessageRole::Model => Some(Self::Assistant),
            AgentMessageRole::System => Some(Self::System),
            AgentMessageRole::Tool => Some(Self::Tool),
            AgentMessageRole::Policy | AgentMessageRole::Adapter => None,
        }
    }
}
