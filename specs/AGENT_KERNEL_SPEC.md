# SDKWork Agent Kernel Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: agent kernel core contracts, runtime lifecycle, provider SPI,
  protocol adapters, security policy, event model, telemetry, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel`
- Implementation baseline: Rust kernel SPI
- Related:
  - [`../README.md`](../README.md)
  - [`../sdkwork-agent-kernel/README.md`](../sdkwork-agent-kernel/README.md)
  - [`AGENT_MANIFEST_SPEC.md`](./AGENT_MANIFEST_SPEC.md)
  - [`AGENT_INSTALLATION_CONFIGURATION_SPEC.md`](./AGENT_INSTALLATION_CONFIGURATION_SPEC.md)
  - [`AGENT_RUNTIME_SPEC.md`](./AGENT_RUNTIME_SPEC.md)
  - [`AGENT_MODEL_PROVIDER_SPI_SPEC.md`](./AGENT_MODEL_PROVIDER_SPI_SPEC.md)
  - [`AGENT_TOOL_PROVIDER_SPI_SPEC.md`](./AGENT_TOOL_PROVIDER_SPI_SPEC.md)
  - [`AGENT_CONTEXT_MEMORY_SPEC.md`](./AGENT_CONTEXT_MEMORY_SPEC.md)
  - [`AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md`](./AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md)
  - [`AGENT_PLANNING_EXECUTION_SPEC.md`](./AGENT_PLANNING_EXECUTION_SPEC.md)
  - [`AGENT_HOST_PROVIDER_SPI_SPEC.md`](./AGENT_HOST_PROVIDER_SPI_SPEC.md)
  - [`AGENT_PROTOCOL_ADAPTER_SPEC.md`](./AGENT_PROTOCOL_ADAPTER_SPEC.md)
  - [`AGENT_SECURITY_POLICY_SPEC.md`](./AGENT_SECURITY_POLICY_SPEC.md)
  - [`AGENT_EVENT_TELEMETRY_SPEC.md`](./AGENT_EVENT_TELEMETRY_SPEC.md)
  - [`AGENT_UI_CONTRACT_SPEC.md`](./AGENT_UI_CONTRACT_SPEC.md)
  - [`AGENT_CONFORMANCE_SPEC.md`](./AGENT_CONFORMANCE_SPEC.md)
  - [`DOMAIN_SPEC.md`](../../sdkwork-specs/DOMAIN_SPEC.md)
  - [`MODULE_SPEC.md`](../../sdkwork-specs/MODULE_SPEC.md)
  - [`RUST_RPC_SPEC.md`](../../sdkwork-specs/RUST_RPC_SPEC.md)
  - [`SDK_SPEC.md`](../../sdkwork-specs/SDK_SPEC.md)

This specification defines the SDKWork Agent Kernel. It is an industry-level
agent runtime standard intended for code agents, workflow agents, operations
agents, research agents, product assistants, and future multi-agent systems.
BirdCoder is a proving application for the standard, not the owner of the
standard.

The specification uses the following terms:

- `MUST` means the rule is required for compatibility.
- `SHOULD` means the rule is recommended unless a documented reason exists.
- `MAY` means the rule is optional.
- `MUST NOT` means the behavior is forbidden.

## 1. Design Goals

The Agent Kernel standard exists to make agent systems portable, secure,
observable, and replaceable.

Goals:

- Define a stable object model for agents, sessions, tasks, runs, steps,
  messages, parts, artifacts, providers, policy decisions, and events.
- Keep provider implementations replaceable through typed SPI.
- Keep external protocols as adapters, not as the internal kernel model.
- Support local, private, SaaS, desktop, CLI, server, and embedded host modes.
- Support UI integration through typed clients and event streams.
- Support model/tool/memory/runtime provider interoperability.
- Make security, sandboxing, audit, and policy explicit kernel contracts.
- Make conformance testable by third-party providers and host applications.

Non-goals:

- This spec does not define code-agent-specific workspace, VCS, patch, terminal,
  build/test, or review behavior. Those belong to `sdkwork-code-kernel`.
- This spec does not define React UI components. Those belong to
  `sdkwork-kernel-ui`.
- This spec does not require one model vendor, external agent protocol, storage
  engine, or transport.

## 2. Architecture Principles

Agent Kernel follows Linux-kernel-style architecture principles.

Rules:

- Kernel core `MUST` define mechanisms, not product policy.
- Provider variation `MUST` be expressed through typed SPI and manifests.
- External protocols `MUST` be implemented as adapters.
- Core contracts `MUST` be versioned and compatibility-managed.
- Security hooks `MUST` exist before risky actions.
- Runtime state transitions `MUST` be observable.
- Host side effects `MUST` go through host/provider SPI.
- Product applications `MUST NOT` mutate kernel SPI to add product-only
  assumptions.

Layering:

```text
product host / application
  -> protocol adapter or typed kernel client
  -> AgentRuntime
  -> session/task/run/step engine
  -> provider SPI: model, tool, context, memory, knowledge, planning, policy,
     telemetry, host, protocol adapter, MCP, Agent Skill, collaboration
  -> core contracts and event model
```

Forbidden dependency direction:

```text
agent-kernel -> code-kernel
agent-kernel -> kernel-ui
agent-kernel -> product package
agent-kernel -> concrete model vendor as required runtime dependency
agent-kernel -> direct filesystem/process/network/secrets side effects
```

## 3. Standard Object Model

### 3.1 Object Catalog

| Object | Stability | Responsibility |
| --- | --- | --- |
| `AgentManifest` | stable | Static agent package identity and kernel compatibility |
| `AgentCard` | stable | Public discovery profile for external systems and other agents |
| `CapabilityManifest` | stable | Runtime capabilities, feature gates, and compatibility ranges |
| `ProviderManifest` | stable | Provider identity, operations, security model, and version |
| `AgentPackageManifest` | stable | Installable agent package source, lifecycle support, provider binding, configuration requirements, and kernel compatibility |
| `AgentInstaller` | stable | Provider-neutral install, uninstall, upgrade, and package lifecycle SPI |
| `AgentConfigurationSpec` | stable | Agent-owned configuration schema for base settings, login auth, LLM API keys, runtime, security, and custom sections |
| `AgentConfiguration` | stable | Profile-specific typed configuration values with secret references for sensitive fields |
| `AgentRuntime` | stable | Runtime entrypoint for sessions, tasks, events, and providers |
| `AgentSession` | stable | Long-lived scope for interaction, policy, memory, and trace context |
| `AgentTask` | stable | User or agent requested unit of work |
| `AgentRun` | stable | One execution attempt for a task |
| `AgentStep` | stable | Ordered execution unit in a run |
| `AgentMessage` | stable | Message exchanged by users, agents, models, tools, or adapters |
| `AgentPart` | stable | Typed message content unit |
| `AgentArtifact` | stable | Durable task output or referenced output |
| `ModelDescriptor` | stable | Provider-neutral catalog record for a selectable LLM |
| `ModelRequest` | stable | Provider-neutral model request |
| `ModelResponse` | stable | Provider-neutral model response |
| `ToolDescriptor` | stable | Tool declaration and invocation schema |
| `ToolCall` | stable | Tool invocation request |
| `ToolResult` | stable | Tool invocation result |
| `ContextFrame` | stable | Bounded context item used by a run |
| `MemoryRecord` | stable | Durable or retrievable memory item |
| `KnowledgeDocument` | stable | External or domain corpus item that can be searched, read, and converted into context |
| `Plan` | stable | Proposed execution plan |
| `Action` | stable | Executable planned action |
| `Observation` | stable | Result observed after an action |
| `PolicyRequest` | stable | Request for security, permission, or sandbox decision |
| `PolicyDecision` | stable | Decision from policy provider or authorized host |
| `KernelEvent` | stable | Event stream item for UI, adapters, telemetry, and replay |
| `AuditRecord` | stable | Security-relevant immutable record |
| `TraceContext` | stable | Cross-boundary trace and correlation metadata |

Rules:

- Public object names `MUST` remain stable once the spec reaches `1.0`.
- Objects `MUST` have typed ids, not unstructured string maps.
- Extension fields `MUST` be namespaced.
- Provider-specific metadata `MUST` be isolated from core fields.
- Sensitive payload fields `MUST` carry redaction classification.
- All objects crossing transport boundaries `MUST` be serializable.

### 3.2 Identity Rules

Required id fields:

| Object | Id field |
| --- | --- |
| `AgentSession` | `session_id` |
| `AgentTask` | `task_id` |
| `AgentRun` | `run_id` |
| `AgentStep` | `step_id` |
| `AgentMessage` | `message_id` |
| `AgentArtifact` | `artifact_id` |
| `ToolCall` | `tool_call_id` |
| `KernelEvent` | `event_id` |
| `AuditRecord` | `audit_id` |

Rules:

- Ids `MUST` be unique within their owning scope.
- Ids crossing system boundaries `SHOULD` be globally unique.
- Ids `MUST NOT` encode secrets, tenant names, usernames, local filesystem
  paths, or provider credentials.
- Request correlation ids are server/runtime-owned unless a protocol adapter
  requires an externally supplied correlation id.

### 3.3 Extension Payloads

Extension payloads allow code-agent, workflow-agent, product, or provider
specialization without mutating the core object model.

Rules:

- Extension keys `MUST` use reverse-domain or SDKWork namespace style.
- Extension payloads `MUST` declare schema version.
- Extension payloads `MUST` be ignored safely by consumers that do not
  understand them.
- Required extension payloads `MUST` be declared in the capability manifest.
- Core objects `MUST NOT` add product-specific fields when an extension payload
  is sufficient.

Example namespace shape:

```text
sdkwork.code.workspace
sdkwork.code.patch
sdkwork.ops.runbook
com.example.product.case-intake
```

## 4. Manifests And Discovery

Detailed manifest schemas, naming rules, examples, discovery metadata,
capability negotiation, security-profile fields, compatibility rules, and
manifest conformance requirements are defined in
`AGENT_MANIFEST_SPEC.md`. This section defines the Agent Kernel ownership of the
manifest family.

Agent package installation, uninstall, upgrade, and configuration contracts are
defined in `AGENT_INSTALLATION_CONFIGURATION_SPEC.md`.

### 4.1 `AgentManifest`

`AgentManifest` declares the static package identity and compatibility contract.

Required fields:

- `agent_id`
- `name`
- `version`
- `domain`
- `capabilities`
- `kernel_compatibility`
- `provider_requirements`
- `protocol_adapters`
- `security_profile`
- `owner`
- `status`

Rules:

- `agent_id` `MUST` be stable.
- `version` `MUST` follow semantic versioning.
- `kernel_compatibility` `MUST` declare supported Agent Kernel spec versions.
- `provider_requirements` `MUST` list required provider families and
  capabilities.
- `security_profile` `MUST` declare required policy hooks and sandbox
  assumptions.
- Product-specific default configuration `MUST NOT` be embedded in the generic
  manifest unless it is namespaced as product metadata.
- Agent manifests `MUST` conform to `AGENT_MANIFEST_SPEC.md`.

### 4.2 `AgentCard`

`AgentCard` is the public discovery profile for external systems and other
agents. It is inspired by A2A-style discovery but remains SDKWork-owned.

Required fields:

- `agent_id`
- `display_name`
- `description`
- `capabilities`
- `input_modes`
- `output_modes`
- `task_types`
- `protocols`
- `auth_requirements`
- `contact_or_owner`
- `version`

Rules:

- Agent cards `MUST NOT` expose secrets or internal endpoint credentials.
- Agent cards `SHOULD` be safe to publish to trusted registries.
- Private capabilities `MUST` be omitted or marked internal.
- Protocol adapter metadata `MUST` be clearly separated from kernel capability
  metadata.
- Agent cards `MUST` conform to `AGENT_MANIFEST_SPEC.md`.

### 4.3 `CapabilityManifest`

`CapabilityManifest` describes runtime capabilities after provider negotiation.

Required capability families:

- `model`
- `tool`
- `context`
- `memory`
- `knowledge`
- `planning`
- `policy`
- `telemetry`
- `host`
- `protocol_adapter`
- `mcp`
- `skill`
- `collaboration`
- `agent_installer`
- `agent_configuration`

Rules:

- Capabilities `MUST` include id, version, provider id, status, and security
  requirements.
- Optional capabilities `SHOULD` be feature-gated.
- Missing required capabilities `MUST` fail closed at runtime initialization.
- Capability negotiation `MUST` be observable through events.
- Capability manifests `MUST` conform to `AGENT_MANIFEST_SPEC.md`.

### 4.4 Installation And Configuration

Every installable agent `MUST` expose or be installable through an
`AgentInstaller` profile.

Required lifecycle operations:

- Install.
- Uninstall.
- Upgrade.
- Configuration spec read.
- Configuration validation.

Rules:

- Installation, uninstall, upgrade, and configuration mutation `MUST` be
  policy-checkable.
- Runtime bootstrap `MUST` register installer providers as `agent_installer`
  and configuration providers as `agent_configuration`.
- Capability manifests `MUST` expose `agent.install`, `agent.uninstall`,
  `agent.upgrade`, and `agent.configure` with operation lists, side-effect
  levels, and policy categories.
- Installable agent packages `MUST` publish an `AgentPackageManifest` that binds
  agent id, version, package source, lifecycle support, provider ids, kernel
  compatibility, and required configuration section kinds.
- Runtime bootstrap `MUST` be able to consume `AgentPackageManifest` directly
  and derive standard installer/configuration provider manifests from its
  provider bindings.
- Runtime bootstrap `MUST` fail closed when an agent package manifest belongs
  to a different agent id, is incompatible with the current agent-kernel
  version, omits required configuration sections, or declares lifecycle/provider
  bindings that do not validate.
- When a typed local `AgentConfigurationProvider` is registered for a package,
  runtime bootstrap `MUST` verify that the provider's `AgentConfigurationSpec`
  declares every section kind required by the package manifest.
- Runtime implementations `MUST` keep a typed provider registry for local
  provider instances, including model, tool, policy, context, memory, knowledge,
  planning, host, protocol adapter, MCP, Agent Skill, collaboration, telemetry,
  installer, and configuration providers.
  Provider manifests are the negotiation and introspection surface; typed
  registry entries are the execution surface.
- Runtime implementations `MUST` support multiple typed providers with
  provider-id selection for the provider families where composition is expected:
  model, tool, policy, context, memory, knowledge, planning, host, protocol
  adapter, MCP, Agent Skill, collaboration, and telemetry. One agent can use
  different LLM implementations, tool implementations, context assembly
  strategies, memory stores, knowledge retrieval backends, planners, host
  capability bridges, protocol bridges, MCP integrations, skill packs,
  collaboration backends, and observability sinks without replacing the kernel
  object model.
- Runtime implementations that claim MCP support `MUST` expose MCP as
  `provider_family: mcp` with `mcp.tools`, `mcp.resources`, and/or
  `mcp.prompts` capabilities. MCP remains an external protocol surface, not the
  internal kernel object model.
- Runtime implementations that claim Agent Skills support `MUST` expose skills
  as `provider_family: skill` with `skill.discover` and `skill.invoke`
  capabilities.
- Runtime implementations that claim multi-agent collaboration support `MUST`
  expose it as `provider_family: collaboration` with `agent.discover`,
  `agent.handoff`, and `agent.delegate` capabilities.
- Manifest-only provider registration `MUST` be sufficient for capability
  negotiation and introspection but `MUST` return `provider_unavailable` when
  local runtime code tries to invoke a typed SPI instance that was not
  registered.
- Stateful provider SPIs, including memory and telemetry providers, `SHOULD`
  be exposed through synchronized runtime handles so their mutable operations
  remain usable without weakening the runtime object boundary.
- Configuration specs `MUST` be typed, sectioned, and redaction-aware.
- Configuration profiles `MUST` record profile id, agent id, configuration
  version, lifecycle status, typed values, and secret bindings.
- Configuration providers `MUST` be able to plan profile upgrades through a
  policy-checkable configuration migration plan.
- Configuration migration plans `MUST` model value preservation, field rename,
  default assignment, field removal, secret-reference preservation, and
  secret-reference rebinding without exposing raw secret values.
- Configuration stores `MUST` provide provider-neutral persistence SPI for
  saving, loading, listing, migrating, and archiving configuration profiles.
- Configuration profile lifecycle changes `MUST` emit
  `agent.configure.profile.created`, `agent.configure.profile.migrated`, or
  `agent.configure.profile.archived` events with internal redaction.
- Standard configuration sections include base settings, login authentication,
  LLM API key settings, runtime settings, security settings, and custom
  extension sections.
- API keys, passwords, tokens, and similar secret values `MUST` be represented
  as host secret references.
- Raw secrets `MUST NOT` appear in manifests, policy context, events, logs, or
  telemetry.
- Installation lifecycle reports `MUST` map to `agent.install.*` events.

## 5. Runtime Lifecycle

### 5.1 Runtime States

```text
manifest_loaded
  -> providers_registered
  -> configured
  -> ready
  -> degraded
  -> stopping
  -> stopped
  -> failed
```

Rules:

- `ready` means all required capabilities are available.
- `degraded` means optional capabilities are unavailable or unhealthy.
- `failed` means a required capability is unavailable or initialization failed.
- Runtime state changes `MUST` emit `agent.runtime.*` events.
- Runtime health `MUST` be introspectable by hosts through diagnostics that
  report typed providers, manifest-only providers, provider health, capability
  gaps, and missing standard provider families.

### 5.2 Session And Task States

Session states:

```text
created -> active -> paused -> closed -> failed
```

Task states:

```text
created
  -> accepted
  -> planned
  -> running
  -> awaiting_permission
  -> paused
  -> completed
  -> failed
  -> cancelled
```

Run states:

```text
created
  -> planning
  -> executing
  -> awaiting_permission
  -> paused
  -> completed
  -> failed
  -> cancelled
```

Step states:

```text
created
  -> ready
  -> running
  -> awaiting_permission
  -> completed
  -> failed
  -> skipped
  -> cancelled
```

Rules:

- Every transition `MUST` validate source and target state.
- Every transition `MUST` emit a `KernelEvent`.
- Every failed transition `MUST` include a stable error code.
- Cancellation `MUST` be best-effort and observable.
- Permission denial `MUST` be represented as a policy decision and step result.
- Task retry `MUST` create a new run, not overwrite the previous run.

## 6. Message, Part, And Artifact Model

### 6.1 Messages

`AgentMessage` represents communication between users, agents, models, tools,
and protocol adapters.

Required fields:

- `message_id`
- `session_id`
- `task_id` when task-scoped
- `run_id` when run-scoped
- `role`
- `parts`
- `created_at`
- `trace_context`
- `metadata`

Standard roles:

- `user`
- `agent`
- `system`
- `model`
- `tool`
- `policy`
- `adapter`

Rules:

- Messages `MUST` contain at least one part.
- Messages `MUST` preserve provenance.
- Messages from untrusted sources `MUST` be marked as untrusted context.
- Messages `MUST NOT` contain unredacted secrets in telemetry exports.

### 6.2 Parts

Standard `AgentPart` types:

- `text`
- `json`
- `binary_ref`
- `file_ref`
- `artifact_ref`
- `image_ref`
- `audio_ref`
- `tool_call_ref`
- `policy_decision_ref`
- `error`

Rules:

- Inline binary payloads `SHOULD` be avoided.
- File and artifact references `MUST` go through host/provider access policy.
- JSON parts `MUST` identify schema when used for typed contracts.
- Unknown part types `MUST` be safely ignored or rejected based on capability
  negotiation.

### 6.3 Artifacts

Artifacts are durable outputs produced by tasks.

Required fields:

- `artifact_id`
- `task_id`
- `producer_step_id`
- `kind`
- `content_ref`
- `mime_type` when applicable
- `created_at`
- `provenance`
- `redaction_classification`

Rules:

- Artifact content `MUST` be retrievable through an authorized provider.
- Artifact metadata `MUST` preserve producer provenance.
- Artifact deletion/retention behavior `MUST` be declared when artifacts may
  contain tenant, personal, sensitive, or regulated data.

### 6.4 Rust Baseline

The Rust SPI baseline exposes `AgentMessage`, `AgentPart`, and `AgentArtifact`
as provider-neutral kernel objects.

Implemented baseline behavior:

- `AgentMessageRole` covers `user`, `agent`, `system`, `model`, `tool`,
  `policy`, and `adapter`.
- `AgentPartKind` covers text, JSON, binary references, file references,
  artifact references, image references, audio references, tool-call
  references, policy-decision references, and errors.
- Message objects preserve session/task/run/step context, trace context,
  created-at metadata, namespaced metadata, untrusted-source marking, and
  redaction aggregation across parts.
- Part objects preserve schema, MIME type, display name, content reference,
  artifact/tool/policy references, provenance, redaction classification, and
  namespaced metadata.
- Artifact objects preserve task scope, producer step, artifact kind, content
  reference, MIME type, display name, provenance, retention policy, redaction
  classification, and namespaced metadata.
- Message and artifact objects map to `agent.message.created` and
  `agent.artifact.created` kernel events for UI timelines, telemetry streams,
  audit observers, and protocol adapters.
- Artifact objects generate `artifact.read` and `artifact.write` policy
  requests so content access remains authorized by kernel policy.

## 7. Provider SPI

Provider-family details are split into focused specifications:

- Model provider: `AGENT_MODEL_PROVIDER_SPI_SPEC.md`
- Tool provider: `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
- Context and memory: `AGENT_CONTEXT_MEMORY_SPEC.md`
- Knowledge provider and RAG retrieval boundary:
  `AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md`
- Planning and execution: `AGENT_PLANNING_EXECUTION_SPEC.md`
- Host provider: `AGENT_HOST_PROVIDER_SPI_SPEC.md`
- Agent installation and configuration:
  `AGENT_INSTALLATION_CONFIGURATION_SPEC.md`
- Security and policy: `AGENT_SECURITY_POLICY_SPEC.md`
- Event and telemetry: `AGENT_EVENT_TELEMETRY_SPEC.md`
- Protocol adapters: `AGENT_PROTOCOL_ADAPTER_SPEC.md`

### 7.1 Common Provider Requirements

Every provider must expose a `ProviderManifest`.

Required fields:

- `provider_id`
- `provider_type`
- `version`
- `kernel_compatibility`
- `capabilities`
- `security_requirements`
- `configuration_schema`
- `health_check`
- `owner`
- `status`

Rules:

- Provider ids `MUST` be stable.
- Provider capabilities `MUST` be explicit.
- Required provider configuration `MUST` be schema-validated.
- Providers `MUST` expose health state.
- Providers `MUST` fail closed when required security policy is unavailable.
- Providers `MUST` support deterministic fake implementations for tests.

### 7.2 `ModelProvider`

`ModelProvider` abstracts model discovery and invocation.

Required operations:

- `list_models`
- `describe_model`
- `invoke`
- `health`

Optional operations:

- `prepare`
- `stream`
- `cancel`

Capability flags:

- `model.catalog`
- `model.chat`
- `model.reasoning`
- `model.tool_call`
- `model.streaming`
- `model.embedding`
- `model.multimodal_input`
- `model.structured_output`
- `model.usage_reporting`
- `model.cancellation`

Rules:

- Model catalogs `MUST` use `ModelDescriptor` so providers can expose more than
  one selectable LLM without adding product-specific DTOs.
- Requests that need a specific model `MUST` set `model_id`; missing `model_id`
  means the provider default.
- Runtime negotiation `MUST` use provider-declared model capabilities such as
  `model.catalog`, `model.tool_call`, and `model.structured_output` instead of
  hard-coding `model.chat`.
- Model requests `MUST` carry trace context.
- Tool-call outputs `MUST` be represented as typed tool-call requests.
- Usage metadata `SHOULD` include token or equivalent accounting when available.
- Provider raw responses `MAY` be retained in redacted diagnostics, but core
  behavior `MUST` use provider-neutral responses.
- Model provider errors `MUST` map to stable kernel error codes.

### 7.3 `ToolProvider`

`ToolProvider` abstracts tools. It may wrap MCP tools, local tools, RPC tools,
HTTP tools, host tools, or product-specific tools.

Required operations:

- `list_tools`
- `describe_tool`
- `authorize_tool_call`
- `invoke_tool`
- `stream_tool_call` when supported
- `cancel_tool_call`
- `health`

Tool descriptor required fields:

- `tool_id`
- `name`
- `version`
- `description`
- `input_schema`
- `output_schema`
- `side_effect_level`
- `permission_requirements`
- `timeout_policy`
- `cancellation_policy`
- `audit_policy`

Rules:

- Tool input and output schemas `MUST` be explicit.
- Tool calls `MUST` pass through policy before invocation.
- Tool results `MUST` include success/failure status and normalized error data.
- Tool providers `MUST` identify side effects.
- Tools that can read or write sensitive data `MUST` declare that capability.
- MCP is a supported adapter pattern, not the only tool provider model.

### 7.4 `ContextProvider`

`ContextProvider` assembles bounded context for a run.

Required operations:

- `collect`
- `rank`
- `trim`
- `explain`

Rules:

- Context frames `MUST` preserve source provenance.
- Untrusted context `MUST` be marked.
- Context trimming `SHOULD` be deterministic where possible.
- Sensitive context `MUST` carry redaction metadata.
- Context providers `MUST NOT` own the canonical knowledge retrieval contract.
  Retrieval over domain corpora, wiki systems, search indexes, graph stores,
  SQL stores, vector stores, or external knowledge APIs belongs to
  `KnowledgeProvider`; context providers select, rank, trim, and explain the
  frames passed to a run.

### 7.5 `MemoryProvider`

`MemoryProvider` manages durable and retrievable memory.

Required operations:

- `provider_manifest`
- `query`
- `write`
- `delete`
- `export`
- `health`

Rules:

- Memory writes `MUST` pass through policy.
- Memory records `MUST` declare retention and classification metadata.
- Tenant/user/session scope `MUST` be explicit when the host is multi-tenant.
- Memory providers `MUST` support deletion/export when required by privacy
  policy.
- Memory providers `MUST NOT` be used as the standard RAG abstraction unless
  the data is truly agent/user/session memory. Domain knowledge bases, product
  documentation, wiki trees, search services, and retrieval APIs belong to
  `KnowledgeProvider`.

### 7.6 `KnowledgeProvider`

`KnowledgeProvider` abstracts provider-neutral knowledge retrieval and document
access. It is the standard RAG retrieval boundary and is independent of vector
stores. Vector retrieval, keyword search, wiki traversal, graph lookup,
structured query, and external search services are implementation details behind
the same SPI.

Required operations:

- `provider_manifest`
- `search`
- `read`
- `list`
- `health`

Capability flags:

- `knowledge.search`
- `knowledge.read`
- `knowledge.list`

Rules:

- Knowledge providers `MUST` declare exact `knowledge.*` capabilities.
- Knowledge search requests `MUST` allow provider-neutral retrieval methods,
  including exact, keyword, full-text, structured, graph, vector, hybrid,
  LLM-rerank, and external retrieval.
- Knowledge results `MUST` preserve provenance, trust level, redaction
  classification, source URI when available, and retrieval method.
- Knowledge documents `MUST` be convertible into `ContextFrame` without losing
  provenance, trust, or redaction classification.
- Knowledge providers `MUST NOT` imply model generation. RAG is composed as
  `KnowledgeProvider -> context selection/assembly -> ModelProvider`.
- Provider implementations such as Rig, Hermes Agent, OpenClaw, Codex, Claude
  Code, OpenCode, Gemini, or custom in-house knowledge bases `MUST` remain
  adapters behind this SPI and `MUST NOT` leak provider-specific document or
  vector-store types into kernel-core contracts.

### 7.7 `PlanningProvider`

`PlanningProvider` may create or validate plans.

Required operations:

- `create_plan`
- `validate_plan`
- `revise_plan`

Rules:

- Plans `MUST` be inspectable.
- Risky actions `MUST` be represented as explicit actions needing policy checks.
- Plan revision `MUST` preserve history.
- Plan generation may be model-backed, rule-backed, or host-provided.

### 7.8 `PolicyProvider`

`PolicyProvider` makes security, permission, and sandbox decisions.

Required operations:

- `evaluate`
- `explain`
- `record_decision`
- `health`

Policy request categories:

- `model.invoke`
- `tool.invoke`
- `memory.write`
- `memory.read`
- `knowledge.search`
- `knowledge.read`
- `knowledge.list`
- `host.filesystem`
- `host.process`
- `host.network`
- `host.secrets`
- `protocol.send`
- `artifact.read`
- `artifact.write`
- `agent.install`
- `agent.uninstall`
- `agent.upgrade`
- `agent.configure`

Rules:

- Missing policy providers for protected actions `MUST` fail closed.
- Policy decisions `MUST` include allow/deny/needs-approval/defer.
- Policy decisions `MUST` include reason codes.
- User approval prompts `MUST` produce a kernel-level policy decision.
- Policy decisions `MUST` be auditable.

### 7.8 `TelemetryProvider`

`TelemetryProvider` exports observability data.

Required operations:

- `record_event`
- `record_audit`
- `record_metric`
- `start_span`
- `finish_span`
- `health`

Rules:

- Trace context `SHOULD` follow W3C Trace Context semantics when crossing
  process or network boundaries.
- Metrics `SHOULD` be compatible with OpenTelemetry concepts.
- Security events `MUST` be auditable even if telemetry export is disabled.
- Telemetry export `MUST` respect redaction classification.

### 7.9 `HostProvider`

`HostProvider` abstracts host capabilities.

Provider subfamilies:

- `filesystem`
- `process`
- `network`
- `secrets`
- `storage`
- `time`
- `environment`
- `executor`

Rules:

- Host operations `MUST` pass through policy when side effects or sensitive data
  are involved.
- Host providers `MUST` declare supported capabilities.
- Host providers `MUST` expose deterministic fake implementations for tests.
- Direct host side effects from kernel core are forbidden.

## 8. Planning And Execution

Planning and execution make agent behavior inspectable and controllable.

Standard loop:

```text
task intake
  -> context collection
  -> plan creation
  -> plan validation
  -> step execution
  -> observation
  -> reconciliation
  -> completion or revision
```

Rules:

- A plan `MUST` contain ordered or dependency-linked actions.
- Each action `MUST` identify required capabilities.
- Each action with side effects `MUST` trigger policy evaluation before
  execution.
- Observations `MUST` preserve provenance.
- Reconciliation `SHOULD` explain why the run continues, revises, completes, or
  fails.
- Execution engines `MUST` support pause and cancellation.

## 9. Security And Risk Management

Agent Kernel security aligns with OWASP LLM and NIST AI risk-management
concerns while remaining SDKWork-specific.

Required risk controls:

- Prompt injection boundary marking.
- Untrusted tool output marking.
- Sensitive information detection and redaction.
- Excessive agency prevention through scoped capabilities.
- Tool misuse prevention through policy and schema validation.
- Supply-chain risk controls for providers and protocol adapters.
- Model/provider output validation before executing side effects.
- Auditability for security-relevant actions.
- Tenant/user/session isolation when the host is multi-tenant.

Rules:

- Untrusted input `MUST` be marked before entering context.
- Tool output `MUST` be treated as untrusted unless the provider declares and
  proves trust semantics.
- Secrets `MUST NOT` be sent to model providers unless policy permits.
- Side-effectful tool calls `MUST` require explicit policy decisions.
- Security bypasses `MUST NOT` be implemented as product-level flags without
  kernel audit.

## 10. Event Model

### 10.1 Event Envelope

`KernelEvent` required fields:

- `event_id`
- `event_type`
- `event_version`
- `occurred_at`
- `source`
- `severity`
- `session_id` when available
- `task_id` when available
- `run_id` when available
- `step_id` when available
- `trace_context` when available
- `correlation_id` when available
- `causation_id` when available
- `redaction_classification`
- `payload_schema`
- `payload`

Rules:

- Event type names `MUST` be stable.
- Event payloads `MUST` be schema-versioned.
- Event ordering `MUST` be preserved within a session when emitted by the same
  runtime.
- Event consumers `MUST` tolerate unknown event types.
- Event streams `MUST` expose completion and error semantics.
- Event replay `MAY` be supported, but replayed events `MUST` be marked.

### 10.2 Standard Event Families

Required event families:

- `agent.runtime.*`
- `agent.manifest.*`
- `agent.provider.*`
- `agent.install.*`
- `agent.configure.*`
- `agent.session.*`
- `agent.task.*`
- `agent.run.*`
- `agent.step.*`
- `agent.message.*`
- `agent.model.*`
- `agent.tool.*`
- `agent.context.*`
- `agent.memory.*`
- `agent.knowledge.*`
- `agent.policy.*`
- `agent.audit.*`
- `agent.telemetry.*`

Recommended event actions:

- `created`
- `updated`
- `started`
- `streamed`
- `completed`
- `failed`
- `cancelled`
- `paused`
- `resumed`
- `denied`
- `approved`
- `degraded`
- `recovered`

## 11. Error Model

Agent Kernel errors must be stable, typed, and safe to expose.

Error fields:

- `code`
- `message`
- `kind`
- `retryable`
- `safe_for_user`
- `provider_id` when provider-scoped
- `source`
- `trace_context`
- `details`

Standard error kinds:

- `validation_error`
- `capability_missing`
- `provider_unavailable`
- `provider_error`
- `policy_denied`
- `permission_required`
- `timeout`
- `cancelled`
- `conflict`
- `rate_limited`
- `resource_exhausted`
- `unsafe_content`
- `security_violation`
- `internal_error`

Rules:

- Internal exception details `MUST NOT` be exposed to UI or external adapters
  unless explicitly marked safe.
- Provider errors `MUST` map to stable kernel error kinds.
- Retryability `MUST` be explicit.
- Policy denial `MUST` not be reported as generic internal failure.

### 11.1 Rust Baseline

The Rust SPI baseline keeps existing `KernelError` variants source-compatible
while adding a typed error view for UI, protocol adapters, telemetry, and
conformance tests.

Implemented baseline behavior:

- Legacy variants `Validation`, `CapabilityMissing`, `ProviderUnavailable`,
  `PolicyDenied`, and `Internal` expose `kind`, `code`, `message`,
  `safe_message`, `retryable`, `safe_for_user`, provider id, source, and
  redaction metadata through typed accessors.
- `KernelErrorKind` covers validation, missing capability, provider
  unavailable, provider error, policy denied, permission required, timeout,
  cancelled, conflict, rate limited, resource exhausted, unsafe content,
  security violation, and internal error.
- `KernelErrorSource` identifies runtime, provider, model, tool, context,
  memory, policy, host, protocol adapter, kernel UI, code kernel, telemetry, or
  unknown source.
- Structured errors preserve custom code, internal message, optional user-safe
  message, retryability, user-safety flag, provider id, source, trace context,
  diagnostic details, and redaction classification.
- `KernelError::to_event` maps errors to `agent.error.occurred` without
  exporting internal diagnostic details in the event payload.
- Protocol adapters map errors through safe messages and stable protocol error
  codes, preserving policy denial as a permission error and internal failures
  as safe internal errors.

## 12. Protocol Adapter Standard

Protocol adapters translate between external protocols and the SDKWork kernel
object model.

Supported adapter families:

- MCP adapter.
- A2A adapter.
- HTTP API adapter.
- RPC/gRPC adapter.
- Local IPC adapter.
- Tauri command adapter.
- WebSocket event-stream adapter.
- Kernel UI client adapter.

Rules:

- Adapters `MUST NOT` own kernel runtime state.
- Adapters `MUST` map external messages to kernel objects.
- Adapters `MUST` map kernel errors to protocol-native errors.
- Adapters `MUST` propagate trace context where the protocol supports it.
- Adapters `MUST` enforce authentication and authorization before exposing
  protected kernel operations.
- Adapter-specific metadata `MUST` be namespaced.

Rust baseline:

- `ProtocolObjectEnvelope` is the SDKWork-owned intermediate object for
  protocol mapping conformance.
- `ProtocolObjectMapper` maps SDKWork messages, artifacts, events, and errors
  into protocol envelopes before protocol-specific serialization.
- Protocol object envelopes carry schema, trace, redaction, external id,
  namespaced metadata, and mapping-loss notes.
- Standard mappings avoid leaking sensitive message part payloads and internal
  error details.

## 13. UI Contract

Kernel UI clients consume typed runtime state and event streams.

UI-facing contract requirements:

- Runtime manifest read.
- Capability manifest read.
- Session create/list/read/close.
- Task create/read/cancel/retry.
- Run read/pause/resume/cancel.
- Event stream subscribe.
- Permission request read/respond.
- Artifact read through authorized references.
- Diagnostics read.

Rules:

- UI clients `MUST` call typed kernel clients or service adapters.
- UI clients `MUST NOT` directly invoke host filesystem, process, network, or
  secret operations for kernel behavior.
- UI permission responses `MUST` be converted to `PolicyDecision`.
- UI event streams `MUST` preserve event ids and trace/correlation metadata.
- UI clients `SHOULD` degrade gracefully when optional capabilities are absent.

## 14. Compatibility And Versioning

Rules:

- Agent Kernel spec versions `MUST` follow semantic versioning after `1.0`.
- Pre-`1.0` versions may change, but breaking changes `SHOULD` include
  migration notes.
- Provider manifests `MUST` declare compatible kernel spec ranges.
- Runtime initialization `MUST` reject providers outside compatible ranges
  unless a documented compatibility shim is enabled.
- Deprecated fields `SHOULD` remain available for at least one minor release
  after deprecation once `1.0` is reached.
- Breaking changes `MUST` update conformance tests.

Compatibility levels:

| Level | Meaning |
| --- | --- |
| `experimental` | Not stable; may change without compatibility guarantee |
| `candidate` | Intended stable shape; migration notes required for breaking changes |
| `stable` | Compatibility guaranteed under semantic versioning |
| `deprecated` | Supported temporarily with migration path |
| `removed` | Not supported |

## 15. Conformance

The conformance suite is the mechanism that makes the standard enforceable.
Detailed profiles and reporting requirements are defined in
`AGENT_CONFORMANCE_SPEC.md`.

Required conformance groups:

- Manifest validation.
- Agent installation, uninstall, upgrade, configuration schema, and secret
  reference validation.
- Capability negotiation.
- Provider registration.
- Runtime lifecycle.
- Session/task/run/step lifecycle.
- Model provider behavior.
- Tool provider behavior.
- Context and memory provenance.
- Policy allow/deny/needs-approval behavior.
- Event envelope and ordering.
- Error mapping.
- Trace propagation.
- Protocol adapter mapping for enabled adapters.
- Security redaction and audit behavior.

Minimum provider tests:

- Provider manifest validates.
- Provider health reports available/unavailable.
- Runtime diagnostics reports typed providers, manifest-only providers,
  provider health, degraded capabilities, and missing standard provider
  families.
- Required capability exists.
- Unsupported capability fails predictably.
- Cancellation behavior is declared and tested.
- Errors map to stable kernel error kinds.
- Sensitive data is redacted from telemetry.

Minimum runtime tests:

- Runtime fails closed without required policy provider.
- Runtime emits lifecycle events in order.
- Runtime typed provider registry covers every core provider family.
- Manifest-only providers report `provider_unavailable` for direct local SPI
  execution.
- Task retry creates a new run.
- Permission denial is auditable.
- Unknown optional capability degrades gracefully.
- Fake providers can run deterministic sessions.

## 16. Documentation Requirements

Every implementation package or provider must document:

- Capability and provider family.
- Public SPI/API exports.
- Manifest fields and example.
- Required configuration.
- Installation, uninstall, upgrade, and configuration behavior when the package
  is an installable agent.
- Security assumptions.
- Supported host modes.
- Event families emitted.
- Error mapping.
- Conformance commands.
- Owner and status.

## 17. Acceptance Checklist

- [ ] Agent object model is stable and product-neutral.
- [ ] External protocols are adapters, not core model owners.
- [ ] Provider SPI families are explicit.
- [ ] Runtime/session/task/run/step lifecycle is explicit.
- [ ] Policy and security decisions are kernel-level contracts.
- [ ] Event envelope supports UI, telemetry, audit, replay, and adapters.
- [ ] Error model is typed and safe to expose.
- [ ] Compatibility levels and versioning rules are defined.
- [ ] Conformance groups are testable.
- [ ] Code-agent-specific concerns remain outside this base spec.
