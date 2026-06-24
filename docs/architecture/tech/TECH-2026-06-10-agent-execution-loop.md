> Migrated from `docs/superpowers/plans/2026-06-10-agent-execution-loop.md` on 2026-06-24.
> Owner: SDKWork maintainers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task in the main agent session. Do not dispatch subagents for this work; the user explicitly requested main-agent execution only. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded SDKWork Agent Kernel execution loop that composes planning, chat/model execution, knowledge and memory context, tool execution, MCP tool execution, policy gates, and observations into one deterministic report.

**Architecture:** Implement `AgentExecutionService` in `sdkwork-agent-kernel/src/execution.rs` and export it from `sdkwork-agent-kernel/src/lib.rs`. The service must reuse existing policy-aware services (`AgentChatService`, `ToolExecutionService`, and `McpToolExecutionService`) and must not import plugin crates or external Rig source.

**Tech Stack:** Rust 2021, existing `sdkwork-agent-kernel` SPI/services, contract tests under `sdkwork-agent-kernel/tests/`, PowerShell verification commands.

---

### Task 1: Execution DTOs And Validation

**Files:**
- Create: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/src/lib.rs`
- Test: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Write the failing validation tests**

Add tests:

```rust
#[test]
fn execution_service_rejects_blank_execution_id_before_runtime_work() {
    let runtime = runtime_with_recording_model();
    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(" ", vec!["hello".to_string()]),
        )
        .expect_err("blank execution id is invalid");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn execution_service_rejects_blank_messages_before_runtime_work() {
    let runtime = runtime_with_recording_model();
    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.blank-message", vec![" ".to_string()]),
        )
        .expect_err("blank messages are invalid");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn execution_service_fails_closed_when_runtime_is_failed() {
    let runtime = RuntimeBuilder::new("runtime.execution.failed", manifest_requiring_model())
        .bootstrap()
        .expect("failed runtime bootstrap still returns report")
        .runtime;

    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.failed-runtime", vec!["hello".to_string()]),
        )
        .expect_err("failed runtime stops before execution");

    assert_eq!(error.kind(), KernelErrorKind::CapabilityMissing);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts
```

Expected: compile failure because `AgentExecutionService`, `AgentExecutionRequest`, and related DTOs do not exist.

- [ ] **Step 3: Implement minimal DTOs and validation**

Create `src/execution.rs` with:

```rust
use crate::{
    AgentChatKnowledgeQuery, AgentChatMemoryQuery, AgentRuntime, KernelError,
    KernelErrorSource, KernelEventRedaction, KernelResult, ModelResponse, Plan,
    RuntimeState, ToolExecutionResponse, McpToolExecutionResponse,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionStatus {
    Completed,
    Failed,
    PermissionRequired,
    Cancelled,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentObservation {
    pub observation_id: String,
    pub source_family: String,
    pub action_id: Option<String>,
    pub status: String,
    pub summary: String,
    pub redaction_classification: KernelEventRedaction,
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionRequest {
    pub execution_id: String,
    pub messages: Vec<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub include_tool_descriptors: bool,
    pub memory_query: Option<AgentChatMemoryQuery>,
    pub knowledge_query: Option<AgentChatKnowledgeQuery>,
    pub mcp_server_id: Option<String>,
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionReport {
    pub execution_id: String,
    pub status: AgentExecutionStatus,
    pub plan: Option<Plan>,
    pub model_response: Option<ModelResponse>,
    pub tool_executions: Vec<ToolExecutionResponse>,
    pub mcp_tool_executions: Vec<McpToolExecutionResponse>,
    pub observations: Vec<AgentObservation>,
    pub error: Option<KernelError>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentExecutionService;
```

Add `AgentExecutionRequest::new`, builder methods, `validate`, `AgentExecutionService::new`, and initial `execute` that validates and rejects `RuntimeState::Failed`.

Modify `src/lib.rs`:

```rust
mod execution;
pub use execution::*;
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts
```

Expected: validation tests pass.

### Task 2: Planning And Chat Invocation

**Files:**
- Modify: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Write the failing planning/chat test**

Add:

```rust
#[test]
fn execution_service_creates_plan_and_invokes_selected_model_provider() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_recording_model_and_planner(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(
                "execution.plan.model",
                vec!["summarize repository".to_string()],
            )
            .with_provider_id("provider.model.recording")
            .for_session("session.execution")
            .for_task("task.execution")
            .for_run("run.execution"),
        )
        .expect("execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.plan.as_ref().unwrap().task_id, "task.execution");
    assert_eq!(captured_model_requests.lock().unwrap().len(), 1);
    assert_eq!(report.model_response.as_ref().unwrap().provider_id, "provider.model.recording");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_creates_plan_and_invokes_selected_model_provider
```

Expected: failure because `execute` does not call planning or chat yet.

- [ ] **Step 3: Implement planning and chat invocation**

In `AgentExecutionService::execute`:

- call `runtime.planning_provider()` when available.
- create a plan using `task_id`, `run_id`, and a summary derived from messages.
- validate the plan.
- map request into `AgentChatRequest`.
- call `AgentChatService::new().invoke(runtime, chat_request)`.
- store `ModelResponse` in `AgentExecutionReport`.
- add a model observation.

- [ ] **Step 4: Run test to verify it passes**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_creates_plan_and_invokes_selected_model_provider
```

Expected: PASS.

### Task 3: Memory And Knowledge Context Through Chat

**Files:**
- Modify: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Write the failing context enrichment test**

Add:

```rust
#[test]
fn execution_service_attaches_memory_and_knowledge_context_through_chat_service() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_memory_knowledge_and_recording_model(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(
                "execution.context",
                vec!["use known context".to_string()],
            )
            .with_provider_id("provider.model.recording")
            .for_session("session.context")
            .for_task("task.context")
            .with_memory_query(MemoryScope::Session, "session.context")
            .with_knowledge_query("kernel knowledge")
            .with_knowledge_provider_id("provider.knowledge.fake")
            .with_knowledge_namespace("docs"),
        )
        .expect("context execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    let requests = captured_model_requests.lock().unwrap();
    assert_eq!(requests[0].context_frames.len(), 2);
    assert_eq!(
        requests[0].context_frames[0].metadata_value("sdkwork.memory.record_id"),
        Some("memory.execution.1")
    );
    assert_eq!(
        requests[0].context_frames[1].metadata_value("sdkwork.knowledge.document_id"),
        Some("knowledge.execution.1")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_attaches_memory_and_knowledge_context_through_chat_service
```

Expected: failure if request-to-chat mapping does not pass memory/knowledge fields.

- [ ] **Step 3: Implement request-to-chat field mapping**

Map all relevant execution request fields into `AgentChatRequest`, including provider id, model id, session/task/run/step ids, tool descriptor flag, memory query, knowledge query, timeout, trace context, subject, and metadata.

- [ ] **Step 4: Run test to verify it passes**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_attaches_memory_and_knowledge_context_through_chat_service
```

Expected: PASS.

### Task 4: Tool Call Execution

**Files:**
- Modify: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Write failing tool execution tests**

Add:

```rust
#[test]
fn execution_service_executes_model_tool_calls_through_tool_service() {
    let runtime = runtime_with_tool_calling_model_and_tool_provider();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .include_tool_descriptors(),
        )
        .expect("tool execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.tool_executions.len(), 1);
    assert_eq!(report.tool_executions[0].result.status, "succeeded");
    assert_eq!(report.observations.iter().filter(|o| o.source_family == "tool").count(), 1);
}

#[test]
fn execution_service_stops_before_tool_when_policy_requires_approval() {
    let runtime = runtime_with_tool_calling_model_and_approval_policy();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool.approval", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .include_tool_descriptors(),
        )
        .expect("permission required is returned as report");

    assert_eq!(report.status, AgentExecutionStatus::PermissionRequired);
    assert!(report.tool_executions.is_empty());
    assert_eq!(report.error.as_ref().unwrap().kind(), KernelErrorKind::PermissionRequired);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_executes_model_tool_calls_through_tool_service
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_stops_before_tool_when_policy_requires_approval
```

Expected: failure because model-returned tool calls are not executed.

- [ ] **Step 3: Implement tool execution routing**

After chat/model response:

- iterate `model_response.tool_calls`.
- use `ToolExecutionService::new().invoke`.
- append `ToolExecutionResponse` on success.
- append a `tool` observation for each result.
- on `PermissionRequired`, return report with `PermissionRequired` and no provider invocation result.
- on tool failure result, return report with `Failed` after preserving prior observations.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_executes_model_tool_calls_through_tool_service
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_stops_before_tool_when_policy_requires_approval
```

Expected: PASS.

### Task 5: MCP Tool Call Execution And Failure Reports

**Files:**
- Modify: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Write failing MCP and failure preservation tests**

Add:

```rust
#[test]
fn execution_service_executes_mcp_tool_calls_through_mcp_service() {
    let runtime = runtime_with_mcp_tool_calling_model_and_mcp_provider();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.mcp", vec!["call mcp".to_string()])
                .with_provider_id("provider.model.mcp-tool-calling")
                .with_mcp_server_id("mcp.fake"),
        )
        .expect("mcp execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.mcp_tool_executions.len(), 1);
    assert_eq!(report.mcp_tool_executions[0].result.status, "succeeded");
}

#[test]
fn execution_service_preserves_prior_observations_when_later_tool_fails() {
    let runtime = runtime_with_two_tool_calls_second_fails();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.partial", vec!["call tools".to_string()])
                .with_provider_id("provider.model.two-tools"),
        )
        .expect("tool failure is represented in report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    assert_eq!(report.observations.len(), 2);
    assert_eq!(report.observations[0].status, "succeeded");
    assert_eq!(report.observations[1].status, "failed");
    assert!(report.error.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_executes_mcp_tool_calls_through_mcp_service
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_preserves_prior_observations_when_later_tool_fails
```

Expected: failure because MCP routing and failure-preserving reports are not implemented.

- [ ] **Step 3: Implement MCP routing and failure preservation**

Implement helper routing:

- If `tool_call.provider_id` resolves through `runtime.tool_provider_by_id`, route to tool.
- If it resolves through `runtime.mcp_provider_by_id`, route to MCP with `mcp_server_id`.
- If no provider id exists, try default tool provider, then default MCP when `mcp_server_id` exists.
- Convert `ToolResult` failure statuses into failed observations and failed reports.
- Preserve all prior observations in the report.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_executes_mcp_tool_calls_through_mcp_service
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts execution_service_preserves_prior_observations_when_later_tool_fails
```

Expected: PASS.

### Task 6: Full Verification

**Files:**
- Modify: `sdkwork-agent-kernel/src/execution.rs`
- Modify: `sdkwork-agent-kernel/src/lib.rs`
- Test: `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`

- [ ] **Step 1: Run narrow execution tests**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts
```

Expected: PASS.

- [ ] **Step 2: Run full Agent Kernel tests**

Run:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml
```

Expected: PASS.

- [ ] **Step 3: Run Rig plugin compatibility tests**

Run:

```powershell
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run kernel standards check**

Run:

```powershell
node scripts/check-kernel-standards.mjs
```

Expected: PASS.

- [ ] **Step 5: Check scoped diff**

Run:

```powershell
git diff -- sdkwork-agent-kernel/src/execution.rs sdkwork-agent-kernel/src/lib.rs sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs docs/superpowers/plans/2026-06-10-agent-execution-loop.md
```

Expected: only execution loop implementation, exports, tests, and this plan changed.

