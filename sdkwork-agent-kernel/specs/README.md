# SDKWork Agent Kernel Component Specs

This directory is the local standards index for `sdkwork-agent-kernel`.

Root SDKWork standards remain authoritative. Local component specs can narrow or document this component, but they must not contradict [the root standards](../../../sdkwork-specs/README.md).

## Component

| Field | Value |
| --- | --- |
| Name | `sdkwork-agent-kernel` |
| Type | `rust-crate` |
| Root | `sdkwork-kernel/sdkwork-agent-kernel` |
| Domain | `intelligence` |
| Capability | `agent-kernel` |
| Languages | `rust` |
| Status | `standardizing` |

## Contract Manifest

- [component.spec.json](./component.spec.json) is the machine-readable component contract.
- Consumers should integrate through public exports, runtime entrypoints, SDK clients, or adapters declared in the manifest.
- Generated SDK language outputs are represented at their SDK family root instead of duplicating local specs in generated folders.

## Canonical Specs

| Spec | Applies Because |
| --- | --- |
| [COMPONENT_SPEC.md](../../../sdkwork-specs/COMPONENT_SPEC.md) | Local component specs directory and manifest rules. |
| [CONFIG_SPEC.md](../../../sdkwork-specs/CONFIG_SPEC.md) | Runtime configuration, environment, SDK bootstrap, and feature flag rules. |
| [DEPLOYMENT_SPEC.md](../../../sdkwork-specs/DEPLOYMENT_SPEC.md) | SaaS/private/local runtime parity and deployment rules. |
| [DOCUMENTATION_SPEC.md](../../../sdkwork-specs/DOCUMENTATION_SPEC.md) | Module README, examples, ADR, changelog, and runbook rules. |
| [DOMAIN_SPEC.md](../../../sdkwork-specs/DOMAIN_SPEC.md) | Canonical domain ownership and naming. |
| [GOVERNANCE_SPEC.md](../../../sdkwork-specs/GOVERNANCE_SPEC.md) | Standard ownership, exception, compatibility, and migration rules. |
| [MODULE_SPEC.md](../../../sdkwork-specs/MODULE_SPEC.md) | Reusable package contract and dependency direction. |
| [OBSERVABILITY_SPEC.md](../../../sdkwork-specs/OBSERVABILITY_SPEC.md) | Log, metric, trace, audit, and diagnostic rules. |
| [PERFORMANCE_SPEC.md](../../../sdkwork-specs/PERFORMANCE_SPEC.md) | Latency, pagination, bundle, scalability, and retry budget rules. |
| [README.md](../../../sdkwork-specs/README.md) | SDKWork root standards entrypoint. |
| [TEST_SPEC.md](../../../sdkwork-specs/TEST_SPEC.md) | Contract, frontend, SDK, security, parity, and documentation verification rules. |

## Public Exports

- `AgentExecutionService::execute`
- `AgentExecutionRequest`
- `AgentExecutionReport`
- `AgentExecutionReport::to_event`
- `AgentExecutionReport::to_events`
- `AgentExecutionStatus`
- `AgentObservation`
- `AgentObservation::to_event`
- `AgentChatService::invoke`
- `AgentChatRequest`
- `AgentChatResponse`
- `AgentChatRpcAdapter`
- `AgentChatRpcHandler`
- `agent_chat_rpc_adapter_manifest`
- `sandbox_runtime::SandboxSessionCreateRequest`
- `sandbox_runtime::SandboxSessionCommandRequest`
- `sandbox_runtime::SandboxSessionRuntimeProjection`
- `sandbox_runtime::SandboxSessionLifecycleAdapter`

## Sandbox Runtime Boundary

- `sdkwork-agents` remains the authority for `AgentWorkspace` and `AgentSession`.
- `SandboxSessionLifecycleAdapter` maps `agent_workspace_id`/`agent_session_id` into caller-supplied `SandboxWorkspaceId`/`SandboxSessionId` values.
- Sandbox-owned fields and variables use the `sandbox_` prefix; the mapped Agents field is explicitly exposed as `agent_runtime_location_id`.
- Dependency direction is `sdkwork-agents -> sdkwork-kernel -> sdkwork-sandbox`; Kernel never selects a concrete Sandbox Provider.
- The existing root `SandboxProvider` remains a legacy one-shot host-command mechanism and is not the `sdkwork-sandbox` lifecycle SPI.

## Events

- `agent.execution.completed`
- `agent.execution.failed`
- `agent.execution.permission_required`
- `agent.execution.cancelled`
- `agent.execution.degraded`
- `agent.execution.observation.*`

## SDK Clients

- No generated SDK client class is declared at this component boundary.

## Local Extension Specs

- No local extension specs are declared yet.

## Verification

- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`
