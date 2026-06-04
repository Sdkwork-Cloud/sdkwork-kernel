# SDKWork Agent Business Database Specification

- Version: `0.1.0`
- Domain: `intelligence`
- Capability: `agent-business`
- Scope: Agent metadata CRUD, provider implementation bindings, deployment
  snapshots, status lifecycle, authorization boundary linkage, audit facts, and
  API contract persistence support.

This document defines the `sdkwork-agent-business` database contract baseline.
It follows:

- `specs/DATABASE_SPEC.md`
- `specs/API_SPEC.md`
- `specs/SECURITY_SPEC.md`
- `specs/IAM_SPEC.md`

## 1. Design Principles

1. New business tables use canonical module prefixes and tenant-first indexing.
2. Internal identifiers are `int64`; API serialization is string.
3. `decimal` fields are serialized as strings in APIs.
4. No plaintext secrets in business tables.
5. Soft-delete plus audit trace for management operations.
6. Tenant and organization boundaries are explicit columns, not implicit filters.

## 2. Table Overview

| Table | Profile | Compliance | Responsibility |
| --- | --- | --- | --- |
| `ai_agent_business` | `tenant_entity` | L2 | Agent management record (manifest summary, lifecycle, visibility, ownership). |
| `ai_agent_provider_binding` | `tenant_entity` | L2 | Provider implementation bindings for a managed agent, including active/default selection. |
| `ai_agent_deployment` | `tenant_entity` | L2 | Deployable agent instance records with provider and configuration snapshots. |
| `ai_agent_business_audit_event` | `audit_log` | L3 | Immutable management action audit facts for create/update/delete/restore/status-change. |

## 3. Table Contracts

### 3.1 `ai_agent_business`

Primary business table for managed agents.

| Column | Type | Null | Description |
| --- | --- | --- | --- |
| `id` | `BIGINT` | N | Internal row id. |
| `uuid` | `VARCHAR(64)` | N | External stable id. |
| `tenant_id` | `BIGINT` | N | Tenant isolation key. |
| `organization_id` | `BIGINT` | N | Organization isolation key, `0` for tenant-level. |
| `owner_user_id` | `BIGINT` | N | Owner user identity id. |
| `agent_id` | `VARCHAR(128)` | N | Stable agent contract id (`AgentManifest.agent_id`). |
| `code` | `VARCHAR(128)` | N | Human-readable stable code within tenant scope. |
| `display_name` | `VARCHAR(255)` | N | Display name. |
| `description` | `TEXT` | Y | Optional business description. |
| `manifest_json` | `TEXT` | N | Canonical agent manifest JSON snapshot. |
| `default_code_task_intent_json` | `TEXT` | Y | Optional default code-task intent snapshot. |
| `implementation_provider_id` | `VARCHAR(128)` | Y | Default implementation provider id selected at agent creation, for example `provider.model.rig-rust`. |
| `implementation_kind` | `VARCHAR(64)` | Y | Implementation kind (`manifest-only`, `typed-local-provider`, `process-adapter`, `protocol-adapter`). |
| `status` | `SMALLINT` | N | Lifecycle status enum (`0 draft`, `1 active`, `2 disabled`, `3 archived`, `4 deleted`). |
| `visibility` | `SMALLINT` | N | Visibility enum (`0 private`, `1 organization`, `2 tenant`, `3 public`). |
| `tags_json` | `TEXT` | N | JSON array of tags. |
| `created_at` | `TIMESTAMP` | N | Creation timestamp (UTC). |
| `updated_at` | `TIMESTAMP` | N | Last update timestamp (UTC). |
| `deleted_at` | `TIMESTAMP` | Y | Soft-delete timestamp (UTC). |
| `version` | `BIGINT` | N | Optimistic concurrency version. |

Constraints:

- PK: `id`
- Unique: `(tenant_id, agent_id)`
- Unique: `(tenant_id, code)`
- Check: `implementation_kind is null or in ('manifest-only','typed-local-provider','process-adapter','protocol-adapter')`
- Check: `implementation_provider_id is null or matches ^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `status in (0,1,2,3,4)`
- Check: `visibility in (0,1,2,3)`

Indexes:

- `idx_ai_agent_business_tenant_org_status_updated` on
  `(tenant_id, organization_id, status, updated_at desc)`
- `idx_ai_agent_business_tenant_owner_status` on
  `(tenant_id, owner_user_id, status)`

### 3.2 `ai_agent_provider_binding`

Provider implementation binding table for managed agents.

| Column | Type | Null | Description |
| --- | --- | --- | --- |
| `id` | `BIGINT` | N | Internal row id. |
| `uuid` | `VARCHAR(96)` | N | External stable row id. |
| `tenant_id` | `BIGINT` | N | Tenant isolation key. |
| `agent_id` | `VARCHAR(128)` | N | Stable managed agent id. |
| `binding_id` | `VARCHAR(128)` | N | Stable binding id within an agent. |
| `provider_id` | `VARCHAR(128)` | N | Implementation provider id, for example `provider.model.rig-rust`. |
| `implementation_kind` | `VARCHAR(64)` | N | Implementation kind (`manifest-only`, `typed-local-provider`, `process-adapter`, `protocol-adapter`). |
| `configuration_profile_id` | `VARCHAR(128)` | N | Safe configuration profile id. Secret values are not stored here. |
| `capabilities_json` | `TEXT` | N | JSON array of capability ids exposed by the provider binding. |
| `active` | `BOOLEAN` | N | Whether this binding is the active/default binding for the agent. |
| `version` | `BIGINT` | N | Optimistic concurrency version. |
| `created_at` | `TIMESTAMP` | N | Creation timestamp (UTC). |
| `updated_at` | `TIMESTAMP` | N | Last update timestamp (UTC). |

Constraints:

- PK: `id`
- Unique: `uuid`
- Unique: `(tenant_id, agent_id, binding_id)`
- Partial unique: only one active binding per `(tenant_id, agent_id)`
- Check: `implementation_kind in ('manifest-only','typed-local-provider','process-adapter','protocol-adapter')`
- Check: `binding_id` matches `^binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `provider_id` matches `^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `configuration_profile_id` matches `^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `capabilities_json` is a JSON array of unique string capability ids;
  each id is at most 128 characters and matches
  `^[a-z0-9_-]+(\.[a-z0-9_-]+)+$`

Indexes:

- `uk_ai_agent_provider_binding_active_default` unique partial index on
  `(tenant_id, agent_id)` where `active = true`
- `idx_ai_agent_provider_binding_tenant_agent_updated` on
  `(tenant_id, agent_id, active desc, updated_at desc, binding_id asc)`
- `idx_ai_agent_provider_binding_tenant_provider` on
  `(tenant_id, provider_id)`

Rules:

- Updates must use `version` as an optimistic concurrency precondition.
- Activating one binding must deactivate the previous active binding in the same
  tenant and agent scope.
- `configuration_profile_id` identifies a profile; raw secrets are forbidden.
- `binding_id`, `provider_id`, `configuration_profile_id`, and
  `capabilities_json` are validated at service, row-mapping, OpenAPI, and
  PostgreSQL CHECK boundaries.
- Local in-memory repositories used by contract tests must enforce the same
  version and single-active-binding rules as PostgreSQL adapters.
- Provider binding list order is standard: `active desc`, `updated_at desc`,
  `binding_id asc`.

### 3.3 `ai_agent_deployment`

Deployable managed agent instance table.

| Column | Type | Null | Description |
| --- | --- | --- | --- |
| `id` | `BIGINT` | N | Internal row id. |
| `uuid` | `VARCHAR(96)` | N | External stable row id. |
| `tenant_id` | `BIGINT` | N | Tenant isolation key. |
| `agent_id` | `VARCHAR(128)` | N | Stable managed agent id. |
| `deployment_id` | `VARCHAR(128)` | N | Stable deployment id within an agent. |
| `binding_id` | `VARCHAR(128)` | N | Binding id used to create the deployment. |
| `provider_id_snapshot` | `VARCHAR(128)` | N | Provider id snapshot copied from the binding at deployment creation. |
| `implementation_kind_snapshot` | `VARCHAR(64)` | N | Implementation kind snapshot copied from the binding. |
| `configuration_profile_id_snapshot` | `VARCHAR(128)` | N | Configuration profile id snapshot copied from the binding. |
| `capabilities_snapshot_json` | `TEXT` | N | JSON array of capability ids copied from the binding. |
| `status` | `SMALLINT` | N | Deployment lifecycle status (`0 created`, `1 active`, `2 failed`, `3 archived`). |
| `version` | `BIGINT` | N | Optimistic concurrency version. |
| `created_at` | `TIMESTAMP` | N | Creation timestamp (UTC). |
| `updated_at` | `TIMESTAMP` | N | Last update timestamp (UTC). |

Constraints:

- PK: `id`
- Unique: `uuid`
- Unique: `(tenant_id, agent_id, deployment_id)`
- Check: `implementation_kind_snapshot in ('manifest-only','typed-local-provider','process-adapter','protocol-adapter')`
- Check: `deployment_id` matches `^deployment\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `binding_id` matches `^binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `provider_id_snapshot` matches `^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `configuration_profile_id_snapshot` matches `^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
- Check: `capabilities_snapshot_json` is a JSON array of unique string
  capability ids; each id is at most 128 characters and matches
  `^[a-z0-9_-]+(\.[a-z0-9_-]+)+$`
- Check: `status in (0,1,2,3)`

Indexes:

- `idx_ai_agent_deployment_tenant_agent_created` on
  `(tenant_id, agent_id, created_at desc, deployment_id asc)`
- `idx_ai_agent_deployment_tenant_provider_status` on
  `(tenant_id, provider_id_snapshot, status)`

Rules:

- Deployment rows preserve provider, implementation kind, configuration profile,
  and capability snapshots. Later binding changes must not rewrite existing
  deployment snapshots.
- Deployment list order is standard: `created_at desc`, `deployment_id asc`.
- `configuration_profile_id_snapshot` is an identifier only. Raw secrets are
  forbidden.
- Snapshot id and capability fields are validated at service, row-mapping,
  OpenAPI, and PostgreSQL CHECK boundaries.

### 3.4 `ai_agent_business_audit_event`

Immutable management audit facts.

| Column | Type | Null | Description |
| --- | --- | --- | --- |
| `id` | `BIGINT` | N | Internal row id. |
| `uuid` | `VARCHAR(64)` | N | External stable id. |
| `tenant_id` | `BIGINT` | N | Tenant isolation key. |
| `organization_id` | `BIGINT` | N | Organization key. |
| `agent_business_id` | `BIGINT` | N | FK-like reference to business record id (logical reference). |
| `agent_id` | `VARCHAR(128)` | N | Stable agent id snapshot. |
| `action` | `VARCHAR(64)` | N | Action code (`created`,`updated`,`deleted`,`restored`,`status_changed`,`provider_binding_changed`,`deployment_created`). |
| `subject_id` | `VARCHAR(128)` | N | Policy subject id. |
| `subject_tenant_id` | `VARCHAR(128)` | N | Policy subject tenant id. |
| `request_id` | `VARCHAR(128)` | Y | Request correlation id when available. |
| `trace_id` | `VARCHAR(128)` | Y | Trace correlation id when available. |
| `payload_json` | `TEXT` | N | Action payload snapshot with safe fields only. |
| `created_at` | `TIMESTAMP` | N | Audit event timestamp (UTC). |

Constraints:

- PK: `id`
- Check: `action in ('created','updated','deleted','restored','status_changed','provider_binding_changed','deployment_created')`

Indexes:

- `idx_ai_agent_business_audit_tenant_agent_created` on
  `(tenant_id, agent_id, created_at desc)`
- `idx_ai_agent_business_audit_tenant_action_created` on
  `(tenant_id, action, created_at desc)`

## 4. Security And Privacy

1. `manifest_json`, `default_code_task_intent_json`, `capabilities_json`,
   `capabilities_snapshot_json`, and `payload_json` must not include secret
   plaintext or token values.
2. Audit payload is redacted-safe by contract; only safe metadata is persisted.
3. Access must be enforced through tenant + organization + subject policy.

## 5. Migration Strategy

Expand-contract sequence:

1. Add new tables and indexes.
2. Wire repository adapters with read/write and policy checks.
3. Backfill any legacy data if required.
4. Enable strict API operations and SDK generation.
5. Add archival/retention jobs after operational baselines are stable.
