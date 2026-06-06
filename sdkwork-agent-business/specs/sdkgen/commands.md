# SDK Generator Commands

This document records the canonical SDK generation flow for
`sdkwork-agent-business` under the current application root.

The application domain is `agent`; SDK generation is owned by the root
`sdks/` workspace.

All SDKs are generated through the SDKWork SDK generator:

```text
D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator
```

The canonical CLI entrypoint is:

```text
D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator\bin\sdkgen.js
```

Do not use `sdkwork-code-generator` for SDK generation. Do not hand-edit
generated SDK output; fix the API/OpenAPI/generator chain and regenerate.

Run all commands from repository root.

## Materialize OpenAPI Boundaries

```powershell
node .\sdks\materialize-agent-v3-openapi-boundaries.mjs
```

This creates or refreshes:

- `sdks/sdkwork-agent-sdk/openapi/sdkwork-agent-open-api.openapi.yaml`
- `sdks/sdkwork-agent-sdk/openapi/sdkwork-agent-open-api.sdkgen.yaml`
- `sdks/sdkwork-agent-app-sdk/openapi/sdkwork-agent-app-api.openapi.yaml`
- `sdks/sdkwork-agent-app-sdk/openapi/sdkwork-agent-app-api.sdkgen.yaml`
- `sdks/sdkwork-agent-backend-sdk/openapi/sdkwork-agent-backend-api.openapi.yaml`
- `sdks/sdkwork-agent-backend-sdk/openapi/sdkwork-agent-backend-api.sdkgen.yaml`

It also materializes the module-local open API test fixture:

- `sdkwork-agent-business/specs/openapi/agent-business-open-openapi-3.1.2.yaml`

## Developer/Open SDK

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --family open --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family open --mode apply
```

Authority: `sdkwork-agent-open-api`

Prefix: `/agent/v3/api`

Package: `@sdkwork/agent-sdk`

Output boundary:
`sdks/sdkwork-agent-sdk/sdkwork-agent-sdk-typescript/generated/server-openapi`

## App SDK

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --family app --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family app --mode apply
```

Authority: `sdkwork-agent-app-api`

Prefix: `/app/v3/api`

Package: `@sdkwork/agent-app-sdk`

Output boundary:
`sdks/sdkwork-agent-app-sdk/sdkwork-agent-app-sdk-typescript/generated/server-openapi`

## Backend SDK

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --family backend --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --family backend --mode apply
```

Authority: `sdkwork-agent-backend-api`

Prefix: `/backend/v3/api`

Package: `@sdkwork/agent-backend-sdk`

Output boundary:
`sdks/sdkwork-agent-backend-sdk/sdkwork-agent-backend-sdk-typescript/generated/server-openapi`

## All Families

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --mode apply
```

All generator commands use:

```text
--standard-profile sdkwork-v3
```

## Compatibility Wrapper

The module-local PowerShell wrapper delegates to the root `sdks/` workspace:

```powershell
powershell -ExecutionPolicy Bypass -File .\sdkwork-agent-business\scripts\verify-sdkgen.ps1 -Mode DryRun
powershell -ExecutionPolicy Bypass -File .\sdkwork-agent-business\scripts\verify-sdkgen.ps1 -Mode DryRun -SkipBuild -JsonReportPath specs/sdkgen/verification-latest.json
```

## Verification Checklist

- OpenAPI version is `3.1.2`.
- Authority files and derived `*.sdkgen.yaml` files are separate.
- Paths use canonical prefixes:
  - `/agent/v3/api/...`
  - `/app/v3/api/...`
  - `/backend/v3/api/...`
- Operation IDs use dotted resource style.
- Security uses dual token (`AuthToken` + `AccessToken`) for protected endpoints.
- Problem responses use `application/problem+json` with RFC 9457 shape.
- Generated output stays under `generated/server-openapi`.
- Generated output is not hand-edited.
