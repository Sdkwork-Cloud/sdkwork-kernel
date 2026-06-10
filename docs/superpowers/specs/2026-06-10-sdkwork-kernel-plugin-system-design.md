# SDKWork Kernel Plugin System Design

## Goal

Establish `plugin` as the canonical SDKWork Kernel extension boundary and make
SDKWork-authored kernel extension packages use plugin naming directly without
legacy public aliases.

## Architecture Direction

The kernel owns stable object models, runtime SPI, policy, diagnostics, and
conformance. Plugins are installable, configurable, discoverable, and
verifiable extension packages that contribute providers, protocol adapters,
agents, package manifests, configuration profiles, and conformance profiles.

External framework, service, and protocol adapters enter through plugin provider
or adapter contributions. They do not define a separate top-level package
family.

```text
sdkwork-kernel
  kernel core       -> SPI, runtime, policy, diagnostics, conformance
  plugins           -> extension packages and official plugin implementations
  providers         -> typed runtime capabilities contributed by plugins
  adapters          -> protocol/process boundaries contributed by plugins
```

## Naming Model

Canonical names:

- `KernelPluginManifest`
- `KernelProviderBinding`
- `KernelPluginDeploymentSnapshot`
- `KernelPluginConformanceProfile`
- `SdkworkKernelPlugin`
- `KERNEL_PLUGIN_SPEC.md`
- `kernel-plugin-manifest.schema.json`

SDKWork-authored packages, crate names, modules, public types, tests, docs, and
manifests use canonical `plugin` names. Legacy extension package and API names
are removed rather than preserved as wrappers because the code has not been
promoted to external consumers.

## Plugin Manifest

A plugin manifest describes an extension package before the runtime executes it.
It declares:

- Stable plugin identity: `plugin_id`, `display_name`, `version`.
- Implementation kind: `typed-local-provider`, `manifest-only`,
  `process-adapter`, `protocol-adapter`, or approved extension kinds.
- Source and package metadata.
- Optional agent/package contribution.
- Provider and adapter contribution ids.
- Required and optional kernel capability requirements.
- Supported conformance profiles.
- Security profile, policy categories, side-effect declarations, sandbox
  requirements, and trust level.
- Configuration and lifecycle provider requirements.

Plugin ids use `plugin.<domain>.<name>`. Provider ids use `provider.*`.
Adapter ids use `adapter.*`. Capabilities are lowercase namespaced ids.

## Contribution Model

Plugins can contribute these standard families:

- Agents: `AgentManifest`, `AgentDefinition`, `AgentPackageManifest`.
- Providers: model, tool, memory, knowledge, context, planning, policy, host,
  MCP, skill, collaboration, telemetry, installer, and configuration providers.
- Adapters: MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, and kernel UI client
  adapters.
- Configuration: configuration specs, profiles, migration plans, and secret
  binding requirements.
- Conformance: supported profiles and plugin-owned compatibility reports.

Contribution manifests are declarative. Local typed execution is optional and
registered through explicit runtime builder paths. Manifest-only plugins remain
valid for discovery and negotiation but return `provider_unavailable` when code
tries to invoke a missing local SPI instance.

## Runtime Flow

Standard bootstrap adds a plugin phase before provider registration:

```text
load kernel config
  -> load agent manifest
  -> load plugin manifests
  -> validate plugin manifests
  -> validate plugin compatibility and policy
  -> load provider/adapter manifests from plugins
  -> register typed local providers when available
  -> negotiate capabilities
  -> build runtime diagnostics and conformance evidence
```

Required plugin capabilities fail closed. Optional plugin capabilities degrade
the runtime. Plugin loading must be deterministic for the same configuration.

## Security And Policy

Plugins are supply-chain and runtime risk surfaces. The standard requires:

- Policy categories for provider registration, provider configuration,
  installation, protocol send, host access, knowledge access, memory mutation,
  and other protected actions.
- Side-effect classification for every contributed executable capability.
- Secret references instead of raw secrets in manifests or profiles.
- Audit records for plugin install, configure, register, upgrade, uninstall,
  and side-effectful execution.
- Untrusted context labeling for retrieved documents, tool output, external
  protocol payloads, and agent-to-agent messages.
- Fail-closed behavior when security profile or policy evaluation is missing.

## Drive And Knowledgebase Alignment

`sdkwork-drive` and `sdkwork-knowledgebase` are infrastructure foundations.
The agent runtime should not be owned by those foundations.

The corrected dependency shape is:

```text
sdkwork-agent-kernel
  defines KnowledgeProvider and HostProvider SPI

sdkwork-kernel plugin layer
  depends on sdkwork-agent-kernel
  depends on sdkwork-drive and sdkwork-knowledgebase contracts/SDKs where needed
  contributes official Drive and Knowledgebase plugins

sdkwork-knowledgebase
  keeps knowledgebase contracts, retrieval, indexing, ingest, and Drive
  composition
```

The existing `sdkwork-knowledgebase-agent-provider` should migrate into the
kernel plugin layer as an official optional knowledge plugin, not into
`sdkwork-agent-kernel` core.

The Drive foundation enters through an official optional storage plugin. The
plugin contributes provider manifests and typed wrappers over Drive object-store
contracts while keeping agent kernel core independent of Drive implementation
crates.

## Migration Strategy

The first phase is canonical-only:

1. Introduce plugin standard docs and schema.
2. Add canonical plugin API names in the existing core crate.
3. Rename old extension directories, packages, tests, docs, and public APIs to
   plugin naming without preserving public wrappers.
4. Update Rig to expose canonical plugin names only.
5. Add standards checks and contract tests.
6. Introduce official Drive and Knowledgebase plugin crates.
7. Migrate `sdkwork-knowledgebase-agent-provider` into the plugin layer.
8. Verify stale extension naming scans and remove ownership of old provider
   packages from their previous repositories when write permissions allow it.

## Testing And Conformance

Minimum first-phase verification:

- Rust contract tests prove canonical plugin names validate ids and assemble
  provider/plugin manifests.
- Standards check requires `KERNEL_PLUGIN_SPEC.md` and
  `kernel-plugin-manifest.schema.json`.
- Schema validates manifest identity, implementation kind, provider/adapter ids,
  capability ids, conformance profiles, and security profile fields.
- Rig plugin tests prove canonical plugin behavior directly.
- Drive and Knowledgebase plugin tests prove both foundations are optional and
  do not require an agent manifest.

## Open Design Decision

The recommended v1 boundary is manifest-first with phased implementation:

- Standard supports typed local providers and external protocol/process
  plugins.
- First implementation focuses on typed Rust providers and manifest-only
  negotiation.
- External process isolation, registry signing, hot reload, and marketplace
  distribution are later phases.
