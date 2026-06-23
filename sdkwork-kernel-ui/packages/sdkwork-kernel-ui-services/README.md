# @sdkwork/kernel-ui-services

Domain: device  
Capability: component  
Package type: node-package  
Status: standardizing

This README is the SDKWork module entrypoint for `@sdkwork/kernel-ui-services`. The machine-readable component contract is `specs/component.spec.json`; canonical standards are under `../../../../sdkwork-specs/`.

## Public API

- `.` — kernel UI client factories, auth providers, and session helpers

## Required SDK Surface

- `@sdkwork/agent-internal-sdk` (`sdkwork-agent-internal-sdk` / `sdkwork-agent-internal-api`)
- Runtime calls use `/internal/v3/api/intelligence/runtime/*` through `createClient()`.

## Configuration

Configuration keys, runtime entrypoints, and integration contracts are declared in `specs/component.spec.json`. Shared modules must receive configuration through typed bootstrap or service boundaries rather than reading host-local environment state directly.

## SaaS/Private/Local Behavior

This component follows the deployment and runtime rules referenced by its `canonicalSpecs` entries. SaaS, private, and local behavior must stay compatible with the relevant SDKWork specs before implementation changes are made.

## Security

Ingress token auth is applied through `buildKernelUiAuthHeaders()` (`Authorization`, `X-API-Key`, `x-sdkwork-tenant-id`, `x-sdkwork-user-id`, and signed `x-sdkwork-identity-mac`). Do not add raw `fetch` clients or hand-built auth headers in UI modules.

## Extension Points

Extension points are limited to public exports, runtime entrypoints, SDK clients, events, and config keys declared in `specs/component.spec.json`.

## Verification

- `pnpm --filter @sdkwork/kernel-ui-services typecheck`

## Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`. Update that contract before changing public integration behavior.
