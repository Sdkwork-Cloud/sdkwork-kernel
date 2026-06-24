> Migrated from `docs/superpowers/specs/2026-06-04-rig-complete-plugin-design.md` on 2026-06-24.
> Owner: SDKWork maintainers

## Goal

Build the first complete SDKWork external agent implementation through Rig, using a strict plugin boundary that proves the Agent Kernel standard can host third-party agent frameworks without coupling kernel core to external source trees.

Rig is the first implementation target because it is Rust-native. It must validate the full standard path: manifests, package lifecycle, typed provider registration, configuration, secret references, deployment snapshots, diagnostics, conformance evidence, and fail-closed runtime behavior.

## Non-Negotiable Rules

- `sdkwork-agent-kernel` and `sdkwork-code-kernel` must not depend on `external/` or any plugin crate.
- `external/rig` remains a reference source tree unless an explicit feature-gated live backend is enabled in the Rig plugin crate.
- Default Rig behavior must be deterministic and fail-closed for live model/tool execution when no live backend is configured.
- Plugin assembly belongs in `sdkwork-kernel-plugins`, not in kernel core.
- Provider ids, agent ids, capability ids, and event families must be stable, lowercase, and namespaced.
- Raw secrets must never appear in manifests, configuration profiles, events, diagnostics, or tests.
- Side-effectful operations must expose policy categories and must not silently execute without a policy boundary.
- Deployment records must preserve provider and binding snapshots so later provider switches do not mutate historical deployments.

## Architecture

The implementation uses four layers.

`sdkwork-agent-kernel` remains the standard core. It owns SPI traits, manifests, runtime registration, capability negotiation, policy hooks, event models, diagnostics, and conformance report types.

`sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core` defines SDKWork-owned plugin assembly contracts for external plugins. It introduces a small, typed `SdkworkKernelPlugin` interface plus manifest, profile, binding, and deployment snapshot helper types. It depends on `sdkwork-agent-kernel`, but kernel core does not depend on it.

`sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig` implements the first complete plugin. It exposes Rig agent/package manifests, installer and configuration providers, model/tool/planning providers, diagnostics, and conformance evidence. Its default backend is fail-closed. A future feature-gated live backend may map SDKWork requests to Rig upstream APIs.

`sdkwork-agent-business` tracks managed agent ownership, provider bindings, active binding selection, and deployments. It treats Rig as an implementation provider, not as special-case business logic.

## Directory Structure

```text
sdkwork-kernel-plugins/
|-- crates/
|   |-- sdkwork-agent-plugin-core/
|   |   |-- Cargo.toml
|   |   |-- README.md
|   |   |-- src/
|   |   |   `-- lib.rs
|   |   `-- tests/
|   |       `-- plugin_contracts.rs
|   `-- sdkwork-agent-plugin-rig/
|       |-- Cargo.toml
|       |-- README.md
|       |-- src/
|       |   |-- agent_definition.rs
|       |   |-- backend.rs
|       |   |-- configuration.rs
|       |   |-- conformance.rs
|       |   |-- deployment.rs
|       |   |-- diagnostics.rs
|       |   |-- ids.rs
|       |   |-- installer.rs
|       |   |-- lib.rs
|       |   |-- manifest.rs
|       |   |-- package.rs
|       |   `-- provider.rs
|       `-- tests/
|           |-- rig_configuration_contracts.rs
|           |-- rig_deployment_contracts.rs
|           |-- rig_installer_contracts.rs
|           |-- rig_manifest_contracts.rs
|           `-- rig_provider_contracts.rs
```

## Plugin Contract

`SdkworkKernelPlugin` is the stable assembly surface for SDKWork-owned external plugins.

It must expose:

- A plugin manifest with plugin id, implementation kind, version, source reference, supported profiles, and provider ids.
- The SDKWork `AgentManifest`.
- The SDKWork `AgentPackageManifest`.
- Provider manifests.
- A runtime assembly method that registers typed providers on `RuntimeBuilder`.
- A conformance profile that states which standard profiles are supported.

The trait intentionally assembles existing kernel SPI instead of introducing a competing runtime model.

## Rig Provider Surface

Rig plugin ids:

- Agent: `agent.intelligence.rig-general`
- Agent card: `agent_card.intelligence.rig-general`
- Plugin: `plugin.intelligence.rig`
- Model provider: `provider.model.rig-rust`
- Tool provider: `provider.tool.rig-rust`
- Planning provider: `provider.planning.rig-rust`
- Installer provider: `provider.agent.installer.rig-rust`
- Configuration provider: `provider.agent.configuration.rig-rust`

Required capabilities:

- `model.catalog`
- `model.chat`
- `policy.evaluate`
- `agent.install`
- `agent.configure`

Optional capabilities:

- `model.streaming`
- `model.tool_call`
- `tool.invoke`
- `planning.create`

The model provider must list at least one SDKWork model descriptor. Invocation must fail with a stable provider-unavailable error until a live backend is configured. Unknown model ids must fail with capability-missing behavior through the model catalog path.

The tool provider must expose typed descriptors and policy metadata. Tool output is untrusted by default. Side-effectful tools must require policy categories.

The planning provider must produce valid `Plan` values with at least one action. Any side-effectful action must include policy categories.

## Configuration

Rig configuration must include base, LLM API key, runtime, and security sections.

Required fields:

- `agent.display_name`
- `llm.rig.provider_id`
- `llm.rig.api_key`
- `runtime.rig.backend_mode`
- `security.fail_closed`

Rules:

- `llm.rig.api_key` must require a secret reference.
- Raw API keys must fail validation.
- `runtime.rig.backend_mode` defaults to `fail_closed`.
- Enabling a live backend must require both provider id and secret reference.

## Installer

Rig installer must support plan-before-mutate behavior.

Install plan steps:

- Verify package.
- Register agent.
- Configure agent.
- Start agent when requested by host policy.

Upgrade plans must preserve rollback intent. Uninstall requests must distinguish package removal from configuration/data removal.

## Business Binding And Deployment

Managed agent records must track:

- `implementation_provider_id`
- `implementation_kind`

Provider bindings must track:

- Binding id.
- Agent id and tenant id.
- Provider id.
- Implementation kind.
- Configuration profile id.
- Capability profile.
- Active/default flag.
- Version and timestamps.

Deployments must track:

- Deployment id.
- Agent id and tenant id.
- Binding id.
- Provider id snapshot.
- Implementation kind snapshot.
- Configuration profile id snapshot.
- Capability profile snapshot.
- Status.
- Version and timestamps.

Switching active binding must deactivate the previous default binding. Existing deployment snapshots must not change when a binding changes later.

## Conformance

Rig completion requires these profiles:

- `runtime-manifest`
- `runtime-local`
- `agent-installation`
- `provider-model`
- `provider-tool`
- `security-baseline` for fail-closed behavior

Verification commands:

```powershell
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml
cargo test --manifest-path sdkwork-agent-business/Cargo.toml
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
node scripts/check-kernel-standards.mjs
```

## Decision

Implement the complete Rig plugin path now. Keep other upstream plugins as mapping and manifest candidates until Rig proves the standard end to end.

