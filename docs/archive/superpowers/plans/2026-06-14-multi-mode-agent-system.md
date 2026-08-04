# Multi-Mode Agent System —Superseded Plan

> **Status:** Superseded. Do not execute this checklist for new work.

The original phased implementation plan is retired. The kernel ships the as-built multi-mode agent stack:

- **As-built architecture:** [docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md](../../../architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md)
- **Upstream mappings:** `sdkwork-kernel-plugins/specs/mappings/*.md`
- **Server plugin selection:** `SDKWORK_KERNEL_AGENT_PLUGIN` in `configs/topology/*.env` (`rig` | `openclaw` | `hermes` | `codex`)
- **Client local bridge:** `SdkModelBridgeRuntime` for OpenClaw, Hermes, and Codex; ZeroCloud remains fail-closed until an upstream adapter exists

Use the as-built doc and mapping specs as the authoritative source for verification and production rollout.
