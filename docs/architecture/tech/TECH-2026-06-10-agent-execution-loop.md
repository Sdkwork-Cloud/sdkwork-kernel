> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **as-built** (replaces the historical implementation plan)

# Agent Kernel Execution Loop

## Goal

Compose planning, chat/model execution, knowledge and memory context, tool execution, MCP tool execution, policy gates, and observations into one deterministic `AgentExecutionService`.

## Implementation

| Concern | Location |
| --- | --- |
| Orchestration service | `sdkwork-agent-kernel/src/execution.rs` |
| Public exports | `sdkwork-agent-kernel/src/lib.rs` (`AgentExecutionService`, DTOs, status) |
| Contract tests | `sdkwork-agent-kernel/tests/agent_execution_service_contracts.rs` |
| Design rules | [TECH-2026-06-10-agent-execution-loop-design.md](TECH-2026-06-10-agent-execution-loop-design.md) |

The service reuses existing policy-aware services (`AgentChatService`, `ToolExecutionService`, `McpToolExecutionService`) and does not import plugin crates or `external/` source trees.

## Behavior contract

- Fail-closed on blank execution ids/messages, policy denial, permission requirements, missing providers, unknown tools, invalid plans, and failed runtimes.
- Single bounded run per request; no infinite autonomous loop in this service.
- Deterministic contract tests with fake providers and Rig fail-closed plugin providers.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml agent_execution
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml
pnpm verify
```

Do not implement from checkbox steps in `docs/archive/superpowers/plans/2026-06-10-agent-execution-loop.md`.
