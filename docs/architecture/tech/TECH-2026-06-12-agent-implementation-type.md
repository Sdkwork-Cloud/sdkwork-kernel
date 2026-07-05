# Agent Implementation Type

Status: canonical reference (kernel scope)
Owner: sdkwork-kernel maintainers
Date: 2026-06-12

## Scope

The managed-agent `implementationKind` / `implementationType` domain model, SQL contract, HTTP DTOs,
and generated app/backend/open SDKs are owned by **`sdkwork-agents`**, not `sdkwork-kernel`.

Kernel responsibility:

- Agent runtime SPI (`sdkwork-agent-kernel`) and operational internal-api HTTP
  (`sdkwork-agent-server`).
- Product consumers integrate through `@sdkwork/agent-internal-sdk`.

## Authority

| Concern | Owner | Location |
| --- | --- | --- |
| Domain enum + validation | sdkwork-agents | `sdkwork-intelligence-agents-service` |
| Service commands + HTTP | sdkwork-agents | managed-store service crates |
| SQL contract | sdkwork-agents | `specs/sql/` under agents repository |
| Architecture decision | kernel (historical) + agents (active) | `ADR-20260612-agent-implementation-type.md` |

## Verification

Kernel gates (this repository):

```bash
node scripts/check-kernel-standards.mjs
node scripts/check-agent-sdk-workspace.mjs
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
```

Agents gates (sibling repository):

```bash
pnpm verify
```
