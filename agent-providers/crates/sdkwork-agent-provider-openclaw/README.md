# sdkwork-agent-provider-openclaw

Process-adapter plugin for the SDKWork agent kernel.

## Purpose

Maps the OpenClaw SDK and gateway runtime contract to SDKWork kernel SPI types,
agent manifests, package manifests, provider manifests, and runtime bootstrap
entrypoints.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.openclaw`
- Canonical agent id: `agent.openclaw`
- Runtime entrypoint: `OpenClawKernelPlugin::configure_runtime`
- Public manifests: `openclaw_agent_definition`, `openclaw_agent_manifest`,
  `openclaw_provider_manifests`, `openclaw_package_manifest`, and
  `openclaw_kernel_plugin_manifest`

Direct in-process model and tool execution is intentionally fail-closed with
`ProviderUnavailable`. Production execution must route through the negotiated
SDK/runtime transport worker so the kernel can preserve policy, audit, trace,
and provider health semantics.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-openclaw/Cargo.toml
```
