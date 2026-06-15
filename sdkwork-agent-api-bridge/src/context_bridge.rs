use sdkwork_agent_kernel::{ContextFrame, KernelResult, RedactionClassification, TrustLevel};
use std::collections::HashMap;

/// Manages context collection and assembly for model requests
pub struct ContextBridge {
    system_instructions: HashMap<String, String>,
}

impl ContextBridge {
    pub fn new() -> Self {
        Self {
            system_instructions: HashMap::new(),
        }
    }

    /// Collect context frames for a session
    pub fn collect_context(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>> {
        let mut frames = Vec::new();

        // Add system instruction if available
        if let Some(instructions) = self.system_instructions.get(session_id) {
            frames.push(
                ContextFrame::new(
                    format!("ctx.{}.system", session_id),
                    session_id,
                    "system.instructions",
                    instructions,
                    TrustLevel::TrustedSystem,
                    RedactionClassification::Internal,
                )
                .with_content_type("text/plain"),
            );
        }

        // Add timestamp context
        frames.push(
            ContextFrame::new(
                format!("ctx.{}.time", session_id),
                session_id,
                "system.time",
                format!("Current time: 2026-01-01T00:00:00Z"),
                TrustLevel::TrustedSystem,
                RedactionClassification::Public,
            )
            .with_content_type("text/plain"),
        );

        Ok(frames)
    }

    /// Set system instructions for a session
    pub fn set_system_instructions(&mut self, session_id: &str, instructions: impl Into<String>) {
        self.system_instructions
            .insert(session_id.to_string(), instructions.into());
    }

    /// Get system instructions for a session
    pub fn get_system_instructions(&self, session_id: &str) -> Option<&str> {
        self.system_instructions.get(session_id).map(|s| s.as_str())
    }
}

impl Default for ContextBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_context_returns_frames() {
        let bridge = ContextBridge::new();
        let frames = bridge.collect_context("session.1").expect("collected");
        assert!(!frames.is_empty());
    }

    #[test]
    fn set_and_get_instructions() {
        let mut bridge = ContextBridge::new();
        bridge.set_system_instructions("session.1", "You are a helpful assistant.");
        let instructions = bridge.get_system_instructions("session.1");
        assert_eq!(instructions, Some("You are a helpful assistant."));
    }
}
