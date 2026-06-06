# SDKWork Agent Backend SDK

`sdkwork-agent-backend-sdk` is the backend/admin SDK family for the agent
domain.

| Field | Value |
| --- | --- |
| SDK family | `sdkwork-agent-backend-sdk` |
| API authority | `sdkwork-agent-backend-api` |
| API prefix | `/backend/v3/api` |
| TypeScript package | `@sdkwork/agent-backend-sdk` |
| SDK generator type | `backend` |
| Audience | Backend console, operators, automation, and control-plane integrations |

The authority OpenAPI is `openapi/sdkwork-agent-backend-api.openapi.yaml`.
The derived generator input is `openapi/sdkwork-agent-backend-api.sdkgen.yaml`.

Generated TypeScript transport output belongs under
`sdkwork-agent-backend-sdk-typescript/generated/server-openapi`.

## Generate

Run from repository root:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --family backend --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family backend --mode apply
```

The generator command uses:

```text
--standard-profile sdkwork-v3
--api-prefix /backend/v3/api
--package-name @sdkwork/agent-backend-sdk
```

## Consume

Backend console, operator tools, and control-plane integrations should consume
`@sdkwork/agent-backend-sdk`. User-facing clients must not consume this family.
