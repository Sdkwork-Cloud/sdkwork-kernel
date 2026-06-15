# SDKWork Agent Kernel UI Contract Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: typed UI client surface, event stream, capability-driven rendering,
  permission response, diagnostics, and `sdkwork-kernel-ui` integration
- Domain: `intelligence`
- Capability: `agent-kernel.ui-contract`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - [`ARCHITECT.md`](../../../docs/ARCHITECT.md)

This spec defines the contract consumed by `sdkwork-kernel-ui` and product UIs.
UI packages render and control kernel behavior through typed clients and service
adapters. They do not implement kernel policy or perform host side effects.

## 1. UI Boundary

Rules:

- UI clients `MUST` use typed kernel clients or service adapters.
- UI clients `MUST NOT` directly call host filesystem, process, network, or
  secret APIs for kernel behavior.
- UI permission responses `MUST` become kernel `PolicyDecision` records.
- UI rendering `MUST` be driven by `CapabilityManifest`.
- UI event timelines `MUST` preserve event ids and trace metadata.

## 2. Required Client Surface

Required operations:

- `getRuntimeManifest`
- `getCapabilityManifest`
- `getHealth`
- `createSession`
- `getSession`
- `listSessions`
- `closeSession`
- `createTask`
- `getTask`
- `cancelTask`
- `retryTask`
- `getRun`
- `pauseRun`
- `resumeRun`
- `cancelRun`
- `subscribeEvents`
- `getPermissionRequest`
- `respondToPermissionRequest`
- `getArtifact`
- `getDiagnostics`

Rules:

- Methods `MUST` return typed results or typed kernel errors.
- Long-running state `MUST` be observed through event streams.
- UI clients `MUST` preserve request cancellation/abort where supported.

## 3. Capability-Driven Rendering

Rules:

- Optional UI controls `MUST` check `CapabilityManifest`.
- Missing optional capability `MUST` degrade gracefully.
- Missing required capability `MUST` be presented as runtime unavailable or
  degraded state.
- UI `MUST` not expose actions not present in effective capabilities.

## 4. Permission UX Contract

Permission request fields:

- `permission_request_id`
- `policy_request_id`
- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `action_summary`
- `risk_summary`
- `requested_scope`
- `expires_at`
- `allowed_responses`

Rules:

- Permission UI `MUST` show action and scope.
- Permission response `MUST` include allow/deny and optional constraints.
- Permission response `MUST` be converted by runtime into `PolicyDecision`.
- UI `MUST` not treat local state as final authorization.

## 5. Event Timeline Contract

Rules:

- UI event consumers `MUST` preserve `event_id`.
- UI event consumers `MUST` group by session/task/run/step when available.
- UI event consumers `MUST` show terminal states distinctly.
- UI event consumers `MUST` respect redaction classification.
- Unknown event types `SHOULD` render as generic diagnostic events.

## 6. Diagnostics Contract

Diagnostics may include:

- Runtime health.
- Capability manifest.
- Provider health.
- Recent errors.
- Event stream status.
- Trace/correlation ids.
- Policy denials.

Rules:

- Diagnostics `MUST` avoid raw secrets.
- Diagnostics `SHOULD` provide enough context for support without exposing
  sensitive payloads.

## 7. TypeScript Package Boundary

`sdkwork-kernel-ui` packages must follow `../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`.

Rules:

- `service/` owns kernel client adapters, data mapping, error normalization,
  mock data, and mock behavior.
- `components/` render only.
- `pages/` compose route-level views only.
- `hooks/` bind service state to React.
- Public exports go through `src/index.ts`.
- Internal package dependencies use `workspace:*`.

## 8. Conformance

Required cases:

- UI service reads runtime manifest.
- UI service reads capability manifest.
- UI service creates and cancels task through typed client.
- UI service subscribes to event stream and preserves event ids.
- UI service responds to permission request.
- UI service hides unavailable optional actions.
- UI service does not import host provider internals.
- UI service maps kernel errors to safe UI errors.

## 9. Acceptance Checklist

- [ ] UI is a typed client of the kernel.
- [ ] Capability manifest drives optional controls.
- [ ] Permission responses become policy decisions.
- [ ] Event ids and trace metadata are preserved.
- [ ] Diagnostics are redacted.
- [ ] TypeScript package layering follows `../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`.
- [ ] Conformance tests cover manifest, task, event, permission, capability, and
      error behavior.
