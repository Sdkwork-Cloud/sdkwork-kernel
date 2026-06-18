# API Contract Workspace

Purpose: author-owned API contracts, API examples, changelogs, route authority inputs, and validation fixtures for cross-component kernel surfaces.

Owner: SDKWork kernel maintainers.

Allowed content: OpenAPI, RPC, async/event API manifests, examples, changelogs, validation fixtures, and indexes that point to component-local API authorities.

Forbidden content: generated SDK transport output, runtime implementation code, generated SDK `.sdkwork/` reports, live secrets, and local runtime state.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/API_SPEC.md`, `../sdkwork-specs/SDK_SPEC.md`, `../sdkwork-specs/TEST_SPEC.md`, and `../specs/README.md`.

## Authority indexes

| Index | Owner component | Surfaces |
| --- | --- | --- |
| [`agent-business/authority-index.json`](./agent-business/authority-index.json) | `sdkwork-agent-business` | `open-api`, `app-api`, `backend-api` |

Component-local OpenAPI files remain authoritative on disk under `sdkwork-agent-business/specs/openapi/` until route crates are extracted per `docs/architecture/decisions/ADR-20260618-platform-framework-adoption.md`.

Verification: run `node scripts/check-kernel-standards.mjs` and `node scripts/check-agent-sdk-workspace.mjs` from the repository root.
