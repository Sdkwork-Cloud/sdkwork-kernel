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
