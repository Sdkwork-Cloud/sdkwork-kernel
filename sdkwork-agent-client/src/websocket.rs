use crate::chat::{ChatClient, ChatEventCallback, SessionConfig, SessionInfo};
use crate::types::*;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// WebSocket-based chat client for bidirectional communication
pub struct WebSocketChatClient {
    url: String,
    tx: Option<mpsc::Sender<WsMessage>>,
    event_callback: Option<ChatEventCallback>,
}

impl WebSocketChatClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            tx: None,
            event_callback: None,
        }
    }

    /// Set event callback
    pub fn with_callback(mut self, callback: ChatEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Connect to WebSocket server
    pub async fn connect(&mut self) -> Result<(), String> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| format!("failed to connect: {}", e))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<WsMessage>(100);

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = write.send(Message::Text(json)).await;
                }
            }
        });

        // Spawn task to handle incoming messages
        let callback = self.event_callback.clone();
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            Self::handle_ws_message(ws_msg, &callback);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                    _ => {}
                }
            }
        });

        self.tx = Some(tx);
        Ok(())
    }

    /// Handle incoming WebSocket message
    fn handle_ws_message(msg: WsMessage, callback: &Option<ChatEventCallback>) {
        if let Some(callback) = callback {
            match msg.message_type.as_str() {
                "message" => {
                    if let Ok(chat_msg) = serde_json::from_value::<ChatMessage>(msg.payload) {
                        callback(ChatEvent::MessageReceived(chat_msg));
                    }
                }
                "chunk" => {
                    let content = msg.payload["content"].as_str().unwrap_or("").to_string();
                    let sequence = msg.payload["sequence"].as_u64().unwrap_or(0) as u32;
                    let message_id = msg.payload["message_id"].as_str().unwrap_or("").to_string();
                    callback(ChatEvent::StreamChunk {
                        message_id,
                        content,
                        sequence,
                    });
                }
                "done" => {
                    let message_id = msg.payload["message_id"].as_str().unwrap_or("").to_string();
                    let final_content = msg.payload["content"].as_str().unwrap_or("").to_string();
                    callback(ChatEvent::StreamCompleted {
                        message_id,
                        final_content,
                    });
                }
                "error" => {
                    let error = msg.payload["error"]
                        .as_str()
                        .unwrap_or("unknown error")
                        .to_string();
                    callback(ChatEvent::Error {
                        error,
                        recoverable: true,
                    });
                }
                _ => {}
            }
        }
    }

    /// Send a message via WebSocket
    pub async fn send_ws_message(&self, msg: WsMessage) -> Result<(), String> {
        if let Some(tx) = &self.tx {
            tx.send(msg)
                .await
                .map_err(|e| format!("send failed: {}", e))
        } else {
            Err("not connected".to_string())
        }
    }
}

impl ChatClient for WebSocketChatClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(async {
            let msg = WsMessage {
                message_type: "chat.send".to_string(),
                payload: json!({
                    "session_id": request.session_id,
                    "content": request.content,
                    "model": request.model,
                    "stream": false,
                }),
            };

            self.send_ws_message(msg).await?;

            // For non-streaming, we need to wait for response
            // This is a simplified implementation
            Ok(ChatResponse {
                message_id: format!("msg.{}", generate_id()),
                session_id: request.session_id,
                content: "WebSocket response pending".to_string(),
                status: ChatStatus::Pending,
                usage: None,
            })
        })
    }

    fn send_message_stream(
        &self,
        request: ChatRequest,
        callback: ChatEventCallback,
    ) -> Result<String, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(async {
            let message_id = format!("msg.{}", generate_id());
            let msg = WsMessage {
                message_type: "chat.send".to_string(),
                payload: json!({
                    "session_id": request.session_id,
                    "content": request.content,
                    "model": request.model,
                    "stream": true,
                    "message_id": message_id,
                }),
            };

            self.send_ws_message(msg).await?;
            Ok(message_id)
        })
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(async {
            let msg = WsMessage {
                message_type: "session.messages".to_string(),
                payload: json!({
                    "session_id": session_id,
                    "limit": limit,
                }),
            };

            self.send_ws_message(msg).await?;

            // For now, return empty - response comes via callback
            Ok(Vec::new())
        })
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(async {
            let msg = WsMessage {
                message_type: "session.create".to_string(),
                payload: json!({
                    "agent_id": config.agent_id,
                    "model": config.model,
                    "title": config.title,
                    "instructions": config.instructions,
                }),
            };

            self.send_ws_message(msg).await?;

            Ok(SessionInfo {
                session_id: format!("session.{}", generate_id()),
                agent_id: config.agent_id,
                model: config.model,
                state: "active".to_string(),
                created_at: chrono_now(),
            })
        })
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(async {
            let msg = WsMessage {
                message_type: "session.close".to_string(),
                payload: json!({
                    "session_id": session_id,
                }),
            };

            self.send_ws_message(msg).await?;
            Ok(())
        })
    }

    fn health(&self) -> Result<bool, String> {
        Ok(self.tx.is_some())
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

fn chrono_now() -> String {
    "2026-01-01T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_client_creation() {
        let client = WebSocketChatClient::new("ws://localhost:8080/ws");
        assert_eq!(client.url, "ws://localhost:8080/ws");
        assert!(client.tx.is_none());
    }

    #[test]
    fn websocket_client_health_disconnected() {
        let client = WebSocketChatClient::new("ws://localhost:8080/ws");
        assert!(!client.health().expect("health"));
    }
}
