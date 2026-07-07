> Owner: SDKWork maintainers
> Updated: 2026-07-08
> Status: **as-built** (replaces the historical implementation plan)

# Multi-Mode Agent Client and Kernel Plugin Integration

## Goal

SDKWork kernel supports multiple external agent runtimes through:

1. **Server runtime plugins** — selected by `SDKWORK_KERNEL_AGENT_PLUGIN` in `sdkwork-agent-server`
2. **Client bridge plugins** — local SQLite sessions + SDK model routing, or remote internal-api HTTP
3. **Process adapters** — negotiated SDK bindings in `agent-providers/crates/sdkwork-agent-provider-*`

Authoritative per-upstream status: `sdkwork-kernel-plugins/specs/mappings/*.md`.

## Server: kernel agent plugin selection

Environment variable: `SDKWORK_KERNEL_AGENT_PLUGIN` (also declared in `specs/topology.spec.json` → `envKeys.kernelAgentPlugin`).

| Value | Provider crate | Default model provider |
| --- | --- | --- |
| `rig` (default) | `sdkwork-agent-provider-rig` | Rig typed providers |
| `openclaw`, `open-claw` | `sdkwork-agent-provider-openclaw` | `provider.model.openclaw` |
| `hermes`, `hermes-agent` | `sdkwork-agent-provider-hermes` | `provider.model.hermes` |
| `codex`, `openai-codex` | `sdkwork-agent-provider-codex` | `provider.model.codex` |
| `claude-code` | `sdkwork-agent-provider-claude-code` | `provider.model.claude-code` |
| `gemini-cli`, `gemini` | `sdkwork-agent-provider-gemini-cli` | `provider.model.gemini-cli` |
| `opencode` | `sdkwork-agent-provider-opencode` | `provider.model.opencode` |

Implementation: `sdkwork-agent-server/src/runtime_bootstrap.rs` bootstraps `RuntimeBuilder` through `SdkworkKernelPlugin::configure_runtime`.

Hosted session `agentId` validation follows the same env key via `sdkwork-agent-server/src/agent_registry.rs` (`active_hosted_agent()`).

Topology profiles set the variable in `configs/topology/*.env` (default `rig` for production safety).

## Client: AgentClient modes

Implementation: `sdkwork-agent-client/src/bridge/client.rs`

| Mode | Behavior |
| --- | --- |
| **Remote** | `SseChatClient` → `/internal/v3/api/intelligence/runtime/*` on `application.public-ingress` |
| **Local** | `AgentBridgeProvider` from `AgentBridgePluginRegistry` |
| **Hybrid** | Local first; fallback to remote per `FallbackStrategy` |

Ingress auth: `sdkwork-agent-client/src/ingress_auth.rs` (MAC / bearer profiles aligned with server).

## Client bridge plugins (builtin)

| Plugin id | Bridge type | Local chat path | Status |
| --- | --- | --- | --- |
| `builtin.openclaw` | OpenClaw | `OpenClawSdkIntegration` → `SdkModelBridgeRuntime` | SDK-backed |
| `builtin.hermes` | Hermes | `HermesSdkIntegration` → `SdkModelBridgeRuntime` | SDK-backed |
| `builtin.codex` | Codex | `CodexSdkIntegration` → `SdkModelBridgeRuntime` | SDK-backed |
| `builtin.zeroclaw` | ZeroClaw | Fail-closed until adapter exists | Session store only |
| `builtin.*` (codex registry) | Codex | same as codex | SDK-backed |

Shared session store: `sdkwork-agent-client/src/session/` (SQLite via `SDKWORK_CLIENT_DATABASE_PATH`).

Streaming on local SDK bridges is rejected; use **Remote** + `HttpRestSse` for streaming.
`SseChatClient` async methods are the preferred API for async hosts. Its sync
`ChatClient` implementation is retained for bridge compatibility and detects an
existing Tokio runtime before blocking; when one is present, the call is
offloaded to a dedicated runtime thread to avoid nested-runtime panics and
executor deadlocks.

## Security and production posture

- SDK workers fail closed when spawn/negotiation fails unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1` (development only; topology-controlled).
- External SDK-backed provider crates also fail closed on direct in-process
  model/tool invocation. Production execution must route through the selected
  SDK/runtime transport worker registered by `SdkworkKernelPlugin::configure_runtime`.
- Production profiles use `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token`, Postgres runtime DB, and Redis rate limits (see topology env files).
- Client local bridges inherit the same mock policy through adapter-core `mock_provider_invocation_allowed`.

## Related crates

| Concern | Crate / path |
| --- | --- |
| Client API | `sdkwork-agent-client` |
| Server bootstrap | `sdkwork-agent-server` |
| Kernel SPI | `sdkwork-agent-kernel` |
| Plugin trait | `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core` |
| SDK SPI | `sdkwork-agent-provider-spi` |
| OpenAPI authority | `apis/internal-api/` → `/internal/v3/api/intelligence/runtime` |

## Verification

```bash
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
cargo test --manifest-path sdkwork-agent-server/Cargo.toml runtime_bootstrap
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-openclaw/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-hermes/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-claude-code/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-opencode/Cargo.toml
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node scripts/check-agent-provider-bindings.mjs
pnpm test:topology
```

## Out of scope (not yet implemented)

- ZeroClaw upstream adapter and kernel plugin
- gRPC client protocol (`AgentProtocol::Grpc`)
- Dynamic `libloading` bridge plugins (builtins only)
- Kernel `BridgeProviderAdapter` / in-process kernel integrator from the original design draft

Historical design drafts were retired in favor of this as-built document and `sdkwork-kernel-plugins/specs/mappings/`.

## Staging / live SDK validation (post-`pnpm verify`)

Contract gate (CI/local merge-ready):

```bash
pnpm verify
node scripts/provider-transport-workers/engine-sdk-live.test.mjs
```

Optional live invokes (require real upstream credentials/runtime; not part of default `pnpm verify`):

| Plugin | Server env | Upstream prerequisites |
| --- | --- | --- |
| OpenClaw | `SDKWORK_KERNEL_AGENT_PLUGIN=openclaw` | `OPENCLAW_GATEWAY_URL` (+ optional `OPENCLAW_GATEWAY_TOKEN`) |
| Hermes | `SDKWORK_KERNEL_AGENT_PLUGIN=hermes` | Hermes `tui_gateway` / IPC when `SDKWORK_HERMES_USE_TUI_GATEWAY=1` |
| Codex | `SDKWORK_KERNEL_AGENT_PLUGIN=codex` | `@openai/codex-sdk` worker path in `engine-sdk-live.mjs` |
| Rig (production default) | `SDKWORK_KERNEL_AGENT_PLUGIN=rig` | Typed Rig providers; mock only when topology allows |

Use a non-production topology profile (for example `standalone.split-services.development`) and never set `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1` on `*.production` profiles unless explicitly approved for a controlled drill.
