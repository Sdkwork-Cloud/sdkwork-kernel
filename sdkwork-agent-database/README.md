# SDKWork Agent Database

Domain: `intelligence`
Capability: `agent-database`
Package type: Rust runtime crate

Repository traits and SQLite, PostgreSQL, and typed in-memory test adapters for agent runtime transient state (sessions, messages, tasks, events, permissions).

`InMemoryDatabase` is a typed repository test double only. It supports the repository traits used by
unit tests, but raw `AgentDatabase::execute` and `query_many` fail closed instead of returning fake
success. Runtime persistence and schema migration verification use SQLite or PostgreSQL.

## Schema authority

Baseline DDL and ordered evolution migrations live under `migrations/`:

- `migrations/agent_runtime.sqlite.sql`
- `migrations/agent_runtime.postgres.sql`
- `migrations/agent_runtime.postgres.v2.sql`
- `migrations/agent_runtime.sqlite.v3.sql`
- `migrations/agent_runtime.postgres.v3.sql`
- `migrations/agent_runtime.sqlite.v4.sql`
- `migrations/agent_runtime.postgres.v4.sql`

`schema_migrations.rs` is the migration authority. It records ordered versions and SHA-256 checksums in `agent_runtime_schema_migration_history`, rejects checksum drift, and commits all pending steps atomically. SQLite uses `BEGIN IMMEDIATE` to serialize concurrent startup; PostgreSQL uses a transaction-scoped advisory lock. The baseline and evolution scripts contain no destructive table drops.

The SQLite v2 compatibility step repairs legacy `sessions` columns and missing child-table cascade foreign keys. Orphan child rows are preserved under explicit `orphaned` recovery sessions before constraints are installed. A rebuild fails closed when an unknown extension column or custom trigger is present, so migration cannot silently discard application-owned data. PostgreSQL performs the equivalent column, orphan, and foreign-key repair in v2. Fresh database creation, legacy upgrade, repeated migration, checksum drift, and concurrent SQLite startup are covered by migration contract tests.

Session upserts use `ON CONFLICT ... DO UPDATE` on both backends (never SQLite `INSERT OR REPLACE`, which would cascade-delete child rows). `RuntimeSessionWrites` owns cross-table transactions for session state plus event, task state plus event, single-message append plus count plus event, completed user/assistant turn plus all turn events, and message purge plus count reset. A completed turn commits the user message, optional assistant message, each `message.sent` event, the single `turn.completed` event, and the final `message_count` in one transaction; a conflict in any later row rolls back the entire turn. Message identity is immutable: `save_message` accepts an exact duplicate row as an idempotent retry, rejects changed payloads or cross-session `message_id` reuse with `ConstraintViolation`, and preserves the original row. Message append is retry-safe: a duplicate `message_id` for the same session returns the current `message_count` without incrementing it again or writing another event, and a duplicate `message_id` for a different session fails with `ConstraintViolation` before writing an event.

## Pagination

List queries use SQL `LIMIT`/`OFFSET` for offset mode and keyset predicates for continuation:

- **Messages:** `MessageQuery.after_message_created_at` plus `after_message_id`, ordered by `(created_at, message_id)`
- **Sessions:** `SessionQuery.after_session_sort_at` plus `after_session_id`, ordered by `(COALESCE(updated_at, created_at), session_id)` descending
- **Tasks:** `TaskQuery.after_task_created_at` plus `after_task_id`, ordered by `(created_at, task_id)`
- **Events:** internal `EventQuery.after_event_id`, ordered by `(created_at, event_id)`

The session/message/task sort key and unique ID are carried inside a versioned, resource-scoped, HMAC-signed opaque HTTP cursor. Repositories seek directly on that tuple, so deleting the last row from a prior page does not truncate later pages. ID-only repository continuation remains available for internal compatibility and returns an empty page when its anchor cannot be resolved; it is not used by the public HTTP pagination path. Event replay intentionally remains ID-anchored because an unknown `Last-Event-ID` must not rewind a retained event stream.

`MessageRepository::load_recent_messages` hydrates a session from a bounded SQL tail (`ORDER BY created_at DESC, message_id DESC LIMIT`) and returns chronological order. The limit is mandatory and restricted to `1..=512`; full-history hydration and deep offset scans are not supported.

`EventQuery` and `PermissionQuery` accept optional tenant/user ownership scope. Scoped event reads validate ownership through `sessions` and exclude global events; scoped permission reads filter indexed ownership columns directly.

## Runtime retention and lifecycle

Runtime rows are transient and are cleaned by the server's bounded retention worker. The worker passes a UTC RFC-3339 cutoff to `RuntimeMaintenance::purge_expired`; each backend selects at most the configured batch size in SQL and commits one transaction. Closed/completed/failed/cancelled sessions, terminal tasks, and resolved permissions are eligible for deletion. Pending or running work is retained. Messages and events older than the cutoff are removed in bounded batches, and message counts are recomputed for affected sessions in the same transaction. Session cascades preserve foreign-key integrity; no backend performs an unbounded collect.

SQLite enables WAL and incremental auto-vacuum for new files. After a successful purge it runs only a passive WAL checkpoint and `incremental_vacuum(1000)`; a one-time full vacuum belongs in an offline operator maintenance window. PostgreSQL delegates dead-row reclamation to autovacuum. `RuntimeMaintenance::schema_status` validates the migration version, required indexes, and foreign-key invariants so readiness fails closed on drift.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-database/Cargo.toml
SDKWORK_DATABASE_URL=<disposable-postgres-uri> cargo test --features postgres-sync --manifest-path sdkwork-agent-database/Cargo.toml --test agent_runtime_postgres_contracts -- --ignored
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
