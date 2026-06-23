pub mod bridge;
mod chat;
pub mod ingress_auth;
pub mod plugins;
mod runtime_guard;
pub mod session;
mod sse;
mod types;

#[cfg(feature = "websocket")]
mod websocket;

pub use bridge::*;
pub use chat::{ChatClient, MockChatClient};
pub use ingress_auth::{
    compute_identity_mac, INGRESS_IDENTITY_MAC_HEADER, INGRESS_TENANT_HEADER, INGRESS_USER_HEADER,
};
pub use session::{sort_bridge_sessions, BridgeSessionQuery, BridgeSessionStore};
pub use sse::{SseChatClient, INTERNAL_RUNTIME_MOUNT_PREFIX};
pub use types::*;

#[cfg(feature = "websocket")]
pub use websocket::WebSocketChatClient;
