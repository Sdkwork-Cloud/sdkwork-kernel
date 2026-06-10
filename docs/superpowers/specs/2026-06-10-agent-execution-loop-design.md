# Agent Execution Loop Design

## Goal

Build the first complete SDKWork Agent Kernel execution loop. The loop must
compose the already implemented planning, model, memory, knowledge, tool, MCP,
policy, and telemetry-facing contracts into one deterministic, testable
runtime service.

The first implementation proves that an SDKWork-compatible agent can execute a
task end to end without binding the kernel to Rig, a model vendor, a product UI,
or a concrete host process.

## Scope

In scope:

- A new Agent Kernel execution orchestration service.
- A single-run execution flow that creates a plan, invokes model/chat, routes
  model tool calls, records observations, and returns a report.
- Tool execution through `ToolExecutionService`.
- MCP tool execution through `McpToolExecutionService`.
- Memory and knowledge context assembly through the existing chat path.
- Fail-closed behavior for policy denial, permission requirements, missing
  providers, unknown tools, invalid plans, and failed runtimes.
- Contract tests using deterministic fake providers and the existing Rig plugin
  fail-closed providers.

Out of scope for this phase:

- Connecting Rig live model or tool execution.
- Infinite autonomous loops.
- Multi-run retry policy.
- Human approval continuation after `permission_required`.
- Durable persistence of execution reports.
- Product UI changes.
- Generated SDK or OpenAPI changes.

## Current Baseline

The current repository already has the required provider-level pieces:

- `AgentChatService` assembles messages, memory context, knowledge context, and
  optional tool descriptors before delegating to `ModelExecutionService`.
- `ModelExecutionService` selects model providers by id, evaluates
  `model.invoke`, gates sensitive context through
  `model.send_sensitive_context`, and validates structured output when
  requested.
- `ToolExecutionService` loads tool descriptors, evaluates policy, and invokes
  tool providers only after an allow decision.
- `McpToolExecutionService` maps MCP tools into SDKWork tool calls and applies
  the same policy gate before MCP invocation.
- Runtime registration supports typed and manifest-only providers for model,
  tool, policy, memory, knowledge, planning, protocol adapter, MCP, skill,
  collaboration, telemetry, installer, and configuration families.
- `sdkwork-agent-plugin-rig` registers typed fail-closed Rig model, tool, MCP,
  memory, knowledge, planning, policy, installer, configuration, and RPC chat
  adapter providers.
- `sdkwork-kernel-plugin-knowledgebase` maps the SDKWork Knowledgebase contract
  to the Agent Kernel `KnowledgeProvider` SPI.

The missing piece is an execution service that composes these contracts into a
standard agent run.

## Architecture

Add `sdkwork-agent-kernel/src/execution.rs` as a focused orchestration module.
Expose it from `sdkwork-agent-kernel/src/lib.rs`.

The service should depend only on existing Agent Kernel SPI and services:

```text
AgentExecutionService
  -> PlanningProvider
  -> AgentChatService
  -> ModelExecutionService
  -> ToolExecutionService
  -> McpToolExecutionService
  -> PolicyProvider through the services above
```

The service must not:

- Import plugin crates.
- Depend on `external/rig`.
- Read files, run processes, call networks, or access secrets directly.
- Reimplement model, tool, MCP, memory, knowledge, or policy gates.
- Execute model-returned tool calls without standard tool/MCP services.

## Public Types

### `AgentExecutionRequest`

Fields:

- `execution_id`
- `messages`
- `provider_id`
- `model_id`
- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `subject`
- `trace_context`
- `timeout_ms`
- `include_tool_descriptors`
- `memory_query`
- `knowledge_query`
- `mcp_server_id`
- `metadata`

The request should mirror the useful chat request fields so the execution loop
can reuse `AgentChatService` instead of creating a second RAG path.

### `AgentExecutionReport`

Fields:

- `execution_id`
- `status`
- `plan`
- `chat_response`
- `tool_executions`
- `mcp_tool_executions`
- `observations`
- `error`

The report is the caller-facing evidence bundle for one execution attempt.

### `AgentExecutionStatus`

Values:

- `Completed`
- `Failed`
- `PermissionRequired`
- `Cancelled`
- `Degraded`

### `AgentObservation`

Fields:

- `observation_id`
- `source_family`
- `action_id`
- `status`
- `summary`
- `redaction_classification`
- `metadata`

Observations are safe summaries of model, tool, and MCP work. They are not raw
provider payload dumps.

## Execution Flow

The initial loop is intentionally bounded:

```text
validate request
  -> ensure runtime is not failed
  -> create a plan through PlanningProvider when available
  -> invoke AgentChatService
  -> collect model messages and model-returned ToolCall values
  -> execute each ToolCall through ToolExecutionService or McpToolExecutionService
  -> append observations
  -> return AgentExecutionReport
```

Tool routing:

- If `ToolCall.provider_id` resolves to a typed tool provider, use
  `ToolExecutionService`.
- If `ToolCall.provider_id` resolves to a typed MCP provider, use
  `McpToolExecutionService`.
- If no provider id is present, try the default tool provider first.
- If the default tool provider cannot describe the tool and an MCP server id is
  present, try the default MCP provider.
- Unknown tools fail closed with a stable kernel error and a failed
  observation.

Planning:

- The service should use `PlanningProvider` when one is registered.
- A missing planning provider should not block execution unless the runtime
  manifest requires planning capabilities.
- Generated plans must validate before model invocation.
- The first implementation may create a single model action plus tool
  observations rather than revising the plan dynamically.

## Error Handling

Rules:

- Blank execution ids or blank messages return `validation_error`.
- A failed runtime returns a runtime-sourced capability error before model,
  knowledge, memory, or tool work.
- Policy deny from model, knowledge, memory, tool, or MCP gates stops execution
  and returns a failed report.
- `needs_approval` maps to `permission_required` and stops before the protected
  provider is invoked.
- Tool/MCP failure records a failed observation and makes the report failed.
- Earlier successful observations remain in the report when a later tool fails.
- Provider errors use existing `KernelError` mapping and must not leak raw
  internal details.

## Test Design

Tests are contract-level Rust tests under
`sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs`.

Required cases:

- Execution service creates a plan and invokes the selected model provider.
- Execution service attaches memory and knowledge context through the chat path.
- Model-returned tool calls execute through `ToolExecutionService`.
- Model-returned MCP tool calls execute through `McpToolExecutionService`.
- Tool policy approval requirement stops before provider invocation.
- MCP policy approval requirement stops before provider invocation.
- Unknown model-returned tool creates a failed observation and failed report.
- Earlier observations are preserved when a later tool fails.
- Failed runtime stops before planning/model/tool work.
- The Rig plugin runtime can be used as a fail-closed execution fixture without
  connecting a live Rig backend.

Verification commands:

```powershell
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml --test agent_execution_service_contracts
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml
node scripts/check-kernel-standards.mjs
```

## Acceptance Criteria

- Agent execution is a kernel service, not a Rig plugin behavior.
- The service composes existing SPI and policy-aware services instead of
  bypassing them.
- Memory and knowledge enrichment reuse existing chat contracts.
- Model-returned tool calls can execute through both tool and MCP provider
  families.
- Protected actions fail closed on policy deny or approval requirement.
- Reports preserve plan, model response, tool results, MCP tool results,
  observations, status, and error evidence.
- Tests prove the service works with deterministic fake providers and remains
  compatible with the Rig plugin fail-closed runtime.
