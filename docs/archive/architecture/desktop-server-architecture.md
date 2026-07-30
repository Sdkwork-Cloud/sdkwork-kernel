> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **superseded**
> Archived: 2026-06-26

# Desktop and Server Architecture — Superseded Draft

The original comparative design (Tauri desktop vs Axum server, unified `AgentRuntimeBridge`, phased checkbox rollout) is **retired**. Do not implement from the removed draft sections or file paths listed in older revisions.

## Authoritative as-built sources

- [TECH-2026-06-14-multi-mode-agent-system.md](../../architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md) — client modes, kernel plugins, bridge plugins
- [TECH-topology-standard.md](../../architecture/tech/TECH-topology-standard.md) — production topology, ingress, Postgres/Redis
- [TECH-2026-06-04-external-agent-plugins.md](../../architecture/tech/TECH-2026-06-04-external-agent-plugins.md) — provider integration matrix
- `sdkwork-agent-server/specs/AGENT_SERVER_HTTP_SURFACE.md` — internal-api runtime HTTP only

## Shipped layout (sdkwork-kernel)

| Concern | Desktop / local profile | Server / cloud profile |
| --- | --- | --- |
> **2026-07:** In-repo `sdkwork-kernel-ui/` was removed. Product UI shells live in application repositories.

| UI shell | Product application repository consuming `@sdkwork/agent-internal-sdk` | Same integration against topology public HTTP |
| Client runtime | `sdkwork-agent-client` `AgentClient` — **Local**, **Remote**, **Hybrid** | Consumers use **Remote** + typed internal SDK |
| Local persistence | SQLite via `SDKWORK_DATABASE_FILE` (`sdkwork-agent-client/src/session/sqlite.rs`) | N/A on client |
| Server persistence | N/A | Postgres runtime DB + Redis rate limits (topology `*.production.env`) |
| Streaming ingress | **Remote:** `SseChatClient` → `/internal/v3/api/intelligence/runtime/*` | Same internal-api surface |
| Local SDK chat | OpenClaw, Hermes, Codex → `SdkModelBridgeRuntime` | Server-side via `SDKWORK_KERNEL_AGENT_PLUGIN` |
| Hosted agent id | Bridge plugin ids | `agent_registry::active_hosted_agent()` per plugin env |
| Business agents | Optional HTTP to agent-business | Postgres-backed marketplace APIs |

Retired application-local prefixes such as `/api/kernel/*` and draft-only REST/WebSocket runtime APIs must not be remounted. Runtime HTTP is internal-api only.

## Database and session crates (actual paths)

| Crate | Role |
| --- | --- |
| `sdkwork-agent-database` | `AgentDatabase`, `SqliteDatabase`, `PostgresDatabase`, `SessionRepository`, `MessageRepository` |
| `sdkwork-agent-client/src/session/` | Client bridge session store (SQLite) |
| `sdkwork-agent-session` | In-process session/conversation helpers (`manager.rs`, `conversation.rs`) — no `router.rs` from the draft |
| `sdkwork-agent-server/src/api/internal_runtime.rs` | Canonical runtime HTTP handlers |

## Intentionally deferred (outside current repo scope)

- Tauri/Electron desktop host (owned by product applications outside `sdkwork-kernel`)
- Draft `UnifiedSessionManager` / `SessionRouter` types as a single merged crate API
- ZeroClaw local SDK bridge (fail-closed; mapping-only upstream)
- opencode / claude-code / gemini-cli adapters (mapping-only)

## Verification

```bash
pnpm verify
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
cargo test --features postgres-sync --manifest-path sdkwork-agent-database/Cargo.toml
```
