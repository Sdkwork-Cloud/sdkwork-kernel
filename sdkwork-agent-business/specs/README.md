# SDKWork Agent Business Component Specs

This directory is the local standards index for `sdkwork-agent-business`.

Root SDKWork standards remain authoritative. Local component specs can narrow
or document this component, but they must not contradict
[the root standards](../../../sdkwork-specs/README.md).

## Component

| Field | Value |
| --- | --- |
| Name | `sdkwork-agent-business` |
| Type | `rust-crate` |
| Root | `sdkwork-kernel/sdkwork-agent-business` |
| Domain | `intelligence` |
| Capability | `agent-business` |
| Languages | `rust` |
| Status | `standardizing` |

## Contract Manifest

- [component.spec.json](./component.spec.json) is the machine-readable component
  contract.
- [sdkgen/commands.md](./sdkgen/commands.md) defines canonical app/backend SDK
  generation commands from OpenAPI.
- [AGENT_BUSINESS_HTTP_TRUST_BOUNDARY.md](./AGENT_BUSINESS_HTTP_TRUST_BOUNDARY.md)
  documents backend/open route trust boundaries and tenant reconciliation rules.
- Consumers should integrate through public exports, runtime entrypoints, SDK
  clients, or adapters declared in the manifest.
- Generated SDK language outputs are represented at their SDK family root
  instead of duplicating local specs in generated folders.

## Canonical Specs

| Spec | Applies Because |
| --- | --- |
| [API_SPEC.md](../../../sdkwork-specs/API_SPEC.md) | OpenAPI 3.1.2 profile, path prefix, operationId, and response contracts. |
| [COMPONENT_SPEC.md](../../../sdkwork-specs/COMPONENT_SPEC.md) | Local component specs directory and manifest rules. |
| [CONFIG_SPEC.md](../../../sdkwork-specs/CONFIG_SPEC.md) | Runtime configuration, environment, SDK bootstrap, and feature flag rules. |
| [DATABASE_SPEC.md](../../../sdkwork-specs/DATABASE_SPEC.md) | Table naming, logical types, schema evolution, and tenant isolation rules. |
| [DEPLOYMENT_SPEC.md](../../../sdkwork-specs/DEPLOYMENT_SPEC.md) | SaaS/private/local runtime parity and deployment rules. |
| [DOCUMENTATION_SPEC.md](../../../sdkwork-specs/DOCUMENTATION_SPEC.md) | Module README, examples, ADR, changelog, and runbook rules. |
| [DOMAIN_SPEC.md](../../../sdkwork-specs/DOMAIN_SPEC.md) | Canonical domain ownership and naming. |
| [GOVERNANCE_SPEC.md](../../../sdkwork-specs/GOVERNANCE_SPEC.md) | Standard ownership, exception, compatibility, and migration rules. |
| [IAM_SPEC.md](../../../sdkwork-specs/IAM_SPEC.md) | Tenant, organization, and authorization context contracts. |
| [MODULE_SPEC.md](../../../sdkwork-specs/MODULE_SPEC.md) | Reusable package contract and dependency direction. |
| [OBSERVABILITY_SPEC.md](../../../sdkwork-specs/OBSERVABILITY_SPEC.md) | Log, metric, trace, audit, and diagnostic rules. |
| [PERFORMANCE_SPEC.md](../../../sdkwork-specs/PERFORMANCE_SPEC.md) | Latency, pagination, scalability, and retry budget rules. |
| [README.md](../../../sdkwork-specs/README.md) | SDKWork root standards entrypoint. |
| [SDK_SPEC.md](../../../sdkwork-specs/SDK_SPEC.md) | SDK generation and service integration contract rules. |
| [SECURITY_SPEC.md](../../../sdkwork-specs/SECURITY_SPEC.md) | Dual-token, authz, secret redaction, and sensitive operation controls. |
| [TEST_SPEC.md](../../../sdkwork-specs/TEST_SPEC.md) | Contract and conformance verification rules. |

## Public Exports

- `sdkwork_agent_business::application::*` command models and service contract.
- `sdkwork_agent_business::domain::*` business status, visibility, and record
  contracts.
- `sdkwork_agent_business::api::*` app-api/backend-api operation declarations.
- `sdkwork_agent_business::dto::*` app-api/backend-api DTO and command mapping
  contracts.
- `sdkwork_agent_business::ports::*` repository and audit sink interfaces.
- `sdkwork_agent_business::infrastructure::*` deterministic in-memory adapters
  for contract tests.
- `sdkwork_agent_business::persistence::*` PostgreSQL-oriented row mapping,
  SQL contracts, repository/audit adapters, and optional `postgres-sync`
  executable adapter.
- `sdkwork_agent_business::{AgentHttpState, build_open_router, build_app_router, build_backend_router, build_combined_router}`
  optional `http-axum` entrypoints for open-api/app-api/backend-api route integration.

## SDK Clients

- Generated SDK clients are owned by the repository root `sdks/` workspace.
- This component defines open-api/app-api/backend-api operation contracts for SDK
  generation.

## Verification

- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml`
- `node sdks/materialize-agent-v3-openapi-boundaries.mjs`
- `node sdks/workspace-agent-sdkgen.mjs --mode dry-run`
- `node scripts/check-agent-sdk-workspace.mjs`
- `powershell -ExecutionPolicy Bypass -File sdkwork-agent-business/scripts/verify-ci.ps1`
