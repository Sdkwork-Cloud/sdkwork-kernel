# SDKGen Verification Report (Latest)

- Module: `sdkwork-agent-business`
- Date (local): `2026-06-01`
- Scope: app/backend OpenAPI -> SDK generation -> package check/build
- Command:
  - `powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-sdkgen.ps1 -Mode Apply -CleanTmp -JsonReportPath specs/sdkgen/verification-latest.json`
  - JSON report: `specs/sdkgen/verification-latest.json`

## Dry-Run Summary

- app
  - resolved version: `1.0.36`
  - change fingerprint: `d6798a705898520e5faeedb0de17d32614ebfccc92bb19833939339849ea6547`
  - risk: `medium`
  - has changes: `true`
- backend
  - resolved version: `1.0.11`
  - change fingerprint: `6695a725196dd2609f26554cf072a7962256f5131dec250637c140cbf927dc29`
  - risk: `high`
  - has changes: `true`

## Apply Result

- app SDK (`sdkwork-agent-business-app-sdk`)
  - fixed version: `1.0.36`
  - output: `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp/agent-business-app-sdk-typescript`
  - generator result: success
  - impact: `build-metadata`, `documentation`
- backend SDK (`sdkwork-agent-business-backend-sdk`)
  - fixed version: `1.0.11`
  - output: `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp/agent-business-backend-sdk-typescript`
  - generator result: success
  - impact: `api-surface`, `models`, `runtime`, `build-metadata`, `publish-workflow`, `documentation`, `custom-scaffold`, `unknown`

## Package Verification

- app generated package
  - `publish-core --action check`: pass
  - `publish-core --action build`: pass
  - npm pack artifact: `sdkwork-app-sdk-1.0.36.tgz`
- backend generated package
  - `publish-core --action check`: pass
  - `publish-core --action build`: pass
  - npm pack artifact: `sdkwork-backend-sdk-1.0.11.tgz`

## Cleanup

- `-CleanTmp` enabled.
- temporary output directory `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp` removed after verification.

## Follow-up Verification (Current Commit Scope)

- `cargo test --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml`: pass
- `cargo test --features http-axum --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml`: pass
- `cargo test --features postgres-sync --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml`: pass
- `powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-ci.ps1`: pass
- CI dry-run JSON report updated: `specs/sdkgen/verification-ci.json`
- RFC3339 strict validation for mutation `requestedAt` covered by dto/http contract tests: pass
- RFC3339 validation error detail now uses API field naming `requestedAt`: pass
- RFC3339 parsing logic unified into shared `validation` module and reused by dto/http query filter: pass
- Audit event `occurred_at` RFC3339 parsing now reuses shared `validation` parser while preserving `internal_error` mapping: pass
- `tenant_id/int64 string` parsing unified into shared `validation` module for dto/http consistency: pass
- Added semantic validation wrappers (`parse_tenant_id/organization_id/owner_user_id`, `validate_requested_at`) and migrated dto/http callers: pass
- Added optional `expectedVersion` optimistic-concurrency validation for update/status/delete/restore across dto/http/service, and aligned app/backend OpenAPI contracts: pass
- Repository-level optimistic locking hardened: in-memory adapter enforces monotonic `version`, and postgres update SQL now includes `WHERE ... AND version = previous_version` precondition with conflict mapping: pass
- API conflict semantics refined: optimistic-concurrency conflicts now return `application/problem+json` code `version_conflict`, and app/backend OpenAPI `Problem` examples include `version_conflict` payload: pass
