> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **superseded**

# Multi-Mode Agent System — Design (Superseded)

The original design draft for bridge providers, hybrid client modes, and kernel integration has been **superseded** by the as-built architecture:

- [TECH-2026-06-14-multi-mode-agent-system.md](TECH-2026-06-14-multi-mode-agent-system.md) — current client/server integration
- `sdkwork-kernel-plugins/specs/mappings/openclaw.md`
- `sdkwork-kernel-plugins/specs/mappings/hermes-agent.md`
- `sdkwork-kernel-plugins/specs/mappings/codex.md`

## What landed

- `AgentBridgeProvider`, `AgentBridgePluginRegistry`, `AgentClient` (Remote / Local / Hybrid)
- Builtin bridge plugins for OpenClaw, Hermes, Codex (SDK-backed); ZeroCloud (fail-closed session store)
- `SdkModelBridgeRuntime` for SDK-backed local chat
- Kernel plugins: Rig, OpenClaw, Hermes, Codex with `SDKWORK_KERNEL_AGENT_PLUGIN`
- Internal-api runtime HTTP via `SseChatClient` (`INTERNAL_RUNTIME_MOUNT_PREFIX`)

## What did not land (intentionally deferred)

- Placeholder runtime stubs that returned `"runtime not implemented"`
- PyO3 ZeroCloud embedding from the draft
- gRPC server/client surfaces
- App packager / dynamic plugin loading from the Phase 5 draft

Do not implement from placeholder signatures in archived superpowers plans; follow mappings and adapter crates instead.
