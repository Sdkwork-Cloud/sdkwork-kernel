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
- Backend audit endpoint `/backend/v3/api/ai/agents/{agentId}/audit_events`
  returns recorded audit events with page/page_size pagination.
- Operation IDs follow dotted resource style, for example `agents.create`,
  `agents.status.update`, and `agents.auditEvents.list`.

## SDK Generation Contract

Use the canonical SDK generator in the repository root:

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

## Verification

```bash
cargo test --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml
```

```bash
cargo test --features http-axum --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml
```

```bash
cargo test --features postgres-sync --manifest-path apps/sdkwork-birdcoder/kernel/sdkwork-agent-business/Cargo.toml
```
