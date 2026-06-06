# SDKWork Kernel UI

Domain: `intelligence`
Capability: `kernel-ui`
Package type: TypeScript + Vite + React workspace
Status: standard candidate

`sdkwork-kernel-ui` is the reusable UI standard for SDKWork agent and
code-kernel runtimes. It is not a BirdCoder application UI. Product
applications may embed these packages and add product-specific navigation,
branding, persistence, and deployment concerns around them.

The workspace follows the SDKWork frontend architecture standard in
`apps/docs/ARCHITECT.md`:

- `src/` is a thin demo/integration shell only.
- `packages/` owns reusable modules and kernel UI capabilities.
- `service/` owns kernel client adapters, mock data, request/response mapping,
  validation, defaulting, and error normalization.
- `components/` owns rendering only.
- `hooks/` owns React state binding and UI behavior.
- Public APIs are exported through package `src/index.ts` or `src/index.tsx`.
- Internal package dependencies use `workspace:*`.
- Deep imports across package internals are not allowed.
- Feature packages use explicit `components/`, `service/`, `hooks/`, and
  `types/` directories so rendering, data normalization, React binding, and
  contracts stay separated.

## Package Model

```text
sdkwork-kernel-ui/
|-- src/
|-- packages/
|   |-- sdkwork-kernel-ui-types/
|   |-- sdkwork-kernel-ui-core/
|   |-- sdkwork-kernel-ui-services/
|   |-- sdkwork-kernel-ui-commons/
|   |-- sdkwork-kernel-ui-agent/
|   |-- sdkwork-kernel-ui-code/
|   |-- sdkwork-kernel-ui-workspace/
|   |-- sdkwork-kernel-ui-terminal/
|   |-- sdkwork-kernel-ui-telemetry/
|   `-- sdkwork-kernel-ui-permissions/
|-- scripts/
|-- package.json
|-- pnpm-workspace.yaml
|-- tsconfig.json
`-- vite.config.ts
```

Package responsibilities:

- `@sdkwork/kernel-ui-types`: shared UI contracts for runtime state, capability
  manifests, events, permissions, code workspaces, patches, verification, and
  review findings.
- `@sdkwork/kernel-ui-core`: provider composition and typed kernel client
  context.
- `@sdkwork/kernel-ui-services`: mock and adapter-ready service layer for
  runtime snapshots, event streams, permission decisions, and code workspace
  projections.
- `@sdkwork/kernel-ui-commons`: reusable shell, status, event, badge, and
  layout components with no business rules.
- `@sdkwork/kernel-ui-agent`: agent runtime and task views.
- `@sdkwork/kernel-ui-code`: patch, verification, and review views that compose
  code-kernel evidence without owning workspace or terminal state.
- `@sdkwork/kernel-ui-workspace`: workspace, VCS branch, dirty state, changed
  file projection, and workspace safety summaries.
- `@sdkwork/kernel-ui-terminal`: command list, terminal output stream, command
  status, and cancellable execution presentation.
- `@sdkwork/kernel-ui-telemetry`: event stream, diagnostics, and health views.
- `@sdkwork/kernel-ui-permissions`: permission prompt and decision controls.

## Layering Conformance

The local architecture check enforces the package family and layering rules:

- Feature packages `agent`, `code`, `workspace`, `terminal`, `telemetry`, and
  `permissions` must expose `src/components/`, `src/service/`, `src/hooks/`,
  `src/types/`, and public `src/index.tsx`.
- `commons` must keep reusable UI primitives in `src/components/` with shared
  UI contracts in `src/types/`.
- `core` must keep runtime composition in `src/runtime/` with runtime contracts
  in `src/types/`.
- `services` must keep kernel client adapters, mock data, mapping, and error
  normalization in `src/service/`.
- Cross-package imports must use public package names such as
  `@sdkwork/kernel-ui-agent`; deep imports into another package's internals are
  rejected.

## Integration Contract

Host applications should integrate through a typed `KernelUiClient`.

The client boundary must support:

- Reading runtime snapshots and capability manifests.
- Subscribing to ordered kernel events.
- Creating tasks or forwarding task intents.
- Responding to permission requests.
- Reading code workspace projections, patch sets, verification reports, and
  review findings.

The UI must not call raw filesystem, process, network, or secret APIs. Those
operations belong to Rust host providers and protocol adapters.

## Verification

```bash
pnpm.cmd --dir kernel/sdkwork-kernel-ui install --frozen-lockfile
pnpm.cmd --dir kernel/sdkwork-kernel-ui lint
pnpm.cmd --dir kernel/sdkwork-kernel-ui build
```

When dependency installation is unavailable, the architecture script can still
be run with Node:

```bash
node kernel/sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs
```

## SDKWork Documentation Contract

Domain: device
Capability: component
Package type: react-package
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

- `pnpm --filter @sdkwork/kernel-ui-standard typecheck`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
