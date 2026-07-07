//! Developer-friendly Agent SPI — industry-aligned builders and types.
//!
//! Use this module as the **primary integration surface** for agent hosts,
//! providers, and protocol adapters. Kernel object types (`AgentMessage`,
//! `AgentPart`, `ModelRequest`) remain the canonical wire model; this layer
//! provides ergonomic builders and stable industry vocabulary.
//!
//! ## Industry alignment
//!
//! | SDKWork API | OpenAI | Anthropic | Gemini | A2A |
//! | --- | --- | --- | --- | --- |
//! | `ConversationRole::Assistant` | `assistant` | `assistant` | `model` | `agent` |
//! | `ContentBlock::Text` | `text` | `text` | `text` | `text` |
//! | `ContentBlock::Image` | `image_url` | `image` | `inline_data` | `file`/`url` |
//! | `ContentBlock::Audio` | `input_audio` | — | `inline_data` | — |
//! | `ContentBlock::ToolCall` | `tool_calls` | `tool_use` | `functionCall` | — |
//! | `ContentBlock::ToolResult` | `tool` role | `tool_result` | `functionResponse` | — |
//! | `AgentInvokeRequest` | chat completion request | messages API | `generateContent` | task message |
//! | `InteractionContractBuilder` | capabilities | — | `input_modes` | agent card |
//!
//! ## Quick start
//!
//! ```ignore
//! use sdkwork_agent_kernel::api::{
//!     AgentConversation, AgentInvokeRequest, InteractionContractBuilder, MessageBuilder,
//! };
//!
//! let conversation = AgentConversation::new()
//!     .system_text("msg.system", "You are a helpful agent.")?
//!     .user_text("msg.user", "Describe this image.")?;
//!
//! let request = AgentInvokeRequest::builder("invoke.1")
//!     .conversation(conversation)
//!     .interaction(InteractionContractBuilder::multimodal_chat().build()?)
//!     .model_id("gpt-4o")
//!     .build()?;
//!
//! let model_request = request.to_model_request("policy.invoke.1")?;
//! ```

mod content_block;
mod contract;
mod invoke;
mod message;
mod role;

pub use content_block::ContentBlock;
pub use contract::InteractionContractBuilder;
pub use invoke::{AgentInvokeRequest, AgentInvokeRequestBuilder};
pub use message::{AgentConversation, MessageBuilder};
pub use role::ConversationRole;
