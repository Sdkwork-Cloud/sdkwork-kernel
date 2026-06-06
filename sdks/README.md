# SDKWork Agent SDK Workspace

This directory is the application-root SDK workspace for `app=agent`.

It owns the OpenAPI authority documents, derived generator inputs, generated
SDK output boundaries, and verification entrypoints for the agent domain. The
runtime API remains in `sdkwork-agent-business`; this workspace materializes
SDK-ready contracts from that source of truth.

This workspace follows the root SDK standard in `specs/SDK_SPEC.md`. All SDKs
are generated through:

```text
D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator
```

The canonical CLI entrypoint is:

```text
D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator\bin\sdkgen.js
```

Do not use `sdkwork-code-generator`, handwritten SDK forks, copied generated
output, or raw HTTP wrappers for remote agent business APIs. If an SDK method
is missing, fix the API/OpenAPI/generator chain and regenerate.

## SDK Families

| Family | Authority | Prefix | Package | Audience |
| --- | --- | --- | --- | --- |
| `sdkwork-agent-sdk` | `sdkwork-agent-open-api` | `/agent/v3/api` | `@sdkwork/agent-sdk` | Developer and integration authors |
| `sdkwork-agent-app-sdk` | `sdkwork-agent-app-api` | `/app/v3/api` | `@sdkwork/agent-app-sdk` | App, desktop, mobile, H5, and user-facing clients |
| `sdkwork-agent-backend-sdk` | `sdkwork-agent-backend-api` | `/backend/v3/api` | `@sdkwork/agent-backend-sdk` | Backend console, operators, automation, and control-plane integrations |

## Boundaries

- Authority OpenAPI files live under each family as
  `openapi/<authority>.openapi.yaml`.
- Derived generator inputs live beside authority files as
  `openapi/<authority>.sdkgen.yaml`.
- Generated TypeScript transport output belongs under
  `<family>/<family>-typescript/generated/server-openapi`.
- Handwritten wrappers or composed facades must stay outside
  `generated/server-openapi`.
- Generated SDK output must not be hand-edited. Fix runtime API, authority
  OpenAPI, or materialization rules, then regenerate.

## Commands

Run from repository root.

```powershell
node .\sdks\materialize-agent-v3-openapi-boundaries.mjs
node .\scripts\check-agent-sdk-workspace.mjs
```

Preview TypeScript SDK generation for all families:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
```

Apply TypeScript SDK generation into each family:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode apply
```

`sdkwork-agent-sdk` is materialized by
`sdks/materialize-agent-open-sdk-from-app.mjs` because the current
`sdkwork-v3` standard profile accepts only `app`, `backend`, and `im`
prefixes. The open SDK still owns `sdkwork-agent-open-api` and `/agent/v3/api`,
and the derivation remains part of the `sdkwork-sdk-generator` chain.

Verify one family:

```powershell
node .\sdks\sdkwork-agent-sdk\bin\verify-sdk.mjs
node .\sdks\sdkwork-agent-app-sdk\bin\verify-sdk.mjs
node .\sdks\sdkwork-agent-backend-sdk\bin\verify-sdk.mjs
```

The generator command always uses `--standard-profile sdkwork-v3`.

## Generator

The default generator entrypoint is:

```text
D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator\bin\sdkgen.js
```

Override it with `SDKWORK_SDKGEN_PATH` only when the same
`sdkwork-sdk-generator` product is checked out or installed at a different
local path.

## Versioning

The initial SDK package version is `0.1.0`, matching the agent business API
contract version. Generator output is reproducible from:

- authority OpenAPI
- derived `*.sdkgen.yaml`
- `sdks/_shared/agent-sdk-families.mjs`
- `sdkwork-sdk-generator`
- `--standard-profile sdkwork-v3`
