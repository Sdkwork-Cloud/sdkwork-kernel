# sdkwork-agent-provider-opencode

Process-adapter plugin for the SDKWork agent kernel.

## Purpose

Maps the OpenCode TypeScript SDK runtime contract to SDKWork kernel SPI types,
agent manifests, package manifests, provider manifests, and runtime bootstrap
entrypoints.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.opencode`
- Canonical agent id: `agent.intelligence.opencode`
- Runtime entrypoint: `OpenCodeKernelPlugin::configure_runtime`
- Public manifests: `opencode_agent_definition`, `opencode_agent_manifest`,
  `opencode_provider_manifests`, `opencode_package_manifest`, and
  `opencode_kernel_plugin_manifest`

Direct in-process model and tool execution is intentionally fail-closed with
`ProviderUnavailable`. Production execution must route through the negotiated
SDK/runtime transport worker so the kernel can preserve policy, audit, trace,
and provider health semantics.

## Provider Session Activity

`OpenCodeSdkIntegration::record_provider_session_activity` accepts live
`session.status` events. Busy and retry map to working; idle maps to idle.
SQLite history remains `Unsupported`.

The managed Node transport verifies SDK sessions before invocation and forwards
incremental CLI JSONL events into the same activity store for operations run by
this integration. Independently running OpenCode processes remain `Unsupported`
unless a runtime host attaches an authoritative event subscriber; database
timestamps are not live evidence.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-opencode/Cargo.toml
```
