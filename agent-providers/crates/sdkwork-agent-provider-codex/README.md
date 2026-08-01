# sdkwork-agent-provider-codex

Codex provider plugin for the SDKWork agent kernel.

## Purpose

Maps the official Codex app-server contract to SDKWork kernel SPI types, agent
manifests, package manifests, provider manifests, and runtime bootstrap
entrypoints. The provider consumes the pinned, read-only `external/codex`
source tree through its public Rust facades.

## Source Integration

- `codex-app-server-client`: official in-process runtime and typed request facade
- `codex-app-server-protocol`: authoritative Thread, Turn, ThreadItem, status,
  request, response, and cursor models
- `codex-core`: official startup configuration and state handle initialization
  required by the in-process client; never used for private table queries

Only this L3 provider owns those upstream dependencies. Kernel core and provider
SPI remain Codex-neutral, and `external/codex` must remain clean.

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

The optional `provider.session-control.codex` extension exposes interrupt,
compact, and fork through the negotiated `sdk.session.control` runtime. Active
Turns retain canonical Session to model-request worker affinity so
`turn/interrupt` reaches the owning resident app-server. Idle control validates
the opaque provider Session through `thread/read`; compact and fork then use
`thread/compact/start` and `thread/fork`. Unsupported compact focus and
message-id fork boundaries fail explicitly, and control never uses mock
fallback.

## Session And History Contract

`CodexSdkIntegration` exposes bounded async methods for:

- thread list and read
- paginated turn list
- paginated thread item list
- SDKWork session and message projections

All list methods require a limit in `1..=200`; omission resolves to 20. Opaque
forward and backwards cursors pass through unchanged. Every projected record
also retains the complete official typed object. Messages include a
`TenantSensitive` raw typed JSON part so new upstream fields are not discarded.

Provider-produced command, file, MCP, dynamic-tool, web, and image output is
marked untrusted at the SDKWork message boundary. MCP `readOnlyHint` is retained
explicitly. The unstable upstream thread path is not copied into persistent
SDKWork metadata.

The provider never resolves or opens a Codex state SQLite file by path, queries
private Codex tables/PRAGMAs, or reads rollout JSONL. Runtime startup obtains
the state handle through the official `codex_core::init_state_db` bootstrap API;
persistence remains an upstream implementation detail behind the app-server
contract.

## Provider Session Activity

`CodexSdkIntegration::record_provider_session_activity` accepts a live Codex
app-server `ThreadStatus` observation. Active flags distinguish approval from
user-input waiting; idle and system-error statuses map explicitly. `NotLoaded`
maps to an unsupported activity snapshot because it is not a live observation.

The managed Node transport forwards official Codex SDK thread events and
incremental CLI JSONL events into the same activity store for operations run by
this integration. Independently running Codex processes remain `Unsupported`
unless a runtime host attaches an authoritative collector; file timestamps and
a historical `active` flag are never substituted.

## Verification

```bash
cargo test -p sdkwork-agent-provider-codex
cargo clippy -p sdkwork-agent-provider-codex --all-targets -- -D warnings
node --test scripts/provider-transport-workers/codex-app-server-live.test.mjs
node scripts/check-kernel-standards.mjs
```
