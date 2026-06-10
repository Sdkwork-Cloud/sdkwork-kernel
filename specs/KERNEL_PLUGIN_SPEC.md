# SDKWork Kernel Plugin Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: kernel plugin identity, manifests, contribution points, provider and
  adapter loading, lifecycle, configuration, security, dependency declaration,
  conformance, distribution, and canonical plugin naming
- Domain: `intelligence`
- Capability: `kernel.plugin`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_INSTALLATION_CONFIGURATION_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_CONFORMANCE_SPEC.md`
  - `CODE_KERNEL_SPEC.md`

Plugins are the SDKWork Kernel extension unit. They package declarative
metadata, provider contributions, protocol adapters, optional agent definitions,
configuration contracts, lifecycle hooks, dependency declarations, diagnostics,
and conformance evidence.

External frameworks, services, protocols, runtimes, and products enter SDKWork
Kernel through plugin provider and adapter contributions.

## 1. Terminology

| Term | Meaning |
| --- | --- |
| Plugin | Installable, configurable, discoverable, and verifiable kernel extension package |
| Provider | Typed runtime capability implementation contributed by a plugin |
| Adapter | Protocol, process, transport, or client boundary contributed by a plugin |
| Contribution | A manifest-declared provider, adapter, agent, package, configuration, or conformance item |
| Host | Application or supervisor that loads plugins and kernel runtimes |

Rules:

- `plugin` is the canonical standard and public architecture name.
- New public standards, schemas, packages, and examples `SHOULD` use plugin
  naming.
- SDKWork-authored packages `MUST` use plugin naming directly.

## 2. Plugin Responsibilities

A plugin may contribute:

- Agent manifests, agent definitions, and agent package manifests.
- Provider manifests and typed provider implementations.
- Protocol adapter manifests and typed adapter implementations.
- Installation and configuration providers.
- Configuration specs, profiles, and migration plans.
- Dependency declarations on SDKWork foundations such as Drive and
  Knowledgebase.
- Runtime diagnostics, health checks, and conformance profile declarations.

Plugins do not own:

- Kernel core object models.
- Runtime state machines.
- Security policy decisions.
- Product UI behavior.
- Generated SDK ownership.
- Direct bypasses around provider SPI, protocol adapters, host providers, or
  policy gates.

## 3. Standard Ids

Required id families:

- Plugin ids: `plugin.<domain>.<name>`.
- Provider ids: `provider.<family>.<name>`.
- Adapter ids: `adapter.<protocol>.<name>`.
- Binding ids: `binding.<scope>.<name>`.
- Deployment ids: `deployment.<scope>.<name>`.
- Profile ids: `profile.<scope>.<name>`.
- Capability ids: lowercase namespaced ids with at least one dot.

Rules:

- Ids `MUST` use lowercase ASCII letters, numbers, `.`, `_`, and `-`.
- Ids `MUST` have non-empty dot-delimited segments.
- Provider and adapter ids contributed by one plugin `MUST` be unique.
- Capability ids contributed by one manifest `MUST` be unique.
- Product-specific capability ids `MUST` be namespaced.

## 4. Plugin Manifest

The machine-readable manifest is
[`schemas/kernel-plugin-manifest.schema.json`](./schemas/kernel-plugin-manifest.schema.json).

Required fields:

- `schema_version`
- `manifest_type`
- `plugin_id`
- `display_name`
- `version`
- `implementation_kind`
- `kernel_compatibility`
- `contributions`
- `security_profile`
- `supported_profiles`
- `owner`
- `status`

Rules:

- `manifest_type` `MUST` be `kernel_plugin`.
- Plugin manifests `MUST NOT` contain raw secrets.
- `kernel_compatibility` `MUST` declare the compatible kernel standard range.
- `implementation_kind` `MUST` identify the loading mode.
- `contributions` `MUST` declare provider ids, adapter ids, and optional agent
  or package ids without requiring local executable code.
- Security profile mismatch `MUST` fail closed.
- Unknown optional extensions `MAY` be ignored when namespaced.

Standard implementation kinds:

- `manifest-only`
- `typed-local-provider`
- `protocol-adapter`
- `process-adapter`
- `external-plugin`
- `official-foundation-plugin`

## 5. Contribution Model

Standard contribution families:

| Family | Examples |
| --- | --- |
| `agent` | AgentManifest, AgentDefinition, AgentPackageManifest |
| `provider` | model, tool, memory, knowledge, context, planning, policy, host, telemetry |
| `adapter` | MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, kernel UI client |
| `lifecycle` | install, uninstall, upgrade, configure |
| `configuration` | specs, profiles, migration plans, secret bindings |
| `conformance` | supported profiles and report claims |

Rules:

- Provider contributions `MUST` have `ProviderManifest` records.
- Adapter contributions `MUST` use `provider_family: protocol_adapter` or a
  protocol-specific provider SPI such as MCP when defined.
- Contribution manifests are valid without local typed SPI implementation.
- Typed local execution requires explicit runtime builder registration.
- Manifest-only execution attempts `MUST` return `provider_unavailable`.

## 6. Dependency Declaration

Plugins may depend on SDKWork foundations, generated SDKs, or external
protocols.

Rules:

- Kernel core crates `MUST NOT` depend on optional product foundations such as
  Drive or Knowledgebase.
- Official foundation plugins `MAY` depend on Drive, Knowledgebase, generated
  SDKs, or approved contracts.
- Dependency direction for Knowledgebase is:

```text
sdkwork-agent-kernel
  defines KnowledgeProvider SPI

sdkwork-kernel plugin layer
  depends on sdkwork-agent-kernel
  depends on sdkwork-knowledgebase contracts or SDKs where needed

sdkwork-knowledgebase
  owns knowledgebase storage, retrieval, indexing, ingest, and Drive composition
```

- A plugin that needs Drive files `MUST` use Drive contracts, SDKs, uploaders,
  or host providers instead of raw filesystem or raw HTTP bypasses.
- Dependency versions `MUST` be declared in plugin manifests or package
  manifests when they affect compatibility.

## 7. Runtime Loading

Standard loading flow:

```text
load kernel config
  -> load agent manifest
  -> load plugin manifests
  -> validate plugin manifests
  -> validate plugin compatibility and security profile
  -> load provider and adapter manifests
  -> register typed local implementations when available
  -> negotiate capabilities
  -> publish diagnostics and conformance evidence
```

Rules:

- Loading `MUST` be deterministic for the same inputs.
- Required plugin capabilities `MUST` fail closed when unavailable.
- Optional plugin capabilities `MUST` degrade the runtime when unavailable.
- Plugins `MUST NOT` perform side-effectful work during manifest validation.
- Provider registration and configuration `MUST` be policy-checkable.
- Runtime diagnostics `MUST` distinguish manifest-only contributions from typed
  local contributions.

## 8. Lifecycle And Configuration

Plugins may be installable packages or built-in host extensions.

Rules:

- Installable plugins `MUST` support plan-before-mutate installation.
- Installation, upgrade, uninstall, and configuration mutation `MUST` declare
  policy categories and side-effect levels.
- Secret-bearing configuration `MUST` use secret references.
- Configuration schemas `MUST` be typed and redaction-aware.
- Upgrade plans `SHOULD` declare rollback requirements when state is mutated.
- Uninstall flows `MUST` distinguish package removal from configuration and
  data removal.

## 9. Security

Required security declarations:

- Required policy categories.
- Side-effect levels.
- Secret handling rules.
- Host operation classes.
- Network and protocol exposure.
- Trust level.
- Audit requirements.
- Sandbox requirements.

Rules:

- Protected actions `MUST` fail closed when policy cannot be evaluated.
- Plugin install, configure, register, upgrade, and uninstall `MUST` be
  auditable.
- Side-effectful providers and adapters `MUST` pass through policy before
  execution.
- Retrieved content, external protocol payloads, tool output, and agent-to-agent
  messages `MUST` be treated as untrusted unless provenance says otherwise.
- Raw secrets `MUST NOT` appear in plugin manifests, provider manifests,
  package manifests, configuration profiles, logs, events, or conformance
  reports.

## 10. Conformance

Standard plugin profiles:

- `plugin-manifest`
- `plugin-local`
- `plugin-lifecycle`
- `plugin-security`
- `plugin-distribution`
- Existing provider and adapter profiles from `AGENT_CONFORMANCE_SPEC.md`.

Required cases:

- Plugin manifest validates.
- Plugin ids, provider ids, adapter ids, and capability ids validate.
- Provider and adapter contributions are unique.
- Supported conformance profiles are declared.
- Required kernel compatibility is enforced.
- Raw secrets are rejected.
- Security profile mismatch fails closed.
- Manifest-only contributions negotiate but return `provider_unavailable` for
  local execution.
- Typed local contributions can be registered and selected by id.
- Optional missing capabilities degrade instead of failing.

## 11. Package Naming

Canonical package names:

- `sdkwork-kernel-plugin-core`
- `sdkwork-agent-plugin-core`
- `sdkwork-agent-plugin-rig`
- `sdkwork-kernel-plugin-drive`
- `sdkwork-kernel-plugin-knowledgebase`
- `sdkwork-kernel-plugin-mcp`
- `sdkwork-kernel-plugin-a2a`

Rules:

- New SDKWork-authored packages `MUST` use plugin naming.
- SDKWork kernel plugin packages `MUST` use plugin naming at the package,
  crate, module, public type, and manifest levels.
- Physical crate and directory names `MUST NOT` use legacy extension package
  prefixes.
- SDKWork-authored public API types `MUST` use plugin naming directly.

## 12. First Official Plugins

Recommended official plugin targets:

- `plugin.sdkwork.drive`
- `plugin.sdkwork.knowledgebase`
- `plugin.intelligence.rig`
- `plugin.protocol.mcp`
- `plugin.protocol.a2a`

Knowledgebase plugin rules:

- The plugin is optional.
- Agents without required knowledge capabilities must run without it.
- Missing optional knowledge capability degrades runtime.
- Missing required knowledge capability fails bootstrap.
- The provider exposes `knowledge.search`, `knowledge.read`, and
  `knowledge.list` through `KnowledgeProvider`.
- Knowledgebase retrieval remains provider-neutral and must not force vector
  store assumptions into kernel core.

## 13. Acceptance Checklist

- [ ] `plugin` is the canonical extension architecture name.
- [ ] Plugin manifests have a machine-readable schema.
- [ ] Plugins declare providers, adapters, dependencies, security profile,
      lifecycle, and conformance.
- [ ] Kernel core remains provider-neutral.
- [ ] Optional foundation plugins do not become core dependencies.
- [ ] Runtime loading distinguishes manifest-only and typed local execution.
- [ ] Protected plugin actions pass through policy and audit.
- [ ] SDKWork-authored plugin packages use plugin naming directly.
