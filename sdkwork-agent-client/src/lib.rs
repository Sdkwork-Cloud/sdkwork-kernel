pub mod bridge;
mod chat;
pub mod plugins;
mod runtime_guard;
pub mod session;
mod sse;
mod types;

#[cfg(feature = "websocket")]
mod websocket;

pub use bridge::*;
pub use chat::{ChatClient, MockChatClient};
pub use session::{sort_bridge_sessions, BridgeSessionQuery, BridgeSessionStore};
pub use sse::SseChatClient;
pub use types::*;

#[cfg(feature = "websocket")]
pub use websocket::WebSocketChatClient;
