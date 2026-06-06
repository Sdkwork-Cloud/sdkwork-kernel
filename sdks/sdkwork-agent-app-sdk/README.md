# SDKWork Agent App SDK

`sdkwork-agent-app-sdk` is the app/client SDK family for the agent domain.

| Field | Value |
| --- | --- |
| SDK family | `sdkwork-agent-app-sdk` |
| API authority | `sdkwork-agent-app-api` |
| API prefix | `/app/v3/api` |
| TypeScript package | `@sdkwork/agent-app-sdk` |
| SDK generator type | `app` |
| Audience | App, desktop, mobile, H5, and user-facing clients |

The authority OpenAPI is `openapi/sdkwork-agent-app-api.openapi.yaml`.
The derived generator input is `openapi/sdkwork-agent-app-api.sdkgen.yaml`.

Generated TypeScript transport output belongs under
`sdkwork-agent-app-sdk-typescript/generated/server-openapi`.

## Generate

Run from repository root:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --family app --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family app --mode apply
```

The generator command uses:

```text
--standard-profile sdkwork-v3
--api-prefix /app/v3/api
--package-name @sdkwork/agent-app-sdk
```

## Consume

User-facing clients should consume `@sdkwork/agent-app-sdk` or approved
service facades built on it. They must not call `/backend/v3/api`.

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent-app-sdk
Package type: sdk-family
Status: standardized

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- `SdkworkAppClient`

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `node sdks/materialize-agent-v3-openapi-boundaries.mjs`
- `node sdks/sdkwork-agent-app-sdk/bin/verify-sdk.mjs`
- `node sdks/test/verify-agent-sdk-ownership-boundaries.test.mjs`
- `node scripts/check-agent-sdk-workspace.mjs`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
