# SDKWork Agent Runtime Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: runtime bootstrap, provider registration, capability negotiation,
  session/task/run/step lifecycle, cancellation, persistence hooks, and runtime
  conformance
- Domain: `intelligence`
- Capability: `agent-kernel.runtime`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`

The Agent Runtime is the kernel execution coordinator. It loads manifests,
registers providers, negotiates capabilities, creates sessions, runs tasks,
emits events, applies policy decisions, and exposes typed APIs to protocol
adapters and UI clients.

## 1. Runtime Responsibilities

The runtime owns:

- Manifest loading and validation.
- Provider registration and health tracking.
- Capability negotiation.
- Runtime state and health.
- Session, task, run, and step lifecycle.
- Event publication.
- Policy gate orchestration.
- Cancellation and timeout coordination.
- Persistence hooks for resumable state.
- Adapter-facing typed APIs.

The runtime does not own:

- Concrete model vendor behavior.
- Concrete tool implementation behavior.
- Product UI behavior.
- Code-agent-specific workspace, patch, terminal, or VCS behavior.
- Direct filesystem/process/network/secrets side effects outside host SPI.

## 2. Runtime State

States:

```text
created
  -> manifest_loaded
  -> providers_registered
  -> configured
  -> ready
  -> degraded
  -> stopping
  -> stopped
  -> failed
```

Rules:

- `ready` requires all required capabilities.
- `degraded` allows missing optional capabilities.
- `failed` is required when required capability, required policy, or manifest
  validation fails.
- Every transition `MUST` emit `agent.runtime.*` event.
- Runtime health `MUST` be queryable by hosts and UI clients.

## 3. Bootstrap Flow

Standard bootstrap:

```text
load kernel config
  -> load agent manifest
  -> validate manifest
  -> load provider manifests
  -> validate provider manifests
  -> register providers
  -> evaluate compatibility
  -> negotiate capabilities
  -> evaluate security profile
  -> build capability manifest
  -> publish runtime health
```

Rules:

- Bootstrap `MUST` be deterministic for the same configuration.
- Bootstrap `MUST` fail closed on security-profile mismatch.
- Bootstrap `MUST` emit manifest/provider/runtime events.
- Bootstrap `MUST` not perform side-effectful agent tasks.
- Runtime builders `MUST` expose explicit registration paths for standard
  `agent_installer` and `agent_configuration` providers.
- Runtime builders `MUST` distinguish manifest-only provider registration from
  typed local SPI provider registration.
- Runtime builders `MUST` expose typed local registration paths for the core
  driver families: `model`, `tool`, `policy`, `context`, `memory`, `planning`,
  `host`, `protocol_adapter`, `mcp`, `skill`, `collaboration`, and
  `telemetry`.
- Runtime builders `MUST` support multiple typed `model` providers in one
  runtime. The default model provider is the deterministic first registered
  typed model provider; callers that need a specific LLM implementation `MUST`
  select it by provider id.
- Typed model provider registration `MUST` preserve the provider manifest's
  declared model capabilities, including `model.catalog`,
  `model.structured_output`, `model.tool_call`, `model.streaming`, and
  `model.cancellation`, rather than forcing every typed model provider to
  expose only `model.chat`.
- Runtime builders `MUST` support multiple typed `tool` providers in one
  runtime. The default tool provider is the deterministic first registered typed
  tool provider; callers that need a specific tool implementation `MUST`
  select it by provider id.
- Runtime builders `MUST` support multiple typed `policy` providers in one
  runtime. The default policy provider is the deterministic first registered
  typed policy provider; callers that need a specific policy implementation
  `MUST` select it by provider id.
- Runtime builders `MUST` support multiple typed `context` providers in one
  runtime. The default context provider is the deterministic first registered
  typed context provider; callers that need a specific retrieval, workspace,
  memory-backed, or host-provided context implementation `MUST` select it by
  provider id.
- Runtime builders `MUST` support multiple typed `memory` providers in one
  runtime. The default memory provider is the deterministic first registered
  typed memory provider; callers that need a specific durable, vector,
  session-scoped, tenant-scoped, or external memory implementation `MUST` select
  it by provider id. Stateful memory provider handles `SHOULD` remain
  synchronized.
- Runtime builders `MUST` support multiple typed `planning` providers in one
  runtime. The default planning provider is the deterministic first registered
  typed planning provider; callers that need a specific model-backed,
  rule-backed, or host-provided planner `MUST` select it by provider id.
- Runtime builders `MUST` support multiple typed `host` providers in one
  runtime. The default host provider is the deterministic first registered typed
  host provider; callers that need a specific filesystem, process, network,
  secret, storage, or remote-host implementation `MUST` select it by provider
  id.
- Runtime builders `MUST` support multiple typed `protocol_adapter` providers in
  one runtime. The default protocol adapter is the deterministic first
  registered typed protocol adapter; callers that need a specific protocol
  adapter implementation `MUST` select it by provider id.
- Runtime builders `MUST` support multiple typed `mcp` providers in one runtime.
  The default MCP provider is the deterministic first registered typed MCP
  provider; callers that need a specific MCP server implementation `MUST` select
  it by provider id.
- Runtime builders `MUST` support multiple typed `skill` providers in one
  runtime. The default Agent Skill provider is the deterministic first
  registered typed skill provider; callers that need a specific skill pack
  implementation `MUST` select it by provider id.
- Runtime builders `MUST` support multiple typed `collaboration` providers in
  one runtime. The default collaboration provider is the deterministic first
  registered typed collaboration provider; callers that need a specific local,
  remote, A2A-backed, or supervisor-backed collaboration implementation `MUST`
  select it by provider id.
- Runtime builders `MUST` support multiple typed `telemetry` providers in one
  runtime. The default telemetry provider is the deterministic first registered
  typed telemetry provider; callers that need a specific audit, event, metrics,
  logs, traces, or external observability sink `MUST` select it by provider id.
  Stateful telemetry provider handles `SHOULD` remain synchronized.
- Registered installer providers `MUST` contribute `agent.install`,
  `agent.uninstall`, and `agent.upgrade` capabilities to the capability
  manifest.
- Registered configuration providers `MUST` contribute `agent.configure` to
  the capability manifest.
- Registered core providers `MUST` contribute their declared capability ids to
  the capability manifest while preserving provider family, provider id,
  version, and status.
- Capability negotiation `MUST` honor `min_version` from agent manifest
  capability requirements. A lower-version provider that declares the
  capability is not compatible for that requirement.
- When more than one provider declares the same capability, bootstrap `MUST`
  deterministically select the first provider that satisfies the requested
  `min_version`.
- Unsatisfied required capability versions `MUST` enter `failed` runtime state;
  unsatisfied optional capability versions `MUST` enter `degraded` runtime
  state.
- Typed installer/configuration provider registration `MUST` make the concrete
  SPI instance available through runtime accessors after bootstrap.
- Typed core provider registration `MUST` make the concrete SPI instance
  available through runtime accessors after bootstrap.
- Typed model provider registration `MUST` expose model catalog discovery
  through `list_models` and `describe_model` when the provider declares
  `model.catalog`.
- Typed MCP provider registration `MUST` expose tools, resources, and prompts
  through the MCP provider SPI without forcing resources or prompts into tool
  descriptors.
- Typed Agent Skill provider registration `MUST` expose skill discovery,
  description, invocation, and health through the skill provider SPI.
- Typed collaboration provider registration `MUST` expose agent discovery,
  handoff, delegation, and health through the collaboration provider SPI.
- Manifest-only providers `MUST` remain valid for negotiation and capability
  introspection but `MUST` report `provider_unavailable` when local runtime code
  attempts direct SPI execution.
- Stateful typed providers whose SPI mutates internal state, such as memory and
  telemetry providers, `SHOULD` be exposed through synchronized runtime handles
  rather than immutable references.
- All core provider SPI families `MUST` expose health. Provider traits that do
  not need a custom probe `MAY` use a default `available` health result, but
  hosts must still be able to override it.
- Installer/configuration provider registration `MUST` emit observable provider
  registration events before the final runtime health event.

## 4. Runtime API Surface

Required runtime operations:

- `get_runtime_manifest`
- `get_capability_manifest`
- `get_health`
- `create_session`
- `get_session`
- `list_sessions`
- `close_session`
- `create_task`
- `get_task`
- `cancel_task`
- `retry_task`
- `get_run`
- `pause_run`
- `resume_run`
- `cancel_run`
- `subscribe_events`
- `respond_to_permission`
- `get_diagnostics`
- `get_agent_installer_provider`
- `get_agent_configuration_provider`
- `get_model_provider`
- `list_model_provider_ids`
- `get_model_provider_by_id`
- `get_tool_provider`
- `list_tool_provider_ids`
- `get_tool_provider_by_id`
- `get_policy_provider`
- `list_policy_provider_ids`
- `get_policy_provider_by_id`
- `get_context_provider`
- `list_context_provider_ids`
- `get_context_provider_by_id`
- `get_memory_provider`
- `list_memory_provider_ids`
- `get_memory_provider_by_id`
- `get_planning_provider`
- `list_planning_provider_ids`
- `get_planning_provider_by_id`
- `get_host_provider`
- `list_host_provider_ids`
- `get_host_provider_by_id`
- `get_protocol_adapter`
- `list_protocol_adapter_ids`
- `get_protocol_adapter_by_id`
- `get_mcp_provider`
- `list_mcp_provider_ids`
- `get_mcp_provider_by_id`
- `get_agent_skill_provider`
- `list_agent_skill_provider_ids`
- `get_agent_skill_provider_by_id`
- `get_collaboration_provider`
- `list_collaboration_provider_ids`
- `get_collaboration_provider_by_id`
- `get_telemetry_provider`
- `list_telemetry_provider_ids`
- `get_telemetry_provider_by_id`

Rules:

- API calls `MUST` be typed.
- Protected calls `MUST` pass through auth/policy context.
- API calls `MUST` emit events for state changes.
- API calls `MUST` return stable kernel errors.

## 4.1 Runtime Diagnostics

`get_diagnostics` returns the standard runtime diagnostic view. It is
side-effect-free and suitable for UI clients, CI gates, registry validation,
support bundles, and conformance runners.
Machine-readable reports `MUST` validate against
[`schemas/agent-runtime-diagnostics.schema.json`](./schemas/agent-runtime-diagnostics.schema.json).

Required fields:

- Runtime id.
- Agent id.
- Runtime state.
- Provider count.
- Capability count.
- Typed provider count.
- Manifest-only provider count.
- Missing required capabilities.
- Degraded capabilities.
- Per-provider diagnostics:
  - Provider id.
  - Provider family.
  - Provider version.
  - Typed local registration flag.
  - Optional provider health.
  - Declared capabilities.

Rules:

- Manifest-only providers `MUST` have no local health result because no typed
  SPI instance is registered.
- Typed providers `SHOULD` report health. When a provider family has no custom
  probe, the default health is `available`.
- Diagnostics `MUST` distinguish runtime degradation caused by missing
  capabilities from local-execution degradation caused by manifest-only
  providers or unhealthy typed providers.
- Diagnostics `MUST` expose missing standard provider families for full-profile
  conformance without forcing every product runtime to claim those families.
- Missing standard provider families are full-profile coverage information and
  `MUST NOT` by themselves mark a partial runtime degraded.
- Missing optional capabilities are degradation evidence, not a local-runtime
  conformance failure unless the profile makes them required.
- Runtime conformance reports `MUST` be generated from diagnostics and the
  capability manifest. The report generator `MUST` be side-effect-free.
- The standard Rust profiles are `runtime-manifest` for negotiation evidence and
  `runtime-local` for typed local SPI execution evidence.
- `AGENT_RUNTIME_DIAGNOSTICS_SCHEMA` exposes the diagnostic schema to Rust
  hosts, generated SDKs, CI gates, and registry validators.

## 4.2 Runtime Host Lifecycle And Multi-Agent Loading

`AgentRuntime` represents one bootstrapped agent runtime. A kernel host or
supervisor manages multiple runtime slots and their standard lifecycle state.

Required host objects:

- `AgentKernelHost`
- `AgentRuntimeRegistration`
- `AgentRuntimeSlot`
- `AgentRuntimeSlotState`
- `AgentRuntimeExecutionHandle`

Rules:

- The host `MUST` load already bootstrapped runtime implementations by
  registration object.
- Runtime slots `MUST` record runtime id, agent id, implementation id, and the
  runtime object.
- Runtime slots `MUST` expose a lifecycle state. Standard states are `loaded`,
  `running`, `stopped`, and `failed`.
- A newly loaded runtime slot `MUST` start in `loaded` state.
- Runtime ids `MUST` be unique inside a host.
- Load, start, stop, fail, unload, runtime lookup, diagnostics aggregation, and
  conformance report aggregation `MUST` be deterministic and preserve
  registration order.
- Starting a `loaded` or `stopped` slot `MUST` move that slot to `running`.
- Starting an already `running` slot `MUST` be idempotent.
- Starting a `failed` slot `MUST` fail closed with `conflict` until the runtime
  is unloaded or replaced.
- Stopping a `loaded`, `running`, or `stopped` slot `MUST` move that slot to
  `stopped`.
- Failing a slot `MUST` move only that slot to `failed` and preserve a safe
  failure reason for diagnostics and host policy.
- Unloading a `running` slot `MUST` fail closed with `conflict`; a host must stop
  or fail the slot before unloading it.
- Running-runtime queries `MUST` be deterministic and preserve registration
  order.
- A host `MUST` be implementation-neutral: it must not know concrete model
  vendors, MCP server transports, skill pack formats, UI frameworks, network
  protocols, or product workflows.
- Multiple different agents `MAY` run side by side under one host. Scheduling,
  process isolation, thread pools, async runtimes, and remote execution remain
  host implementation policy, but the SPI boundary `MUST` allow independent
  runtime slots to be loaded, queried, and unloaded without mutating other
  slots.

## 5. Session Lifecycle

Session states:

```text
created -> active -> paused -> closed -> failed
```

Rules:

- Sessions `MUST` carry policy context and trace context.
- Sessions `MUST` declare memory/context bindings when used.
- Closing a session `SHOULD` cancel active runs unless host policy says
  otherwise.
- Session resume `MUST` restore visible state and explain unavailable state.

## 6. Task, Run, And Step Lifecycle

Task states:

```text
created -> accepted -> planned -> running -> awaiting_permission -> paused -> completed -> failed -> cancelled
```

Run states:

```text
created -> planning -> executing -> awaiting_permission -> paused -> completed -> failed -> cancelled
```

Step states:

```text
created -> ready -> running -> awaiting_permission -> completed -> failed -> skipped -> cancelled
```

Rules:

- A task may have multiple runs.
- Retry `MUST` create a new run.
- Steps `MUST` preserve order or dependency metadata.
- Step side effects `MUST` pass policy before execution.
- Step result `MUST` map to stable status and error kinds.

## 7. Cancellation And Timeout

Rules:

- Runtime cancellation `MUST` be best-effort and observable.
- Cancellation `MUST` propagate to model, tool, host, and adapter operations
  when supported.
- Timeout policy `MUST` be explicit per operation family or runtime default.
- Cancelled and timed-out states `MUST` be distinct from generic failure.
- Repeated cancellation requests `MUST` be idempotent.

## 8. Persistence And Resume

Rules:

- Runtime state persistence `MAY` be pluggable.
- Persisted state `MUST` include enough metadata to reconstruct session, task,
  run, step, event, and policy history needed for user-visible resume.
- Secrets `MUST NOT` be persisted in runtime state unless using approved secret
  storage.
- Resume `MUST` record an event.
- Missing provider during resume `MUST` degrade or fail according to required
  capability status.

## 9. Conformance

Required cases:

- Valid manifests bootstrap to `ready`.
- Missing optional provider bootstraps to `degraded`.
- Missing required provider bootstraps to `failed`.
- Required `agent.install` or `agent.configure` capabilities bootstrap to
  `failed` when their providers are missing.
- Installer/configuration capabilities expose operations, side-effect
  classification, and policy categories in the capability manifest.
- Core SPI providers can be registered as typed local providers and invoked
  through runtime accessors.
- Multiple model providers can be registered and selected by provider id.
- Multiple tool providers can be registered and selected by provider id.
- Multiple policy providers can be registered and selected by provider id.
- Multiple context providers can be registered and selected by provider id.
- Multiple memory providers can be registered and selected by provider id.
- Multiple planning providers can be registered and selected by provider id.
- Multiple host providers can be registered and selected by provider id.
- Multiple protocol adapters can be registered and selected by provider id.
- Multiple MCP providers can be registered and selected by provider id.
- Multiple Agent Skill providers can be registered and selected by provider id.
- Multiple collaboration providers can be registered and selected by provider
  id.
- Multiple telemetry providers can be registered and selected by provider id.
- MCP providers can expose tools, resources, and prompts through typed SPI.
- Agent Skill providers can list, describe, and invoke skills through typed SPI.
- Manifest-only core providers negotiate capabilities but return
  `provider_unavailable` for direct local SPI execution.
- Security-profile mismatch fails closed.
- Session lifecycle emits events.
- Task retry creates a new run.
- Cancellation is idempotent.
- Timeout is distinct from failure.
- Runtime API rejects protected call without policy context.
- Resume records an event.

## 10. Acceptance Checklist

- [ ] Runtime bootstrap is deterministic.
- [ ] Capability negotiation drives ready/degraded/failed state.
- [ ] Runtime API is typed and policy-aware.
- [ ] Runtime typed provider registry covers all core provider families, not
      only installer/configuration providers.
- [ ] Session/task/run/step state machines are explicit.
- [ ] Cancellation and timeout are observable.
- [ ] Resume preserves explainable state.
- [ ] Runtime conformance cases cover bootstrap, lifecycle, policy, cancellation,
      timeout, and resume.
