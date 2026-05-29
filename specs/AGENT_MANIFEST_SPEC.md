# SDKWork Agent Manifest Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: agent manifest, agent card, capability manifest, provider manifest,
  discovery, capability negotiation, compatibility, and manifest conformance
- Domain: `intelligence`
- Capability: `agent-kernel`
- Related:
  - [`AGENT_KERNEL_SPEC.md`](./AGENT_KERNEL_SPEC.md)
  - [`sdkwork-agent-kernel/README.md`](../sdkwork-agent-kernel/README.md)
  - [`DOMAIN_SPEC.md`](../../../../specs/DOMAIN_SPEC.md)
  - [`MODULE_SPEC.md`](../../../../specs/MODULE_SPEC.md)
  - [`SDK_SPEC.md`](../../../../specs/SDK_SPEC.md)

This specification defines how SDKWork-compatible agents, runtimes, providers,
and protocol adapters declare identity, capabilities, compatibility, security
requirements, and public discovery metadata.

Manifest quality determines whether the agent ecosystem can be integrated
without reading implementation internals. A professional agent standard needs
machine-readable manifests that support validation, discovery, capability
negotiation, UI rendering, policy evaluation, and conformance testing.

The specification uses `MUST`, `SHOULD`, `MAY`, and `MUST NOT` with the same
meaning as `AGENT_KERNEL_SPEC.md`.

## 1. Manifest Family

The Agent Kernel uses four manifest types.

| Manifest | Audience | Purpose |
| --- | --- | --- |
| `AgentManifest` | Runtime, host, product integrator | Static package identity, ownership, required kernel compatibility, required providers, and security profile |
| `AgentCard` | Other agents, registries, external clients, humans | Public discovery profile and interoperability hints |
| `CapabilityManifest` | Runtime, UI, host, conformance suite | Runtime-negotiated capabilities after providers and adapters are registered |
| `ProviderManifest` | Runtime, policy provider, conformance suite | Provider identity, operations, configuration schema, security requirements, and health model |

Rules:

- Manifests `MUST` be machine-readable.
- Manifests `MUST` be schema-valid before runtime registration.
- Manifest fields `MUST` use stable snake_case names in serialized forms.
- Manifests `MUST` include a `schema_version`.
- Unknown optional fields `MUST` be ignored safely.
- Unknown required fields or unsupported `required_extensions` `MUST` fail
  validation.
- Secrets, access tokens, private keys, raw credentials, and local absolute
  secrets paths `MUST NOT` appear in manifests.
- Product-specific metadata `MUST` be namespaced under `extensions`.

## 2. Common Manifest Envelope

Every manifest uses a common envelope.

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | string | Semantic version of the manifest schema |
| `manifest_type` | enum | `agent`, `agent_card`, `capability`, or `provider` |
| `id` | string | Stable manifest id |
| `version` | string | Semantic version of the subject |
| `status` | enum | Lifecycle status |
| `owner` | object | Owning organization or team |
| `created_at` | timestamp | Manifest creation timestamp |
| `updated_at` | timestamp | Last manifest update timestamp |

Status values:

- `experimental`
- `candidate`
- `stable`
- `deprecated`
- `removed`

Rules:

- `id` `MUST` be stable and globally unique within the registry scope.
- `version` `MUST` use semantic versioning.
- Timestamps `MUST` be ISO 8601 UTC.
- `removed` manifests `MUST NOT` be accepted by new runtime registration.
- `deprecated` manifests `MUST` include deprecation metadata.

Recommended owner shape:

```yaml
owner:
  name: sdkwork-platform
  contact: sdkwork-platform@example.invalid
  url: https://example.invalid/sdkwork-platform
```

## 3. Naming Rules

Canonical id formats:

| Subject | Format | Example |
| --- | --- | --- |
| Agent | `agent.<domain>.<name>` | `agent.intelligence.research` |
| Agent card | `agent_card.<domain>.<name>` | `agent_card.intelligence.research` |
| Capability | `<family>.<capability>` | `model.chat`, `tool.invoke`, `memory.query` |
| Provider | `provider.<family>.<name>` | `provider.model.openai`, `provider.tool.mcp` |
| Protocol adapter | `adapter.<protocol>.<name>` | `adapter.mcp.default`, `adapter.a2a.public` |
| Extension | reverse-domain or `sdkwork.*` | `sdkwork.code.workspace`, `com.example.case` |

Rules:

- Ids `MUST` use lowercase ASCII letters, numbers, dots, and hyphens.
- Ids `MUST NOT` contain spaces, tenant names, user names, credentials, or local
  filesystem paths.
- SDKWork-owned extension ids `MUST` use the `sdkwork.` prefix.
- Third-party extension ids `SHOULD` use reverse-domain ownership.
- A provider id `MUST` remain stable even when the provider implementation is
  refactored.

## 4. `AgentManifest`

`AgentManifest` declares the static identity and requirements of an agent
package.

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | string | Manifest schema version |
| `manifest_type` | string | Must be `agent` |
| `agent_id` | string | Stable agent id |
| `name` | string | Machine-friendly name |
| `display_name` | string | Human-readable name |
| `description` | string | Safe human-readable summary |
| `version` | string | Agent semantic version |
| `domain` | string | SDKWork domain, normally `intelligence` |
| `kernel_compatibility` | object | Supported Agent Kernel spec versions |
| `required_capabilities` | array | Capabilities required to start |
| `optional_capabilities` | array | Capabilities used when available |
| `provider_requirements` | array | Required provider families and constraints |
| `protocol_adapters` | array | Supported protocol adapters |
| `security_profile` | object | Required policy and sandbox assumptions |
| `runtime_profile` | object | Supported host/runtime modes |
| `event_families` | array | Event families emitted by the agent |
| `owner` | object | Owner metadata |
| `status` | enum | Lifecycle status |

Recommended fields:

- `tags`
- `homepage`
- `documentation`
- `license`
- `deprecation`
- `extensions`

Rules:

- `agent_id` `MUST` use the `agent.<domain>.<name>` format.
- `kernel_compatibility` `MUST` include supported spec version ranges.
- `required_capabilities` `MUST` be enough to decide whether the agent can
  start.
- Capability requirements `MUST` preserve `capability_id` and `min_version`;
  runtime negotiation `MUST` select a provider whose declared capability and
  provider version satisfy that minimum.
- A required capability whose `min_version` is not satisfied by any provider
  `MUST` be reported as missing and fail runtime readiness.
- An optional capability whose `min_version` is not satisfied by any provider
  `MUST` be reported as degraded, not silently bound to a lower-version
  provider.
- Optional capabilities `MUST NOT` be required for initialization.
- Runtime mode support `MUST` distinguish local, private, SaaS, desktop, CLI,
  server, and embedded host modes when relevant.
- Security profile `MUST` identify policy categories required by the agent.
- Manifests `MUST NOT` include product-specific UI layout or route metadata
  outside namespaced extensions.

Example:

```yaml
schema_version: 0.1.0
manifest_type: agent
agent_id: agent.intelligence.general
name: sdkwork-general-agent
display_name: SDKWork General Agent
description: Provider-neutral agent runtime for planning, tool use, memory, and policy-controlled execution.
version: 0.1.0
domain: intelligence
kernel_compatibility:
  agent_kernel: ">=0.1.0 <0.2.0"
required_capabilities:
  - capability_id: model.chat
    min_version: 0.1.0
  - capability_id: tool.invoke
    min_version: 0.1.0
  - capability_id: policy.evaluate
    min_version: 0.1.0
optional_capabilities:
  - capability_id: memory.query
    min_version: 0.1.0
  - capability_id: telemetry.trace
    min_version: 0.1.0
provider_requirements:
  - family: model
    required: true
    capabilities:
      - chat
      - streaming
  - family: policy
    required: true
    capabilities:
      - evaluate
      - record_decision
protocol_adapters:
  - adapter_id: adapter.mcp.default
    required: false
  - adapter_id: adapter.a2a.public
    required: false
security_profile:
  fail_closed: true
  required_policy_categories:
    - model.invoke
    - tool.invoke
    - memory.write
  redaction_required: true
runtime_profile:
  modes:
    - local
    - private
    - saas
    - desktop
    - cli
event_families:
  - agent.session.*
  - agent.task.*
  - agent.run.*
  - agent.step.*
  - agent.tool.*
  - agent.policy.*
owner:
  name: sdkwork-platform
status: candidate
```

## 5. `AgentCard`

`AgentCard` is the public discovery profile. It is inspired by A2A-style agent
cards, but it remains a SDKWork contract and may be mapped to A2A by adapters.

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | string | Manifest schema version |
| `manifest_type` | string | Must be `agent_card` |
| `agent_id` | string | Linked agent id |
| `card_id` | string | Stable card id |
| `display_name` | string | Human-readable name |
| `description` | string | Public description |
| `version` | string | Card version |
| `capabilities` | array | Public capabilities |
| `task_types` | array | Supported task categories |
| `input_modes` | array | Supported input modes |
| `output_modes` | array | Supported output modes |
| `protocols` | array | Supported public protocols |
| `auth_requirements` | array | Public auth requirements |
| `status` | enum | Lifecycle status |

Rules:

- Agent cards `MUST` be safe to show to trusted external systems.
- Agent cards `MUST NOT` expose internal-only provider details.
- Private endpoints, tokens, tenant ids, local paths, and environment variable
  names containing secrets `MUST NOT` appear in agent cards.
- `protocols` `SHOULD` identify adapter id, protocol name, version, transport,
  and auth mode.
- `capabilities` `MUST` be public capability summaries, not full provider
  configuration.

Example:

```yaml
schema_version: 0.1.0
manifest_type: agent_card
agent_id: agent.intelligence.general
card_id: agent_card.intelligence.general
display_name: SDKWork General Agent
description: General SDKWork-compatible agent for planning and tool-assisted execution.
version: 0.1.0
capabilities:
  - planning
  - tool_use
  - streaming
  - policy_controlled_execution
task_types:
  - question_answering
  - workflow_execution
  - tool_orchestration
input_modes:
  - text
  - json
output_modes:
  - text
  - json
  - artifact
protocols:
  - protocol: a2a
    adapter_id: adapter.a2a.public
    version: 0.1.0
    transport: https
auth_requirements:
  - bearer_token
status: candidate
```

## 6. `ProviderManifest`

`ProviderManifest` declares a provider implementation.

Provider families:

- `model`
- `tool`
- `context`
- `memory`
- `planning`
- `policy`
- `telemetry`
- `host`
- `protocol_adapter`
- `mcp`
- `skill`
- `agent_installer`
- `agent_configuration`

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | string | Manifest schema version |
| `manifest_type` | string | Must be `provider` |
| `provider_id` | string | Stable provider id |
| `provider_family` | enum | Provider family |
| `name` | string | Machine-friendly name |
| `display_name` | string | Human-readable name |
| `version` | string | Provider version |
| `kernel_compatibility` | object | Supported Agent Kernel versions |
| `capabilities` | array | Supported provider capabilities |
| `operations` | array | Operation names exposed by the provider |
| `configuration_schema` | object | JSON-schema-compatible config schema or reference |
| `security_requirements` | object | Required policy and data controls |
| `health_model` | object | Health states and checks |
| `owner` | object | Owner metadata |
| `status` | enum | Lifecycle status |

Rules:

- `provider_id` `MUST` use `provider.<family>.<name>` format.
- Provider operation names `MUST` match the provider SPI operation names.
- Provider configuration schema `MUST NOT` contain actual secret values.
- Secret configuration fields `MUST` be represented as secret references or
  host secret keys, not raw values.
- Provider manifests `MUST` declare whether operations are read-only,
  side-effectful, or destructive.
- Providers `MUST` declare cancellation support per operation when relevant.
- Providers `MUST` declare timeout policy per operation or provider default.
- `agent_installer` providers `MUST` declare `agent.install`,
  `agent.uninstall`, and `agent.upgrade` capabilities when they implement the
  standard install lifecycle.
- `agent_configuration` providers `MUST` declare `agent.configure` when they
  implement the standard configuration SPI.
- `mcp` providers `MUST` declare one or more of `mcp.tools`,
  `mcp.resources`, or `mcp.prompts`.
- `skill` providers `MUST` declare `skill.discover` and `skill.invoke` when
  they implement the standard Agent Skill SPI.

Example:

```yaml
schema_version: 0.1.0
manifest_type: provider
provider_id: provider.tool.mcp
provider_family: tool
name: sdkwork-mcp-tool-provider
display_name: SDKWork MCP Tool Provider
version: 0.1.0
kernel_compatibility:
  agent_kernel: ">=0.1.0 <0.2.0"
capabilities:
  - tool.discovery
  - tool.invoke
  - tool.cancel
operations:
  - list_tools
  - describe_tool
  - authorize_tool_call
  - invoke_tool
  - cancel_tool_call
configuration_schema:
  type: object
  required:
    - server_ref
  properties:
    server_ref:
      type: string
      description: Host-managed MCP server reference.
security_requirements:
  fail_closed: true
  required_policy_categories:
    - tool.invoke
  secret_handling:
    raw_secrets_allowed: false
health_model:
  states:
    - available
    - degraded
    - unavailable
owner:
  name: sdkwork-platform
status: candidate
```

## 7. `CapabilityManifest`

`CapabilityManifest` is produced after runtime/provider negotiation. It is the
runtime truth for UI, hosts, conformance tests, and adapters.

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | string | Manifest schema version |
| `manifest_type` | string | Must be `capability` |
| `runtime_id` | string | Runtime instance or runtime profile id |
| `agent_id` | string | Agent id |
| `kernel_version` | string | Agent Kernel implementation/spec version |
| `providers` | array | Registered providers |
| `capabilities` | array | Effective capabilities |
| `missing_required_capabilities` | array | Missing capabilities that block ready state |
| `degraded_capabilities` | array | Optional or unhealthy capabilities |
| `protocol_adapters` | array | Enabled adapters |
| `security_profile` | object | Effective runtime security posture |
| `generated_at` | timestamp | Manifest generation timestamp |

Rules:

- Capability manifests `MUST` be generated by runtime negotiation, not manually
  authored as source of truth.
- Ready runtimes `MUST` have an empty `missing_required_capabilities`.
- Degraded runtimes `MUST` identify optional capabilities that are missing or
  unhealthy.
- UI clients `MUST` use capability manifests to decide which optional controls
  to render.
- Protocol adapters `MUST` use capability manifests to decide whether a
  requested operation can be exposed.

Example capability entry:

```yaml
capability_id: tool.invoke
version: 0.1.0
provider_id: provider.tool.mcp
status: available
required: true
side_effect_level: side_effectful
policy_categories:
  - tool.invoke
operations:
  - invoke_tool
```

## 8. Capability Negotiation

Negotiation determines whether an agent can run with the available providers.

Negotiation phases:

```text
load agent manifest
  -> validate manifest schema
  -> load provider manifests
  -> validate provider schemas
  -> match kernel compatibility
  -> match required provider families
  -> match required capabilities
  -> evaluate security profile
  -> build capability manifest
  -> enter ready, degraded, or failed runtime state
```

Rules:

- Required provider families `MUST` be available before runtime enters `ready`.
- Required capabilities `MUST` be available before runtime enters `ready`.
- Optional missing capabilities `SHOULD` move runtime to `degraded`, not
  `failed`.
- Kernel version incompatibility `MUST` fail validation unless a documented
  compatibility shim is enabled.
- Security profile mismatch `MUST` fail closed.
- Negotiation results `MUST` emit `agent.manifest.*`,
  `agent.provider.*`, and `agent.runtime.*` events.

Failure examples:

| Failure | Runtime result |
| --- | --- |
| Missing required model provider | `failed` |
| Missing optional memory provider | `degraded` |
| Provider supports wrong kernel version | `failed` |
| Policy provider unavailable for side-effectful tools | `failed` |
| Telemetry exporter unavailable but audit sink available | `degraded` |

## 9. Security Profile

Manifests declare the security posture required by agents and providers.

Required security profile fields:

- `fail_closed`
- `required_policy_categories`
- `redaction_required`
- `audit_required`
- `secret_handling`
- `untrusted_context_handling`
- `sandbox_requirements`

Rules:

- `fail_closed` `MUST` be true when tools, memory writes, host operations, or
  protocol sends can have side effects.
- Side-effectful providers `MUST` require policy evaluation.
- Providers handling secrets `MUST` use host-managed secret references.
- Manifests `MUST` identify whether untrusted context can enter prompts,
  memory, tool input, or protocol messages.
- Audit-required operations `MUST` declare audit categories.

Example:

```yaml
security_profile:
  fail_closed: true
  redaction_required: true
  audit_required: true
  required_policy_categories:
    - model.invoke
    - tool.invoke
    - memory.write
  secret_handling:
    raw_secrets_allowed: false
    secret_refs_required: true
  untrusted_context_handling:
    mark_untrusted: true
    allow_untrusted_tool_output_to_model: policy_required
  sandbox_requirements:
    filesystem: policy_required
    process: policy_required
    network: policy_required
```

## 10. Protocol Adapter Metadata

Protocol adapter metadata describes how external protocols are exposed.

Supported protocol values:

- `mcp`
- `a2a`
- `http`
- `rpc`
- `ipc`
- `tauri`
- `websocket`
- `kernel-ui-client`

Required adapter fields:

- `adapter_id`
- `protocol`
- `version`
- `transport`
- `auth_mode`
- `exposed_capabilities`
- `required`
- `status`

Rules:

- Adapter metadata `MUST` not expose private credentials.
- Adapter exposed capabilities `MUST` be a subset of effective runtime
  capabilities.
- A2A metadata `SHOULD` map to public `AgentCard` fields.
- MCP metadata `SHOULD` map to provider/tool/resource/prompt exposure.
- UI client adapters `MUST` expose event stream support and permission response
  support when available.

## 11. Schema Validation

Manifest validation must be deterministic.

Rules:

- Manifest schemas `MUST` be expressible with JSON Schema Draft 2020-12 or an
  equivalent generated validation model.
- Validation `MUST` reject unknown `manifest_type`.
- Validation `MUST` reject missing required fields.
- Validation `MUST` reject invalid semantic versions.
- Validation `MUST` reject unsupported compatibility ranges.
- Validation `MUST` reject raw secret values in known secret fields.
- Validation `SHOULD` warn about unknown optional fields.
- Validation `MUST` be available in conformance tests.

Recommended schema file layout:

```text
kernel/specs/schemas/
|-- agent-manifest.schema.json
|-- agent-card.schema.json
|-- provider-manifest.schema.json
`-- capability-manifest.schema.json
```

Schema artifacts:

- [`schemas/agent-manifest.schema.json`](./schemas/agent-manifest.schema.json)
- [`schemas/agent-card.schema.json`](./schemas/agent-card.schema.json)
- [`schemas/provider-manifest.schema.json`](./schemas/provider-manifest.schema.json)
- [`schemas/capability-manifest.schema.json`](./schemas/capability-manifest.schema.json)

## 12. Registry And Distribution

Manifests may be distributed in local files, package metadata, registries, or
runtime APIs.

Rules:

- Source-controlled manifests `SHOULD` live next to the provider or agent
  package they describe.
- Runtime-generated capability manifests `MUST` identify generation time and
  source providers.
- Registries `MUST` preserve manifest version history.
- Registries `MUST` expose deprecation and removal metadata.
- Registry consumers `MUST` validate manifests locally before trusting them.
- Remote manifests `SHOULD` be integrity-checked when used for production
  registration.

## 13. Compatibility

Compatibility is evaluated across four axes:

- Agent Kernel spec version.
- Manifest schema version.
- Provider SPI version.
- Protocol adapter version.

Rules:

- Agents `MUST` declare compatible Agent Kernel spec ranges.
- Providers `MUST` declare compatible Agent Kernel and provider SPI ranges.
- Adapters `MUST` declare compatible external protocol versions.
- Runtime negotiation `MUST` reject incompatible major versions once a spec
  reaches `1.0`.
- Pre-`1.0` compatibility `SHOULD` be strict.
- Compatibility shims `MUST` be explicit and auditable.

Compatibility result values:

- `compatible`
- `compatible_with_warning`
- `requires_shim`
- `incompatible`

## 14. Deprecation

Deprecation metadata is required for deprecated manifests, capabilities, fields,
providers, and adapters.

Required deprecation fields:

- `deprecated_since`
- `replacement`
- `removal_not_before`
- `migration_notes`

Rules:

- Deprecated required capabilities `MUST` name replacements.
- Deprecated providers `MUST` identify replacement providers when available.
- Deprecated fields `SHOULD` remain readable until the declared removal version.
- Removed manifests `MUST NOT` be selected by default negotiation.

## 15. Conformance

Manifest conformance makes ecosystem integration testable.

Required test groups:

- Agent manifest schema validation.
- Agent card schema validation.
- Provider manifest schema validation.
- Capability manifest schema validation.
- Semantic version validation.
- Compatibility range validation.
- Required capability negotiation.
- Optional capability degradation.
- Missing provider fail-closed behavior.
- Security profile fail-closed behavior.
- Raw secret rejection.
- Deprecation metadata validation.
- Protocol adapter exposure subset validation.

Minimum conformance cases:

| Case | Expected result |
| --- | --- |
| Valid agent manifest and providers | Runtime can enter `ready` |
| Missing required model provider | Runtime enters `failed` |
| Missing optional memory provider | Runtime enters `degraded` |
| Raw secret in provider manifest | Manifest validation fails |
| Adapter exposes unsupported capability | Manifest validation fails |
| Deprecated provider with replacement | Validation passes with warning |
| Unsupported kernel version range | Negotiation fails |
| Side-effectful tool without policy provider | Runtime enters `failed` |

## 16. Documentation Requirements

Every agent or provider package must document:

- Manifest file path.
- Manifest type and schema version.
- Public capabilities.
- Required providers.
- Optional providers.
- Security profile.
- Protocol adapters.
- Compatibility range.
- Conformance command.
- Owner and status.

## 17. Acceptance Checklist

- [ ] Manifest family includes `AgentManifest`, `AgentCard`,
      `CapabilityManifest`, and `ProviderManifest`.
- [ ] Manifest ids and provider ids use stable naming rules.
- [ ] Required and optional capabilities are distinct.
- [ ] Capability negotiation has ready/degraded/failed outcomes.
- [ ] Security profile can fail closed.
- [ ] Agent cards are safe public discovery documents.
- [ ] Provider manifests declare operations, configuration schema, health, and
      side-effect/security requirements.
- [ ] Runtime-generated capability manifests are the source of truth for UI and
      adapters.
- [ ] Raw secrets are forbidden in manifests.
- [ ] Conformance tests can validate manifests and negotiation behavior.
