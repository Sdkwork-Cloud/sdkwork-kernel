# SDKWork Code Kernel Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: code-agent object model, provider SPI, runtime registry, capability
  negotiation, session/task state, code events, knowledge providers, safety
  assessment, artifact handling, and conformance
- Domain: `intelligence`
- Capability: `code-kernel`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_HOST_PROVIDER_SPI_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

The SDKWork Code Kernel is the provider-neutral SPI standard for software
engineering agents. It builds on `sdkwork-agent-kernel` and adds repository,
patch, terminal, verification, language, review, artifact, session, task,
event, knowledge, and safety mechanisms.

BirdCoder is one proving application for this standard. The Code Kernel must
remain reusable by IDEs, desktop hosts, CLIs, SaaS runtimes, private runtimes,
automation services, and future SDKWork applications.

## 1. Design Principles

The Code Kernel follows the same design discipline as the Agent Kernel:

- Mechanism belongs in the kernel SPI; product policy belongs in products and
  configured policy providers.
- Runtime capability manifests are the source of truth for what can be used.
- Provider families are driver-like extension points with typed Rust SPI.
- Concrete Git, shell, parser, LSP, storage, sandbox, model, and UI
  integrations belong outside the SPI crate.
- Side-effectful operations must be explicit, policy-checkable, auditable, and
  cancellable where the provider supports cancellation.
- Standard fake providers must be enough to test conformance without touching a
  real repository or spawning a real process.

## 2. Relationship To Agent Kernel

`sdkwork-code-kernel` depends on `sdkwork-agent-kernel` and reuses:

- `ProviderManifest` and `Capability` for capability negotiation.
- `KernelResult`, `KernelError`, and stable error kinds.
- `ProviderHealth` for health reporting.
- `SideEffectLevel` and policy categories for protected actions.
- Host environment policy, event redaction, trace, and telemetry primitives.

The Agent Kernel must not depend on the Code Kernel. Repository-specific data
must not be added to generic agent objects unless it is modeled as a standard
extension payload.

## 3. Core Objects

The Code Kernel standard defines these object families:

| Object | Responsibility |
| --- | --- |
| `Workspace` | Logical repository/workspace identity, root, trust level, generated-file policy, and language hints |
| `WorkspaceFile` | File projection returned through `WorkspaceProvider`, with optional content and readonly/generated flags |
| `VcsSnapshot` | Normalized branch, revision, dirty state, changed files, and summary |
| `PatchSet` | Structured edit group used for validation, review, application, rollback, and audit |
| `PatchOperation` | Create, update, or delete intent for a workspace path |
| `PatchApplyResult` | Patch application result with rollback metadata |
| `TerminalCommand` | Command, args, working directory, timeout, environment policy, and policy categories |
| `TerminalOutputChunk` | Ordered stdout/stderr/system stream chunk with redaction classification |
| `CommandResult` | Exit code, stdout/stderr summaries, cancellation, and timeout state |
| `VerificationPlan` | Build/test/lint/typecheck plan expressed as commands |
| `VerificationReport` | Command evidence, parsed failures, and success/failure summary |
| `LanguageDiagnostic` | Normalized compiler/LSP/static-analysis diagnostic |
| `LanguageSymbol` | Normalized symbol for navigation, review, and context gathering |
| `LanguageFormatResult` | Formatting result without direct workspace mutation |
| `ReviewFinding` | Severity, file/line, message, remediation, and missing-test signal |
| `ReviewReport` | Structured review of patches or verification evidence |
| `CodeArtifact` | Persisted code-agent output such as patches, diffs, logs, reports, diagnostics, or snapshots |
| `CodeSession` | Workspace-bound session state with tasks and provider bindings |
| `CodeTask` | Code-agent task state with intent, workspace, plan, checkpoints, review status, and trace refs |
| `CodeKernelEvent` | Stable code event helper that maps to the shared `KernelEvent` envelope |
| `KnowledgeDocument` | Repository docs, specs, ADRs, generated contracts, and external references |
| `CodeSafetyAssessment` | Workspace, patch, or terminal risk result with side-effect, policy, reason, and approval metadata |
| `CodeKernelRuntimeDiagnostics` | Runtime introspection report covering provider counts, capability counts, typed vs manifest-only registration, health, and missing standard provider families |
| `CodeConformanceReport` | Machine-readable compatibility report for manifest and local-runtime conformance profiles |

## 4. Provider SPI Families

Provider families use `code_*` provider-family ids in manifests. Each family
has manifest-only registration for negotiation and typed local registration for
direct Rust invocation.

| Provider family | Rust trait | Standard capabilities | Current typed operations |
| --- | --- | --- | --- |
| `code_workspace` | `WorkspaceProvider` | `code.workspace.list`, `code.workspace.read`, `code.workspace.write`, `code.workspace.stat`, `code.workspace.watch` | `list_files`, `read_file`, `write_file`, `stat_file`, `watch_events`, `health` |
| `code_vcs` | `VcsProvider` | `code.vcs.status`, `code.vcs.diff`, `code.vcs.blame`, `code.vcs.commit_metadata`, `code.vcs.restore` | `snapshot`, `diff`, `blame`, `commit_metadata`, `restore`, `health` |
| `code_patch` | `PatchProvider` | `code.patch.validate`, `code.patch.preview`, `code.patch.apply`, `code.patch.reject`, `code.patch.rollback`, `code.patch.explain` | `validate_patch`, `preview_patch`, `apply_patch`, `reject_patch`, `rollback_patch`, `explain_patch`, `health` |
| `code_terminal` | `TerminalProvider` | `code.terminal.run` | `run_command`, `stream_output`, `cancel_command`, `health` |
| `code_verification` | `VerificationProvider` | `code.verification.run` | `discover_plans`, `run_verification`, `health` |
| `code_language` | `LanguageProvider` | `code.language.diagnostics`, `code.language.symbols`, `code.language.format` | `diagnostics`, `symbols`, `format`, `health` |
| `code_review` | `ReviewProvider` | `code.review.produce` | `review_patch`, `review_verification`, `health` |
| `code_artifact` | `ArtifactProvider` | `code.artifact.read`, `code.artifact.write` | `put_artifact`, `get_artifact`, `list_artifacts`, `health` |
| `code_knowledge` | `KnowledgeProvider` | `code.knowledge.search`, `code.knowledge.read` | `search_documents`, `get_document`, `list_documents`, `health` |
| `code_safety` | `CodeSafetyProvider` | `code.safety.assess` | `assess_workspace`, `assess_patch`, `assess_terminal_command`, `health` |

Rules:

- Provider manifests must declare provider id, provider family, version, and
  at least one capability.
- Capability ids must remain stable and namespaced under `code.*`.
- Providers must return `KernelResult<T>` and map failures to stable kernel
  errors.
- Concrete providers must not bypass host, policy, redaction, or audit SPI when
  performing side effects.
- Every standard provider trait must expose `health()` returning
  `ProviderHealth` so hosts and conformance runners can diagnose runtime
  readiness consistently.
- Capabilities that are not represented by the current Rust trait must not be
  advertised as available by the standard builder.

## 5. Runtime Registry

The Code Kernel runtime registry is the code-agent equivalent of the Agent
Kernel driver model.

Required runtime builder behavior:

- `CodeKernelRuntimeBuilder::new(runtime_id)` starts deterministic bootstrap.
- `register_*_provider_manifest(provider_id, version)` registers a manifest-only
  provider for negotiation and introspection.
- `register_*_provider(provider_id, version, provider)` registers both the
  provider manifest and the typed local SPI instance.
- `register_provider(ProviderManifest)` allows manifest-level extension
  providers without inventing product-local DTOs.
- `bootstrap()` validates the runtime id and provider manifests, then returns a
  `CodeKernelRuntime`.

Required runtime behavior:

- `capability_manifest()` returns a `CodeKernelCapabilityManifest`.
- The manifest must use `manifest_type = "capability"`.
- The manifest must preserve provider family, provider id, version, capability
  ids, side-effect level, policy categories, and operation metadata.
- Typed accessors must exist for workspace, VCS, patch, terminal,
  verification, language, review, artifact, knowledge, and safety providers.
- If a typed provider exists, the accessor must return it.
- If a provider manifest exists but the typed provider instance is absent, the
  accessor must fail closed with `provider_unavailable` and the provider id.
- If neither manifest nor typed provider exists for the family, the accessor
  must fail with `capability_missing` and the relevant capability id.
- `diagnostics()` must return `CodeKernelRuntimeDiagnostics` derived from the
  current capability manifest and typed provider registry.
- `conformance_report(profile)` must return `CodeConformanceReport` derived
  from the current capability manifest and runtime diagnostics.

This split is intentional: manifests describe negotiated capabilities; typed
providers execute local SPI. A host may load manifests from a registry before
the concrete driver is available, but local execution cannot pretend the driver
exists.

### Runtime Diagnostics

Runtime diagnostics are the code-kernel equivalent of `/proc` and `/sys`
introspection. They must be derived from registered manifests and typed
providers, not from product-local state.

Required diagnostic fields:

- Runtime id.
- Provider count and capability count.
- Typed provider count.
- Manifest-only provider count.
- Per-provider diagnostic records containing provider id, provider family,
  version, typed-registration flag, optional `ProviderHealth`, and declared
  capabilities.
- Missing standard provider families for workspace, VCS, patch, terminal,
  verification, language, review, artifact, knowledge, and safety.
- A degraded signal when any standard family is missing, any provider is
  manifest-only, or any typed provider reports non-available health.

Rules:

- Typed provider health may only be reported when the typed Rust provider is
  registered for the same provider family and provider id as the manifest.
- Manifest-only providers must report no health snapshot; a host may still use
  their manifest for negotiation and loading decisions.
- Diagnostics must not perform workspace, VCS, terminal, model, or network
  side effects.
- Diagnostics are conformance evidence. Third-party providers should be able
  to attach this report to compatibility test output.

### Conformance Reports

The Code Kernel defines a machine-readable conformance report so compatibility
claims can be checked without relying on product-specific narration.

Machine-readable JSON Schema artifacts:

- `schemas/code-capability-manifest.schema.json`
- `schemas/code-runtime-diagnostics.schema.json`
- `schemas/code-conformance-report.schema.json`

Rust constants:

- `CODE_CAPABILITY_MANIFEST_SCHEMA`
- `CODE_RUNTIME_DIAGNOSTICS_SCHEMA`
- `CODE_CONFORMANCE_REPORT_SCHEMA`

Standard profiles:

- `Manifest`: validates manifest-level compatibility. This profile requires
  all standard provider families to be declared and standard code capabilities
  to use the `code.*` namespace. Providers may be manifest-only.
- `LocalRuntime`: validates direct local Rust execution. This profile includes
  manifest checks and also requires declared providers to have typed local SPI
  instances with available health.

Required report fields:

- Runtime id.
- Conformance profile.
- Overall pass/fail status.
- Ordered conformance cases with stable case ids, pass/fail result, and
  diagnostic message.

Required standard case ids:

- `code.conformance.standard_provider_families.complete`
- `code.conformance.standard_capabilities.complete`
- `code.conformance.standard_capabilities.namespaced`
- `code.conformance.local_providers.typed`
- `code.conformance.local_providers.health_available`

Rules:

- Manifest conformance must not call typed provider methods other than using
  existing manifest data and diagnostics.
- Standard code capabilities must match the `code.` namespace and use only
  lowercase ASCII letters, numbers, `.`, `_`, or `-`.
- Local-runtime conformance must not run workspace, VCS, patch, terminal,
  verification, language, review, artifact, knowledge, or safety operations.
- Conformance report generation must be side-effect free and deterministic.
- Failed reports must preserve enough detail for provider authors to identify
  missing standard families, incomplete standard capability coverage,
  manifest-only providers, and degraded typed providers.
- A declared standard provider family must declare every standard capability
  assigned to that family. Partial manifests may be useful during provider
  discovery, but they do not satisfy the standard conformance profile.

## 6. Capability Metadata

Capability metadata must be policy-aware:

- Read-only inspection capabilities use `read_only`.
- Workspace writes, patch application, terminal execution, verification command
  execution, and artifact writes use `side_effectful`.
- VCS restore and patch rollback use `destructive`.
- Terminal and verification capabilities represent potentially broad host
  effects; concrete command policy must narrow the effective permission.
- Policy categories should match the capability id for code-specific actions,
  for example `code.patch.apply` or `code.terminal.run`.
- Unknown extension capabilities may be carried in manifests with empty
  metadata, but products must not treat them as standard Code Kernel
  capabilities without a published spec update.

### Policy Request Helpers

Side-effectful code operations must be able to produce standard
`PolicyRequest` values from `sdkwork-agent-kernel`. This keeps policy
evaluation consistent across hosts, providers, CLIs, IDEs, and UI adapters.

Required helper mappings:

| Code operation | Helper | Category | Action | Side effect |
| --- | --- | --- | --- | --- |
| Workspace write | `WorkspaceWriteRequest::to_policy_request` | `code.workspace.write` | `workspace.write` | `side_effectful` |
| Patch apply | `PatchSet::apply_policy_request` | `code.patch.apply` | `patch.apply` | patch-derived `read_only`, `side_effectful`, or `destructive` |
| Patch rollback | `PatchSet::rollback_policy_request` | `code.patch.rollback` | `patch.rollback` | `destructive` |
| VCS restore | `VcsRestoreRequest::to_policy_request` | `code.vcs.restore` | `vcs.restore` | `destructive` |
| Terminal run | `TerminalCommand::to_policy_request` | `code.terminal.run` | `terminal.run` | `side_effectful` |
| Verification run | `VerificationPlan::to_policy_request` | `code.verification.run` | `verification.run` | `side_effectful` |
| Artifact write | `CodeArtifact::write_policy_request` | `code.artifact.write` | `artifact.write` | `side_effectful` |

Rules:

- Request resources must use stable workspace-scoped resource strings such as
  `workspace://{workspace_id}/...`.
- Code-specific categories must be represented as
  `PolicyCategory::ProductSpecific("code.*")` until the Agent Kernel promotes
  them to first-class typed policy categories.
- Requests must include operation context such as workspace id, path, patch id,
  affected files, command id, command, verification id, artifact id, and
  policy categories where available.
- Artifact policy requests must preserve artifact redaction classification.
- Patch application is destructive when any patch operation deletes a file;
  otherwise it is side-effectful when operations are present and read-only when
  no operation is present.
- Helpers must only build policy metadata; they must not execute providers,
  mutate workspaces, run commands, or read artifact content from storage.

## 7. Code Session And Task State

Code-agent work must have repository-aware session and task state in addition
to the generic agent lifecycle.

Required state objects:

- `CodeSession` binds a workspace, tasks, provider bindings, and session state.
- `CodeProviderBinding` records the provider family, provider id, and
  capabilities active in a session.
- `CodeTask` binds a `Workspace`, user intent, optional `CodePlan`,
  checkpoints, review status, and trace references.
- `CodeTaskIntent` preserves the prompt plus relevant context paths and
  constraints.
- `CodePlan` and `CodePlanStep` preserve ordered code operations and the
  capability each step requires.
- `CodeCheckpoint` records resumable points such as VCS revision and artifact
  ids.
- `CodeReviewStatus` records whether review is required, active, approved, or
  requesting changes.

Rules:

- Code-session transitions must be explicit and reject invalid transitions.
- Code-task transitions must be explicit and reject invalid transitions.
- Checkpoints must not contain raw secrets.
- A plan step that requires a side-effectful capability must declare that policy
  is required.

## 8. Safety Requirements

A compliant Code Kernel implementation must:

- Deny path traversal and out-of-workspace operations.
- Preserve user changes and distinguish user-owned edits from agent-created
  edits when the provider can observe that distinction.
- Require policy for writes, deletes, patch application, terminal execution,
  generated-file changes, VCS restore operations, network access, and secret
  access.
- Build protected action checks through standard `PolicyRequest` helpers for
  workspace writes, patch application/rollback, VCS restore, terminal
  execution, verification execution, and artifact writes.
- Keep terminal output, artifacts, diagnostics, and review reports redacted
  according to their classification.
- Require rollback metadata for applied patch sets.
- Treat verification as evidence, not informal narration.
- Keep direct filesystem, process, network, and secret operations behind host
  or provider SPI.

## 9. Knowledge Requirements

Repository knowledge is part of the code-agent kernel because modern coding
agents depend on docs, specs, ADRs, generated contracts, and external
references.

Rules:

- Knowledge providers must expose search, read, and list operations through
  typed SPI.
- Generated contracts must be represented as `GeneratedContract` documents
  rather than product-local DTOs.
- Sensitive documents must carry redaction classification.
- External references must be opt-in through query/filter flags.

## 10. Event And Artifact Expectations

Code-agent operations should emit or produce data in these families:

- `code.workspace.*`
- `code.vcs.*`
- `code.patch.*`
- `code.terminal.*`
- `code.verification.*`
- `code.language.*`
- `code.review.*`
- `code.artifact.*`
- `code.knowledge.*`
- `code.safety.*`

Long-running operations should be observable through event streams. Generated
outputs that need retention, review, replay, or audit should be modeled as
`CodeArtifact` instead of ad hoc strings.

Rules:

- `CodeKernelEvent` must map to `KernelEvent` with `CodeKernel` source.
- Code events must use stable `code.*` event types.
- Code event payloads must use the `sdkwork.code.event.v1` payload schema.
- Event payloads must preserve workspace id and may include session, task,
  artifact, trace, and redaction metadata.

## 11. Protocol Object Mapping

Code Kernel objects must map to the shared Agent Kernel
`ProtocolObjectEnvelope` so IPC, RPC, WebSocket, Tauri, Kernel UI clients,
MCP/A2A bridges, audit sinks, and SDK adapters can consume one canonical
envelope shape.

Required mapper:

- `StandardCodeProtocolObjectMapper`
- `CodeProtocolObjectMapper`

Required mappings:

| Code object | Protocol object kind | Payload schema | Required metadata |
| --- | --- | --- | --- |
| `CodeSession` | `ExtensionObject` | `sdkwork.code.session.v1` | `sdkwork.code.object_kind=code_session`, workspace id, session state, task count, provider binding count |
| `CodeTask` | `ExtensionObject` | `sdkwork.code.task.v1` | `sdkwork.code.object_kind=code_task`, workspace id, task state, review status, context path summary |
| `PatchSet` | `ExtensionObject` | `sdkwork.code.patch.v1` | `sdkwork.code.object_kind=patch_set`, workspace id, patch id, side-effect level, affected files |
| `CodeArtifact` | `ExtensionObject` | `sdkwork.code.artifact.v1` | `sdkwork.code.object_kind=code_artifact`, workspace id, artifact id, artifact kind |
| `CodeKernelEvent` | `KernelEvent` | `sdkwork.code.event.v1` | event type, event version, workspace id, optional session/task/artifact ids |

Rules:

- Code object metadata keys must be namespaced, usually under `sdkwork.code.*`.
- `ExtensionObject` keeps the Agent Kernel generic while allowing Code Kernel
  object families to remain first-class in protocol envelopes.
- Protocol payloads must summarize stable identifiers and counts; they must
  not leak raw prompts, raw artifact content, secrets, or unredacted terminal
  output.
- Redaction classification must be preserved for events and artifacts.
- Mapping must be side-effect free and must not call providers.

## 12. Conformance

Required conformance cases:

- The runtime registers every standard code provider family as a typed local
  provider and invokes the provider through runtime accessors.
- The runtime registers every standard code provider family as manifest-only
  and returns `provider_unavailable` for direct local execution.
- Missing provider families return `capability_missing`.
- Capability manifests contain `code_*` provider families and `code.*`
  capability ids.
- Declared standard provider families contain the full standard capability set
  for that family.
- Capability metadata includes operations, side-effect levels, and policy
  categories for standard capabilities.
- Every standard provider exposes health through typed SPI.
- Runtime diagnostics report provider counts, capability counts, typed vs
  manifest-only providers, provider health, and missing standard provider
  families.
- Conformance reports distinguish manifest compatibility from direct local
  runtime compatibility with stable case ids.
- Workspace path traversal is denied by compliant workspace providers.
- Patch application returns rollback metadata.
- Patch preview, rejection, rollback, and explanation are typed provider
  operations.
- Terminal execution declares working directory, timeout, environment policy,
  cancellation behavior, and policy categories.
- Verification reports preserve command evidence and parsed failures.
- Language providers normalize diagnostics, symbols, and formatting output.
- Review providers produce stable severity, location, remediation, and
  missing-test data.
- Artifact providers preserve workspace binding, artifact type, redaction, and
  retention metadata.
- Code sessions preserve workspace, task, provider binding, and state
  transition metadata.
- Code tasks preserve intent, plan, checkpoint, review status, and trace refs.
- Code events map to the shared kernel event envelope with stable `code.*`
  families.
- Code sessions, tasks, patches, artifacts, and events map to shared protocol
  envelopes without leaking raw sensitive payloads.
- Knowledge providers normalize search, read, and list operations for docs,
  specs, ADRs, generated contracts, and external references.
- Safety providers assess workspace, patch, and terminal command risk with
  typed side-effect levels, policy categories, reasons, and approval signals.
- Side-effectful code operations build standard `PolicyRequest` values with
  stable category, action, resource, side-effect, context, and redaction
  metadata.

## 13. Acceptance Checklist

- [ ] Code provider families are provider-neutral and namespaced.
- [ ] Capability manifests are the source of truth for available code
      capabilities.
- [ ] Manifest-only providers fail closed for local execution.
- [ ] Typed provider registry covers workspace, VCS, patch, terminal,
      verification, language, review, artifact, knowledge, and safety SPI.
- [ ] Runtime diagnostics expose typed/manifest-only registration, health, and
      missing standard provider families.
- [ ] Conformance reports produce deterministic pass/fail cases for manifest
      and local-runtime profiles.
- [ ] Code-session state is typed and provider bindings are explicit.
- [ ] Code-task state is typed and checkpointable.
- [ ] Code events use stable `code.*` families and shared kernel envelopes.
- [ ] Code protocol mappings use shared `ProtocolObjectEnvelope` objects with
      namespaced metadata and no raw sensitive payload leakage.
- [ ] Side-effectful operations carry policy metadata.
- [ ] Side-effectful operations can build standard Agent Kernel
      `PolicyRequest` values.
- [ ] Concrete host integrations are outside the SPI crate.
- [ ] Conformance tests can run with deterministic fake providers.
