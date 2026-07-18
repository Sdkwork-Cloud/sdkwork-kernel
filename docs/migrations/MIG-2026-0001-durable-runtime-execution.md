# MIG-2026-0001: Durable Runtime Execution

```yaml
id: MIG-2026-0001
owner: SDKWork kernel maintainers
status: active
requirement: REQ-2026-0001
decision: ADR-20260716-durable-runtime-execution
type: mixed
scope:
  producers:
    - sdkwork-agent-database
    - sdkwork-agent-server
    - apis/internal-api/intelligence
    - sdks/sdkwork-agent-internal-sdk
  consumers:
    - sdkwork-agent-client
    - sdkwork-agents-runtime-facade
    - first-party internal runtime callers
compatibility_window:
  starts_at: 2026-07-17
  ends_at: before first production release
strategy: no-compatibility-approved
rollback:
  supported: false
  steps:
    - Stop promotion and forward-fix the pre-release revision; v5 is additive and v1-v4 data remains readable.
    - Restore the pre-migration backup only in a disposable pre-production environment when forward-fix validation cannot proceed.
verification:
  - cargo test --features postgres-sync --manifest-path sdkwork-agent-database/Cargo.toml
  - cargo test --manifest-path sdkwork-agent-server/Cargo.toml
  - node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
  - node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
  - node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
  - pnpm verify
  - pnpm verify:commercial
```

## Scope And Approval

The repository owner approved the no-compatibility pre-release cutover on
2026-07-17. The application has not entered production, so preserving the old
`POST .../tasks -> 201` behavior would create prohibited operation-pattern
debt. The authored internal OpenAPI changes first, followed by route materialization,
generated SDK regeneration, server handlers, and consumers.

Database migration v5 is an additive expand migration. It creates `runs`,
`steps`, and `permission_operations` plus claim and retention indexes in both
SQLite and PostgreSQL. It does not modify or delete v1-v4 rows. Migration
history checksums and structural drift validation fail startup when the applied
schema differs from source authority.

## Cutover Order

1. Apply and verify v5 schema on disposable SQLite and PostgreSQL databases.
2. Deploy code that understands v5 and keeps task execution disabled until
   readiness confirms schema version 5.
3. Materialize the async task/run OpenAPI and regenerate the internal SDK.
4. Update server, client, and sibling facade consumers to `202 SdkWorkAsyncData`.
5. Enable bounded workers only after claim, fencing, cancellation, retry,
   permission-resume, load, and restart tests pass.
6. Remove the old task-submit route and generated operation before the first
   production release.

## Rollback And Recovery

The database change is forward-only because destructive down migrations would
risk deleting accepted work and audit history. Rollback means stopping the
promotion and deploying a forward fix that continues to understand v5. A
pre-migration backup may be restored only for a disposable pre-production
environment after operators confirm no accepted v5 work must be retained.

Production promotion remains blocked until live PostgreSQL, provider staging,
HA, backup/restore, failover, load, memory, and graceful-shutdown evidence pass
on the exact revision.
