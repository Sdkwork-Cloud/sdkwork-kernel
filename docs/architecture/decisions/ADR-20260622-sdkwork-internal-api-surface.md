# ADR: SDKWork internal-api HTTP surface for agent runtime

Status: accepted  
Date: 2026-06-22  
Owner: SDKWork kernel maintainers

## Context

`sdkwork-agent-server` historically exposed kernel UI routes under `/api/kernel/*` on `application.public-ingress`. Business managed-agent APIs already use canonical SDKWork surfaces (`app-api`, `backend-api`, `open-api`) on `platform.api-gateway`. The runtime host needed a fourth HTTP surface that:

- stays on application ingress, not platform gateway
- uses ingress-token auth instead of dual-token IAM
- supports OpenAPI authority + generated SDK like other HTTP surfaces
- standardizes on SDKWork `internal-api` naming and path prefix

## Decision

Adopt **`internal-api`** as the fourth canonical HTTP surface with locked prefix **`/internal/v3/api`**.

For SDKWork kernel agent runtime:

| Item | Value |
| --- | --- |
| Canonical paths | `/internal/v3/api/intelligence/runtime/*` |
| OpenAPI authority | `sdkwork-agent-internal-api` |
| SDK family | `sdkwork-agent-internal-sdk` (`@sdkwork/agent-internal-sdk`) |
| Ingress | `application.public-ingress` |
| Retired alias | `/api/kernel/*` (removed; not mounted) |

Standards live in `sdkwork-specs/INTERNAL_API_SPEC.md` and `API_SPEC.md` section 4.

## Consequences

- Kernel UI consumes `@sdkwork/agent-internal-sdk` via `@sdkwork/kernel-ui-services`; handwritten `/api/kernel` fetches are retired.
- List endpoints return `{ items: [...] }` envelopes aligned with OpenAPI and generated SDK types.
- Validators and SDK workspace checks include the internal family.
- `sdkwork-routes-agent-internal-api` exposes the route manifest and re-exports runtime route builders from `sdkwork-agent-server`.
- Retired prefixes (`/api/kernel/*`, `/api/sessions/*`, `/api/chat/*`) are not mounted; callers must use canonical internal-api paths.

## Verification

- `node sdks/materialize-agent-internal-api-openapi.mjs`
- `node sdks/workspace-agent-sdkgen.mjs --mode apply`
- `node scripts/check-agent-sdk-workspace.mjs`
- `node scripts/check-kernel-standards.mjs`
- `cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml`
- `node tests/kernel_ui_server_api_alignment.test.mjs`
