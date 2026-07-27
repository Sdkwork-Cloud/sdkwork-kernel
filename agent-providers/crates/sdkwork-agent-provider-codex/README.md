# sdkwork-agent-provider-codex

Process-adapter plugin for the SDKWork agent kernel.

## Purpose

Maps the Codex app-server and CLI runtime contract to SDKWork kernel SPI types,
agent manifests, package manifests, provider manifests, and runtime bootstrap
entrypoints.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.codex`
- Canonical agent id: `agent.intelligence.codex`
- Runtime entrypoint: `CodexKernelPlugin::configure_runtime`
- Public manifests: `codex_agent_definition`, `codex_agent_manifest`,
  `codex_provider_manifests`, `codex_package_manifest`, and
  `codex_kernel_plugin_manifest`

Direct in-process model and tool execution is intentionally fail-closed with
`ProviderUnavailable`. Production execution must route through the negotiated
SDK/runtime transport worker so the kernel can preserve policy, audit, trace,
and provider health semantics.

## Provider Session Activity

`CodexSdkIntegration::record_provider_session_activity` accepts a live Codex
app-server `ThreadStatus` observation. Active flags distinguish approval from
user-input waiting; idle and system-error statuses map explicitly. `NotLoaded`
and static SQLite/rollout discovery remain `Unsupported`.

The managed Node transport forwards official Codex SDK thread events and
incremental CLI JSONL events into the same activity store for operations run by
this integration. Independently running Codex processes remain `Unsupported`
unless a runtime host attaches an authoritative collector; file timestamps and
a historical `active` flag are never substituted.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml
```
