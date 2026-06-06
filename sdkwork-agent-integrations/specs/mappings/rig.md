# Rig Mapping

## Source

- Local path: `external/rig`
- Upstream: `https://github.com/0xPlaygrounds/rig.git`

## SDKWork Surface

Rig maps first to a complete SDKWork typed plugin:

- `AgentDefinition` with explicit provider bindings
- `ModelProvider`
- `ToolProvider`
- `MemoryProvider`
- `KnowledgeProvider`
- `PlanningProvider`
- `PolicyProvider` for deterministic local conformance
- `AgentInstaller`
- `AgentConfigurationProvider`
- `ContextProvider` only where context assembly, ranking, trimming, or
  explanation is implemented

## Initial Registration Mode

`typed-local-provider`

Rig is Rust-native, so it is the first direct SDKWork Rust SPI adapter. The
SDKWork-owned implementation lives in
`sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig` and depends on
kernel SPI contracts, not on `sdkwork-agent-kernel` depending on Rig.

The Rig adapter exposes `rig_agent_definition()` as the executable standard
definition. It binds `provider.model.rig-rust`,
`provider.tool.rig-rust`, `provider.memory.rig-rust`,
`provider.knowledge.rig-rust`, `provider.planning.rig-rust`,
`provider.policy.rig-rust`, `provider.agent.installer.rig-rust`, and
`provider.agent.configuration.rig-rust` by stable provider id.

Its `model_selection` uses the Rig model provider as the non-fallback default.
Its `tool_call_policy` requires policy before tool execution and only claims
`tool.invoke` until a live `ToolServer` bridge implements streaming or
cancellation. Its `memory_strategy` uses `provider.memory.rig-rust` as an
optional default for session and agent memory scopes.

Rig retrieval, RAG, vector-store lookup, and wiki-like lookup map to SDKWork
`KnowledgeProvider`, not to memory or context. The current typed provider is a
SDKWork-owned local knowledge provider that exposes `knowledge.search`,
`knowledge.read`, and `knowledge.list` while keeping retrieval methods
provider-neutral. A user-owned knowledge base can implement the same SPI
directly; the Rig adapter can be used as one backend option rather than the
standard abstraction.

Rig source inspection shows native custom memory support through
`ConversationMemory`, `InMemoryConversationMemory`, `AgentBuilder::memory(...)`,
and optional `rig-memory` policy wrappers. SDKWork maps that surface to
`MemoryProvider` records with scope, owner context, trust level, and redaction
classification. The current typed provider is a SDKWork-owned local memory
provider that preserves these records and provides the adapter point for a
feature-gated live Rig `ConversationMemory` backend.

Rig source inspection also shows vector store and embedding-oriented retrieval
surfaces in `rig-core`. SDKWork maps those surfaces to `KnowledgeProvider`
requests and results. The optional `rig-core-adapter` Cargo feature wraps
`rig-core` inside the Rig integration crate only and returns SDKWork-owned plan
objects, so Rig types do not leak into `sdkwork-agent-kernel`. Non-vector RAG
approaches such as llm-wiki, keyword, structured, graph, or external lookup
also map to `knowledge.search` with the matching retrieval method.

Rig source inspection also shows first-class custom tool support through
`Tool`, `ToolSet`, dynamic tools, and `ToolServer`. SDKWork maps that surface to
`ToolProvider` and policy-aware `ToolDescriptor` values.

Rig core does not expose an SDKWork-equivalent first-class Agent Skill SPI.
Skill-like behavior in Rig-based systems should be modeled in SDKWork as a
`provider_family: skill` only when an adapter can discover and invoke stable
skills through tools, pipelines, workflows, or sub-agent compositions. The Rig
adapter therefore does not claim `skill.discover` or `skill.invoke` today.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Model abstraction | `model.chat`, `model.streaming`, `model.tool_call` |
| Tool composition | `tool.invoke` |
| Conversation memory | `memory.query`, `memory.write`, `memory.delete`, `memory.export` |
| Retrieval/RAG | `knowledge.search`, `knowledge.read`, `knowledge.list` |
| Agent orchestration | `planning.*` |
| Installation | `agent.install`, `agent.uninstall`, `agent.upgrade` |
| Configuration | `agent.configure`, secret-ref validation |
| Context assembly | `context.*` only when ranking, trimming, or explaining selected context frames |
| Skill-like workflows | `skill.*` only through a future SDKWork skill adapter |

## Policy Boundaries

Model invocation with sensitive context, tool invocation, knowledge retrieval
over private documents, memory writes, and external sends must use SDKWork
policy.
Rig adapters must not read secrets directly; they must consume SDKWork secret
references resolved through host providers.

## Event Mapping

Model, tool, memory, knowledge, planning, context, and policy activity should
map to `agent.model.*`, `agent.tool.*`, `agent.memory.*`,
`agent.knowledge.*`, `agent.step.*`, `agent.context.*`, and
`agent.policy.*`.

## Error Mapping

Unknown model maps to `capability_missing`. Provider setup failure maps to
`provider_unavailable`. Invocation failure maps to `provider_error`.

## Conformance

Implemented target: local-runtime profile with typed model, tool, memory,
knowledge, planning, policy, installer, and configuration providers. Live
upstream Rig model and tool execution remains fail-closed until a feature-gated
backend is deliberately configured. Memory and knowledge are executable through
SDKWork SPI today and are kept provider-neutral so they can be backed by Rig
`ConversationMemory`, Rig vector-store surfaces, or non-vector knowledge-base
implementations without changing kernel contracts.

## Status

Reference source is present. SDKWork adapter code is implemented as a
local plugin with manifests, package lifecycle, configuration, memory SPI,
knowledge SPI, deployment snapshots, diagnostics, and conformance contract
tests. Model and tool execution are fail-closed without a configured live Rig
backend; memory, knowledge, planning, policy, installer, and configuration
contracts are executable through SDKWork SPI.
