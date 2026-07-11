# Hermes Agent Mapping



## Source



- Local path: `external/hermes-agent`

- Upstream: `https://github.com/NousResearch/hermes-agent.git`

- PyPI distribution: `hermes-agent` (import probe module: `run_agent`)

- TUI gateway IPC: `tui_gateway` JSON-RPC over stdio



## SDKWork Surface



Hermes Agent maps first to the general Agent Kernel surface:



- `AgentRuntime`

- `ToolProvider`

- `ContextProvider`

- `MemoryProvider`

- `AgentSkillProvider`

- `AgentCollaborationProvider` when handoff or delegation behavior is verified



## Initial Registration Mode



`process-adapter`



`sdkwork-agent-provider-hermes` under `agent-providers/crates/` provides session/message adapters, SDK binding
manifest negotiation, Python-process runtime routing, runtime-backed kernel
providers, and server bootstrap registration when `SDKWORK_KERNEL_AGENT_PLUGIN=hermes`.



## Capability Mapping



| Upstream area | SDKWork capability family |

| --- | --- |

| `hermes_state.SessionDB` / `session.list` | `sdk.session.lifecycle` |

| `run_agent.AIAgent` / `prompt.submit` | `sdk.model.chat` |

| `model_tools` tool dispatch | `sdk.tool.invoke` |

| `agent.skill_commands` | `sdk.skill.invoke` |

| Agent execution | `agent.runtime.*` |

| Tool use | `tool.*` |

| Context assembly | `context.*` |

| Long-term state | `memory.*` |

| Skill-like behavior | `skill.*` |

| Multi-agent behavior | `agent.discover`, `agent.handoff`, `agent.delegate` |



## Policy Boundaries



All tool calls, memory writes, external sends, filesystem access, process

execution, network access, and secret resolution must build SDKWork

`PolicyRequest` values before execution. Upstream tool output remains

untrusted context unless a policy decision narrows trust.



## Event Mapping



Runtime start, task creation, tool call start/completion/failure, policy

decisions, and memory writes should map to `agent.runtime.*`, `agent.task.*`,

`agent.tool.*`, `agent.policy.*`, and `agent.memory.*` events.



## Error Mapping



Unknown capabilities map to `capability_missing`; unavailable upstream runtime

maps to `provider_unavailable`; upstream execution failure maps to

`provider_error`; policy denial maps to `policy_denied`.



## Conformance



Target: manifest profile, adapter crate contract tests, and kernel plugin crate

registration through `SDKWORK_KERNEL_AGENT_PLUGIN`. Local-runtime conformance

requires a live Hermes Agent install or TUI gateway JSON-RPC process.



## Status



- Provider crate: `agent-providers/crates/sdkwork-agent-provider-hermes`

- SDK binding: `bindings/agent-providers/hermes/provider-binding.manifest.json`

- Client bridge plugin: `sdkwork-agent-client` `builtin.hermes` routes local chat through `HermesSdkIntegration` model provider (`SdkModelBridgeRuntime`); remote mode uses internal-api `SseChatClient`

- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=hermes`

- Local source pin (2026-06-24, not a latest-registry claim): `external/hermes-agent` @ `a4a74ca9e` with source `pyproject.toml` version `0.17.0`

- Runtime worker: `scripts/provider-transport-workers/generic_python_sdk_worker.py` via `PythonSdkBackendRuntime` (`run_agent` module probe)

- IPC backend: set `SDKWORK_HERMES_USE_TUI_GATEWAY=1` to prefer `jsonrpc_stdio` via `tui_gateway`

- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`, optional `sdk.skill.invoke`

- Binding execution: `sdk.session.lifecycle` uses provider-local lifecycle
  state through provider-core and declares `execution_scope: provider_local`
  with `runtime_operations: ["ping"]`. Model, tool, and skill capabilities use
  `execution_scope: transport_runtime`; the runtime router rejects any
  operation not declared by the selected backend `runtime_operations` allowlist.

- Fail-closed worker contract: `node scripts/provider-transport-workers/generic-python-sdk-worker.test.mjs`

- Release proof: Hermes-specific staging gateway proof remains required before
  GA or commercial release; the Node staging live SDK gate does not cover the
  Python/TUI Hermes gateway path.

- Production safety: Python SDK backends fail closed when workers cannot spawn,
  Python modules cannot be resolved to an importable entry, selected runtime
  health is unhealthy, or a requested runtime operation is absent from
  `runtime_operations`, unless non-production mock fallback is explicitly
  enabled.
