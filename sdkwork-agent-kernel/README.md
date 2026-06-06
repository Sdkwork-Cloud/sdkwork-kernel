# SDKWork Agent Kernel

Domain: `intelligence`
Capability: `agent-kernel`
Package type: Rust kernel SPI
Status: standard candidate

`sdkwork-agent-kernel` defines the base kernel SPI for SDKWork-compatible
agents. It is the foundation below code agents, workflow agents, operations
agents, research agents, product assistants, and future multi-agent systems.
BirdCoder validates this kernel through a code-agent scenario, but the contracts
in this package must remain product-neutral and reusable across applications.

The agent kernel follows a Linux-kernel-style design: stable core contracts,
pluggable providers, explicit subsystem boundaries, typed host APIs, security
hooks, event streams, observability, and conformance testing.

## Canonical Specifications

- Kernel overview: [`../README.md`](../README.md)
- Kernel specs index: [`../specs/README.md`](../specs/README.md)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent manifest spec:
  [`../specs/AGENT_MANIFEST_SPEC.md`](../specs/AGENT_MANIFEST_SPEC.md)
- Agent installation and configuration spec:
  [`../specs/AGENT_INSTALLATION_CONFIGURATION_SPEC.md`](../specs/AGENT_INSTALLATION_CONFIGURATION_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
- Agent model provider SPI spec:
  [`../specs/AGENT_MODEL_PROVIDER_SPI_SPEC.md`](../specs/AGENT_MODEL_PROVIDER_SPI_SPEC.md)
- Agent MCP provider SPI spec:
  [`../specs/AGENT_MCP_PROVIDER_SPI_SPEC.md`](../specs/AGENT_MCP_PROVIDER_SPI_SPEC.md)
- Agent Skill provider SPI spec:
  [`../specs/AGENT_SKILL_PROVIDER_SPI_SPEC.md`](../specs/AGENT_SKILL_PROVIDER_SPI_SPEC.md)
- Agent tool provider SPI spec:
  [`../specs/AGENT_TOOL_PROVIDER_SPI_SPEC.md`](../specs/AGENT_TOOL_PROVIDER_SPI_SPEC.md)
- Agent context and memory spec:
  [`../specs/AGENT_CONTEXT_MEMORY_SPEC.md`](../specs/AGENT_CONTEXT_MEMORY_SPEC.md)
- Agent planning and execution spec:
  [`../specs/AGENT_PLANNING_EXECUTION_SPEC.md`](../specs/AGENT_PLANNING_EXECUTION_SPEC.md)
- Agent host provider SPI spec:
  [`../specs/AGENT_HOST_PROVIDER_SPI_SPEC.md`](../specs/AGENT_HOST_PROVIDER_SPI_SPEC.md)
- Agent protocol adapter spec:
  [`../specs/AGENT_PROTOCOL_ADAPTER_SPEC.md`](../specs/AGENT_PROTOCOL_ADAPTER_SPEC.md)
- Agent security and policy spec:
  [`../specs/AGENT_SECURITY_POLICY_SPEC.md`](../specs/AGENT_SECURITY_POLICY_SPEC.md)
- Agent event and telemetry spec:
  [`../specs/AGENT_EVENT_TELEMETRY_SPEC.md`](../specs/AGENT_EVENT_TELEMETRY_SPEC.md)
- Agent UI contract spec:
  [`../specs/AGENT_UI_CONTRACT_SPEC.md`](../specs/AGENT_UI_CONTRACT_SPEC.md)
- Agent conformance spec:
  [`../specs/AGENT_CONFORMANCE_SPEC.md`](../specs/AGENT_CONFORMANCE_SPEC.md)
- SDKWork domain standard: [`../../../../specs/DOMAIN_SPEC.md`](../../../../specs/DOMAIN_SPEC.md)
- SDKWork module standard: [`../../../../specs/MODULE_SPEC.md`](../../../../specs/MODULE_SPEC.md)
- SDKWork documentation standard: [`../../../../specs/DOCUMENTATION_SPEC.md`](../../../../specs/DOCUMENTATION_SPEC.md)
- SDKWork Rust RPC standard: [`../../../../specs/RUST_RPC_SPEC.md`](../../../../specs/RUST_RPC_SPEC.md)

External standards are reference inputs, not direct kernel ownership:

- MCP for tool, resource, prompt, transport, and external-context integration.
- A2A for agent discovery, agent cards, tasks, messages, parts, artifacts, and
  agent-to-agent interoperability.
- OpenTelemetry, W3C Trace Context, and CloudEvents for trace, correlation,
  observability, and event envelope design.
- OWASP LLM guidance and NIST AI RMF for agent security and risk management.

## Scope

The agent kernel owns the generic mechanisms that every SDKWork-compatible
agent needs:

- Agent identity, metadata, manifest, capability negotiation, and lifecycle.
- Agent definition contracts that bind executable agents to model, tool,
  memory, policy, MCP, skill, collaboration, lifecycle, and configuration SPI
  providers by stable provider id.
- Agent card, capability manifest, provider manifest, discovery, and manifest
  conformance rules.
- Agent package installation, uninstall, upgrade, configuration declaration,
  configuration validation, and configuration policy hooks.
- Agent sessions, tasks, runs, steps, messages, parts, and artifacts.
- Model provider SPI, including multiple model providers per runtime and
  provider-id selection for different LLM implementations.
- Tool provider SPI with multiple tool implementations and provider-id
  selection.
- Policy provider SPI with multiple policy engines and provider-id selection.
- MCP provider SPI for tools, resources, prompts, server descriptors, and
  provider-id selection across multiple MCP implementations.
- Agent Skill provider SPI for discoverable and invocable skill packs with
  provider-id selection across multiple skill implementations.
- Collaboration provider SPI for agent discovery, agent cards, handoff,
  delegation, input filtering, and provider-id selection across multiple
  collaboration implementations.
- Context SPI with multiple context assembly implementations and provider-id
  selection, plus memory SPI for durable/retrievable state.
- Planning and execution SPI with multiple planner implementations and
  provider-id selection.
- Policy, permission, security, sandbox, and audit hooks.
- Protocol adapter SPI with multiple protocol bridges and provider-id
  selection.
- Memory, host, and telemetry provider SPI families also support multiple
  registered implementations with deterministic defaults and provider-id
  selection while preserving synchronized handles for stateful providers.
- Runtime host SPI for environment, storage, time, secrets, process, network,
  filesystem, and task execution.
- Event, telemetry, trace, log, metrics, and diagnostic contracts.
- Protocol adapter SPI for MCP, A2A, HTTP/RPC, local IPC, Tauri commands,
  WebSocket streams, and kernel UI clients.
- Conformance requirements for providers, runtimes, and adapters.

The agent kernel does not own code-specific behavior such as patch application,
repository diffing, terminal command plans, build/test parsing, or code review.
Those belong in `sdkwork-code-kernel`, which builds on this package.

## Non-Goals

`sdkwork-agent-kernel` must not:

- Encode BirdCoder-specific workflows, routes, UI state, branding, or defaults.
- Depend on React, Vite, browser APIs, or product UI packages.
- Bind directly to a concrete model vendor.
- Bind directly to MCP, A2A, OpenAI, Anthropic, Gemini, Ollama, or another
  external protocol as the internal object model.
- Execute host filesystem, process, network, or secret operations without going
  through typed host/provider SPI.
- Hide missing provider capabilities behind raw HTTP, manual auth headers,
  dynamic maps, or untyped escape hatches.
- Treat UI permission prompts as final authorization decisions.

## Kernel Object Model

The agent kernel standard is centered on these stable objects:

| Object | Responsibility |
| --- | --- |
| `AgentManifest` | Static identity, ownership, version, supported protocol adapters, and required kernel compatibility |
| `AgentDefinition` | Executable provider-aware agent definition that embeds an `AgentManifest`, binds SPI provider families, and declares default LLM selection, tool-call policy, and memory strategy |
| `AgentCard` | Public discovery profile for other agents and applications |
| `CapabilityManifest` | Runtime capabilities, feature gates, provider ids, security requirements, and compatibility ranges |
| `ProviderManifest` | Model/tool/memory/runtime/provider metadata and declared operations |
| `AgentPackageManifest` | Installable agent package source, lifecycle support, provider binding, configuration section requirements, and kernel compatibility |
| `AgentInstaller` | Provider-neutral install, uninstall, upgrade, and package lifecycle SPI |
| `AgentConfigurationSpec` | Agent-owned configuration schema for base settings, login auth, LLM API keys, runtime, security, and custom sections |
| `AgentConfiguration` | Profile-specific typed configuration values using secret references for sensitive fields |
| `AgentConfigurationProfile` | Versioned configuration profile with lifecycle status, typed values, and secret bindings |
| `AgentConfigurationUpgradePlan` | Policy-checkable configuration migration plan for preserving, renaming, defaulting, removing, preserving secrets, or rebinding secrets |
| `AgentRuntime` | Kernel runtime that creates sessions, executes tasks, dispatches events, and owns provider wiring |
| `AgentSession` | Long-lived interaction scope containing tasks, memory bindings, policy context, and trace context |
| `AgentTask` | User or agent requested unit of work |
| `AgentRun` | One execution attempt for a task |
| `AgentStep` | Ordered execution unit such as model call, tool call, policy check, observation, or handoff |
| `AgentMessage` | Conversation or protocol message exchanged with user, model, tool, or another agent |
| `AgentPart` | Typed message content part such as text, JSON, file reference, image, audio, or artifact reference |
| `AgentArtifact` | Durable output produced by an agent task |
| `ModelDescriptor` | Provider-neutral catalog record for a selectable LLM |
| `ModelRequest` / `ModelResponse` | Provider-neutral model invocation contract with request-level model selection |
| `ToolDescriptor` / `ToolCall` / `ToolResult` | Provider-neutral tool registration and invocation contract |
| `ContextFrame` | Bounded context item made available to a run |
| `MemoryRecord` | Durable or retrievable memory entry |
| `Plan` / `Action` / `Observation` | Planning and execution loop primitives |
| `PolicyRequest` / `PolicyDecision` | Security, permission, and sandbox decision contract |
| `KernelEvent` | Event stream item for UI, diagnostics, audit, replay, and integrations |
| `AuditRecord` | Security-relevant immutable record |
| `TraceContext` | Cross-boundary trace and correlation context |

External protocols may map to these objects, but they must not replace the
kernel model. MCP tools map to `ToolDescriptor` and `ToolCall`. A2A task
messages map to `AgentTask`, `AgentMessage`, `AgentPart`, and `AgentArtifact`.
OpenTelemetry trace context maps to `TraceContext`.

## Runtime Lifecycle

The kernel lifecycle is explicit and observable.

```text
manifest_loaded
  -> providers_registered
  -> configured
  -> ready
  -> session_created
  -> task_created
  -> run_created
  -> planned
  -> awaiting_permission
  -> running
  -> paused
  -> completed
  -> failed
  -> cancelled
```

Rules:

- Every state transition must be emitted as a `KernelEvent`.
- Long-running operations must support cancellation where the provider supports
  cancellation.
- A run may fail without invalidating the session.
- A task may have multiple runs when retry, resume, or revised instructions are
  used.
- Permission denial is a terminal state for the blocked step, not necessarily
  for the whole session.
- Resume must restore enough state to explain prior decisions, not just continue
  with hidden provider memory.

## Rust Package Shape

The concrete crate layout may evolve, but the Rust implementation should keep
these boundaries visible:

```text
sdkwork-agent-kernel/
|-- README.md
|-- Cargo.toml
`-- src/
    |-- lib.rs
    |-- manifest.rs
    |-- installation.rs
    |-- configuration.rs
    |-- capability.rs
    |-- runtime.rs
    |-- session.rs
    |-- task.rs
    |-- message.rs
    |-- model.rs
    |-- tool.rs
    |-- context.rs
    |-- memory.rs
    |-- planning.rs
    |-- policy.rs
    |-- event.rs
    |-- telemetry.rs
    |-- host.rs
    |-- adapter.rs
    `-- error.rs
```

Dependency direction:

```text
adapter -> runtime -> provider SPI -> core contracts
policy -> core contracts
telemetry -> event and trace contracts
host providers -> host SPI
product applications -> runtime builders and adapters
```

Forbidden direction:

```text
agent-kernel -> code-kernel
agent-kernel -> kernel-ui
agent-kernel -> product packages
agent-kernel -> concrete model vendor SDK as required dependency
agent-kernel -> direct filesystem/process/network side effects outside host SPI
```

## Provider SPI Families

The first-class provider families are:

- `ModelProvider`: model catalog discovery, model descriptor lookup,
  request-level model selection, invocation, streaming, tool-call support,
  usage, and cancellation.
- `ToolProvider`: tool discovery, schema, invocation, result mapping,
  permission metadata, and cancellation.
- `ContextProvider`: context assembly, ranking, trimming, and provenance.
- `MemoryProvider`: durable memory, retrieval, write policy, retention, and
  deletion/export where required.
- `PlanningProvider`: optional plan generation, plan validation, and step
  decomposition.
- `PolicyProvider`: capability filtering, sandbox checks, approval routing,
  sensitive-data handling, and final authorization decisions.
- `TelemetryProvider`: traces, logs, metrics, audit sinks, and event exporters.
- `HostProvider`: filesystem, network, process, secrets, storage, time,
  environment, and executor capabilities.
- `ProtocolAdapter`: MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, and kernel UI
  client integration.
- `McpProvider`: `mcp` provider family for MCP server descriptors, tools,
  resources, prompts, invocation, and health.
- `AgentSkillProvider`: `skill` provider family for skill discovery,
  description, invocation, cancellation, model hints, allowed tools, and health.
- `AgentCollaborationProvider`: `collaboration` provider family for agent
  discovery, agent cards, handoff, delegation, input filtering, and health.
- `AgentInstaller`: `agent_installer` provider family for install, uninstall,
  upgrade, package-source handling, and install health.
- `AgentConfigurationProvider`: `agent_configuration` provider family for
  configuration specs, validation, login auth settings, and LLM API key secret
  references.

Provider registration must be manifest-driven. Runtime code should not infer
critical capabilities from type names, environment variables, or hard-coded
vendor assumptions.

Manifest requirements are defined in
[`../specs/AGENT_MANIFEST_SPEC.md`](../specs/AGENT_MANIFEST_SPEC.md). Provider
authors must declare identity, operations, configuration schema, security
requirements, health model, compatibility range, and side-effect level through
machine-readable manifests before runtime registration.

## Rust SPI Baseline

The current Rust crate establishes the first executable baseline for the
standard. It intentionally keeps the public surface dependency-light and
provider-neutral.

Implemented SPI groups:

- Manifest and capability contracts: `AgentManifest`, `ProviderManifest`,
  `AgentDefinition`, `AgentProviderBinding`, `ModelSelectionPolicy`,
  `ToolCallPolicy`, `MemoryStrategy`, `CapabilityRequirement`,
  `CapabilityManifest`, and schema constants.
- Installation and configuration SPI: `AgentInstaller`,
  `AgentPackageManifest`, `AgentPackageLifecycle`,
  `AgentPackageProviderBinding`, `AgentPackageVersionCompatibility`,
  `AgentInstallRequest`, `AgentInstallPlan`, `AgentInstallReport`,
  `AgentUpgradeRequest`, `AgentUpgradePlan`, `AgentUpgradeReport`,
  `AgentUninstallRequest`, `AgentUninstallReport`,
  `AgentConfigurationProvider`, `AgentConfigurationSpec`,
  `AgentConfiguration`, `AgentConfigurationProfile`,
  `AgentConfigurationUpgradeRequest`, `AgentConfigurationUpgradePlan`,
  `AgentConfigurationStore`, `AgentConfigurationStoreRecord`,
  `AgentProfileArchiveRequest`, `AgentSecretBinding`, typed configuration
  sections and fields, login authentication fields, LLM API key fields,
  secret-reference enforcement, profile secret-binding validation,
  policy-checkable configuration migration plans, provider-neutral profile
  persistence, profile lifecycle events, machine-readable package manifest,
  configuration spec, configuration profile, and configuration migration
  parsing, schema constants for `AgentPackageManifest`,
  `AgentConfigurationSpec`, `AgentConfigurationProfile`, and
  `AgentConfigurationUpgradePlan`, install/upgrade/uninstall policy categories,
  and `agent.install.*` event mapping.
- Runtime bootstrap: `RuntimeBuilder`, `RuntimeBootstrapReport`,
  `AgentKernelHost`, `AgentRuntimeRegistration`, `AgentRuntimeSlot`,
  `AgentRuntimeSlotState`, `AgentRuntimeExecutionHandle`, capability
  negotiation, standard installer/configuration provider registration,
  `agent.install`/`agent.uninstall`/`agent.upgrade`/`agent.configure`
  capability metadata, typed local provider registry accessors for
  `ModelProvider`, `ToolProvider`, `PolicyProvider`, `ContextProvider`,
  `MemoryProvider`, `PlanningProvider`, `HostProvider`, `ProtocolAdapter`,
  `McpProvider`, `AgentSkillProvider`, `AgentCollaborationProvider`,
  `TelemetryProvider`, `AgentInstaller`, and `AgentConfigurationProvider`,
  multiple typed model, tool, policy, context, memory, planning, host, protocol
  adapter, MCP, Agent Skill, collaboration, and telemetry provider registration with
  provider-id lookup,
  deterministic default provider selection for multi-provider families,
  `min_version`-aware capability negotiation,
  deterministic compatible-provider selection,
  host-level loading, starting, stopping, failing, and active-unload protection
  for multiple runtime implementations,
  deterministic diagnostics/conformance aggregation for multiple agents,
  manifest-only `provider_unavailable` behavior, synchronized handles for
  stateful memory/telemetry providers, package-manifest-driven provider
  binding, package agent-id and kernel-compatibility validation,
  package-required configuration section enforcement for typed providers,
  ready/degraded/failed state derivation, `RuntimeState::as_str()`, and
  `agent.runtime.*` events.
- Runtime diagnostics: `AgentRuntimeDiagnostics`, `AgentProviderDiagnostic`,
  `AgentRuntime::diagnostics()`, provider/capability counts, typed vs
  manifest-only provider counts, missing required capabilities, degraded
  capabilities, per-provider family/version/capability/health summaries,
  manifest-only provider id helpers, missing standard provider family helpers,
  degraded signal helpers that do not treat missing full-profile families as a
  partial-runtime failure, and `AGENT_RUNTIME_DIAGNOSTICS_SCHEMA` for host
  support, UI, registry validation, and conformance runners.
- Runtime conformance reports: `AgentRuntimeConformanceProfile`,
  `AgentRuntime::conformance_report()`, `runtime-manifest` and
  `runtime-local` profiles, standard runtime conformance case ids, required
  capability matrix population, manifest-only provider detection, degraded
  optional capability reporting without failing optional gaps, and unhealthy
  typed provider detection using the generic `KernelConformanceReport`.
- Model, tool, context, memory, policy, planning, lifecycle, and event
  abstractions, with health SPI across all core provider families.
- Message and artifact object model: `AgentMessage`, `AgentPart`,
  `AgentArtifact`, standard message roles, standard part kinds, artifact kinds,
  trace/provenance metadata, untrusted-context marking, redaction aggregation,
  `agent.message.created` and `agent.artifact.created` event mapping, and
  artifact read/write policy request generation.
- Model SPI: `ModelDescriptor` catalog records, request-level `model_id`
  selection, request context propagation across session/task/run/step,
  context-frame references, attached tool descriptors, structured response
  formats, namespaced model parameters, policy and trace binding, timeout
  metadata, provider-neutral usage accounting, tool-call handoff, streaming
  chunks, cancellation fallback behavior, finish reasons, redaction
  classification, diagnostics, and runtime negotiation of provider-declared
  capabilities such as `model.catalog`, `model.tool_call`, and
  `model.structured_output`, with multiple LLM provider ids exposed by the
  runtime.
- MCP SPI: `McpProvider`, `McpServerDescriptor`, `McpResourceDescriptor`,
  `McpResourceContent`, `McpPromptDescriptor`, and `McpPromptMessage` for MCP
  tools, resources, prompts, invocation, and provider health without making MCP
  the internal kernel model.
- Agent Skill SPI: `AgentSkillProvider`, `AgentSkillDescriptor`,
  `AgentSkillInvocationMode`, `AgentSkillRequest`, `AgentSkillResult`, and
  `AgentSkillStatus` for reusable skills with model hints, allowed tools,
  side-effect metadata, policy categories, invocation, and cancellation
  fallback behavior.
- Tool SPI: descriptor schema metadata, stable tool naming/versioning,
  side-effect classification, policy category mapping, timeout metadata,
  cancellation declaration, audit requirements, contextual tool calls,
  standardized call status, streamed output chunks, timing/error metadata,
  redaction classification, audit references, authorization request generation,
  and missing-capability behavior for unsupported streaming/cancellation.
- Policy SPI: typed policy categories, subject/session/task/run context,
  side-effect level, redaction classification, decision constraints, safe
  reasons, expiry metadata, audit requirements, and policy decision events.
- Event envelope: source subsystem, occurrence time, session/task/run/step
  identifiers, trace context, correlation/causation ids, payload schema,
  redaction classification, replay marker, recorder filters, ordered
  `EventStream` sequences, subscription filters, cursors, replay batches, and
  stream completion/error status.
- Telemetry and audit SPI: `AuditRecord`, `TelemetryMetric`,
  `TelemetryLogRecord`, `TelemetrySpan`, `TelemetryProvider`, policy-decision
  audit derivation, audit-to-event mapping, metric/log/span contextual
  metadata, redaction classification, labels/attributes, and provider sinks for
  events, metrics, logs, audit records, and spans.
- Error model: backward-compatible `KernelError` variants, `KernelErrorKind`,
  `KernelErrorSource`, structured error metadata, retryability,
  user-safe-message separation, provider/source/trace context, redaction
  classification, diagnostic details, `agent.error.occurred` event mapping, and
  protocol-safe error conversion that avoids leaking internal details.
- Host provider SPI: `HostProvider`, `HostPathPolicy`, filesystem, process,
  network, secret, storage, time, environment, and executor request/result
  types.
- Protocol adapter SPI: `ProtocolAdapter`, `ProtocolAdapterManifest`, adapter
  auth/transport/protocol enums, protocol request/response mapping, stream
  update mapping, `ProtocolObjectEnvelope`, `ProtocolObjectKind`,
  `ProtocolObjectMapper`, standard object mapping for messages, artifacts,
  events, errors, and generic extension objects, namespaced adapter metadata
  validation, trace/redaction propagation, and safe protocol error conversion.
- Generic conformance reporting SPI: `KernelConformanceReport`,
  `KernelConformanceCase`, `KernelConformanceCaseStatus`, pass/fail/skip
  aggregation helpers, required-skipped failure semantics, profile/spec/test
  suite/security metadata, required capability matrix, and
  `KERNEL_CONFORMANCE_REPORT_SCHEMA` for CI runners, registries, provider
  certification, and product compatibility gates.

This baseline is not a final completeness claim. It is the kernel ABI seed:
each SPI family must continue to gain conformance tests before being treated as
stable.

## Security Model

Agent execution is treated as untrusted until policy allows a concrete action.

Required controls:

- Explicit capability grants for model, tool, memory, host, and protocol access.
- Prompt-injection and untrusted-context boundary metadata.
- Tool permission checks before invocation.
- Secret redaction in prompts, tool outputs, model outputs, logs, events, and
  traces.
- Sandbox decisions for filesystem, process, network, and host operations.
- Tenant/user/session context propagation for multi-tenant hosts.
- Provider allowlists and deny rules.
- Immutable audit records for permission grants, denials, provider changes,
  memory writes, tool calls, and host operations.
- Fail-closed behavior when policy providers are unavailable.

UI may request approval from a user, but final authorization must be represented
as a `PolicyDecision` emitted by the kernel or host policy provider.

## Event And Telemetry Model

Kernel events are the shared source of truth for UI, protocol streams,
diagnostics, replay, and integration observers.

Required event families:

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
- `agent.policy.*`
- `agent.audit.*`
- `agent.telemetry.*`

Every event must carry:

- Stable event id.
- Event type and version.
- Timestamp.
- Source subsystem.
- Session id when available.
- Task id and run id when available.
- Trace context when available.
- Causation id and correlation id when available.
- Redaction classification.
- Payload schema version.

Event exporters may use CloudEvents-compatible envelopes and OpenTelemetry
correlation, but the internal event contract remains SDKWork-owned.

## Protocol Adapter Position

MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, and kernel UI clients are adapters.
They translate external protocol concepts into SDKWork kernel objects.

Rules:

- Adapter code must not own kernel state.
- Adapter-specific fields must be isolated in adapter metadata.
- Kernel core must be testable without starting an MCP server, A2A endpoint,
  HTTP server, Tauri host, or React UI.
- Adapter conformance tests must prove object mapping, streaming behavior,
  cancellation, error mapping, trace propagation, and authorization behavior.

## Integration With `sdkwork-code-kernel`

`sdkwork-code-kernel` extends the agent kernel.

Allowed extension points:

- Code-specific provider families, such as workspace, VCS, patch, terminal,
  build/test, language intelligence, and review providers.
- Code-specific event families under `code.*`.
- Code-specific policy requests for patch application, terminal execution,
  destructive filesystem changes, and generated-client protection.
- Code-specific UI views implemented in `sdkwork-kernel-ui`.

Forbidden changes:

- Adding code-specific fields to generic `AgentTask` when an extension payload
  is sufficient.
- Making generic model/tool/memory SPI depend on repository concepts.
- Requiring code-agent providers for non-code agents.
- Reversing the dependency so the agent kernel imports the code kernel.

## Conformance Expectations

A compliant implementation must prove:

- Manifest schema validation.
- Agent installation, uninstall, upgrade, configuration schema, and secret
  reference validation.
- Capability negotiation and feature gating.
- Provider registration and missing-capability failure behavior.
- Session/task/run/step lifecycle events.
- Model provider invocation, streaming, cancellation, and usage mapping.
- Tool provider invocation, permission denial, cancellation, and result mapping.
- Context/memory provenance and redaction metadata.
- Policy fail-closed behavior.
- Event ordering, correlation, and trace propagation.
- Adapter mapping for any enabled external protocol.
- Deterministic fake providers for tests.

The conformance suite will be specified under `kernel/specs/` as the standard
matures.

## Verification

Required crate verification commands:

```bash
cargo test --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml
cargo clippy --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml --check
```

## Acceptance Checklist

- [ ] Agent kernel is product-neutral and not BirdCoder-specific.
- [ ] Public concepts are defined in `AGENT_KERNEL_SPEC.md`.
- [ ] Rust implementation boundaries keep core, runtime, provider SPI, policy,
      telemetry, host, and adapter code separated.
- [ ] External protocols are adapters, not the internal object model.
- [ ] Security and policy decisions are kernel-level contracts.
- [ ] Events are suitable for UI, observability, audit, and protocol streaming.
- [ ] Code-agent behavior extends the agent kernel without polluting it.
- [ ] Conformance requirements are explicit enough for third-party providers.

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent
Package type: rust-crate
Status: standardizing

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- None declared in `specs/component.spec.json`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `cargo test --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-kernel/Cargo.toml`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
