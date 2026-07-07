use crate::{
    BridgeEvent, BridgeEventSeverity, BridgeMessageResponse, BridgeModelResult,
    BridgeSessionConfig, BridgeSnapshot, BridgeToolResult, ContextBridge, EventBridge, ModelBridge,
    SessionBridge, ToolBridge,
};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelResult, ModelRequest,
    ModelStreamChunk, ToolCall,
};

/// Main bridge connecting kernel runtime to business API layer
pub struct AgentRuntimeBridge {
    session_bridge: SessionBridge,
    model_bridge: ModelBridge,
    tool_bridge: ToolBridge,
    context_bridge: ContextBridge,
    event_bridge: EventBridge,
}

impl AgentRuntimeBridge {
    /// Create a new bridge instance
    pub fn new() -> Self {
        Self {
            session_bridge: SessionBridge::new(),
            model_bridge: ModelBridge::new(),
            tool_bridge: ToolBridge::new(),
            context_bridge: ContextBridge::new(),
            event_bridge: EventBridge::new(),
        }
    }

    #[cfg(test)]
    pub fn new_with_mock_fallback() -> Self {
        Self {
            session_bridge: SessionBridge::new(),
            model_bridge: ModelBridge::with_mock_fallback_enabled(),
            tool_bridge: ToolBridge::with_mock_fallback_enabled(),
            context_bridge: ContextBridge::new(),
            event_bridge: EventBridge::new(),
        }
    }

    /// Create a bridge wired to a bootstrapped typed agent runtime.
    pub fn with_agent_runtime(
        agent_runtime: std::sync::Arc<sdkwork_agent_kernel::AgentRuntime>,
        allow_mock_fallback: bool,
    ) -> Self {
        Self {
            session_bridge: SessionBridge::new(),
            model_bridge: ModelBridge::with_agent_runtime(
                agent_runtime.clone(),
                allow_mock_fallback,
            ),
            tool_bridge: ToolBridge::with_agent_runtime(agent_runtime, allow_mock_fallback),
            context_bridge: ContextBridge::new(),
            event_bridge: EventBridge::new(),
        }
    }

    // =========================================================================
    // Session Management
    // =========================================================================

    /// Register a persisted session in the in-memory bridge runtime.
    pub fn register_session(
        &mut self,
        session_id: &str,
        config: BridgeSessionConfig,
    ) -> KernelResult<AgentSession> {
        self.session_bridge.register_session(session_id, config)
    }

    /// Create a new agent session
    pub fn create_session(&mut self, config: BridgeSessionConfig) -> KernelResult<AgentSession> {
        self.session_bridge.create_session(config)
    }

    /// Get an existing session by ID
    pub fn get_session(&self, session_id: &str) -> KernelResult<AgentSession> {
        self.session_bridge.get_session(session_id)
    }

    /// List all active sessions
    pub fn list_sessions(&self) -> KernelResult<Vec<AgentSession>> {
        self.session_bridge.list_sessions()
    }

    /// Close a session
    pub fn close_session(&mut self, session_id: &str) -> KernelResult<AgentSession> {
        self.session_bridge.close_session(session_id)
    }

    /// Remove a session and its transient bridge-owned state.
    pub fn remove_session(&mut self, session_id: &str) -> bool {
        self.event_bridge.clear_events(session_id);
        self.session_bridge.remove_session(session_id)
    }

    // =========================================================================
    // Message Handling
    // =========================================================================

    /// Send a user message and get the complete response (including model + tool calls)
    pub fn send_message(
        &mut self,
        session_id: &str,
        content: &str,
    ) -> KernelResult<BridgeMessageResponse> {
        let user_message = AgentMessage::new(
            format!("msg.{}", crate::types::generate_id()),
            AgentMessageRole::User,
            vec![AgentPart::text(
                format!("part.{}", crate::types::generate_id()),
                content,
            )],
        );
        self.send_user_message(session_id, user_message)
    }

    /// Send a structured user message (multimodal-safe) and return the assistant turn.
    pub fn send_user_message(
        &mut self,
        session_id: &str,
        user_message: AgentMessage,
    ) -> KernelResult<BridgeMessageResponse> {
        let (model_bridge, model_request, provider_id, user_payload_len) =
            self.prepare_user_message_turn(session_id, user_message)?;
        let model_result = model_bridge.invoke(&model_request, provider_id.as_deref())?;
        self.complete_user_message_turn(session_id, user_payload_len, model_result)
    }

    /// Append a text user message and build the model request for the turn.
    pub fn prepare_send_message_turn(
        &mut self,
        session_id: &str,
        content: &str,
    ) -> KernelResult<(ModelBridge, ModelRequest, Option<String>, usize)> {
        let user_message = AgentMessage::new(
            format!("msg.{}", crate::types::generate_id()),
            AgentMessageRole::User,
            vec![AgentPart::text(
                format!("part.{}", crate::types::generate_id()),
                content,
            )],
        );
        self.prepare_user_message_turn(session_id, user_message)
    }

    /// Append a structured user message and build the model request for the turn.
    pub fn prepare_user_message_turn(
        &mut self,
        session_id: &str,
        user_message: AgentMessage,
    ) -> KernelResult<(ModelBridge, ModelRequest, Option<String>, usize)> {
        user_message.validate()?;
        let session = self.session_bridge.get_session(session_id)?;

        self.session_bridge
            .append_message(session_id, user_message.clone())?;

        let context = self.context_bridge.collect_context(session_id)?;
        let model_request = self.model_bridge.build_request(
            session_id,
            &session,
            &self.session_bridge.get_history(session_id)?,
            &context,
            None,
        );

        let provider_id = session
            .metadata_value("modelProvider")
            .map(std::string::ToString::to_string);
        let user_payload_len = sdkwork_agent_kernel::flatten_message_to_text(&user_message).len();
        Ok((
            self.model_bridge.clone(),
            model_request,
            provider_id,
            user_payload_len,
        ))
    }

    /// Append the assistant response and record bridge events for a prepared turn.
    pub fn complete_user_message_turn(
        &mut self,
        session_id: &str,
        user_payload_len: usize,
        model_result: BridgeModelResult,
    ) -> KernelResult<BridgeMessageResponse> {
        let assistant_message = AgentMessage::new(
            format!("msg.{}", crate::types::generate_id()),
            AgentMessageRole::Agent,
            vec![AgentPart::text(
                format!("part.{}", crate::types::generate_id()),
                model_result
                    .response
                    .messages
                    .first()
                    .cloned()
                    .unwrap_or_default(),
            )],
        );

        self.session_bridge
            .append_message(session_id, assistant_message.clone())?;

        let mut events = model_result.events.clone();
        events.push(BridgeEvent {
            event_type: "agent.message.user".to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            payload: format!("content_length={user_payload_len}"),
            severity: BridgeEventSeverity::Info,
        });
        events.push(BridgeEvent {
            event_type: "agent.message.assistant".to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            payload: format!(
                "content_length={}",
                model_result
                    .response
                    .messages
                    .first()
                    .map(|m| m.len())
                    .unwrap_or(0)
            ),
            severity: BridgeEventSeverity::Info,
        });

        self.event_bridge.record_events(&events);

        Ok(BridgeMessageResponse {
            session_id: session_id.to_string(),
            message: assistant_message,
            model_response: Some(model_result.response),
            tool_results: Vec::new(),
            events,
        })
    }

    /// Stream a user message turn and return ordered output chunks plus the assistant message id.
    pub fn stream_message(
        &mut self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(String, Vec<sdkwork_agent_kernel::ModelStreamChunk>)> {
        let (model_bridge, model_request, provider_id, user_payload_len) =
            self.prepare_stream_message_turn(session_id, content, model_override)?;
        let chunks = model_bridge.stream(&model_request, provider_id.as_deref())?;
        self.complete_stream_message_turn(session_id, user_payload_len, chunks)
    }

    /// Append a user message and build the stream request for the turn.
    pub fn prepare_stream_message_turn(
        &mut self,
        session_id: &str,
        content: &str,
        model_override: Option<&str>,
    ) -> KernelResult<(ModelBridge, ModelRequest, Option<String>, usize)> {
        let mut session = self.session_bridge.get_session(session_id)?;
        if let Some(model_id) = model_override {
            session.model = Some(model_id.to_string());
        }

        let user_message = AgentMessage::new(
            format!("msg.{}", crate::types::generate_id()),
            AgentMessageRole::User,
            vec![AgentPart::text(
                format!("part.{}", crate::types::generate_id()),
                content,
            )],
        );

        self.session_bridge
            .append_message(session_id, user_message.clone())?;

        let context = self.context_bridge.collect_context(session_id)?;
        let model_request = self.model_bridge.build_request(
            session_id,
            &session,
            &self.session_bridge.get_history(session_id)?,
            &context,
            None,
        );

        let provider_id = session
            .metadata_value("modelProvider")
            .map(std::string::ToString::to_string);
        Ok((
            self.model_bridge.clone(),
            model_request,
            provider_id,
            content.len(),
        ))
    }

    /// Append the streamed assistant response and record bridge stream events.
    pub fn complete_stream_message_turn(
        &mut self,
        session_id: &str,
        user_payload_len: usize,
        chunks: Vec<ModelStreamChunk>,
    ) -> KernelResult<(String, Vec<ModelStreamChunk>)> {
        let assistant_text: String = chunks.iter().map(|chunk| chunk.content.as_str()).collect();
        let assistant_message_id = format!("msg.{}", crate::types::generate_id());
        let assistant_message = AgentMessage::new(
            assistant_message_id.clone(),
            AgentMessageRole::Agent,
            vec![AgentPart::text(
                format!("part.{}", crate::types::generate_id()),
                assistant_text.clone(),
            )],
        );

        self.session_bridge
            .append_message(session_id, assistant_message)?;

        let mut events = vec![
            BridgeEvent {
                event_type: "agent.message.user".to_string(),
                session_id: Some(session_id.to_string()),
                task_id: None,
                payload: format!("content_length={user_payload_len}"),
                severity: BridgeEventSeverity::Info,
            },
            BridgeEvent {
                event_type: "agent.message.assistant".to_string(),
                session_id: Some(session_id.to_string()),
                task_id: None,
                payload: format!("content_length={}", assistant_text.len()),
                severity: BridgeEventSeverity::Info,
            },
        ];
        for chunk in &chunks {
            events.push(BridgeEvent {
                event_type: "agent.model.output.streamed".to_string(),
                session_id: Some(session_id.to_string()),
                task_id: None,
                payload: format!(
                    "model_request_id={};sequence={}",
                    chunk.model_request_id, chunk.sequence
                ),
                severity: BridgeEventSeverity::Info,
            });
        }
        self.event_bridge.record_events(&events);

        Ok((assistant_message_id, chunks))
    }

    /// Get message history for a session
    pub fn get_messages(&self, session_id: &str) -> KernelResult<Vec<AgentMessage>> {
        self.session_bridge.get_history(session_id)
    }

    // =========================================================================
    // Model Invocation
    // =========================================================================

    /// Invoke the model directly (without conversation context)
    pub fn invoke_model(&self, request: ModelRequest) -> KernelResult<BridgeModelResult> {
        self.model_bridge.invoke(&request, None)
    }

    /// Invoke the model using persisted session history and context frames.
    pub fn invoke_model_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
    ) -> KernelResult<BridgeModelResult> {
        let (request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, None)?;
        self.model_bridge.invoke(&request, provider_id.as_deref())
    }

    /// Stream model response
    pub fn stream_model(
        &self,
        request: ModelRequest,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        self.model_bridge.stream(&request, None)
    }

    /// Stream model output using session history, with optional message override.
    pub fn stream_model_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
    ) -> KernelResult<Vec<sdkwork_agent_kernel::ModelStreamChunk>> {
        let (request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, override_messages)?;
        self.model_bridge.stream(&request, provider_id.as_deref())
    }

    /// Stream model output incrementally using session history.
    pub fn stream_model_for_session_into(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
        sink: &mut dyn sdkwork_agent_kernel::ModelStreamSink,
    ) -> KernelResult<()> {
        let (request, provider_id) =
            self.prepare_model_request_for_session(session_id, model_id, override_messages)?;
        self.model_bridge
            .stream_into(&request, provider_id.as_deref(), sink)
    }

    /// Build a model request from bridge session state without invoking a provider.
    ///
    /// Server runtimes use this to keep the bridge state lock scoped to local
    /// session/history reads. Provider calls can then run outside the lock.
    pub fn prepare_model_request_for_session(
        &self,
        session_id: &str,
        model_id: Option<String>,
        override_messages: Option<Vec<String>>,
    ) -> KernelResult<(ModelRequest, Option<String>)> {
        let session = self.get_session(session_id)?;
        let provider_id = session
            .metadata_value("modelProvider")
            .map(std::string::ToString::to_string);
        let context = self.context_bridge.collect_context(session_id)?;
        let history = if let Some(messages) = override_messages {
            if messages.is_empty() {
                self.get_messages(session_id)?
            } else {
                sdkwork_agent_kernel::agent_messages_from_text_lines(
                    sdkwork_agent_kernel::AgentMessageRole::User,
                    &messages,
                )
            }
        } else {
            self.get_messages(session_id)?
        };
        let mut request = self
            .model_bridge
            .build_request(session_id, &session, &history, &context, None);
        if let Some(model_id) = model_id {
            request = request.with_model_id(model_id);
        }
        Ok((request, provider_id))
    }

    /// Cancel an in-flight model invocation by its model request id.
    pub fn cancel_model(
        &self,
        model_request_id: &str,
        model_provider_id: Option<&str>,
    ) -> KernelResult<sdkwork_agent_kernel::ModelResponse> {
        self.model_bridge
            .cancel(model_request_id, model_provider_id)
    }

    /// List registered model descriptors
    pub fn list_models(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ModelDescriptor>> {
        Ok(self.model_bridge.list_models())
    }

    // =========================================================================
    // Tool Execution
    // =========================================================================

    /// List available tools
    pub fn list_tools(&self) -> KernelResult<Vec<sdkwork_agent_kernel::ToolDescriptor>> {
        self.tool_bridge.list_tools()
    }

    /// Execute a tool call
    pub fn execute_tool(
        &mut self,
        session_id: &str,
        tool_name: &str,
        arguments: &str,
    ) -> KernelResult<BridgeToolResult> {
        let call = ToolCall::new(
            format!("call.{}", crate::types::generate_id()),
            tool_name,
            arguments,
        );

        let result = self.tool_bridge.execute(&call)?;

        let events = vec![BridgeEvent {
            event_type: "agent.tool.executed".to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            payload: format!("tool={};status={}", tool_name, result.status),
            severity: BridgeEventSeverity::Info,
        }];

        self.event_bridge.record_events(&events);

        Ok(BridgeToolResult {
            call_id: call.tool_call_id,
            result,
            events,
        })
    }

    // =========================================================================
    // Snapshot
    // =========================================================================

    /// Get a complete snapshot of the bridge state for UI consumption
    pub fn get_snapshot(&self, session_id: &str) -> KernelResult<BridgeSnapshot> {
        let session = self.session_bridge.get_session(session_id)?;
        let messages = self.session_bridge.get_history(session_id)?;
        let tools = self.tool_bridge.list_tools()?;
        let events = self.event_bridge.get_events(session_id);

        Ok(BridgeSnapshot {
            session_id: session_id.to_string(),
            session,
            messages,
            available_tools: tools,
            pending_tool_calls: Vec::new(),
            events,
        })
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get reference to session bridge
    pub fn session_bridge(&self) -> &SessionBridge {
        &self.session_bridge
    }

    /// Get mutable reference to session bridge
    pub fn session_bridge_mut(&mut self) -> &mut SessionBridge {
        &mut self.session_bridge
    }

    /// Get reference to model bridge
    pub fn model_bridge(&self) -> &ModelBridge {
        &self.model_bridge
    }

    /// Get reference to tool bridge
    pub fn tool_bridge(&self) -> &ToolBridge {
        &self.tool_bridge
    }

    /// Get reference to context bridge
    pub fn context_bridge(&self) -> &ContextBridge {
        &self.context_bridge
    }

    /// Get reference to event bridge
    pub fn event_bridge(&self) -> &EventBridge {
        &self.event_bridge
    }
}

impl Default for AgentRuntimeBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_create_session() {
        let mut bridge = AgentRuntimeBridge::new();
        let config = BridgeSessionConfig {
            agent_id: "agent.test".to_string(),
            tenant_id: 100_001,
            user_ref: Some("user.1".to_string()),
            model: Some("gpt-4".to_string()),
            instructions: None,
            cwd: None,
            metadata: Vec::new(),
        };

        let session = bridge.create_session(config).expect("session created");
        assert_eq!(session.agent_id, Some("agent.test".to_string()));
        assert_eq!(session.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn bridge_send_message() {
        let mut bridge = AgentRuntimeBridge::new_with_mock_fallback();
        let config = BridgeSessionConfig {
            agent_id: "agent.test".to_string(),
            tenant_id: 100_001,
            user_ref: Some("user.1".to_string()),
            model: Some("gpt-4".to_string()),
            instructions: None,
            cwd: None,
            metadata: Vec::new(),
        };

        let session = bridge.create_session(config).expect("session created");
        let response = bridge
            .send_message(&session.session_id, "Hello")
            .expect("message sent");

        assert_eq!(response.session_id, session.session_id);
        assert!(response.model_response.is_some());
    }

    #[test]
    fn remove_session_deletes_recorded_bridge_events() {
        let mut bridge = AgentRuntimeBridge::new_with_mock_fallback();
        let config = BridgeSessionConfig {
            agent_id: "agent.test".to_string(),
            tenant_id: 100_001,
            user_ref: Some("user.1".to_string()),
            model: Some("gpt-4".to_string()),
            instructions: None,
            cwd: None,
            metadata: Vec::new(),
        };

        let session = bridge.create_session(config).expect("session created");
        bridge
            .send_message(&session.session_id, "Hello")
            .expect("message sent");
        assert!(
            !bridge
                .event_bridge()
                .get_events(&session.session_id)
                .is_empty(),
            "message turn should record bridge events for the session"
        );

        bridge.remove_session(&session.session_id);

        assert!(
            bridge
                .event_bridge()
                .get_events(&session.session_id)
                .is_empty(),
            "removing a session must clear per-session bridge events"
        );
    }

    #[test]
    fn bridge_list_tools() {
        let bridge = AgentRuntimeBridge::new();
        let tools = bridge.list_tools().expect("tools listed");
        assert!(!tools.is_empty());
    }

    #[test]
    fn bridge_get_snapshot() {
        let mut bridge = AgentRuntimeBridge::new();
        let config = BridgeSessionConfig {
            agent_id: "agent.test".to_string(),
            tenant_id: 100_001,
            user_ref: None,
            model: None,
            instructions: None,
            cwd: None,
            metadata: Vec::new(),
        };

        let session = bridge.create_session(config).expect("session created");
        let snapshot = bridge
            .get_snapshot(&session.session_id)
            .expect("snapshot created");

        assert_eq!(snapshot.session_id, session.session_id);
        assert!(!snapshot.available_tools.is_empty());
    }
}
