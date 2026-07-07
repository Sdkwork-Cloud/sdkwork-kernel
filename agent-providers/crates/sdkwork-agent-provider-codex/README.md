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

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml
```
