# SDK Generator Commands

This document defines canonical SDK generation commands for
`sdkwork-agent-business`.

Use the repository root as the command working directory.

## App SDK (TypeScript)

```bash
node sdk/sdkwork-sdk-generator/bin/sdkgen.js generate \
  -i apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/specs/openapi/agent-business-app-openapi-3.1.2.yaml \
  -o spring-ai-plus-app-api/sdkwork-sdk-app/sdks/agent-business-app-sdk-typescript \
  -n sdkwork-agent-business-app-sdk \
  -t app \
  -l typescript \
  --base-url http://localhost:8080 \
  --api-prefix /app/v3/api \
  --standard-profile sdkwork-v3
```

## Backend SDK (TypeScript)

```bash
node sdk/sdkwork-sdk-generator/bin/sdkgen.js generate \
  -i apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/specs/openapi/agent-business-backend-openapi-3.1.2.yaml \
  -o spring-ai-plus-backend-api/sdkwork-sdk-backend/sdks/agent-business-backend-sdk-typescript \
  -n sdkwork-agent-business-backend-sdk \
  -t backend \
  -l typescript \
  --base-url http://localhost:8080 \
  --api-prefix /backend/v3/api \
  --standard-profile sdkwork-v3
```

## Verification Checklist

- OpenAPI version is `3.1.2`.
- Paths use canonical prefixes:
  - `/app/v3/api/...`
  - `/backend/v3/api/...`
- Operation IDs use dotted lowerCamelCase resource style.
- Security uses dual token (`AuthToken` + `AccessToken`) for protected endpoints.
- Problem responses use `application/problem+json` with RFC 9457 shape.

## Module-Local SDK Verification

Run inside repository root to validate both app/backend OpenAPI contracts
against `sdkwork-sdk-generator` without touching external SDK repositories.

```powershell
powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-sdkgen.ps1 -Mode DryRun
```

Apply generation into `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp`
and run generated package check/build:

```powershell
powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-sdkgen.ps1 -Mode Apply
```

Optional cleanup of temporary outputs after verification:

```powershell
powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-sdkgen.ps1 -Mode Apply -CleanTmp
```
