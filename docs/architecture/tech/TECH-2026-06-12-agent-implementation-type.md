> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **as-built**

# Agent Implementation Type

## Goal

First-class `implementationType` (framework/runtime family) alongside `implementationKind` (adapter shape) for managed agents across domain, persistence, HTTP, and SDK surfaces.

## What landed

| Layer | Location |
| --- | --- |
| Domain enum + validation | `sdkwork-agent-business/src/domain.rs` |
| Service commands | `sdkwork-agent-business/src/application.rs` |
| HTTP DTOs | `sdkwork-agent-business/src/dto.rs`, `http.rs` |
| SQL contract | `sdkwork-agent-business/specs/sql/agent_business_postgres.sql` |
| DB spec | `sdkwork-agent-business/specs/AGENT_BUSINESS_DATABASE_SPEC.md` |
| Service tests | `sdkwork-agent-business/tests/agent_business_service_contracts.rs` |
| HTTP tests | `sdkwork-agent-business/tests/http_axum_contracts.rs` |
| ADR | `docs/architecture/decisions/ADR-20260612-agent-implementation-type.md` |

Defaults: missing/legacy values resolve to `sdkwork-native`. Invalid framework strings fail at domain/DTO boundaries with problem-detail `400`.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml implementation_type
cargo test --features http-axum --test http_axum_contracts --manifest-path sdkwork-agent-business/Cargo.toml
cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml
node scripts/check-agent-sdk-workspace.mjs
pnpm verify
```

Do not implement from checkbox steps in `docs/archive/superpowers/plans/2026-06-12-agent-implementation-type.md`.
