# ADR: SDKWork Agent Kernel SPI Comprehensive Assessment

- **ADR ID**: ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT
- **Status**: Proposed
- **Date**: 2025-06-28
- **Decision Scope**: Kernel SPI Architecture, Provider Integration, Industry Alignment
- **Stakeholders**: Kernel Architects, Provider Integrators, Product Teams

## Context and Problem Statement

The SDKWork Agent Kernel aims to be an industry-level agent runtime standard, but requires comprehensive assessment against:

1. **Industry Frameworks**: Claude Code, Codex CLI, OpenCode, OpenClaw, Hermes patterns
2. **SPI Completeness**: Coverage of all agent runtime capabilities
3. **Provider Integration**: Binding mechanism for external agent SDKs
4. **Commercial Readiness**: Production deployment viability

This ADR documents the current state, identifies gaps, and proposes concrete improvements.

## Current Kernel SPI Architecture (Completed Exploration)

### Core Trait System (22 Primary SPI Traits)

| Trait | File Location | Purpose | Methods | Notes |
|-------|---------------|---------|---------|-------|
| `ModelProvider` | provider.rs:46 | Model invocation | `invoke(request) -> response`, `list_models()`, `health()` | Complete, well-designed |
| `ToolProvider` | tool.rs:358 | Tool execution | `list_tools()`, `invoke_tool()`, `health()` | Complete, policy-aware |
| `HostProvider` | host.rs:4 | Host resource access | `filesystem()`, `process()`, `network()`, `resolve_secret()`, `storage()`, `time()`, `environment()`, `executor()` | **Gap**: No sandbox integration |
| `ContextProvider` | context_memory.rs:113 | Context collection | `collect()`, `rank()`, `trim()`, `explain()`, `health()` | Complete with defaults |
| `MemoryProvider` | context_memory.rs:450 | Memory persistence | `query()`, `write()`, `delete()`, `export()`, `consolidate()`, `evolve()`, `health()` | **Excellent**: Tier-aware (Ephemeral/ShortTerm/LongTerm/Permanent/Growing), scope-aware (Session/User/Tenant/Organization/Agent/Application) |
| `KnowledgeProvider` | knowledge.rs:442 | Knowledge search | `search()`, `read()`, `list()`, `health()` | Complete with retrieval methods |
| `AgentSkillProvider` | skill.rs:353 | Skill invocation | `list_skills()`, `describe_skill()`, `invoke_skill()`, `cancel_skill()`, `health()` | Complete with invocation modes (ModelInvocable/ToolBacked/Workflow/HostProvided) |
| `PolicyProvider` | policy.rs:181 | Security policy | `evaluate()`, `explain()`, `record_decision()`, `health()` | Complete with Allow/Deny/NeedsApproval/Defer decisions |
| `McpProvider` | mcp.rs:237 | MCP integration | `list_servers()`, `list_tools()`, `invoke_tool()`, `list_resources()`, `read_resource()`, `list_prompts()`, `get_prompt()`, `health()` | Complete MCP protocol coverage |
| `TelemetryProvider` | telemetry.rs:497 | Observability | `record_event()`, `record_metric()`, `record_log()`, `record_audit()`, `start_span()`, `finish_span()`, `health()` | **Excellent**: Complete observability stack (events/metrics/logs/audit/traces) |
| `AgentInstaller` | installation.rs | Package lifecycle | `install()`, `uninstall()`, `upgrade()`, `check_updates()` | Complete installer SPI |
| `AgentConfigurationProvider` | configuration.rs | Configuration management | `load()`, `save()`, `validate()`, `migrate()` | Complete config SPI |
| `PlanningProvider` | planning.rs | Execution planning | `create_plan()`, `execute_plan()`, `observe_result()`, `health()` | **Gap**: No multi-agent orchestration |
| `TaskSchedulingProvider` | task_scheduling.rs | Task scheduling | `schedule()`, `cancel()`, `list_scheduled()`, `health()` | Complete scheduling SPI |
| `MessageQueryProvider` | message_query.rs | Message history | `query()`, `list_conversations()`, `health()` | Complete query SPI |
| `AgentClassificationProvider` | classification.rs | Message classification | `classify()`, `health()` | Complete classification SPI |
| `AgentCollaborationProvider` | collaboration.rs | Multi-agent collaboration | `list_agents()`, `delegate_task()`, `broadcast_message()`, `health()` | **Gap**: No A2A protocol binding |
| `ProtocolAdapter` | protocol.rs | Protocol translation | `translate_request()`, `translate_response()`, `health()` | Complete adapter pattern |
| `ConversationManager` | provider-core/lib.rs:229 | Conversation state | `begin_turn()`, `end_turn()`, `get_history()`, `append_message()`, `compress_history()` | Complete conversation management |
| `SessionLifecycleProvider` | provider-spi/mapping.rs | Session lifecycle | `create_session()`, `resume_session()`, `close_session()`, `list_sessions()`, `get_conversation_history()` | Complete session SPI with defaults |
| `AgentSdkCapabilityDriver` | provider-spi/driver.rs:51 | SDK capability driver | `driver_id()`, `capability_id()`, `backend_kind()`, `health()` | SDK binding SPI |
| `SdkBackendRuntime` | provider-spi/runtime.rs:143 | Backend runtime | `backend_kind()`, `health()`, `invoke()` | Backend invocation SPI |

### Provider Binding System (provider-spi crate)

**Binding Flow**:
1. **Manifest Registration**: `register_manifest_drivers(manifest, &mut DriverRegistry)` - registers `StaticCapabilityDriver` per capability
2. **Capability Negotiation**: `BindingRegistry::negotiate()` - selects healthy backend by priority
3. **Backend Selection**: `select_backend()` - picks first healthy backend matching priority order
4. **Bootstrap**: `bootstrap_binding()` - combines registration + negotiation
5. **Runtime Invocation**: `SdkRuntimeRouter` - routes requests to negotiated backend's `SdkBackendRuntime`

**Backend Kinds** (5 transport types):
- `RustNative` - Direct Rust crate integration
- `TypeScriptNode` - Node.js process transport
- `PythonProcess` - Python process transport
- `HttpOpenApi` - HTTP API transport
- `IpcProtocol` - IPC transport

**Binding Manifests** (JSON):
- Codex: `agent.intelligence.codex`, 5 capabilities, TypeScript priority
- Hermes: `agent.intelligence.hermes`, 4 capabilities, Python priority
- OpenClaw: `agent.intelligence.openclaw`, 3 capabilities, TypeScript + HTTP priority

### Resilience Layer (retry.rs + resilience.rs)

**Circuit Breaker** (resilience.rs):
- States: Closed/Open/HalfOpen
- Config profiles: `RpcDefault`, `RpcReadOnly`, `RpcIdempotentWrite`, `RpcCriticalWrite`, `RpcStream`, `RpcLocalDev`
- Rolling window failure tracking
- Probe request recovery

**Retry Policy** (retry.rs):
- Exponential backoff with jitter
- Retry budget per service (token bucket)
- Idempotency-aware retry
- Deadline propagation
- Retryable error classification

### Error System (error.rs)

**Structured Errors**:
- `KernelErrorKind`: 13 error categories (ValidationError/CapabilityMissing/ProviderUnavailable/PolicyDenied/Timeout/Cancelled/Conflict/RateLimited/ResourceExhausted/UnsafeContent/SecurityViolation/InternalError)
- `KernelErrorSource`: 12 error sources (Runtime/Provider/Model/Tool/Context/Memory/Policy/Host/ProtocolAdapter/KernelUi/CodeKernel/Telemetry)
- Safe message exposure (`safe_message()` for user-facing, `message()` for internal)
- Retryable classification
- Trace context propagation
- Redaction classification

### Event System (event.rs + telemetry.rs)

**Event Model**:
- `KernelEvent`: Complete event structure (event_id/event_type/severity/source/session/task/run/step/payload/trace_context/correlation/causation/redaction/schema)
- `EventStream`: Subscription-based streaming with cursor pagination
- `EventRecorder`: In-memory event collection
- `TraceContext`: W3C trace context propagation

**Telemetry Stack**:
- `AuditRecord`: Security audit trail
- `TelemetryMetric`: Counter/Gauge/Histogram metrics
- `TelemetryLogRecord`: Structured logs
- `TelemetrySpan`: Distributed tracing spans

## Industry Framework Comparison

### Claude Code Pattern Analysis

**Strengths**:
- Unified `Agent` interface for conversation, tool use, and file operations
- Permission system with allow/deny lists per tool
- Conversation persistence and replay
- Streaming responses with progress events
- Built-in file read/write/edit operations as tools

**Gaps vs SDKWork**:
1. **No multi-provider binding**: Claude Code is Anthropic-only
2. **No knowledge/memory tiering**: Single context window approach
3. **No policy SPI**: Hard-coded permission system
4. **No telemetry SPI**: Built-in logging only
5. **No protocol adapter pattern**: Direct API integration only

**SDKWork Advantages**:
- Multi-provider binding with negotiation
- Tiered memory system (Ephemeral/ShortTerm/LongTerm/Permanent/Growing)
- Policy SPI with Allow/Deny/NeedsApproval/Defer decisions
- Complete telemetry stack (events/metrics/logs/audit/traces)
- Protocol adapter pattern for MCP/A2A/HTTP/RPC

### Codex CLI Pattern Analysis

**Strengths**:
- Sandbox isolation (Linux namespaces, Windows sandbox)
- Approval presets (suggest/auto-edit/full-auto)
- Thread management with history
- Model provider abstraction (OpenAI/Anthropic/Ollama/LMStudio)
- Skill system with custom commands
- Plugin system for extensions

**Gaps vs SDKWork**:
1. **No session lifecycle SPI**: Thread-centric, not session-centric
2. **No capability negotiation**: Provider selection is manual
3. **No knowledge provider**: Limited context augmentation
4. **No collaboration SPI**: Single agent only
5. **No protocol adapter**: Direct CLI integration only

**SDKWork Advantages**:
- Session lifecycle SPI with resume/close/list
- Capability negotiation with backend selection
- Knowledge provider SPI with retrieval methods
- Collaboration provider SPI (list_agents/delegate_task)
- Protocol adapter pattern

### OpenCode Pattern Analysis

**Strengths**:
- Multi-provider model support
- LSP integration for code intelligence
- Terminal integration
- Context management with file watchers
- Skill/extension system

**Gaps vs SDKWork**:
1. **No policy SPI**: No explicit security hooks
2. **No memory provider**: Context-only, no persistence
3. **No knowledge provider**: No external corpus integration
4. **No telemetry provider**: Limited observability
5. **No host provider abstraction**: Direct filesystem/process access

**SDKWork Advantages**:
- Policy provider SPI before risky actions
- Memory provider with tiering and scopes
- Knowledge provider with retrieval methods
- Telemetry provider with audit trail
- Host provider SPI with policy checks

### OpenClaw Pattern Analysis

**Strengths**:
- Extension system for multiple LLM providers
- Tool system with custom tools
- Web search integration
- Vector memory (mem0/supermemory)
- Multi-platform UI (web/desktop/mobile)

**Gaps vs SDKWork**:
1. **No kernel abstraction**: Extension-based, not SPI-based
2. **No session object model**: Conversation-centric
3. **No policy SPI**: No security hooks
4. **No protocol adapter**: Extension-only integration
5. **No conformance testing**: No compatibility contracts

**SDKWork Advantages**:
- Kernel SPI abstraction layer
- Session object model with task/run/step
- Policy provider SPI
- Protocol adapter pattern
- Conformance testing framework

### Hermes Pattern Analysis

**Strengths**:
- Plugin system for memory/providers/platforms
- Multi-provider support (OpenAI/Anthropic/LMStudio/Ollama)
- Memory plugins (mem0/supermemory/honcho/hindsight)
- Platform integrations (Photon desktop)
- Agent achievements system

**Gaps vs SDKWork**:
1. **No kernel SPI**: Plugin-based, not trait-based
2. **No session object model**: Chat-centric
3. **No policy SPI**: No security hooks
4. **No protocol adapter**: Plugin-only integration
5. **No capability negotiation**: Manual provider selection

**SDKWork Advantages**:
- Kernel SPI trait system
- Session object model
- Policy provider SPI
- Protocol adapter pattern
- Capability negotiation with backend selection

## Identified Gaps and Deficiencies

### Critical Gaps (P0)

1. **No Sandbox Integration in HostProvider**
   - **Current**: HostProvider has filesystem/process/network operations without sandbox hooks
   - **Industry**: Codex CLI has Linux namespaces, Windows sandbox, process hardening
   - **Impact**: Security risk for production deployment
   - **Proposal**: Add `SandboxProvider` trait with sandbox configuration, isolation levels, and escape detection

2. **No Multi-Agent Orchestration in PlanningProvider**
   - **Current**: PlanningProvider handles single-agent plan execution
   - **Industry**: A2A protocol defines agent-to-agent task delegation
   - **Impact**: Cannot support multi-agent workflows
   - **Proposal**: Add `AgentOrchestratorProvider` trait with agent discovery, task delegation, result aggregation

3. **No A2A Protocol Binding in CollaborationProvider**
   - **Current**: CollaborationProvider has list_agents/delegate_task without A2A binding
   - **Industry**: A2A is emerging standard for agent-to-agent communication
   - **Impact**: Cannot integrate with external agent systems
   - **Proposal**: Add A2A adapter in protocol.rs with AgentCard translation, task/message mapping

4. **No Backend Health Check Loop**
   - **Current**: Backend health is checked on negotiation, not continuously
   - **Industry**: Production systems require continuous health monitoring
   - **Impact**: Degraded backends may be selected after negotiation
   - **Proposal**: Add `BackendHealthMonitor` background task with periodic health checks and auto-degradation

### High Priority Gaps (P1)

5. **No Streaming Model Response Pattern**
   - **Current**: ModelProvider.invoke() returns complete response
   - **Industry**: Claude Code, Codex CLI support streaming responses
   - **Impact**: No real-time progress for long-running model calls
   - **Proposal**: Add `ModelStreamProvider` trait with `invoke_stream(request) -> impl Stream<ModelStreamChunk>`

6. **No Cancellation Propagation Pattern**
   - **Current**: ToolProvider has no cancellation propagation
   - **Industry**: Codex CLI, Hermes support task cancellation
   - **Impact**: Long-running tools cannot be cancelled gracefully
   - **Proposal**: Add `CancellationProvider` trait with cancel registration, propagation, and cleanup hooks

7. **No Secret Management Integration**
   - **Current**: HostProvider.resolve_secret() exists but no secret management provider
   - **Industry**: Production systems require secret rotation, vault integration
   - **Impact**: Secrets management is manual
   - **Proposal**: Add `SecretProvider` trait with resolve/rotate/audit operations

8. **No Rate Limiting Provider**
   - **Current**: KernelError has RateLimited kind, but no rate limiting provider
   - **Industry**: Production systems require tenant/user quota management
   - **Impact**: Rate limiting is ad-hoc
   - **Proposal**: Add `RateLimitProvider` trait with quota management, enforcement, and metrics

### Medium Priority Gaps (P2)

9. **No Model Fallback Chain**
   - **Current**: Backend selection picks single backend
   - **Industry**: Production systems require fallback chains (primary -> fallback -> emergency)
   - **Impact**: Single backend failure breaks capability
   - **Proposal**: Add fallback chain configuration in binding manifests with automatic retry on fallback

10. **No Context Compression Provider**
    - **Current**: ContextProvider.trim() exists but no compression provider
    - **Industry**: Long-running sessions require context compression (summarization, extraction)
    - **Impact**: Context windows may overflow
    - **Proposal**: Add `ContextCompressionProvider` trait with summarize/compress operations

11. **No Artifact Storage Provider**
    - **Current**: AgentArtifact exists but no storage provider
    - **Industry**: Task outputs require durable storage
    - **Impact**: Artifacts are ephemeral
    - **Proposal**: Add `ArtifactStorageProvider` trait with store/retrieve/list operations

12. **No Notification Provider**
    - **Current**: Events are recorded but not pushed
    - **Industry**: UI requires real-time notifications
    - **Impact**: No push notification mechanism
    - **Proposal**: Add `NotificationProvider` trait with subscribe/push operations

### Minor Gaps (P3)

13. **No Model Token Budget Provider**
    - **Current**: ModelRequest has no token budget
    - **Industry**: Production systems require token budget management per session/task
    - **Impact**: Token usage is unbounded
    - **Proposal**: Add `TokenBudgetProvider` trait with budget allocation, tracking, and enforcement

14. **No User Feedback Provider**
    - **Current**: No feedback collection mechanism
    - **Industry**: Production systems collect user feedback on responses
    - **Impact**: No quality improvement loop
    - **Proposal**: Add `FeedbackProvider` trait with collect/aggregate operations

15. **No Version Compatibility Provider**
    - **Current**: Kernel has spec version, but no compatibility checking
    - **Industry**: Production systems require version compatibility checks for upgrades
    - **Impact**: Upgrades may break compatibility silently
    - **Proposal**: Add `CompatibilityProvider` trait with version checking, migration paths

16. **No Locale/Localization Provider**
    - **Current**: No localization hooks
    - **Industry**: Global systems require localization
    - **Impact**: Single-language responses
    - **Proposal**: Add `LocaleProvider` trait with locale detection, translation operations

## Proposed Improvements with Implementation Plans

### Phase 1: Critical Security Fixes (P0)

#### 1.1 Sandbox Provider Integration

**Proposed Trait** (`sandbox.rs`):
```rust
pub trait SandboxProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn create_sandbox(&self, config: SandboxConfig) -> KernelResult<SandboxHandle>;
    
    fn execute_in_sandbox(
        &self,
        sandbox: &SandboxHandle,
        operation: SandboxOperation,
    ) -> KernelResult<SandboxResult>;
    
    fn destroy_sandbox(&self, sandbox: &SandboxHandle) -> KernelResult<()>;
    
    fn sandbox_status(&self, sandbox: &SandboxHandle) -> KernelResult<SandboxStatus>;
}

pub struct SandboxConfig {
    pub isolation_level: SandboxIsolationLevel,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub network_policy: SandboxNetworkPolicy,
    pub resource_limits: SandboxResourceLimits,
    pub timeout_ms: Option<u64>,
}

pub enum SandboxIsolationLevel {
    None,
    ProcessIsolation,
    NamespaceIsolation,
    HypervisorIsolation,
}

pub enum SandboxOperation {
    Filesystem(FilesystemRequest),
    Process(ProcessRequest),
    Network(NetworkRequest),
}
```

**Implementation Plan**:
1. Add `sandbox.rs` module in kernel
2. Implement `LinuxNamespaceSandboxProvider` (reference Codex CLI sandbox.rs)
3. Implement `WindowsSandboxProvider` (reference Codex CLI windows-sandbox-rs)
4. Add `sandbox_config` to `HostProviderManifest`
5. Update `HostProvider` to route operations through `SandboxProvider` when configured
6. Add conformance tests for sandbox isolation

#### 1.2 Multi-Agent Orchestration Provider

**Proposed Trait** (`orchestration.rs`):
```rust
pub trait AgentOrchestratorProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn discover_agents(&self, query: AgentDiscoveryQuery) -> KernelResult<Vec<AgentCard>>;
    
    fn delegate_task(
        &self,
        target_agent: &AgentCard,
        task: AgentTask,
        delegation_policy: DelegationPolicy,
    ) -> KernelResult<DelegationHandle>;
    
    fn collect_result(&self, handle: &DelegationHandle) -> KernelResult<AgentTaskResult>;
    
    fn cancel_delegation(&self, handle: &DelegationHandle) -> KernelResult<()>;
    
    fn list_active_delegations(&self) -> KernelResult<Vec<DelegationHandle>>;
}

pub struct DelegationPolicy {
    pub timeout_ms: u64,
    pub retry_policy: Option<RetryConfig>,
    pub fallback_agent: Option<AgentCard>,
    pub result_aggregation: ResultAggregationPolicy,
}

pub enum ResultAggregationPolicy {
    FirstSuccess,
    AllSuccess,
    BestResult,
    Consensus,
}
```

**Implementation Plan**:
1. Add `orchestration.rs` module in kernel
2. Implement `LocalOrchestratorProvider` for same-kernel agent delegation
3. Implement `RemoteOrchestratorProvider` for remote agent delegation (HTTP/RPC)
4. Add A2A protocol binding in `protocol.rs`
5. Update `PlanningProvider` to support multi-agent plan nodes
6. Add conformance tests for delegation and result collection

#### 1.3 A2A Protocol Adapter

**Proposed Adapter** (`protocol.rs`):
```rust
pub struct A2AProtocolAdapter {
    adapter_id: String,
    agent_card_mapper: StandardAgentCardMapper,
    task_mapper: StandardAgentTaskMapper,
    message_mapper: StandardAgentMessageMapper,
}

impl ProtocolAdapter for A2AProtocolAdapter {
    fn adapter_manifest(&self) -> ProviderManifest;
    
    fn translate_request(
        &self,
        external: ProtocolObjectEnvelope,
    ) -> KernelResult<KernelRequest>;
    
    fn translate_response(
        &self,
        kernel: KernelResponse,
    ) -> KernelResult<ProtocolObjectEnvelope>;
    
    fn health(&self) -> ProviderHealth;
}

// A2A-specific mappings
pub struct StandardAgentCardMapper {
    // Maps A2A AgentCard to SDKWork AgentCard
}

pub struct StandardAgentTaskMapper {
    // Maps A2A Task to SDKWork AgentTask
}
```

**Implementation Plan**:
1. Add A2A protocol family to `ProtocolObjectKind`
2. Implement `A2AProtocolAdapter` with AgentCard/Task/Message mapping
3. Add A2A agent discovery integration
4. Add A2A task delegation integration
5. Add A2A message routing integration
6. Add conformance tests for A2A object mapping

#### 1.4 Backend Health Monitor

**Proposed Module** (`backend_health.rs`):
```rust
pub struct BackendHealthMonitor {
    driver_registry: Arc<DriverRegistry>,
    check_interval: Duration,
    degradation_threshold: u32,
    recovery_threshold: u32,
}

impl BackendHealthMonitor {
    pub fn spawn(driver_registry: Arc<DriverRegistry>) -> JoinHandle<()>;
    
    fn check_backend_health(driver: &dyn AgentSdkCapabilityDriver) -> SdkDriverHealth;
    
    fn update_driver_status(driver_id: &str, health: SdkDriverHealth);
    
    fn should_degrade(health_history: &[SdkDriverHealth]) -> bool;
    
    fn should_recover(health_history: &[SdkDriverHealth]) -> bool;
}

pub struct HealthHistory {
    driver_id: String,
    checks: Vec<(Instant, SdkDriverHealth)>,
    current_status: SdkDriverStatus,
}
```

**Implementation Plan**:
1. Add `backend_health.rs` module in kernel
2. Implement background health check loop
3. Add health history tracking per driver
4. Implement auto-degradation on consecutive failures
5. Implement auto-recovery on consecutive successes
6. Add health metrics to telemetry

### Phase 2: Production Readiness (P1)

#### 2.1 Streaming Model Provider

**Proposed Trait** (`model.rs` extension):
```rust
pub trait ModelStreamProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn invoke_stream(
        &self,
        request: ModelRequest,
    ) -> KernelResult<impl Stream<Item = KernelResult<ModelStreamChunk>> + Send>;
    
    fn cancel_stream(&self, request_id: &str) -> KernelResult<()>;
}

pub struct ModelStreamChunk {
    pub request_id: String,
    pub sequence: u32,
    pub content: String,
    pub is_final: bool,
    pub usage: Option<ModelUsage>,
    pub tool_calls: Vec<ToolCall>,
}
```

**Implementation Plan**:
1. Add streaming support to `ModelProvider` trait (optional method)
2. Implement async streaming infrastructure
3. Add SSE/WebSocket streaming adapters
4. Implement stream cancellation propagation
5. Add stream progress events
6. Add streaming conformance tests

#### 2.2 Cancellation Provider

**Proposed Trait** (`cancellation.rs`):
```rust
pub trait CancellationProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn register_cancellable(
        &self,
        operation_id: String,
        operation_type: CancellableOperation,
    ) -> KernelResult<CancellationHandle>;
    
    fn request_cancellation(&self, handle: &CancellationHandle) -> KernelResult<()>;
    
    fn wait_for_completion(
        &self,
        handle: &CancellationHandle,
        timeout_ms: u64,
    ) -> KernelResult<CancellationResult>;
    
    fn cleanup_handle(&self, handle: &CancellationHandle) -> KernelResult<()>;
}

pub enum CancellableOperation {
    ModelInvoke,
    ToolInvoke,
    SkillInvoke,
    PlanExecution,
    TaskExecution,
}

pub enum CancellationResult {
    Completed,
    Cancelled,
    Timeout,
    NotFound,
}
```

**Implementation Plan**:
1. Add `cancellation.rs` module
2. Implement cancellation handle registry
3. Add cancellation hooks to ModelProvider/ToolProvider/SkillProvider
4. Implement cancellation propagation to backends
5. Add cancellation events to telemetry
6. Add cancellation conformance tests

#### 2.3 Secret Provider

**Proposed Trait** (`secret.rs`):
```rust
pub trait SecretProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn resolve_secret(&self, ref: SecretRef) -> KernelResult<SecretValue>;
    
    fn rotate_secret(&self, ref: SecretRef) -> KernelResult<SecretRotationResult>;
    
    fn audit_secret_access(&self, ref: &SecretRef, operation: SecretOperation) -> KernelResult<AuditRecord>;
    
    fn list_secrets(&self, scope: SecretScope) -> KernelResult<Vec<SecretRef>>;
}

pub enum SecretOperation {
    Resolve,
    Rotate,
    List,
}

pub enum SecretScope {
    Session,
    Agent,
    Tenant,
    Application,
}

pub struct SecretRotationResult {
    pub ref: SecretRef,
    pub rotated_at: String,
    pub previous_version: u64,
    pub current_version: u64,
}
```

**Implementation Plan**:
1. Add `secret.rs` module
2. Implement `VaultSecretProvider` (HashiCorp Vault integration)
3. Implement `EnvSecretProvider` (environment variable secrets)
4. Implement `FileSecretProvider` (file-based secrets)
5. Add secret rotation background task
6. Add secret access audit trail

#### 2.4 Rate Limit Provider

**Proposed Trait** (`rate_limit.rs`):
```rust
pub trait RateLimitProvider {
    fn provider_manifest(&self) -> ProviderManifest;
    
    fn health(&self) -> ProviderHealth;
    
    fn check_quota(&self, subject: PolicySubject, operation: RateLimitedOperation) -> KernelResult<QuotaStatus>;
    
    fn consume_quota(&self, subject: PolicySubject, operation: RateLimitedOperation, amount: u64) -> KernelResult<QuotaConsumptionResult>;
    
    fn reset_quota(&self, subject: PolicySubject, operation: RateLimitedOperation) -> KernelResult<()>;
    
    fn list_quotas(&self, subject: &PolicySubject) -> KernelResult<Vec<QuotaStatus>>;
}

pub enum RateLimitedOperation {
    ModelInvoke,
    ToolInvoke,
    SessionCreate,
    TaskCreate,
    ArtifactStore,
}

pub struct QuotaStatus {
    pub subject_id: String,
    pub operation: RateLimitedOperation,
    pub limit: u64,
    pub consumed: u64,
    pub remaining: u64,
    pub reset_at: String,
}

pub struct QuotaConsumptionResult {
    pub allowed: bool,
    pub remaining: u64,
    pub retry_after_ms: Option<u64>,
}
```

**Implementation Plan**:
1. Add `rate_limit.rs` module
2. Implement `TokenBucketRateLimitProvider` (token bucket algorithm)
3. Implement `TenantQuotaProvider` (tenant-based quotas)
4. Implement `UserQuotaProvider` (user-based quotas)
5. Add rate limit metrics to telemetry
6. Add rate limit enforcement hooks in runtime

### Phase 3: Quality Improvements (P2-P3)

**Implementation Plans** for remaining gaps are detailed in the full technical specification document (see `docs/architecture/improvement-plans/`).

## Commercial Readiness Assessment

### Production Deployment Viability

| Aspect | Status | Gap | Priority |
|--------|--------|-----|----------|
| Security | **Critical Gap** | No sandbox integration | P0 |
| Multi-Agent | **Critical Gap** | No orchestration SPI | P0 |
| Protocol Integration | **Critical Gap** | No A2A binding | P0 |
| Reliability | **Critical Gap** | No backend health monitoring | P0 |
| Streaming | **Gap** | No model streaming | P1 |
| Cancellation | **Gap** | No cancellation propagation | P1 |
| Secrets | **Gap** | No secret management provider | P1 |
| Rate Limiting | **Gap** | No rate limit provider | P1 |
| Fallback | **Gap** | No fallback chain | P2 |
| Context Compression | **Gap** | No compression provider | P2 |
| Artifact Storage | **Gap** | No storage provider | P2 |
| Notifications | **Gap** | No push notifications | P2 |
| Token Budget | **Minor Gap** | No budget provider | P3 |
| Feedback | **Minor Gap** | No feedback provider | P3 |
| Version Compatibility | **Minor Gap** | No compatibility provider | P3 |
| Localization | **Minor Gap** | No locale provider | P3 |

### Go-to-Market Recommendations

**Phase 1 (Security Foundation)**:
1. **Sandbox Provider**: Essential for production deployment (target: Q1 2026)
2. **Backend Health Monitor**: Essential for reliability (target: Q1 2026)
3. **Secret Provider**: Essential for security (target: Q1 2026)

**Phase 2 (Production Features)**:
4. **Streaming Model Provider**: Competitive necessity (target: Q2 2026)
5. **Rate Limit Provider**: Production necessity (target: Q2 2026)
6. **Cancellation Provider**: UX necessity (target: Q2 2026)

**Phase 3 (Advanced Features)**:
7. **Multi-Agent Orchestration**: Differentiation feature (target: Q3 2026)
8. **A2A Protocol Binding**: Interoperability feature (target: Q3 2026)
9. **Fallback Chain**: Reliability feature (target: Q3 2026)

**Phase 4 (Quality Features)**:
10. **Context Compression**: Performance feature (target: Q4 2026)
11. **Artifact Storage**: Durability feature (target: Q4 2026)
12. **Notifications**: UX feature (target: Q4 2026)

### Competitive Positioning

**SDKWork Advantages**:
- **Multi-provider binding**: Unique capability negotiation system
- **Tiered memory**: Advanced memory persistence model
- **Complete telemetry**: Industry-leading observability stack
- **Policy SPI**: Explicit security hooks before risky actions
- **Protocol adapters**: Clean separation from external protocols

**Competitive Gaps**:
- **Sandbox isolation**: Codex CLI has production sandbox (must add)
- **Streaming responses**: Claude Code has streaming (must add)
- **Multi-agent**: Emerging A2A standard (must add)
- **Backend health**: Production necessity (must add)

**Differentiation Opportunities**:
1. **Industry Standard Position**: SDKWork aims to be Linux-like kernel standard (unique)
2. **Provider Portability**: Multi-provider binding with negotiation (unique)
3. **Tiered Memory**: Growing-tier memory with consolidation (unique)
4. **Conformance Testing**: Third-party compatibility testing (unique)

## Decision

**Accept the Proposed Improvements** with phased implementation:

1. **Phase 1 (P0)**: Security foundation - sandbox, health monitoring, secret management (Q1 2026)
2. **Phase 2 (P1)**: Production features - streaming, rate limiting, cancellation (Q2 2026)
3. **Phase 3 (P0/P2)**: Multi-agent orchestration, A2A binding, fallback chains (Q3 2026)
4. **Phase 4 (P2/P3)**: Quality improvements - compression, storage, notifications (Q4 2026)

## Consequences

**Positive**:
- Production-ready kernel with security foundation
- Industry-leading multi-provider binding system
- Competitive streaming and multi-agent features
- Clear standard candidate for agent runtime

**Negative**:
- Significant implementation effort (estimated 18 months for all phases)
- Dependency on sandbox isolation expertise (Linux namespaces, Windows sandbox)
- A2A protocol is emerging standard (may evolve during implementation)

**Risks**:
- Sandbox implementation complexity may delay Phase 1
- A2A protocol stability may affect Phase 3
- Streaming implementation may require async runtime refactor

## Implementation Tracking

**Progress Tracking**: Update `IMPROVEMENT-PROGRESS-REPORT.md` per phase completion

**Conformance Testing**: Add conformance cases per new provider SPI

**Documentation**: Update specs per new provider SPI trait

## References

- `sdkwork-agent-kernel/src/*.rs` - Current SPI implementation
- `sdkwork-agent-provider-spi/src/*.rs` - Provider binding system
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- `specs/AGENT_PROTOCOL_ADAPTER_SPEC.md` - Protocol adapter specification
- `external/codex/codex-rs/sandboxing/` - Codex sandbox reference
- `external/codex/codex-rs/utils/stream-parser/` - Codex streaming reference