# SDKWork Agent Database

Domain: `intelligence`
Capability: `agent-database`
Package type: Rust runtime crate

Repository traits and SQLite, PostgreSQL, and typed in-memory test adapters for agent runtime transient state (sessions, messages, tasks, events, permissions).

`InMemoryDatabase` is a typed repository test double only. It supports the repository traits used by
unit tests, but raw `AgentDatabase::execute` and `query_many` fail closed instead of returning fake
success. Runtime persistence and schema migration verification use SQLite or PostgreSQL.

## Schema authority

DDL for both backends lives in a single pair of migration files:

- `migrations/agent_runtime.sqlite.sql`
- `migrations/agent_runtime.postgres.sql`

`schema_migrations.rs` applies SQLite statements; `postgres.rs` applies the PostgreSQL batch. Parity is guarded by `schema_migrations` unit tests and backend contract tests under `tests/`.

Session upserts use `ON CONFLICT ... DO UPDATE` on both backends (never SQLite `INSERT OR REPLACE`, which would cascade-delete child rows). Cross-table message append and message purge use `RuntimeSessionWrites` transactions (`append_message_with_event`, `delete_messages_and_reset_count`). Message identity is immutable: `save_message` accepts an exact duplicate row as an idempotent retry, rejects changed payloads or cross-session `message_id` reuse with `ConstraintViolation`, and preserves the original row. Message append is retry-safe: a duplicate `message_id` for the same session returns the current `message_count` without incrementing it again or writing another event, and a duplicate `message_id` for a different session fails with `ConstraintViolation` before writing an event.

## Pagination

List queries use SQL `LIMIT`/`OFFSET` for offset mode and keyset predicates for continuation:

- **Messages:** `MessageQuery.after_message_id` (HTTP `cursor` on `GET .../messages`)
- **Sessions:** `SessionQuery.after_session_id` (HTTP `cursor` on `GET .../sessions`)
- **Tasks:** `TaskQuery.after_task_id` (HTTP `cursor` on `GET .../sessions/{id}/tasks`)
- **Events:** `EventQuery.after_event_id` (SSE `Last-Event-ID` / `lastEventId`)

Unknown cursors return an empty page (strict replay), not a full rewind.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-database/Cargo.toml
cargo test --features postgres-sync --manifest-path sdkwork-agent-database/Cargo.toml --test agent_runtime_postgres_contracts
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
