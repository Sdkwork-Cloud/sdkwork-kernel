# SDKWork Agent Internal SDK

`sdkwork-agent-internal-sdk` is the application-ingress internal SDK family for the agent runtime domain.

| Field | Value |
| --- | --- |
| SDK family | `sdkwork-agent-internal-sdk` |
| API authority | `sdkwork-agent-internal-api` |
| API prefix | `/internal/v3/api` |
| TypeScript package | `@sdkwork/agent-internal-sdk` |
| SDK generator type | `custom` |
| Audience | Kernel UI, embedded consoles, trusted in-app automation |

Authoring OpenAPI: `apis/internal-api/intelligence/sdkwork-agent-internal-api.openapi.yaml`  
Materialized authority: `openapi/sdkwork-agent-internal-api.openapi.yaml`  
Derived generator input: `openapi/sdkwork-agent-internal-api.sdkgen.yaml`

Generated TypeScript transport output belongs under
`sdkwork-agent-internal-sdk-typescript/generated/server-openapi`.

## Generate

Run from repository root:

```powershell
node .\sdks\materialize-agent-internal-api-openapi.mjs
node .\sdks\workspace-agent-sdkgen.mjs --family internal --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family internal --mode apply
```

The generator command uses:

```text
--standard-profile sdkwork-v3
--api-prefix /internal/v3/api
--package-name @sdkwork/agent-internal-sdk
```

Canonical generator entrypoint: `..\sdkwork-sdk-generator\bin\sdkgen.js`

## Consume

Kernel UI and trusted in-app automation should consume `@sdkwork/agent-internal-sdk`
or approved service facades built on it. Canonical runtime paths are
`/internal/v3/api/intelligence/runtime/*` on `application.public-ingress`.

## SDKWork Documentation Contract

Domain: intelligence  
Capability: agent-internal-sdk  
Surface: internal-api
