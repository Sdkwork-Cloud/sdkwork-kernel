# API Contract Workspace

Purpose: author-owned API contracts, API examples, changelogs, route authority inputs, and validation fixtures for **kernel runtime** surfaces.

Owner: SDKWork kernel maintainers.

Allowed content: OpenAPI, RPC, async/event API manifests, examples, changelogs, validation fixtures, and indexes that point to component-local API authorities.

Forbidden content: generated SDK transport output, runtime implementation code, generated SDK `.sdkwork/` reports, live secrets, and local runtime state.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/API_SPEC.md`, `../sdkwork-specs/SDK_SPEC.md`, `../sdkwork-specs/TEST_SPEC.md`, and `../specs/README.md`.

## Authority indexes

| Index | Owner component | Surfaces |
| --- | --- | --- |
| [`internal-api/authority-index.json`](./internal-api/authority-index.json) | `sdkwork-agent-server` / product shells via `@sdkwork/agent-internal-sdk` | `internal-api` |

Managed agents open/app/backend API authorities (`sdkwork-agents-open-api`, `sdkwork-agents-app-api`, `sdkwork-agents-backend-api`) are owned by the sibling application repository [`../sdkwork-agents/apis/agents/authority-index.json`](../sdkwork-agents/apis/agents/authority-index.json).

See `docs/architecture/decisions/ADR-20260626-agents-application-layer-separation.md` for the kernel vs application layering decision.

Verification: run `node scripts/check-kernel-standards.mjs` and `node scripts/check-agent-sdk-workspace.mjs` from the repository root.
