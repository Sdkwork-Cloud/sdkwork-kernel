# sdkwork-agent-provider-hermes

Process-adapter plugin for the SDKWork agent kernel.

## Purpose

Maps the Hermes Agent Python runtime and optional TUI gateway IPC contract to
SDKWork kernel SPI types, agent manifests, package manifests, provider
manifests, and runtime bootstrap entrypoints.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.hermes`
- Canonical agent id: `agent.intelligence.hermes`
- Runtime entrypoint: `HermesKernelPlugin::configure_runtime`
- Public manifests: `hermes_agent_definition`, `hermes_agent_manifest`,
  `hermes_provider_manifests`, `hermes_package_manifest`, and
  `hermes_kernel_plugin_manifest`

Direct in-process model and tool execution is intentionally fail-closed with
`ProviderUnavailable`. Production execution must route through the negotiated
SDK/runtime transport worker so the kernel can preserve policy, audit, trace,
and provider health semantics.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-hermes/Cargo.toml
```
