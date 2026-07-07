//! Registry-backed A2A protocol adapter for in-process agent interop.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    A2AAdapterHealth, A2AAdapterStatus, A2AAgentCard, A2AError, A2AProtocolAdapter, A2ATaskRequest,
    A2ATaskResponse,
};

/// Handler invoked when a registered agent receives an A2A task.
pub trait A2ATaskHandler: Send + Sync {
    fn execute(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError>;
}

/// In-memory registry of discoverable A2A agents and task handlers.
#[derive(Default)]
pub struct A2AAgentRegistry {
    cards: HashMap<String, A2AAgentCard>,
    handlers: HashMap<String, Arc<dyn A2ATaskHandler>>,
}

impl A2AAgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_agent(
        &mut self,
        card: A2AAgentCard,
        handler: Arc<dyn A2ATaskHandler>,
    ) -> Result<(), A2AError> {
        if !card.endpoints.is_empty() && card.capabilities.is_empty() {
            return Err(A2AError::InvalidRequest(
                "agent card must declare at least one capability".to_string(),
            ));
        }
        let agent_id = card.agent_id.clone();
        self.handlers.insert(agent_id.clone(), handler);
        self.cards.insert(agent_id, card);
        Ok(())
    }

    pub fn unregister_agent(&mut self, agent_id: &str) -> bool {
        self.handlers.remove(agent_id);
        self.cards.remove(agent_id).is_some()
    }

    pub fn agent_count(&self) -> usize {
        self.cards.len()
    }
}

/// Thread-safe registry-backed [`A2AProtocolAdapter`] implementation.
pub struct RegistryA2AProtocolAdapter {
    registry: Arc<RwLock<A2AAgentRegistry>>,
    adapter_version: String,
}

impl RegistryA2AProtocolAdapter {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(A2AAgentRegistry::new())),
            adapter_version: "1.0.0".to_string(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.adapter_version = version.into();
        self
    }

    pub fn registry(&self) -> Arc<RwLock<A2AAgentRegistry>> {
        Arc::clone(&self.registry)
    }

    pub fn register_agent(
        &self,
        card: A2AAgentCard,
        handler: Arc<dyn A2ATaskHandler>,
    ) -> Result<(), A2AError> {
        self.registry
            .write()
            .map_err(|_| A2AError::AdapterUnavailable)?
            .register_agent(card, handler)
    }
}

impl Default for RegistryA2AProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl A2AProtocolAdapter for RegistryA2AProtocolAdapter {
    fn discover_agents(&self) -> Result<Vec<A2AAgentCard>, A2AError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| A2AError::AdapterUnavailable)?;
        Ok(registry.cards.values().cloned().collect())
    }

    fn get_agent_card(&self, agent_id: &str) -> Result<A2AAgentCard, A2AError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| A2AError::AdapterUnavailable)?;
        registry
            .cards
            .get(agent_id)
            .cloned()
            .ok_or_else(|| A2AError::AgentNotFound(agent_id.to_string()))
    }

    fn execute_task(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError> {
        request.validate()?;
        let started = now_ms();
        let handler = {
            let registry = self
                .registry
                .read()
                .map_err(|_| A2AError::AdapterUnavailable)?;
            let card = registry
                .cards
                .get(&request.target_agent_id)
                .ok_or_else(|| A2AError::AgentNotFound(request.target_agent_id.clone()))?;
            if !card.has_capability(&request.capability_id) {
                return Err(A2AError::CapabilityNotSupported(
                    request.capability_id.clone(),
                ));
            }
            registry
                .handlers
                .get(&request.target_agent_id)
                .cloned()
                .ok_or(A2AError::AdapterUnavailable)?
        };
        let mut response = handler.execute(request)?;
        if response.execution_time_ms == 0 {
            response.execution_time_ms = now_ms().saturating_sub(started);
        }
        Ok(response)
    }

    fn cancel_task(&self, task_id: &str) -> Result<(), A2AError> {
        let _ = task_id;
        Err(A2AError::InvalidRequest(
            "task cancellation is not supported by the in-memory registry adapter".to_string(),
        ))
    }

    fn health_check(&self) -> Result<A2AAdapterHealth, A2AError> {
        let connected_agents = self
            .registry
            .read()
            .map_err(|_| A2AError::AdapterUnavailable)?
            .agent_count();
        Ok(A2AAdapterHealth {
            status: if connected_agents > 0 {
                A2AAdapterStatus::Healthy
            } else {
                A2AAdapterStatus::Degraded
            },
            connected_agents,
            last_check_time_ms: now_ms(),
            adapter_version: self.adapter_version.clone(),
        })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{A2ACapability, A2ATaskContext, AgentMessage, AgentMessageRole, AgentPart};

    struct EchoHandler;

    impl A2ATaskHandler for EchoHandler {
        fn execute(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError> {
            let text = request.primary_user_text().unwrap_or_default();
            let mut output = HashMap::new();
            output.insert("echo".to_string(), text);
            Ok(A2ATaskResponse::success(request.task_id, output, 1))
        }
    }

    #[test]
    fn registry_adapter_discovers_and_executes_multimodal_task() {
        let adapter = RegistryA2AProtocolAdapter::new();
        let card = A2AAgentCard::new("agent.echo", "Echo", "Echo agent", "1.0.0")
            .with_capability(A2ACapability::new("echo", "Echo", "Echo user text"))
            .with_endpoint(crate::A2AEndpoint::new("memory", "registry://agent.echo"));

        adapter
            .register_agent(card, Arc::new(EchoHandler))
            .expect("register");

        let discovered = adapter.discover_agents().expect("discover");
        assert_eq!(discovered.len(), 1);

        let request = A2ATaskRequest::new("task.1", "agent.echo", "echo")
            .with_message(AgentMessage::new(
                "msg.1",
                AgentMessageRole::User,
                vec![AgentPart::text("part.1", "hello multimodal")],
            ))
            .with_context(A2ATaskContext::default().with_session("session.1"));

        let response = adapter.execute_task(request).expect("execute");
        assert_eq!(response.status, crate::A2ATaskStatus::Completed);
        assert_eq!(
            response.output.get("echo").map(String::as_str),
            Some("hello multimodal")
        );
    }
}
