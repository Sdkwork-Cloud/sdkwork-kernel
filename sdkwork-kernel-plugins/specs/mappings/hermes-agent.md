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



`sdkwork-agent-adapter-hermes` provides session/message adapters, SDK binding

manifest negotiation, Python-process runtime routing, and runtime-backed kernel

providers. `sdkwork-agent-plugin-hermes` registers typed providers through

`sdkwork-agent-server` `runtime_bootstrap` when

`SDKWORK_KERNEL_AGENT_PLUGIN=hermes`.



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



- Adapter crate: `sdkwork-kernel-plugins/crates/sdkwork-agent-adapter-hermes`

- Kernel plugin crate: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-hermes`

- SDK binding: `sdks/external-agent-sdks/hermes/sdk-binding.manifest.json`

- Client bridge plugin: `sdkwork-agent-client` `builtin.hermes` routes local chat through `HermesSdkIntegration` model provider (`SdkModelBridgeRuntime`); remote mode uses internal-api `SseChatClient`

- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=hermes`

- Upstream pin (2026-06-24): `external/hermes-agent` @ `a4a74ca9e` (`hermes-agent` PyPI `0.17.0`)

- Runtime worker: `scripts/sdk-backend-workers/generic_python_sdk_worker.py` via `PythonSdkBackendRuntime` (`run_agent` module probe)

- IPC backend: set `SDKWORK_HERMES_USE_TUI_GATEWAY=1` to prefer `jsonrpc_stdio` via `tui_gateway`

- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`, optional `sdk.skill.invoke`

- Production safety: Python SDK backend fail-closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`

