# SDKWork External Agent Integration Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: external agent source references, mapping documents, manifest-only
  discovery, future typed provider adapters, process adapter behavior, policy,
  telemetry, and conformance
- Domain: `intelligence`
- Capability: `external-agent-integrations`

## 1. Positioning

External projects under `external/` are implementation references. They may
inspire or back SDKWork providers, but they do not replace SDKWork kernel
objects, events, policy decisions, or manifests.

Rules:

- `external/` repositories MUST be treated as source references.
- `sdkwork-agent-kernel` and `sdkwork-code-kernel` MUST NOT depend on external
  submodule paths.
- Third-party capabilities MUST enter SDKWork through manifests, typed provider
  SPI, protocol adapters, host providers, and conformance reports.
- Unknown upstream behavior MUST be modeled as unsupported or degraded rather
  than hidden behind dynamic maps.

## 2. Integration Modes

| Mode | Purpose | Execution |
| --- | --- | --- |
| `manifest-only` | Declare compatibility intent and capability mapping | No direct execution |
| `typed-local-provider` | Implement SDKWork Rust SPI directly | In-process through provider traits |
| `process-adapter` | Wrap an external CLI or process runtime | Through host/process provider and policy |
| `protocol-adapter` | Map external protocol objects to SDKWork objects | Through protocol adapter SPI |

Rules:

- Every upstream project SHOULD start as `manifest-only`.
- A typed local provider MUST implement the relevant SDKWork trait before it is
  registered as executable.
- A process adapter MUST run commands through SDKWork host/process policy.
- A protocol adapter MUST map into `ProtocolObjectEnvelope` or SDKWork kernel
  objects before routing.

## 3. Upstream Classification

| Upstream | Initial Classification | SDKWork Focus |
| --- | --- | --- |
| Hermes Agent | general agent runtime | Runtime, tool, memory/context, skill |
| OpenClaw | general agent application/runtime | Runtime, tool orchestration, memory/context |
| Codex | code-agent CLI/runtime | Workspace, patch, terminal, verification, review |
| Claude Code | code-agent CLI/runtime | Task lifecycle, permission flow, tool orchestration |
| OpenCode | code-agent runtime/CLI | Provider abstraction, workspace, terminal |
| Gemini CLI | code-agent CLI/runtime | Model/tool integration, command workflow |
| Rig | Rust-native agent framework | Model, tool, memory, planning, typed providers |

## 4. Mapping Requirements

Each upstream mapping document MUST record:

- Source path and upstream URL.
- Primary SDKWork surface.
- Initial registration mode.
- Candidate capability mapping.
- Policy boundaries.
- Event and error mapping expectations.
- Conformance profile expectations.
- Current implementation status.

## 5. Policy And Host Rules

External execution is side-effect-capable by default.

Rules:

- Process execution MUST use host/process provider boundaries.
- Filesystem reads, writes, deletes, VCS restore, patch apply, terminal
  execution, network access, secret reads, and protocol sends MUST build
  policy requests before execution.
- Policy evaluation failure MUST fail closed.
- Third-party output MUST be marked untrusted unless a narrower policy decision
  marks it trusted.
- Raw secrets MUST NOT appear in manifests, events, logs, diagnostics, or
  mapping documents.

## 6. Events And Diagnostics

External adapters MUST map observable activity into SDKWork events.

Required event families:

- `agent.provider.*`
- `agent.policy.*`
- `agent.tool.*`
- `agent.runtime.*`
- `code.terminal.*` when code-agent processes are involved
- `code.patch.*` when patches are produced or applied
- `code.verification.*` when verification commands are run

Diagnostics MUST distinguish:

- Missing upstream source.
- Manifest-only registration.
- Typed provider unavailable.
- Process runtime unavailable.
- Policy denied.
- Timeout.
- Cancellation.
- Provider error.

## 7. Conformance

External integrations MUST NOT claim local-runtime compatibility until typed
providers or process adapters can be exercised through SDKWork policy and host
contracts.

Required first-phase evidence:

- Submodule source exists.
- Mapping document exists.
- Experimental manifests parse.
- Manifest-only conformance profile is documented.
- Process adapter conformance profile is documented for CLI-backed agents.
- Local-runtime conformance profile is documented for future typed providers.

## 8. Non-Goals

This spec does not define UI behavior, product workflows, provider credentials,
model vendor defaults, or real execution of third-party agents.
