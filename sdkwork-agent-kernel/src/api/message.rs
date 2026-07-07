//! Fluent builders for kernel `AgentMessage` values.

use super::content_block::ContentBlock;
use super::role::ConversationRole;
use crate::{AgentMessage, AgentMessageRole, KernelResult};

/// Builds a single conversation turn with industry-friendly helpers.
#[derive(Debug, Clone)]
pub struct MessageBuilder {
    role: AgentMessageRole,
    blocks: Vec<ContentBlock>,
}

impl MessageBuilder {
    pub fn with_role(role: ConversationRole) -> Self {
        Self {
            role: role.to_kernel_role(),
            blocks: Vec::new(),
        }
    }

    pub fn user() -> Self {
        Self::with_role(ConversationRole::User)
    }

    pub fn assistant() -> Self {
        Self::with_role(ConversationRole::Assistant)
    }

    pub fn system() -> Self {
        Self::with_role(ConversationRole::System)
    }

    pub fn tool() -> Self {
        Self::with_role(ConversationRole::Tool)
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.blocks.push(ContentBlock::text(text));
        self
    }

    pub fn json(mut self, value: impl Into<String>) -> Self {
        self.blocks.push(ContentBlock::Json {
            value: value.into(),
            schema: None,
        });
        self
    }

    pub fn block(mut self, block: ContentBlock) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn image_url(
        mut self,
        uri: impl AsRef<str>,
        mime_type: impl Into<String>,
    ) -> KernelResult<Self> {
        self.blocks.push(ContentBlock::image_url(uri, mime_type)?);
        Ok(self)
    }

    pub fn tool_result(mut self, tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        self.blocks
            .push(ContentBlock::tool_result(tool_call_id, text));
        self
    }

    pub fn build(self, message_id: impl Into<String>) -> KernelResult<AgentMessage> {
        if self.blocks.is_empty() {
            return Err(crate::KernelError::validation(
                "message builder requires at least one content block",
            ));
        }

        let mut parts = Vec::with_capacity(self.blocks.len());
        for (index, block) in self.blocks.into_iter().enumerate() {
            parts.push(block.to_part(format!("part.{index}"))?);
        }

        let message = AgentMessage::new(message_id, self.role, parts);
        message.validate()?;
        Ok(message)
    }
}

/// Ordered multimodal conversation — canonical input for invoke APIs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentConversation {
    messages: Vec<AgentMessage>,
}

impl AgentConversation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            messages: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn messages(&self) -> &[AgentMessage] {
        &self.messages
    }

    pub fn into_messages(self) -> Vec<AgentMessage> {
        self.messages
    }

    pub fn push(mut self, message: AgentMessage) -> KernelResult<Self> {
        message.validate()?;
        self.messages.push(message);
        Ok(self)
    }

    pub fn push_builder(
        mut self,
        message_id: impl Into<String>,
        builder: MessageBuilder,
    ) -> KernelResult<Self> {
        self.messages.push(builder.build(message_id)?);
        Ok(self)
    }

    pub fn user_text(
        mut self,
        message_id: impl Into<String>,
        text: impl Into<String>,
    ) -> KernelResult<Self> {
        self.messages
            .push(MessageBuilder::user().text(text).build(message_id)?);
        Ok(self)
    }

    pub fn system_text(
        mut self,
        message_id: impl Into<String>,
        text: impl Into<String>,
    ) -> KernelResult<Self> {
        self.messages
            .push(MessageBuilder::system().text(text).build(message_id)?);
        Ok(self)
    }
}
