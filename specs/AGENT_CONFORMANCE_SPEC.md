# SDKWork Agent Kernel Conformance Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: conformance profiles, required test groups, provider tests, adapter
  tests, runtime tests, security tests, manifest tests, and reporting
- Domain: `intelligence`
- Capability: `agent-kernel.conformance`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_INSTALLATION_CONFIGURATION_SPEC.md`
  - `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

Conformance turns the SDKWork Agent Kernel from documentation into a standard.
A provider, adapter, runtime, or product integration is not compatible unless it
can pass the relevant conformance profile.

## 1. Profiles

| Profile | Target | Required for |
| --- | --- | --- |
| `runtime-core` | Agent runtime implementation | Any runtime claiming SDKWork Agent Kernel compatibility |
| `runtime-manifest` | Runtime manifest and capability negotiation | Any runtime publishing a compatibility manifest |
| `runtime-local` | Runtime with typed local SPI providers | Any host that executes providers in-process |
| `agent-installation` | Agent installer and configuration provider | Any installable agent package |
| `provider-model` | Model provider | Model provider registration |
| `provider-tool` | Tool provider | Tool provider registration |
| `provider-mcp` | MCP provider | MCP tools/resources/prompts registration |
| `provider-skill` | Agent Skill provider | Skill discovery and invocation registration |
| `provider-collaboration` | Collaboration provider | Agent discovery, handoff, and delegation |
| `provider-memory` | Memory provider | Memory provider registration |
| `provider-policy` | Policy provider | Protected operations |
| `provider-host` | Host provider | Filesystem/process/network/secrets/storage host access |
| `adapter-protocol` | Protocol adapter | MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, UI client adapters |
| `security-baseline` | Runtime plus providers | Any runtime that executes side-effectful actions |
| `ui-contract` | Kernel UI client/service adapter | `sdkwork-kernel-ui` integration |

Rules:

- Implementations `MUST` state which profiles they support.
- Product applications `MUST` not claim full compatibility when only a subset
  profile passes.
- Conformance reports `MUST` include spec versions and implementation versions.

## 2. Manifest Conformance

Required cases:

- Agent manifest validates.
- Agent card validates.
- Provider manifest validates.
- Capability manifest validates.
- Semantic versions validate.
- Raw secrets are rejected.
- Required capabilities are enforced.
- Optional missing capabilities degrade runtime.
- Incompatible provider version fails negotiation.
- Deprecated provider reports warning and replacement.

## 3. Runtime Core Conformance

Required cases:

- Runtime loads valid manifests.
- Runtime rejects invalid manifests.
- Runtime registers providers.
- Runtime builds capability manifest.
- Runtime produces `AgentRuntimeDiagnostics`.
- Runtime produces `KernelConformanceReport` for `runtime-manifest` and
  `runtime-local` profiles.
- Runtime registers `agent_installer` and `agent_configuration` providers when
  install/configuration capabilities are enabled.
- Runtime can register multiple model providers and select them by provider id.
- Runtime typed model registration preserves provider-declared model
  capabilities such as `model.catalog`, `model.tool_call`, and
  `model.structured_output` in the capability manifest.
- Runtime can register multiple tool providers and select them by provider id.
- Runtime can register multiple policy providers and select them by provider id.
- Runtime can register multiple context providers and select them by provider
  id.
- Runtime can register multiple memory providers and select them by provider
  id.
- Runtime can register multiple planning providers and select them by provider
  id.
- Runtime can register multiple host providers and select them by provider id.
- Runtime can register multiple protocol adapters and select them by provider id.
- Runtime can register multiple MCP providers for tools, resources, and prompts,
  then select them by provider id.
- Runtime can register multiple Agent Skill providers for skill discovery and
  invocation, then select them by provider id.
- Runtime can register multiple collaboration providers for agent discovery,
  handoff, and delegation, then select them by provider id.
- Runtime can register multiple telemetry providers and select them by provider
  id.
- Runtime host can load, start, stop, fail, query, unload, and aggregate
  diagnostics for multiple runtime implementations.
- Runtime host prevents running runtime slots from being unloaded until they are
  stopped or failed.
- Runtime enters `ready` when required capabilities exist.
- Runtime enters `degraded` when optional capabilities are missing.
- Runtime enters `failed` when required capabilities are missing.
- Session can be created and closed.
- Task can be created, accepted, run, completed, failed, and cancelled.
- Retry creates a new run.
- Every state transition emits a kernel event.
- Unknown optional extension is ignored safely.

Rust baseline report cases:

- `agent.conformance.runtime.required_capabilities.available`
- `agent.conformance.runtime.optional_capabilities.available`
- `agent.conformance.runtime.capabilities.namespaced`
- `agent.conformance.runtime.providers.declared`
- `agent.conformance.runtime.local_providers.typed`
- `agent.conformance.runtime.local_providers.health_available`

Rules:

- Capability ids in runtime conformance reports `MUST` be lowercase ASCII
  namespace ids containing at least one `.` and only letters, numbers, `.`,
  `_`, or `-`.
- `runtime-manifest` validates negotiation evidence and skips local typed
  provider cases.
- `runtime-local` treats manifest-only providers and unhealthy typed providers
  as failed local-runtime conformance.
- Missing optional capabilities are degradation evidence and `MUST NOT` fail a
  profile unless the optional capability is explicitly claimed as required by
  that profile.
- Runtime reports are generated from the capability manifest and diagnostics;
  they must not execute model, tool, host, network, filesystem, or protocol
  operations.

## 4. Agent Installation And Configuration Conformance

Required cases:

- Package manifest declares agent id, version, source, lifecycle support,
  provider bindings, compatible agent-kernel version range, default profile, and
  required configuration sections.
- Package manifest derives standard install, upgrade, and uninstall requests
  without duplicating agent id, version, source, or profile defaults.
- Package manifest rejects missing installer or configuration provider
  bindings.
- Configuration spec declares base settings, login auth, and LLM API key
  sections when those settings are required by the agent.
- Required configuration fields are enforced.
- Login password, token, and LLM API key fields reject raw string secrets when
  secret references are required.
- Installer provider exposes `agent.install`, `agent.uninstall`, and
  `agent.upgrade` capabilities with operations, side-effect levels, and policy
  categories in the runtime capability manifest.
- Configuration provider exposes `agent.configure` with operations,
  side-effect level, and policy category in the runtime capability manifest.
- Runtime typed provider registry can invoke the registered installer and
  configuration provider instances.
- Runtime returns `provider_unavailable` when a manifest-only lifecycle provider
  is negotiated but no typed local SPI instance is registered.
- Missing required install/configuration providers fail closed during runtime
  bootstrap.
- Installer can generate install plan before mutating host state.
- Install plan declares `agent.install` policy category for side-effectful
  installation.
- Install report maps to `agent.install.installed`.
- Upgrade plan declares source version, target version, and rollback
  requirement when applicable.
- Uninstall distinguishes package removal from configuration/data removal.
- Installer and configuration provider expose deterministic fake behavior for
  tests.

## 5. Model Provider Conformance

Required cases:

- Provider manifest validates.
- Provider exposes `ModelDescriptor` catalog entries when it declares
  `model.catalog`.
- Model descriptors declare stable model ids, provider ids, supported
  capabilities, supported modes, response formats, tool capabilities, policy
  categories, and context limits when known.
- Request-level `model_id` selects a model from the provider catalog.
- Unknown model ids fail with stable kernel error mapping.
- Chat invocation returns normalized `ModelResponse`.
- Streaming returns ordered chunks/events.
- Tool-call output maps to typed tool-call request.
- Cancellation behavior matches manifest.
- Usage metadata is reported when capability declares usage reporting.
- Provider error maps to kernel error kind.
- Sensitive prompt data is redacted in telemetry according to policy.

## 6. Tool Provider Conformance

Required cases:

- Tool descriptor validates.
- Invalid input is rejected before invocation.
- Read-only tool invokes successfully under allowed policy.
- Side-effectful tool without policy provider fails closed.
- Denied tool returns denied result and audit record.
- Streaming tool emits start/chunk/completion events.
- Cancellation is idempotent.
- Provider error maps to kernel error kind.
- Tool output is marked untrusted by default.

## 7. Memory And Context Conformance

Required cases:

- Context frames preserve source provenance.
- Untrusted context is marked.
- Context trimming preserves classification metadata.
- Memory write requires policy.
- Memory query respects tenant/user/session scope.
- Memory delete/export is supported when provider declares regulated or
  personal data handling.
- Sensitive memory payload is redacted in telemetry.

## 8. Policy And Security Conformance

Required cases:

- Protected action without policy provider fails closed.
- Policy allow permits constrained operation.
- Policy deny blocks operation and emits audit.
- Needs-approval creates permission request.
- User/host approval becomes `PolicyDecision`.
- Secret read requires policy.
- Prompt injection marker survives context assembly.
- Tool output cannot silently become system instruction.
- Filesystem path traversal is denied.
- Process timeout is enforced.
- Audit sink failure fails closed for audit-required action.

## 9. Event And Telemetry Conformance

Required cases:

- Event envelope includes required fields.
- Event type and version are stable.
- Session event ordering is preserved.
- Trace context propagates across model/tool/policy events.
- Event stream supports completion and errors.
- Unauthorized subscriber is denied.
- Replay marks events as replayed.
- Redaction removes secret payloads.
- Audit export is separate from general telemetry export.

## 10. Protocol Adapter Conformance

Required cases:

- Adapter manifest validates.
- Exposed capabilities are subset of capability manifest.
- External task/message maps to SDKWork objects.
- SDKWork errors map to protocol errors.
- Trace context propagates when protocol supports it.
- Protected operation requires auth and policy.
- Streaming preserves event ids.
- Cancellation maps to kernel cancellation.
- Adapter metadata is namespaced.
- Kernel core tests run without starting adapter server.

## 11. UI Contract Conformance

Required cases:

- UI service can read runtime manifest.
- UI service can read capability manifest.
- UI service can create/read/cancel task.
- UI service can subscribe to event stream.
- UI service can respond to permission request.
- UI service preserves event id and trace metadata.
- UI service degrades when optional capability is missing.
- UI service does not call host filesystem/process/network/secrets directly.

## 12. Reporting

Conformance reports must include:

- Report id.
- Profile id.
- Implementation id and version.
- Spec versions.
- Test suite version.
- Pass/fail/skip counts.
- Skipped tests with reason.
- Required capability matrix.
- Security profile.
- Timestamp.

Rules:

- Skipped required tests `MUST` make the profile fail unless the profile is not
  claimed.
- Experimental capabilities `MAY` have non-blocking tests.
- Reports `MUST` be machine-readable when a compatibility claim is published.
- Generic kernel reports `MUST` validate against
  [`schemas/kernel-conformance-report.schema.json`](./schemas/kernel-conformance-report.schema.json).
- Required capability ids in a report `MUST` be unique while preserving stable
  insertion order for deterministic diffs.

Rust baseline:

- `KernelConformanceReport` is the provider-neutral DTO shared by runtime,
  provider, adapter, security, and installation conformance suites.
- `AgentRuntimeConformanceProfile` defines the Rust runtime report profiles:
  `runtime-manifest` and `runtime-local`.
- `KernelConformanceCase` records stable case id, status, message, required
  flag, optional capability id, and skip reason.
- `KernelConformanceCaseStatus` uses `passed`, `failed`, and `skipped` values
  for machine-readable reports.
- `KernelConformanceReport::is_passed()` fails when any case fails or when a
  required case is skipped.
- `KERNEL_CONFORMANCE_REPORT_SCHEMA` exposes the schema for runners,
  registries, CI gates, provider certification, and host applications.

## 13. Acceptance Checklist

- [ ] Profiles distinguish runtime, providers, adapters, security, and UI.
- [ ] Manifest tests validate discovery and negotiation.
- [ ] Agent installation and configuration tests cover install, uninstall,
      upgrade, required fields, and secret references.
- [ ] Runtime tests cover lifecycle and events.
- [ ] Provider tests cover model/tool/memory/policy/host behavior.
- [ ] Security tests cover fail-closed and audit behavior.
- [ ] Adapter tests prove mapping and isolation.
- [ ] UI tests prove typed client boundary.
- [ ] Reports are machine-readable, schema-validated, and tied to spec
      versions.
