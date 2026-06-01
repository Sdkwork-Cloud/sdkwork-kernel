# SDKGen Verification Report (Latest)

- Module: `sdkwork-agent-business`
- Date (local): `2026-06-01`
- Scope: app/backend OpenAPI -> SDK generation -> package check/build
- Command:
  - `powershell -ExecutionPolicy Bypass -File apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/scripts/verify-sdkgen.ps1 -Mode Apply -CleanTmp -JsonReportPath specs/sdkgen/verification-latest.json`
  - JSON report: `specs/sdkgen/verification-latest.json`

## Dry-Run Summary

- app
  - resolved version: `1.0.35`
  - change fingerprint: `18bfcb0c2705e66889bd89b823999045d6386b2d0e926522084b0f7f161c0275`
  - risk: `high`
  - has changes: `true`
- backend
  - resolved version: `1.0.11`
  - change fingerprint: `8a71d744793e81d7f2422b12563385eafcd46e5b4b2982b5d700721d55b34efb`
  - risk: `high`
  - has changes: `true`

## Apply Result

- app SDK (`sdkwork-agent-business-app-sdk`)
  - fixed version: `1.0.35`
  - output: `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp/agent-business-app-sdk-typescript`
  - generator result: success
  - impact: `api-surface`, `models`, `runtime`, `build-metadata`, `publish-workflow`, `documentation`, `custom-scaffold`, `unknown`
- backend SDK (`sdkwork-agent-business-backend-sdk`)
  - fixed version: `1.0.11`
  - output: `apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/.tmp/agent-business-backend-sdk-typescript`
  - generator result: success
  - impact: `api-surface`, `models`, `runtime`, `build-metadata`, `publish-workflow`, `documentation`, `custom-scaffold`, `unknown`

## Package Verification

- app generated package
  - `publish-core --action check`: pass
  - `publish-core --action build`: pass
  - npm pack artifact: `sdkwork-app-sdk-1.0.35.tgz`
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
- Operation-level OpenAPI responses now include explicit `409` `version_conflict` examples for update/delete/restore/status operations, and SDK apply verification completed (`verify-sdkgen.ps1 -Mode Apply -CleanTmp`): pass
- Create conflict semantics completed: app/backend `agents.create` now include operation-level `409 conflict` examples (duplicate `agent_id/code`), with HTTP contract test coverage for duplicate create: pass
- `ProblemDetail` contract enriched with `errorCategory` and `retryable` in runtime response + app/backend OpenAPI schema/examples; HTTP contract tests assert validation/conflict/version_conflict category and retryability semantics: pass
- `errorCategory` mapping refactored to a centralized internal enum (`ErrorCategory`) for deterministic classification and lower maintenance risk when adding new error codes: pass
