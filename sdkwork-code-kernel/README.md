# SDKWork Code Kernel

Domain: `intelligence`
Capability: `code-kernel`
Package type: Rust kernel SPI crate
Status: standard candidate

`sdkwork-code-kernel` is the industry-facing code-agent kernel standard. It is
not a BirdCoder-private layer. BirdCoder is one validation application that
uses this kernel to refine provider boundaries, safety hooks, UI contracts, and
conformance expectations.

The code kernel extends `sdkwork-agent-kernel`. The agent kernel owns generic
agent runtime concepts such as manifests, sessions, tasks, provider
registration, policy, events, host operations, memory, and protocol adapters.
The code kernel adds software-engineering-specific mechanisms: workspaces,
version control, patches, terminal execution, build/test verification,
language intelligence, code review, artifact handling, code-task state,
code-session state, stable code events, knowledge/documentation access, and
repository safety.

## Design Goals

- Keep code-agent mechanisms separate from product policy.
- Keep Codex/Claude Code/OpenCode-style workflows expressible without copying
  any single product's internal model.
- Keep workspace, VCS, patch, terminal, and verification behavior behind typed
  provider SPI.
- Require policy gates for destructive edits, terminal execution, network
  operations, generated-client changes, and secret exposure.
- Emit stable code events that UI, protocol adapters, audit sinks, and
  conformance runners can consume.
- Make deterministic fake providers part of the standard so third-party
  implementations can be tested without real repositories or shells.
- Expose a typed local runtime registry that keeps capability manifests as the
  negotiation source of truth while allowing local hosts to invoke concrete code
  SPI providers.

## Relationship To `sdkwork-agent-kernel`

`sdkwork-code-kernel` depends on `sdkwork-agent-kernel` and may reuse:

- `AgentManifest`, `AgentRuntime`, `AgentTask`, and lifecycle objects.
- `PolicyProvider`, `PolicyDecision`, and side-effect classifications.
- `KernelEvent`, trace context, and event recorder/exporter contracts.
- `HostProvider` for filesystem, process, network, secrets, time, storage, and
  environment access.
- `ProtocolAdapter` for MCP, A2A, UI client, IPC, WebSocket, HTTP/RPC, and host
  integration.

It must not change agent-kernel objects to add repository-only fields. Code
metadata belongs in code-kernel objects, extension payloads, provider manifests,
or code-specific event payloads.

## Core Object Model

The code kernel standard is centered on these objects:

- `Workspace`: one logical coding workspace with id, root, trust level,
  generated-file policy, ignore policy, and language/profile hints.
- `WorkspaceFile`: file metadata and optional content projected through the
  workspace provider, never by direct kernel filesystem access.
- `VcsSnapshot`: branch, head revision, clean/dirty state, changed files, and
  repository status summary.
- `PatchSet`: structured changes grouped for validation, review, application,
  rollback, and audit.
- `PatchOperation`: create/update/delete edit intent with file path and
  before/after content where applicable.
- `TerminalCommand`: command, args, working directory, timeout, environment
  policy, redaction policy, and policy categories.
- `TerminalOutputChunk`: ordered stdout/stderr/system stream chunks with
  redaction classification for UI, audit, and protocol adapters.
- `CommandResult`: normalized exit status, stdout/stderr summaries, timeout and
  cancellation flags.
- `VerificationPlan`: build/test/lint/typecheck commands and expected evidence.
- `VerificationReport`: command results, parsed failures, evidence summary, and
  residual risk.
- `LanguageDiagnostic`: normalized diagnostics from compiler, LSP,
  tree-sitter-backed analyzers, or static analysis providers.
- `LanguageSymbol`: normalized symbols for navigation, review context,
  formatting decisions, and future semantic search providers.
- `ReviewFinding`: severity, file/line reference, issue category, explanation,
  remediation hint, and test gap.
- `ReviewReport`: structured review output for patches and verification
  evidence, including risk summary, findings, missing tests, and artifact links.
- `CodeArtifact`: persisted code-agent output such as patches, diffs,
  verification reports, terminal logs, review reports, diagnostics, and
  workspace snapshots.
- `CodeTask`: code-agent task binding intent, workspace, plan, checkpoints,
  review state, and trace references.
- `CodeSession`: workspace-bound session state with tasks and provider
  bindings.
- `CodeKernelEvent`: stable `code.*` event helper that maps to the shared
  `KernelEvent` envelope.
- `KnowledgeDocument`: repository docs, specs, ADRs, generated contracts, and
  external references projected through a provider.
- `CodeSafetyAssessment`: typed risk assessment for workspaces, patches, and
  terminal commands before policy decisions or user approvals.

## Provider SPI Families

Required provider families:

- `WorkspaceProvider`: list/read/write/stat/watch workspace resources with path
  policy, generated-file protection, and health reporting.
- `VcsProvider`: expose status, branch, diff, blame, commit metadata, and
  restore operations through typed contracts with health reporting.
- `PatchProvider`: validate, preview, apply, reject, rollback, and explain
  structured patches with health reporting.
- `TerminalProvider`: run commands with streaming output, timeout,
  cancellation, working directory, env policy, redaction, and policy gates.
- `VerificationProvider`: discover and run build/test/lint/typecheck
  verification plans, return `VerificationReport` evidence, and expose health.
- `LanguageProvider`: return diagnostics, symbols, and formatting results from
  compiler, LSP, parser, or static-analysis integrations through normalized
  request/response objects.
- `ReviewProvider`: review patches and verification reports, producing
  `ReviewReport` objects with findings, risk summaries, missing-test context,
  and artifact references.
- `ArtifactProvider`: store, retrieve, and list generated reports, patches,
  diffs, logs, diagnostics, and review artifacts with retention and redaction
  rules.
- `KnowledgeProvider`: search, read, and list repository documentation, specs,
  ADRs, generated SDK/API contracts, and external references.
- `CodeSafetyProvider`: assess workspace scope, patches, and terminal commands
  for risk level, side-effect level, policy categories, and approval needs.

## Runtime Registry

`CodeKernelRuntimeBuilder` is the code-agent driver registry. It mirrors the
agent-kernel runtime model while staying focused on code-specific SPI.

Registration paths:

- `register_*_provider_manifest(provider_id, version)` registers a provider for
  capability negotiation and introspection only.
- `register_*_provider(provider_id, version, provider)` registers the same
  manifest data plus a typed local SPI instance.
- `register_provider(ProviderManifest)` allows extension providers to
  participate in the manifest without adding product-local DTOs.

Runtime accessors:

- `workspace_provider`
- `vcs_provider`
- `patch_provider`
- `terminal_provider`
- `verification_provider`
- `language_provider`
- `review_provider`
- `artifact_provider`
- `knowledge_provider`
- `safety_provider`

Accessor behavior is fail-closed:

- A typed local provider returns the provider instance.
- A manifest-only provider returns `provider_unavailable` with the provider id.
- A missing provider family returns `capability_missing` with the relevant
  `code.*` capability id.

The runtime produces `CodeKernelCapabilityManifest`, preserving `code_*`
provider families, `code.*` capability ids, operation metadata,
side-effect levels, and policy categories. The standard builder only advertises
capabilities represented by the current Rust traits; future capabilities must
be added with matching trait contracts and conformance tests.

Runtime diagnostics:

- `diagnostics()` returns `CodeKernelRuntimeDiagnostics`.
- The report includes runtime id, provider count, capability count, typed
  provider count, manifest-only provider count, per-provider diagnostic records,
  and missing standard provider families.
- Each `CodeProviderDiagnostic` records provider id, provider family, version,
  typed-registration state, declared capabilities, and optional
  `ProviderHealth`.
- Health is only reported for typed providers whose provider id and family match
  the manifest. Manifest-only providers keep `health = None`.
- `is_degraded()` is true when a standard provider family is missing, a
  provider is manifest-only, or a typed provider reports non-available health.

This diagnostic surface is the code-kernel compatibility report that hosts,
protocol adapters, UI clients, and third-party conformance runners can consume
without invoking workspace, VCS, terminal, model, or network side effects.

Conformance reports:

- `conformance_report(profile)` returns `CodeConformanceReport`.
- `code-capability-manifest.schema.json`,
  `code-runtime-diagnostics.schema.json`, and
  `code-conformance-report.schema.json` define machine-readable standard
  artifacts for registries, hosts, and conformance tools.
- `CODE_CAPABILITY_MANIFEST_SCHEMA`, `CODE_RUNTIME_DIAGNOSTICS_SCHEMA`, and
  `CODE_CONFORMANCE_REPORT_SCHEMA` expose those artifacts from the Rust crate.
- `CodeConformanceProfile::Manifest` validates manifest-level compatibility:
  all standard provider families are declared and standard capabilities use the
  `code.*` namespace. Providers may be manifest-only.
- `CodeConformanceProfile::LocalRuntime` validates direct local execution:
  manifest checks pass, declared providers have typed Rust SPI instances, and
  typed providers report available health.
- Reports contain stable case ids, pass/fail results, and diagnostic messages
  for provider authors and host integrators.
- Manifest conformance checks both standard provider family presence and full
  standard capability coverage for each declared family.
- Report generation is side-effect free; it must not run repository, VCS,
  terminal, verification, model, network, or safety operations.

Policy request helpers:

- `WorkspaceWriteRequest::to_policy_request`
- `PatchSet::apply_policy_request`
- `PatchSet::rollback_policy_request`
- `VcsRestoreRequest::to_policy_request`
- `TerminalCommand::to_policy_request`
- `VerificationPlan::to_policy_request`
- `CodeArtifact::write_policy_request`

These helpers map protected code operations to Agent Kernel `PolicyRequest`
values with stable `code.*` categories, typed
`PolicyCategory::ProductSpecific`, workspace-scoped resources, action names,
side-effect levels, and operation context. They only build metadata; provider
execution remains behind the relevant SPI trait.

Protocol object mapping:

- `StandardCodeProtocolObjectMapper` maps code sessions, tasks, patch sets,
  code artifacts, and code events to Agent Kernel `ProtocolObjectEnvelope`.
- Code sessions, tasks, patches, and artifacts use
  `ProtocolObjectKind::ExtensionObject` with `sdkwork.code.object_kind`
  metadata so the Agent Kernel remains generic.
- Code events map to `ProtocolObjectKind::KernelEvent` with
  `sdkwork.code.event.v1`.
- Mapped payloads contain stable identifiers and counts, not raw prompts, raw
  artifact bodies, secrets, or unredacted terminal output.

## Safety Model

Code-agent execution is high-risk because it can change source, run commands,
read secrets, and modify generated clients. A compliant implementation must:

- Deny path traversal and out-of-workspace writes.
- Require policy for writes, deletes, patch application, terminal execution,
  network access, generated-client changes, and VCS restore operations.
- Preserve user changes and distinguish user-owned changes from agent-created
  changes when possible.
- Require explicit rollback metadata for applied patch sets.
- Keep raw secrets out of model prompts, logs, events, command summaries, and
  review artifacts unless policy explicitly allows exposure.
- Treat verification evidence as auditable data, not informal narration.
- Keep destructive commands cancellable and observable.

## Runtime Flow

Standard code-agent flow:

```text
load agent runtime
  -> register code providers
  -> negotiate workspace/code capabilities
  -> open workspace
  -> create code session and code task
  -> inspect repository state
  -> plan code actions
  -> request policy for side effects
  -> apply patch or run terminal command
  -> run verification plan
  -> emit review/summary artifacts
  -> persist resumable state
```

Rules:

- Repository state inspection must go through `WorkspaceProvider` and
  `VcsProvider`.
- Edits must be represented as `PatchSet` or typed workspace writes.
- Verification must return `VerificationReport` with command evidence.
- Review findings must use stable severity and file/line references.
- Code events must use `code.*` families and preserve agent trace context.

## Rust Crate Shape

The crate is intentionally SPI-first:

```text
sdkwork-code-kernel/
|-- Cargo.toml
|-- src/
|   |-- lib.rs
|   |-- conformance.rs
|   |-- runtime.rs
|   |-- session.rs
|   |-- task.rs
|   |-- code_event.rs
|   |-- workspace.rs
|   |-- vcs.rs
|   |-- patch.rs
|   |-- protocol.rs
|   |-- terminal.rs
|   |-- language.rs
|   |-- verification.rs
|   |-- review.rs
|   |-- knowledge.rs
|   |-- safety.rs
|   `-- artifact.rs
`-- tests/
    |-- code_conformance_contracts.rs
    |-- code_kernel_contracts.rs
    |-- code_policy_request_contracts.rs
    |-- code_protocol_mapping_contracts.rs
    |-- code_provider_spi_contracts.rs
    |-- code_runtime_diagnostics_contracts.rs
    |-- code_runtime_registry_contracts.rs
    |-- code_session_event_contracts.rs
    `-- code_task_knowledge_safety_contracts.rs
```

The crate depends on `sdkwork-agent-kernel` by path. It should not introduce
concrete Git, shell, parser, LSP, or UI dependencies in the SPI layer. Concrete
providers belong in provider crates or host integrations.

Current Rust SPI exports include:

- Workspace: `WorkspaceProvider`, `Workspace`, `WorkspaceFile`,
  `WorkspaceFileEntry`, `WorkspaceFileKind`, `WorkspaceFileStat`,
  `WorkspaceWriteRequest`, `WorkspaceWriteResult`, `WorkspaceWatchEvent`,
  `WorkspaceWatchEventKind`, `GeneratedFilePolicy`.
- VCS: `VcsProvider`, `VcsSnapshot`, `VcsDiffRequest`, `VcsDiff`,
  `VcsDiffFile`, `VcsFileChangeKind`, `VcsBlameLine`, `VcsCommitMetadata`,
  `VcsRestoreRequest`, `VcsRestoreReport`.
- Patch: `PatchProvider`, `PatchSet`, `PatchOperation`, `PatchPreview`,
  `PatchApplyResult`, `PatchRejection`, `PatchRollbackResult`,
  `PatchExplanation`; `PatchOperation` has create/update/delete constructors
  and patch application can build policy requests.
- Terminal: `TerminalProvider`, `TerminalCommand`, `TerminalOutputChunk`,
  `TerminalOutputChannel`.
- Verification: `VerificationProvider`, `VerificationPlan`,
  `VerificationReport`, `CommandResult`.
- Language intelligence: `LanguageProvider`, diagnostics requests/results,
  symbol requests/results, and formatting requests/results.
- Review: `ReviewProvider`, `ReviewReport`, `ReviewFinding`,
  `ReviewSeverity`.
- Artifacts: `ArtifactProvider`, `CodeArtifact`, `CodeArtifactKind`,
  `ArtifactReceipt`, `ArtifactDescriptor`, `ArtifactFilter`; artifact writes
  can build redaction-aware policy requests.
- Runtime registry: `CodeKernelRuntimeBuilder`, `CodeKernelRuntime`,
  `CodeKernelCapabilityManifest`, `CodeKernelRuntimeDiagnostics`,
  `CodeProviderDiagnostic`.
- Conformance: `CodeConformanceProfile`, `CodeConformanceReport`,
  `CodeConformanceCase`.
- Code task state: `CodeTask`, `CodeTaskIntent`, `CodeTaskState`, `CodePlan`,
  `CodePlanStep`, `CodeCheckpoint`, `CodeReviewStatus`, `CodeTraceRef`.
- Code session and events: `CodeSession`, `CodeSessionState`,
  `CodeProviderBinding`, `CodeKernelEvent`, `CodeEventKind`.
- Knowledge: `KnowledgeProvider`, `KnowledgeDocument`,
  `KnowledgeDocumentKind`, `KnowledgeDocumentFilter`, `KnowledgeQuery`,
  `KnowledgeSearchResult`.
- Safety: `CodeSafetyProvider`, `CodeSafetyScope`, `CodeSafetyAssessment`,
  `CodeSafetyRiskLevel`.
- Protocol mapping: `CodeProtocolObjectMapper`,
  `StandardCodeProtocolObjectMapper`.

## Conformance Expectations

A compliant code-kernel implementation must prove:

- Workspace path policy denies traversal and out-of-root access.
- Workspace writes and generated-file edits are policy-controlled.
- Every standard provider reports typed `ProviderHealth`.
- VCS status, diff, blame, commit metadata, and restore are normalized and
  deterministic in fake providers.
- Patch validation catches missing files, conflicts, and destructive edits.
- Patch preview, application, rejection, rollback, and explanation are typed and
  emit auditable metadata; patch application emits rollback metadata.
- Terminal execution declares working directory, timeout, env policy, and
  policy categories.
- Verification reports preserve command evidence and parsed failures.
- Language providers normalize diagnostics, symbols, and formatting output
  without exposing concrete compiler, LSP, parser, or indexer dependencies.
- Review findings include severity, location, risk, and missing-test context.
- Artifact providers preserve artifact type, workspace binding, redaction
  classification, and retention policy.
- Code tasks preserve intent, plan, checkpoint, review, and trace state.
- Code sessions preserve workspace, tasks, provider bindings, and session
  state transitions.
- Side-effectful operations produce standard Agent Kernel `PolicyRequest`
  values for policy providers, permission prompts, and audit.
- Code events map to shared `KernelEvent` with `CodeKernel` source, stable
  `code.*` event type, and `sdkwork.code.event.v1` payload schema.
- Code protocol mapping uses shared `ProtocolObjectEnvelope` with namespaced
  `sdkwork.code.*` metadata and no raw sensitive payload leakage.
- Knowledge providers normalize docs, specs, ADRs, generated contracts, and
  external references without leaking sensitive content.
- Safety providers return typed risk, side-effect, policy category, reason, and
  approval metadata for workspace, patch, and terminal operations.
- Code-kernel providers do not bypass agent-kernel host or policy SPI.
- Manifest-only code providers negotiate capabilities but return
  `provider_unavailable` for direct local SPI execution.
- Typed code providers can be invoked through `CodeKernelRuntime` accessors
  after bootstrap.
- Runtime diagnostics report provider counts, capability counts, typed vs
  manifest-only provider registration, provider health, degraded state, and
  missing standard provider families.
- Conformance reports separate manifest compatibility from direct
  local-runtime compatibility with deterministic case ids and side-effect-free
  pass/fail evidence, including incomplete standard capability coverage.

## Verification

Required crate verification commands:

```bash
cargo test --manifest-path kernel/sdkwork-code-kernel/Cargo.toml
cargo clippy --manifest-path kernel/sdkwork-code-kernel/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path kernel/sdkwork-code-kernel/Cargo.toml --check
```

The agent kernel should also be verified because this crate extends it:

```bash
cargo test --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml
```

## SDKWork Documentation Contract

Domain: intelligence
Capability: code-kernel
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

- `cargo test --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-code-kernel/Cargo.toml`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
