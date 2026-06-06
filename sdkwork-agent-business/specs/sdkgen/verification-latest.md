# Latest SDK Verification Record

- Date: 2026-06-05
- Application domain: `agent`
- SDK workspace: `sdks/`
- Generator profile: `--standard-profile sdkwork-v3`
- Generator entrypoint:
  `D:/javasource/spring-ai-plus/sdk/sdkwork-sdk-generator/bin/sdkgen.js`

## Commands

```powershell
node .\sdks\materialize-agent-v3-openapi-boundaries.mjs
node .\sdks\workspace-agent-sdkgen.mjs --mode apply
node .\scripts\check-agent-sdk-workspace.mjs
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
```

Generated package checks:

```powershell
node .\bin\publish-core.mjs --language typescript --project-dir . --action check
node .\bin\publish-core.mjs --language typescript --project-dir . --action build
```

The package checks were run from:

- `sdks/sdkwork-agent-app-sdk/sdkwork-agent-app-sdk-typescript/generated/server-openapi`
- `sdks/sdkwork-agent-backend-sdk/sdkwork-agent-backend-sdk-typescript/generated/server-openapi`

## Families

- developer/open SDK (`sdkwork-agent-sdk`)
  - authority: `sdkwork-agent-open-api`
  - prefix: `/agent/v3/api`
  - package: `@sdkwork/agent-sdk`
  - output:
    `sdks/sdkwork-agent-sdk/sdkwork-agent-sdk-typescript/generated/server-openapi`
  - status: authority and derived sdkgen inputs materialized
  - generation status: script-derived from the strict-profile app SDK source,
    because the current `sdkwork-v3` standard profile supports `app`,
    `backend`, and `im` prefixes only.
  - check: pass
  - build: pass

- app SDK (`sdkwork-agent-app-sdk`)
  - authority: `sdkwork-agent-app-api`
  - prefix: `/app/v3/api`
  - package: `@sdkwork/agent-app-sdk`
  - output:
    `sdks/sdkwork-agent-app-sdk/sdkwork-agent-app-sdk-typescript/generated/server-openapi`
  - check: pass
  - build: pass

- backend SDK (`sdkwork-agent-backend-sdk`)
  - authority: `sdkwork-agent-backend-api`
  - prefix: `/backend/v3/api`
  - package: `@sdkwork/agent-backend-sdk`
  - output:
    `sdks/sdkwork-agent-backend-sdk/sdkwork-agent-backend-sdk-typescript/generated/server-openapi`
  - check: pass
  - build: pass

## Dry-Run Summary

The latest dry-run report is stored in:

- `sdkwork-agent-business/specs/sdkgen/verification-latest.json`
- `sdkwork-agent-business/specs/sdkgen/verification-ci.json`
- `sdks/.sdkgen-agent-workspace-report.json`

Latest dry-run state:

- `sdkwork-agent-sdk`: standard-profile generator skipped with recorded
  support gap; open SDK derivation `hasChanges=false`.
- `sdkwork-agent-app-sdk`: `hasChanges=false`, `riskLevel=low`.
- `sdkwork-agent-backend-sdk`: `hasChanges=false`, `riskLevel=low`.

## Contract Checks

- Authority OpenAPI and derived `*.sdkgen.yaml` are separated.
- `*.sdkgen.yaml` inputs inline explicit RFC 9457
  `application/problem+json` responses for generator strict profile.
- `X-Request-Id` is not exposed.
- Generated output stays under `generated/server-openapi`.
- Runtime now exposes `/agent/v3/api`, `/app/v3/api`, and `/backend/v3/api`
  route families.
