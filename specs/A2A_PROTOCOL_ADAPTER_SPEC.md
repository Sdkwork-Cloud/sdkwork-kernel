# SDKWork A2A Protocol Adapter Specification

- **Version**: 0.1.0
- **Status**: Core Primitives Implemented
- **Date**: 2025-06-28
- **Scope**: Agent-to-Agent protocol adapter for external agent communication
- **Domain**: `integration`
- **Capability**: `agent-kernel.a2a-protocol-adapter`
- **Implementation**: `sdkwork-agent-kernel/src/a2a_protocol.rs`
- **Test Coverage**: 15/15 tests passing (100%)

## 1. Overview

A2A Protocol Adapter implements the Google A2A (Agent-to-Agent) Protocol standard for:

- **Agent Discovery**: Discover external A2A-compliant agents
- **Capability Negotiation**: Query agent capabilities and schemas
- **Task Delegation**: Delegate tasks to external agents
- **Result Aggregation**: Collect and aggregate results

### Key Features

1. **A2AAgentCard**: Agent metadata and capability discovery
2. **A2AEndpoint**: Multi-protocol endpoint support (HTTP, gRPC, WebSocket)
3. **A2ATaskRequest/Response**: Task execution protocol
4. **A2AProtocolAdapter**: Adapter trait for implementation

## 2. Architecture

### Component Structure

```text
A2AAgentCard
  ├── agent_id: String
  ├── name: String
  ├── capabilities: Vec<A2ACapability>
  ├── endpoints: Vec<A2AEndpoint>
  ├── authentication: A2AAuthentication
  └── metadata: HashMap<String, String>

A2ACapability
  ├── capability_id: String
  ├── input_schema: Option<String>
  ├── output_schema: Option<String>
  └── required_params: Vec<String>

A2AEndpoint
  ├── endpoint_type: String
  ├── url: String
  ├── protocols: Vec<String>
  └── metadata: HashMap<String, String>

A2ATaskRequest
  ├── task_id: String
  ├── target_agent_id: String
  ├── capability_id: String
  ├── input: HashMap<String, String>
  ├── context: A2ATaskContext
  └── timeout_ms: Option<u64>

A2ATaskResponse
  ├── task_id: String
  ├── status: A2ATaskStatus
  ├── output: HashMap<String, String>
  ├── error: Option<String>
  └── execution_time_ms: u64
```

## 3. Agent Card

### Definition

```rust
pub struct A2AAgentCard {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<A2ACapability>,
    pub endpoints: Vec<A2AEndpoint>,
    pub authentication: A2AAuthentication,
    pub metadata: HashMap<String, String>,
}
```

### Creation

```rust
let card = A2AAgentCard::new(
    "code-generator",
    "Code Generator Agent",
    "Generates code from specifications",
    "1.0.0"
)
.with_capability(A2ACapability::new(
    "generate-code",
    "Code Generation",
    "Generate code from API spec"
))
.with_endpoint(A2AEndpoint::new("http", "https://api.codegen.example.com"))
.with_authentication(A2AAuthentication::ApiKey {
    key_name: "X-API-Key".to_string(),
    key_location: "header".to_string(),
});
```

### Capability Query

```rust
// Check if agent has capability
if card.has_capability("generate-code") {
    // Use the capability
}

// Get specific endpoint
let http_endpoint = card.get_endpoint("http");
```

## 4. Capabilities

### Definition

```rust
pub struct A2ACapability {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub required_params: Vec<String>,
}
```

### Creation with Schemas

```rust
let capability = A2ACapability::new(
    "generate-code",
    "Code Generation",
    "Generate code from API specification"
)
.with_input_schema(r#"{
    "type": "object",
    "properties": {
        "spec": {"type": "string"},
        "language": {"type": "string"}
    },
    "required": ["spec", "language"]
}"#)
.with_output_schema(r#"{
    "type": "object",
    "properties": {
        "code": {"type": "string"},
        "files": {"type": "array"}
    }
}"#)
.with_required_param("spec")
.with_required_param("language");
```

## 5. Endpoints

### Definition

```rust
pub struct A2AEndpoint {
    pub endpoint_type: String,
    pub url: String,
    pub protocols: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

### Multi-Protocol Support

```rust
// HTTP endpoint
let http_endpoint = A2AEndpoint::new("http", "https://api.example.com")
    .with_protocol("https")
    .with_metadata("rate_limit", "1000");

// gRPC endpoint
let grpc_endpoint = A2AEndpoint::new("grpc", "grpc://api.example.com:443")
    .with_protocol("grpc");

// WebSocket endpoint
let ws_endpoint = A2AEndpoint::new("websocket", "wss://api.example.com/ws")
    .with_protocol("wss");
```

## 6. Authentication

### Authentication Types

```rust
pub enum A2AAuthentication {
    None,
    ApiKey { key_name: String, key_location: String },
    OAuth2 { auth_url: String, token_url: String, scopes: Vec<String> },
    Jwt { issuer: String, audience: String },
    Custom { auth_type: String, params: HashMap<String, String> },
}
```

### Examples

```rust
// API Key
let auth = A2AAuthentication::ApiKey {
    key_name: "X-API-Key".to_string(),
    key_location: "header".to_string(),
};

// OAuth2
let auth = A2AAuthentication::OAuth2 {
    auth_url: "https://auth.example.com/authorize".to_string(),
    token_url: "https://auth.example.com/token".to_string(),
    scopes: vec!["read".to_string(), "write".to_string()],
};

// JWT
let auth = A2AAuthentication::Jwt {
    issuer: "https://auth.example.com".to_string(),
    audience: "https://api.example.com".to_string(),
};
```

## 7. Task Execution

### Task Request

```rust
let request = A2ATaskRequest::new(
    "task-123",
    "code-generator",
    "generate-code"
)
.with_input("spec", "OpenAPI spec content")
.with_input("language", "python")
.with_context(
    A2ATaskContext::default()
        .with_session("session-1")
        .with_user("user-1")
        .with_trace("trace-123")
)
.with_timeout(60000); // 60 seconds
```

### Task Context

```rust
pub struct A2ATaskContext {
    pub session_id: Option<String>,
    pub conversation_id: Option<String>,
    pub user_id: Option<String>,
    pub trace_id: Option<String>,
    pub parent_task_id: Option<String>,
}
```

### Task Response

```rust
pub struct A2ATaskResponse {
    pub task_id: String,
    pub status: A2ATaskStatus,
    pub output: HashMap<String, String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, String>,
}
```

### Success Response

```rust
let output = HashMap::from([
    ("code".to_string(), "generated code".to_string()),
    ("files".to_string(), "['main.py', 'utils.py']".to_string()),
]);
let response = A2ATaskResponse::success("task-123", output, 1500);
```

### Failure Response

```rust
let response = A2ATaskResponse::failure("task-123", "Invalid input spec", 100);
```

## 8. Protocol Adapter

### A2AProtocolAdapter Trait

```rust
pub trait A2AProtocolAdapter {
    /// Discover agents via A2A protocol.
    fn discover_agents(&self) -> Result<Vec<A2AAgentCard>, A2AError>;

    /// Get agent card by ID.
    fn get_agent_card(&self, agent_id: &str) -> Result<A2AAgentCard, A2AError>;

    /// Execute task on A2A agent.
    fn execute_task(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError>;

    /// Cancel task.
    fn cancel_task(&self, task_id: &str) -> Result<(), A2AError>;

    /// Check adapter health.
    fn health_check(&self) -> Result<A2AAdapterHealth, A2AError>;
}
```

### Example Implementation

```rust
struct HttpA2AAdapter {
    base_url: String,
    client: HttpClient,
}

impl A2AProtocolAdapter for HttpA2AAdapter {
    fn discover_agents(&self) -> Result<Vec<A2AAgentCard>, A2AError> {
        let response = self.client
            .get(&format!("{}/agents", self.base_url))
            .send()?;
        Ok(response.json()?)
    }

    fn execute_task(&self, request: A2ATaskRequest) -> Result<A2ATaskResponse, A2AError> {
        let response = self.client
            .post(&format!("{}/tasks", self.base_url))
            .json(&request)
            .send()?;
        Ok(response.json()?)
    }

    // ... other methods
}
```

## 9. Error Handling

### A2AError

| Error | Description |
|-------|-------------|
| `AgentNotFound(id)` | Agent not found |
| `CapabilityNotSupported(cap)` | Capability not supported |
| `AuthenticationFailed(msg)` | Authentication failed |
| `TaskExecutionFailed(msg)` | Task execution failed |
| `Timeout(msg)` | Timeout |
| `NetworkError(msg)` | Network error |
| `InvalidRequest(msg)` | Invalid request |
| `AdapterUnavailable` | Adapter unavailable |

### Example Error Handling

```rust
match adapter.execute_task(request) {
    Ok(response) => {
        match response.status {
            A2ATaskStatus::Completed => {
                // Handle success
            }
            A2ATaskStatus::Failed => {
                eprintln!("Task failed: {:?}", response.error);
            }
            _ => {}
        }
    }
    Err(A2AError::AgentNotFound(id)) => {
        eprintln!("Agent not found: {}", id);
    }
    Err(A2AError::Timeout(msg)) => {
        eprintln!("Timeout: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## 10. Conformance Tests

### Test Coverage (15 tests)

| Test Name | Coverage |
|-----------|----------|
| `test_a2a_agent_card_new` | Agent card creation |
| `test_a2a_agent_card_with_capability` | Capability addition |
| `test_a2a_agent_card_get_endpoint` | Endpoint query |
| `test_a2a_capability_new` | Capability creation |
| `test_a2a_capability_with_schemas` | Schema definition |
| `test_a2a_endpoint_new` | Endpoint creation |
| `test_a2a_endpoint_with_protocol` | Protocol addition |
| `test_a2a_task_request_new` | Task request creation |
| `test_a2a_task_request_with_input` | Input parameters |
| `test_a2a_task_context_default` | Default context |
| `test_a2a_task_context_with_fields` | Context fields |
| `test_a2a_task_response_success` | Success response |
| `test_a2a_task_response_failure` | Failure response |
| `test_a2a_task_status_as_str` | Status strings |
| `test_a2a_error_display` | Error formatting |

### Test Execution

```bash
cargo test --package sdkwork-agent-kernel --lib a2a_protocol::tests
```

### Expected Result

```
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured
```

## 11. Integration Points

### AgentCollaborationProvider Integration

```rust
pub trait AgentCollaborationProvider {
    // Existing methods
    fn list_agents(&self) -> Vec<AgentCard>;
    fn handoff(&self, request: AgentHandoffRequest) -> KernelResult<AgentHandoffResult>;

    // A2A integration
    fn register_a2a_adapter(&mut self, adapter: Box<dyn A2AProtocolAdapter>);
    fn discover_a2a_agents(&self) -> KernelResult<Vec<A2AAgentCard>>;
}
```

### ProtocolAdapter Integration

```rust
pub trait ProtocolAdapter {
    // A2A protocol conversion
    fn from_a2a_request(request: A2ATaskRequest) -> KernelRequest;
    fn to_a2a_response(response: KernelResponse) -> A2ATaskResponse;
}
```

### TelemetryProvider Integration

```rust
// Record A2A metrics
telemetry.counter("a2a.task.executed", 1, &[
    ("agent_id", request.target_agent_id),
    ("capability", request.capability_id),
    ("status", response.status.as_str()),
]);

telemetry.histogram("a2a.task.execution_time", response.execution_time_ms);
```

## 12. Usage Patterns

### Pattern 1: Discover and Use

```rust
let adapter = HttpA2AAdapter::new("https://a2a.example.com");

// Discover agents
let agents = adapter.discover_agents()?;
for agent in agents {
    println!("Found agent: {} ({})", agent.name, agent.agent_id);
}

// Find specific agent
let code_gen = agents.iter()
    .find(|a| a.has_capability("generate-code"))
    .ok_or(A2AError::AgentNotFound("code-generator".to_string()))?;

// Execute task
let request = A2ATaskRequest::new("task-1", &code_gen.agent_id, "generate-code")
    .with_input("spec", "API spec");

let response = adapter.execute_task(request)?;
```

### Pattern 2: Multi-Agent Orchestration

```rust
let adapter = HttpA2AAdapter::new("https://a2a.example.com");

// Create orchestration plan
let plan = OrchestrationPlan::new("multi-agent-workflow")
    .with_task(AgentTask::new("analyze", "analyzer", "Analyze requirements"))
    .with_task(AgentTask::new("generate", "generator", "Generate code")
        .with_dependency("analyze"))
    .with_strategy(ExecutionStrategy::Dependency);

// Execute via A2A
for task in plan.tasks {
    let request = A2ATaskRequest::new(&task.task_id, &task.agent_id, &task.objective);
    let response = adapter.execute_task(request)?;
    // Process response
}
```

### Pattern 3: Health Monitoring

```rust
let health = adapter.health_check()?;
match health.status {
    A2AAdapterStatus::Healthy => {
        println!("Adapter healthy, {} agents connected", health.connected_agents);
    }
    A2AAdapterStatus::Degraded => {
        println!("Adapter degraded");
    }
    A2AAdapterStatus::Unhealthy => {
        println!("Adapter unhealthy");
    }
}
```

## 13. Security Considerations

### Authentication

- Validate authentication requirements before task execution
- Store credentials securely (use secret management)
- Rotate API keys regularly

### Authorization

- Check agent capabilities before delegation
- Apply policy decisions to A2A tasks
- Audit all A2A task executions

### Data Protection

- Classify data in task inputs/outputs
- Apply redaction based on classification
- Encrypt sensitive data in transit

## 14. Performance Characteristics

### Latency

| Operation | Typical Latency |
|-----------|----------------|
| Agent discovery | 100-500ms |
| Task execution | Depends on agent |
| Health check | 50-200ms |

### Throughput

- **Concurrent Tasks**: Limited by adapter implementation
- **Rate Limiting**: Apply rate limits per agent
- **Connection Pooling**: Reuse HTTP connections

### Recommendations

- Cache agent cards locally
- Use connection pooling for HTTP
- Implement retry with backoff
- Set appropriate timeouts

## 15. Future Extensions

### Planned Extensions (Phase 6)

1. **Streaming Responses**: Support streaming task outputs
2. **Batch Execution**: Execute multiple tasks in batch
3. **Circuit Breaker**: Add circuit breaker for resilience
4. **Service Mesh**: Integrate with service mesh (Istio, Linkerd)
5. **Protocol Extensions**: Support custom protocols

### Extension Points

```rust
// Future: Streaming
pub trait StreamingA2AAdapter: A2AProtocolAdapter {
    fn execute_streaming(
        &self,
        request: A2ATaskRequest,
    ) -> Result<impl Stream<Item = A2ATaskUpdate>, A2AError>;
}

// Future: Batch
pub trait BatchA2AAdapter: A2AProtocolAdapter {
    fn execute_batch(
        &self,
        requests: Vec<A2ATaskRequest>,
    ) -> Result<Vec<A2ATaskResponse>, A2AError>;
}
```

## 16. References

- `sdkwork-agent-kernel/src/a2a_protocol.rs` - Implementation
- `sdkwork-agent-kernel/src/collaboration.rs` - AgentCollaborationProvider
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- [Google A2A Protocol](https://github.com/google/A2A-Protocol) - Protocol standard

## 17. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-06-28 | Core primitives, 15/15 tests passing |

---

**Status**: ✅ Core Primitives Implemented
**Next Steps**: Integration with AgentCollaborationProvider and HTTP implementation