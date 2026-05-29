# SDKWork Agent & Code Kernel Standard

Domain: `intelligence`
Capability: `agent-kernel`, `code-kernel`, `kernel-ui`
Package type: industry kernel standard
Status: standard candidate

`kernel/` defines the SDKWork kernel standard for agent and code-agent systems.
It is not a BirdCoder-private implementation detail. BirdCoder is the first
application scenario used to validate, pressure-test, and refine the standard.
The contracts defined here must remain reusable by other SDKWork applications,
desktop hosts, local/private runtimes, SaaS runtimes, IDE-like products, CLI
tools, and future agent applications.

The kernel follows the design spirit of the Linux kernel: keep the core
contracts stable, separate mechanisms from product policy, expose typed
extension points, isolate host-specific drivers, and treat compatibility,
security, observability, and module boundaries as first-class engineering
concerns.

## Canonical References

- SDKWork standards entry: [`../../../specs/README.md`](../../../specs/README.md)
- SDKWork domain standard: [`../../../specs/DOMAIN_SPEC.md`](../../../specs/DOMAIN_SPEC.md)
- SDKWork module standard: [`../../../specs/MODULE_SPEC.md`](../../../specs/MODULE_SPEC.md)
- SDKWork documentation standard: [`../../../specs/DOCUMENTATION_SPEC.md`](../../../specs/DOCUMENTATION_SPEC.md)
- SDKWork Rust RPC standard: [`../../../specs/RUST_RPC_SPEC.md`](../../../specs/RUST_RPC_SPEC.md)
- SDKWork frontend architecture standard for TypeScript/Vite/React workspaces:
  [`../../docs/ARCHITECT.md`](../../docs/ARCHITECT.md)
- Kernel-local specs index: [`./specs/README.md`](./specs/README.md)

Local kernel documents may extend these standards, but they must not contradict
the canonical SDKWork standards.

## Design Positioning

The SDKWork kernel standard is the stable foundation for intelligent agent
systems. It defines the contracts between an agent runtime, model providers,
tool providers, workspace providers, code execution providers, UI surfaces,
security policies, and host applications.

BirdCoder validates this standard through a code-agent product scenario:
repository reading, code editing, patch application, terminal execution,
build/test orchestration, review loops, and developer-facing UI. Those are
important validation cases, but kernel contracts must not encode BirdCoder-only
assumptions.

The kernel standard must support these integration modes:

- Headless agent runtime embedded into another Rust host.
- Code-agent runtime embedded into an IDE, CLI, desktop app, web shell, or
  automation service.
- Standard React kernel UI embedded into a product application.
- Full product assembly, such as BirdCoder, that combines Rust kernel,
  code-agent kernel, kernel UI, product modules, and host adapters.
- Custom providers for model, tool, memory, storage, workspace, VCS, terminal,
  sandbox, policy, telemetry, MCP, Agent Skills, agent installation, and agent
  configuration behavior.

## Linux Kernel Design Mapping

| Linux kernel idea | SDKWork kernel standard |
| --- | --- |
| Kernel core | `sdkwork-agent-kernel` core runtime, lifecycle, capability registry, event bus, error model, and policy hooks |
| Kernel subsystem | Agent, code, workspace, memory, tool, sandbox, model, execution, telemetry, and UI subsystems |
| System call / stable ABI | Typed kernel client API exposed to hosts and UI through IPC, RPC, SDK ports, or adapters |
| Driver model | Provider SPI for models, tools, storage, VCS, terminal, language intelligence, sandbox, and host capabilities |
| Module loader / supervisor | `AgentKernelHost` loads multiple runtime implementations as independent runtime slots |
| VFS | Workspace and file-system abstraction used by code agents instead of direct host file access |
| LSM / security hooks | Capability policy, permission prompts, sandbox decisions, audit hooks, and tenant/user context checks |
| Netlink / event channels | Kernel event stream for session state, tool calls, patches, terminal output, telemetry, and diagnostics |
| `/proc` and `/sys` style introspection | Runtime manifest, capability manifest, health, diagnostics, and observability views |
| Loadable modules | Pluggable providers registered through manifests and typed SPI contracts |
| Stable internal layering | Rust kernel crates do not depend on React/Vite or product UI; UI depends on typed kernel clients only |

The most important rule is the same rule that makes kernel architectures
survive: stable contracts outlive implementations. Providers may change,
products may change, and UI shells may change, but kernel SPI and integration
contracts must remain versioned, explicit, and testable.

## Directory Model

```text
kernel/
|-- README.md
|-- specs/
|-- sdkwork-agent-kernel/
|-- sdkwork-code-kernel/
`-- sdkwork-kernel-ui/
```

### `sdkwork-agent-kernel`

Rust implementation boundary for the general agent kernel SPI.

This layer defines what every SDKWork-compatible agent runtime must expose:

- Agent identity, metadata, capabilities, and lifecycle.
- Agent package installation, uninstall, upgrade, base configuration, login
  authentication configuration, LLM API key configuration, and configuration
  validation SPI.
- Agent session, task, step, and execution state contracts.
- Model provider SPI for chat, reasoning, embedding, tool-call, streaming, and
  multimodal-capable models when supported, including multiple LLM providers
  per runtime with provider-id selection.
- Tool provider SPI for typed tool registration, invocation, cancellation,
  output mapping, permission checks, and audit.
- MCP provider SPI for MCP server descriptors, tools, resources, prompts,
  invocation, and health without replacing kernel objects with MCP objects.
- Agent Skill provider SPI for discoverable, invocable, policy-aware skill
  packs.
- Context and memory ports for short-term context, durable memory, retrieval,
  summarization, and checkpointing.
- Planning and execution contracts for plan creation, step execution,
  reconciliation, retry, pause, resume, and cancellation.
- Runtime host adapters for filesystem, process, network, time, secrets,
  environment, storage, and platform capabilities.
- Security hooks for sandboxing, permission prompts, policy evaluation,
  capability filtering, and sensitive-data handling.
- Event and telemetry contracts for traces, logs, metrics, audit events,
  session events, tool events, and provider diagnostics.
- Plugin/provider registration contracts with explicit manifests and versioned
  capabilities.
- Capability negotiation that honors manifest `min_version` requirements when
  binding providers.

This layer must not contain product-specific workflows, React UI, manually
hard-coded provider credentials, BirdCoder-only defaults, or direct dependencies
on a concrete host shell.

### `sdkwork-code-kernel`

Rust implementation boundary for the code-agent kernel SPI.

This layer inherits `sdkwork-agent-kernel` and specializes it for software
engineering agents. It provides industry-level code-agent abstractions inspired
by Codex, Claude Code, OpenCode, and similar systems, while keeping SDKWork's
own provider-neutral SPI.

Required capability areas:

- Workspace abstraction: repositories, roots, file trees, file reads/writes,
  ignore rules, generated-file policy, and path safety.
- VCS abstraction: Git status, branch, diff, commit metadata, blame, restore,
  and review context through typed providers.
- Patch abstraction: structured diff generation, patch validation, apply,
  reject, rollback, conflict reporting, and user review gates.
- Terminal/process abstraction: command execution, streaming output, exit
  status, working directory, environment, timeout, cancellation, and policy.
- Build/test/lint abstraction: task discovery, command plans, result parsing,
  failure summaries, and reproducible verification evidence.
- Language intelligence abstraction: symbols, references, diagnostics,
  formatting, semantic search, code navigation, and optional LSP/tree-sitter
  providers.
- Code review abstraction: findings, severities, file/line references,
  remediation hints, regression risks, and missing-test reporting.
- Task/session abstraction: user intent, plan, edits, checkpoints, tool traces,
  review status, and resume state.
- Knowledge and documentation abstraction: repository docs, specs, ADRs,
  generated SDK contracts, and external reference metadata.
- Safety abstraction: workspace allowlists, destructive-operation policy,
  secret redaction, network policy, generated-client protection, and audit.

The code kernel must expose code-agent mechanisms. Product policy belongs in
applications and configured providers. For example, whether a product asks the
user before a specific class of command is a policy decision; the kernel must
provide the permission hook, event, and decision point.

### `sdkwork-kernel-ui`

TypeScript + Vite + React implementation boundary for the standard kernel UI
subsystem.

This is a first-class kernel subsystem, not an ad hoc BirdCoder UI. It defines
reusable UI packages, service adapters, hooks, and components that any SDKWork
application can embed to present and control agent/code-agent kernel behavior.

The UI architecture and package standard must follow
[`../../docs/ARCHITECT.md`](../../docs/ARCHITECT.md):

- Use `pnpm` workspace semantics.
- Use TypeScript for types, React for UI, and Vite for development/build.
- Keep the root app thin.
- Put reusable modules and business modules under `packages/`.
- Keep business processing, mock data, kernel client adapters, request/response
  mapping, validation, and error normalization in `service/`.
- Keep `pages/` focused on route-level composition.
- Keep `components/` focused on rendering.
- Keep `hooks/` focused on React state binding and UI behavior.
- Export public APIs only from `src/index.ts`.
- Use internal dependencies with `workspace:*`.
- Do not import another package through deep internal paths.

Recommended kernel UI package family:

```text
kernel/sdkwork-kernel-ui/
|-- src/                              # thin demo/integration shell only
|-- packages/
|   |-- sdkwork-kernel-ui-types/
|   |-- sdkwork-kernel-ui-core/
|   |-- sdkwork-kernel-ui-commons/
|   |-- sdkwork-kernel-ui-services/
|   |-- sdkwork-kernel-ui-agent/
|   |-- sdkwork-kernel-ui-code/
|   |-- sdkwork-kernel-ui-workspace/
|   |-- sdkwork-kernel-ui-terminal/
|   |-- sdkwork-kernel-ui-telemetry/
|   `-- sdkwork-kernel-ui-permissions/
|-- package.json
|-- pnpm-workspace.yaml
|-- tsconfig.json
`-- vite.config.ts
```

Package manifest names must use the scoped SDKWork form:

```text
@sdkwork/kernel-ui-types
@sdkwork/kernel-ui-core
@sdkwork/kernel-ui-commons
@sdkwork/kernel-ui-services
@sdkwork/kernel-ui-agent
@sdkwork/kernel-ui-code
@sdkwork/kernel-ui-workspace
@sdkwork/kernel-ui-terminal
@sdkwork/kernel-ui-telemetry
@sdkwork/kernel-ui-permissions
```

Kernel UI responsibilities:

- Agent session views: session header, task status, plan, step timeline,
  tool-call timeline, streaming output, pause/resume/cancel controls.
- Code-agent views: workspace explorer, changed-file list, diff viewer, patch
  review, apply/reject controls, build/test result summaries, and review
  findings.
- Terminal views: command list, streaming stdout/stderr, exit status, retry,
  cancellation, and permission prompts.
- Permission views: capability prompts, sandbox decisions, destructive action
  confirmations, policy explanations, and audit context.
- Provider views: model/tool/provider capability display, health state,
  configuration status, and diagnostics.
- Telemetry views: trace ids, request ids, event stream, logs, metrics summaries,
  and troubleshooting panels.
- Integration components: embeddable panels and hooks that product applications
  can compose without reimplementing kernel behavior.

Kernel UI must not:

- Directly mutate workspace files outside the kernel client/service boundary.
- Run terminal commands directly through browser APIs or ad hoc host calls.
- Apply patches without going through code-kernel patch SPI.
- Parse or enforce security policy in place of the Rust kernel.
- Construct raw HTTP requests, manual auth headers, or local DTO forks for
  kernel operations.
- Depend on BirdCoder product packages for standard kernel behavior.

## Layered Architecture

```text
SDKWork applications and hosts
  -> product shell and product modules
  -> sdkwork-kernel-ui packages (TypeScript + Vite + React)
  -> kernel UI service adapters and typed clients
  -> IPC / RPC / SDK port / event stream
  -> sdkwork-code-kernel (Rust)
  -> sdkwork-agent-kernel (Rust)
  -> provider SPI: model, tool, memory, storage, workspace, VCS, terminal,
     sandbox, policy, telemetry, host
```

Dependency rules:

- Rust kernel crates must not depend on React, Vite, product UI packages, or
  browser-only APIs.
- Kernel UI packages must call Rust kernel behavior through typed clients,
  service adapters, IPC/RPC, SDK ports, or event streams.
- Product applications may compose kernel UI packages, but kernel UI packages
  must not depend on product applications.
- Provider implementations depend on kernel SPI; kernel SPI must not depend on
  provider implementations.
- `sdkwork-code-kernel` may depend on `sdkwork-agent-kernel`; the reverse
  dependency is forbidden.
- Shared contracts must remain smaller and more stable than feature packages.

## SPI And Compatibility Rules

All kernel-facing contracts must be explicit and versioned.

Required SPI contract elements:

- Stable Rust traits or equivalent typed interfaces for every provider family.
- Capability manifest with provider id, version, supported operations, security
  requirements, feature flags, and compatibility range.
- Typed request/response models with stable error types.
- Streaming event model for long-running operations.
- Cancellation and timeout behavior.
- Permission and policy decision points.
- Audit and telemetry metadata.
- Deterministic test doubles for provider and UI service tests.

Compatibility rules:

- Breaking SPI changes require a documented migration path.
- New capabilities should be additive and feature-gated.
- Providers must fail closed when a required capability is missing.
- UI packages must degrade gracefully when a kernel capability is unavailable.
- Generated clients, event schemas, and manifests must not be hand-edited.
- Kernel-local specs under `kernel/specs/` must record contract decisions that
  affect external integrations.

## Runtime And Event Model

Agent and code-agent work is long-running, observable, cancellable, and
resumable. The kernel must model this explicitly.

Standard lifecycle:

```text
created
  -> configured
  -> planned
  -> awaiting_permission
  -> running
  -> paused
  -> completed
  -> failed
  -> cancelled
```

Standard event categories:

- `agent.install.*`
- `agent.configure.*`
- `agent.session.*`
- `agent.plan.*`
- `agent.step.*`
- `agent.tool.*`
- `agent.model.*`
- `agent.memory.*`
- `code.workspace.*`
- `code.vcs.*`
- `code.patch.*`
- `code.terminal.*`
- `code.build.*`
- `code.review.*`
- `kernel.policy.*`
- `kernel.telemetry.*`

Every event should include a stable event id, session id, task id when
available, timestamp, source subsystem, severity when relevant, correlation
metadata, and redacted payload rules.

Current Rust baseline also defines ordered event streams: subscriptions filter
by session/task/run/source/severity/family, cursors resume after a sequence,
replay batches mark replayed events, and stream completion/error state is typed
for UI, protocol, and audit consumers.

Current agent-kernel Rust baseline also defines installation and configuration
SPI: every installable agent can expose typed install, uninstall, upgrade, and
configuration contracts; configuration specs can define base settings, login
authentication, LLM API key secret references, runtime/security sections, and
custom sections; raw secret values are rejected when a field requires host
secret references. Configuration profiles now carry profile id, agent id,
configuration version, lifecycle status, typed values, and profile secret
bindings, so login credentials and LLM API keys can be preserved or rebound
through policy-checkable migration plans during agent upgrades. A
provider-neutral configuration store SPI persists, migrates, lists, loads, and
archives these profiles while emitting `agent.configure.profile.*` lifecycle
events. Agent package manifests, configuration specs, configuration profiles,
and configuration migration plans have machine-readable JSON schema contracts
and Rust parsers so registries, installers, UI configuration screens, and
conformance tests can share the same artifacts without product-local DTOs.
Agent package manifests bind package source, version, lifecycle support,
installer/configuration provider ids, kernel compatibility, default profile,
and required configuration sections. Runtime bootstrap
can consume package manifests directly, registers their provider bindings as
standard `agent_installer` and `agent_configuration` provider families, emits
provider registration events, and exposes `agent.install`, `agent.uninstall`,
`agent.upgrade`, and `agent.configure` as policy-aware capability manifest
entries. Package bootstrap fails closed when agent ids do not match, kernel
version compatibility is not satisfied, or a typed configuration provider omits
required package configuration sections. Local runtime execution uses a typed
provider registry so hosts can invoke concrete model, tool, policy, context,
memory, planning, host, protocol adapter, MCP, Agent Skill, telemetry,
installer, and configuration SPI instances without replacing the capability
manifest as the source of truth. Multiple model providers may be registered in
one runtime and selected by provider id, allowing each agent to support
different LLM implementations. Capability negotiation now preserves
`min_version` from agent manifests and only binds providers whose version
satisfies the requested capability requirement. Manifest-only providers remain valid for
negotiation and introspection, but direct local SPI execution fails closed with
`provider_unavailable` until the typed provider is registered. `AgentKernelHost`
loads multiple bootstrapped runtime implementations as independent runtime
slots, rejects duplicate runtime ids, unloads slots deterministically, and
aggregates diagnostics and conformance reports so multiple different agents can
run side by side under one host/supervisor boundary.
The generic agent-kernel baseline now also defines
`KernelConformanceReport`, `KernelConformanceCase`, and
`KernelConformanceCaseStatus`, plus a machine-readable
`kernel-conformance-report` JSON schema. This gives runtime, provider,
adapter, installation, and security suites a shared evidence format before
domain-specific kernels add their own profile cases.
The runtime baseline also exposes `AgentRuntimeDiagnostics` with typed vs
manifest-only provider counts, provider health snapshots, degraded/missing
capability lists, and missing standard provider families. All core provider SPI
families now have a health contract, with default `available` health for
families that do not need a custom probe. `AGENT_RUNTIME_DIAGNOSTICS_SCHEMA`
exposes that report shape as a machine-readable contract. Runtime instances can
also generate generic `KernelConformanceReport` values for `runtime-manifest`
and `runtime-local` profiles directly from their capability manifest and
diagnostics, without executing provider side effects. Missing full-profile
provider families remain visible for conformance coverage, but do not by
themselves degrade a partial runtime; missing optional capabilities are reported
as degradation evidence rather than profile failure unless a profile explicitly
requires them.

Current code-kernel Rust baseline defines provider SPI for workspace, VCS,
patch, terminal execution, verification, language intelligence, review, and
artifacts, plus code-session/task state, stable code events, repository
knowledge, and safety assessment SPI. It also defines a typed code runtime
registry: hosts can register manifest-only code providers for negotiation or
typed local providers for direct SPI execution, and `CodeKernelRuntime` exposes
workspace, VCS, patch, terminal, verification, language, review, artifact,
knowledge, and safety provider accessors. Manifest-only providers remain valid
for introspection but fail closed with `provider_unavailable` when local code
attempts direct execution. These provider families are intentionally
mechanism-only: concrete Git, shell, parser, LSP, storage, sandbox, and
product-policy integrations belong in provider crates or host adapters. The
runtime now also exposes `CodeKernelRuntimeDiagnostics`, a standard
introspection report with provider counts, capability counts, typed vs
manifest-only registration, provider health snapshots, degraded state, and
missing standard provider families. That report is conformance evidence for
hosts, adapters, UI clients, and third-party provider certification. The code
runtime can also generate `CodeConformanceReport` for manifest-level and direct
local-runtime profiles, with deterministic pass/fail cases and no repository,
terminal, network, or model side effects. Protected code operations now build
standard Agent Kernel `PolicyRequest` values for workspace writes, patch
application and rollback, VCS restore, terminal execution, verification runs,
and artifact writes, so hosts and UI clients can evaluate permission,
side-effect, resource, and redaction metadata consistently. Code sessions,
tasks, patch sets, artifacts, and code events also map to the shared Agent
Kernel `ProtocolObjectEnvelope`, using generic extension objects plus
namespaced `sdkwork.code.*` metadata for protocol, UI, IPC, and audit
integration without making the Agent Kernel depend on Code Kernel objects.
Code capability manifests, runtime diagnostics, and conformance reports are
also exposed as machine-readable schema constants for registry, CI, and
cross-application integration.

Current kernel UI baseline defines a 10-package TypeScript + Vite + React
workspace and enforces layered package conformance: feature packages separate
`components`, `service`, `hooks`, and `types`; shared primitives live in
`commons`; runtime composition lives in `core`; kernel client adapters and mock
data live in `services`; cross-package deep imports are rejected.

## Security And Policy

The kernel must assume agent execution can be risky. Security and policy hooks
are part of the core design, not optional UI behavior.

Required policy areas:

- Workspace root allowlist and path traversal protection.
- Destructive filesystem operation checks.
- Terminal command policy, timeout, cancellation, and output redaction.
- Network access policy when tools or providers can call external services.
- Secret detection and redaction in logs, prompts, tool outputs, and telemetry.
- Patch review gates for generated edits.
- Provider capability restrictions.
- Tenant/user/session context propagation where the host application is
  multi-tenant.
- Audit events for permission grants, denied operations, patch application,
  terminal execution, provider changes, and policy overrides.

UI permission prompts are presentation surfaces. Final authorization and
auditable policy decisions belong to the kernel and host policy providers.

## Observability

Kernel implementations must make agent behavior inspectable without leaking
secrets or overwhelming users.

Required observability surfaces:

- Structured logs with request/session/task correlation.
- Traces for model calls, tool calls, terminal commands, patch application,
  provider calls, and long-running kernel operations.
- Metrics for latency, token usage when available, tool failure rate, command
  failure rate, patch success/failure, retries, cancellations, and policy
  denials.
- Health and capability manifests for host applications.
- UI-readable event streams for progress, diagnostics, and troubleshooting.

## BirdCoder Role

BirdCoder is a proving application for this kernel standard.

BirdCoder may:

- Compose `sdkwork-agent-kernel`, `sdkwork-code-kernel`, and
  `sdkwork-kernel-ui`.
- Provide product-specific defaults, routes, shell layout, branding, commands,
  and workflow presets.
- Add BirdCoder-specific providers or product modules through published SPI.
- Feed implementation lessons back into kernel specs.

BirdCoder must not:

- Add BirdCoder-only assumptions to kernel SPI.
- Make kernel UI depend on BirdCoder product modules for standard behavior.
- Bypass code-kernel patch, terminal, workspace, policy, or audit contracts.
- Treat a product route or component as the source of truth for kernel
  behavior.

## Documentation Requirements

Every kernel subsystem must eventually have its own README with:

- Capability and domain.
- Package/crate type and language.
- Public SPI/API exports.
- Required provider or client surface.
- Configuration and initialization model.
- SaaS/private/local behavior when relevant.
- Security and policy assumptions.
- Extension points.
- Verification commands.
- Owner and status.

The root README is the kernel standard overview. Subsystem READMEs should add
implementation-specific details without redefining the root boundaries.

## Verification Expectations

When implementation files exist, the following commands should be available and
documented by each subsystem:

```bash
# Repository-level checks
pnpm lint

# Kernel standard conformance
node kernel/scripts/check-kernel-standards.mjs
node kernel/sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs

# Rust kernel checks, paths may be refined by concrete crate layout
cargo test --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml
cargo test --manifest-path kernel/sdkwork-code-kernel/Cargo.toml

# Kernel UI checks
pnpm --dir kernel/sdkwork-kernel-ui install --frozen-lockfile
pnpm --dir kernel/sdkwork-kernel-ui build
pnpm --dir kernel/sdkwork-kernel-ui typecheck
pnpm --dir kernel/sdkwork-kernel-ui test
```

If a command is not yet available, the owning subsystem README must state what
is missing and which contract is being implemented first.

## Acceptance Checklist

- [ ] `kernel/` is documented as an industry-level SDKWork kernel standard, not
      a BirdCoder-private directory.
- [ ] BirdCoder is described as a proving application, not the owner of the SPI.
- [ ] `sdkwork-agent-kernel` is the Rust base SPI for all agents.
- [ ] `sdkwork-code-kernel` is the Rust code-agent SPI built on top of the
      agent kernel.
- [ ] `sdkwork-kernel-ui` is a first-class TypeScript + Vite + React kernel UI
      standard subsystem.
- [ ] Kernel UI package structure follows `apps/docs/ARCHITECT.md`.
- [ ] Rust kernel does not depend on React/Vite/product UI.
- [ ] Kernel UI talks to Rust kernel through typed service adapters, IPC/RPC,
      SDK ports, or event streams.
- [ ] Provider and plugin variation is expressed through typed SPI and manifests.
- [ ] Security, policy, telemetry, lifecycle, and compatibility are defined as
      kernel-level concerns.
- [ ] Product applications can integrate or replace providers without mutating
      kernel SPI.
