# SDKWork Agent Business

`sdkwork-agent-business` is the backend business module for agent management.
It depends on:

- `sdkwork-agent-kernel` for core agent runtime contracts, policy requests,
  events, and error models.
- `sdkwork-code-kernel` for code-task intent contracts used by managed agents.

This module is backend-focused and does not include frontend UI or app shell
integration. It defines:

- agent business domain model and lifecycle
- CRUD service orchestration with policy checks
- audit event contracts
- repository/audit ports and deterministic in-memory adapters for tests
- postgres-oriented row mapping and SQL contract constants for persistence
  adapters
- optional `postgres-sync` feature with executable PostgreSQL adapter for
  repository/audit writes
- optional `http-axum` feature with app-api/backend-api router composition and
  RFC 9457 problem detail responses
- app-api/backend-api route and operation contract declarations
- app-api/backend-api DTO mapping contracts
- database contract and DDL baseline for deployment-specific adapters

## Architecture

```text
sdkwork-agent-business/
|-- src/
|   |-- lib.rs
|   |-- api.rs               # app-api/backend-api operation contract declarations
|   |-- domain.rs            # entities, status machine, business enums
|   |-- dto.rs               # API DTO <-> command/entity mapping
|   |-- application.rs       # command models + business service orchestration
|   |-- ports.rs             # repository/audit interfaces
|   |-- infrastructure.rs    # in-memory adapters + policy stub for tests
|   |-- persistence.rs       # postgres row mapping, SQL constants, adapter wrappers
|   `-- http.rs              # optional axum app-api/backend-api route entrypoints
|-- tests/
|   |-- agent_business_service_contracts.rs
|   `-- http_axum_contracts.rs
|-- scripts/
|   |-- verify-sdkgen.ps1
|   `-- verify-ci.ps1
`-- specs/
    |-- README.md
    |-- component.spec.json
    |-- AGENT_BUSINESS_DATABASE_SPEC.md
    |-- sdkgen/commands.md
    |-- sql/agent_business_postgres.sql
    `-- openapi/
        |-- agent-business-app-openapi-3.1.2.yaml
        `-- agent-business-backend-openapi-3.1.2.yaml
```

## API Surfaces

- App API prefix: `/app/v3/api`
- Backend API prefix: `/backend/v3/api`
- Canonical resources:
  - `/app/v3/api/ai/agents`
  - `/backend/v3/api/ai/agents`
- Restore endpoints:
  - `/app/v3/api/ai/agents/{agentId}/restore`
  - `/backend/v3/api/ai/agents/{agentId}/restore`
- Backend audit endpoint `/backend/v3/api/ai/agents/{agentId}/audit_events`
  returns recorded audit events with `page/page_size` pagination and optional
  `action/from/to` filters; `from`/`to` must be RFC3339 and `from <= to`.
- Agent list endpoint supports optional `q` fuzzy search over `agentId`, `code`,
  `displayName`, and `description`.
- All mutation requests validate `requestedAt` strictly as RFC3339
  date-time (`create/update/delete/status/restore`).
- Operation IDs follow dotted resource style, for example `agents.create`,
  `agents.status.update`, and `agents.auditEvents.list`.

## SDK Generation Contract

Use the repository root `sdks/` workspace as the canonical SDK generation
boundary. The current application domain is `agent`.

```powershell
node .\sdks\materialize-agent-v3-openapi-boundaries.mjs
```

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
node .\sdks\workspace-agent-sdkgen.mjs --mode apply
```

SDK families:

- `sdkwork-agent-sdk`: `sdkwork-agent-open-api`, `/agent/v3/api`, `@sdkwork/agent-sdk`
- `sdkwork-agent-app-sdk`: `sdkwork-agent-app-api`, `/app/v3/api`, `@sdkwork/agent-app-sdk`
- `sdkwork-agent-backend-sdk`: `sdkwork-agent-backend-api`, `/backend/v3/api`, `@sdkwork/agent-backend-sdk`

All SDK generator commands use `--standard-profile sdkwork-v3`.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml
```

```bash
cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml
```

```bash
cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml
```

```powershell
node .\scripts\check-agent-sdk-workspace.mjs
```

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
```

```powershell
powershell -ExecutionPolicy Bypass -File .\sdkwork-agent-business\scripts\verify-ci.ps1
```

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent-business
Package type: rust-crate
Status: standardizing

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- `sdkwork-agent-sdk: sdkwork-agent-open-api -> @sdkwork/agent-sdk`
- `sdkwork-agent-app-sdk: sdkwork-agent-app-api -> @sdkwork/agent-app-sdk`
- `sdkwork-agent-backend-sdk: sdkwork-agent-backend-api -> @sdkwork/agent-backend-sdk`

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml`
- `node sdks/materialize-agent-v3-openapi-boundaries.mjs`
- `node sdks/workspace-agent-sdkgen.mjs --mode dry-run`
- `node scripts/check-agent-sdk-workspace.mjs`
- `powershell -ExecutionPolicy Bypass -File sdkwork-agent-business/scripts/verify-ci.ps1`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
