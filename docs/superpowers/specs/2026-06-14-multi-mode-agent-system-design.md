# Multi-Mode Agent System Design — Superseded

> **Status:** Superseded on 2026-06-24. Do not implement from this file.

The historical design draft (bridge provider trait signatures, PyO3 ZeroClaw embedding, gRPC server phases, dynamic `libloading` plugins) is retired.

## Authoritative sources

- As-built architecture: [docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md](../../architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md)
- Superseded design notice: [docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system-design.md](../../architecture/tech/TECH-2026-06-14-multi-mode-agent-system-design.md)
- Upstream mappings: `sdkwork-kernel-plugins/specs/mappings/*.md`
- Server plugin selection: `SDKWORK_KERNEL_AGENT_PLUGIN` in `configs/topology/*.env` and `sdkwork-agent-server/src/runtime_bootstrap.rs`

## What landed

- `AgentBridgeProvider`, `AgentBridgePluginRegistry`, `AgentClient` (Remote / Local / Hybrid)
- SDK-backed local bridges: OpenClaw, Hermes, Codex via `SdkModelBridgeRuntime`
- Kernel plugins: Rig, OpenClaw, Hermes, Codex
- ZeroClaw: client registry entry only; fail-closed for chat until upstream adapter exists
- Internal-api runtime HTTP via `SseChatClient` on `/internal/v3/api/intelligence/runtime`

## Intentionally deferred

- gRPC client/server surfaces
- Dynamic bridge plugin loading
- Kernel `BridgeProviderAdapter` from the original draft
- ZeroClaw upstream adapter and kernel plugin
