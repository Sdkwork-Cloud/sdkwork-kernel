> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **superseded**

# Rig Agent Provider Deployments — Implementation Plan (Superseded)

The checkbox plan for Rig-backed provider bindings and managed-agent deployments is **superseded** by the shipped business and plugin contracts.

## Authoritative sources

- Domain and service contracts: `sdkwork-agent-business/src/domain.rs`, `application.rs`, `ports.rs`
- SQL contract: `sdkwork-agent-business/specs/sql/agent_business_postgres.sql`
- Database spec: `sdkwork-agent-business/specs/AGENT_BUSINESS_DATABASE_SPEC.md`
- Contract tests: `sdkwork-agent-business/tests/agent_provider_deployment_contracts.rs` (and related marketplace/provider tests)
- Rig plugin crate: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig`

## What landed

- Agents record `implementation_provider_id` and `implementation_kind`
- Provider binding lifecycle with optimistic concurrency (`expectedVersion`)
- Deployment snapshots preserve provider/binding state at deploy time
- Rig remains an implementation provider; business logic stays provider-neutral

## Verification

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml
cargo test --features http-axum --test http_axum_contracts --manifest-path sdkwork-agent-business/Cargo.toml
```

Do not implement from unchecked steps in `docs/superpowers/plans/2026-06-04-rig-agent-provider-deployments.md`.
