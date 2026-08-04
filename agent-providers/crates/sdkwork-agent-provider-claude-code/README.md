# sdkwork-agent-provider-claude-code

Process-adapter plugin for the SDKWork agent kernel.

## Purpose

Maps the Claude Code CLI and agent SDK runtime contract to SDKWork kernel SPI
types, agent manifests, package manifests, provider manifests, and runtime
bootstrap entrypoints.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.claude-code`
- Canonical agent id: `agent.claude-code`
- Runtime entrypoint: `ClaudeCodeKernelPlugin::configure_runtime`
- Public manifests: `claude_code_agent_definition`,
  `claude_code_agent_manifest`, `claude_code_provider_manifests`,
  `claude_code_package_manifest`, and `claude_code_kernel_plugin_manifest`

Direct in-process model and tool execution is intentionally fail-closed with
`ProviderUnavailable`. Production execution must route through the negotiated
SDK/runtime transport worker so the kernel can preserve policy, audit, trace,
and provider health semantics.

## Provider Session Activity

`ClaudeCodeSdkIntegration::record_provider_session_activity` accepts explicit
Claude Code hook events. Prompt/tool hooks map to working, permission requests
map to approval waiting, and stop/session-end map to idle. JSONL history remains
`Unsupported`.

The managed Node transport forwards official Claude Agent SDK events and
incremental CLI JSONL events into the same activity store for operations run by
this integration. Independently running Claude Code processes remain
`Unsupported` unless a runtime host attaches an authoritative hook collector;
JSONL history timestamps are not live evidence.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-claude-code/Cargo.toml
```
