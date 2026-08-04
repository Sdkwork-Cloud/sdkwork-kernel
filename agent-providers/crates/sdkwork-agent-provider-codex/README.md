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
- app-server client compatibility configuration facade: startup configuration is
  consumed only through the public app-server client surface

Only this L3 provider owns those upstream dependencies. Kernel core and provider
SPI remain Codex-neutral, and `external/codex` must remain clean.

## Runtime Contract

- Canonical plugin id: `plugin.intelligence.codex`
- Canonical agent id: `agent.codex`
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

Turn execution maps every SPI execution option that the app-server protocol can
express: `approvalPolicy`, `approvalsReviewer`, `sandbox`, and `ephemeral`
(`full_auto` falls back to `on-failure` approval plus `workspace-write`
sandbox). `ephemeral` is a start-only thread property: it is scoped to
`thread/start` and never sent on `thread/resume` (whose params carry no such
field), so no start-only attribute leaks into the resume request. The protocol
boundary is documented precisely:
`temperature` / `top_p` / `max_tokens` have no `turn/start` wire field in the
Codex app-server protocol and are intentionally not invented; `personality`,
`effort`, instruction overrides (`baseInstructions` /
`developerInstructions`), `serviceTier`, `modelProvider`, and `historyMode`
are valid `thread/start` / `turn/start` fields but currently have no upstream
input anywhere in the SDKWork turn chain, so they are extension points rather
than lossy mappings. `skip_git_repo_check` and `require_live_provider` are
kernel-side routing concerns, not app-server parameters.

## Session And History Contract

Provider metadata follows one namespace convention across every code engine:
part/message metadata keys are either `sdkwork.provider.*` (kernel-owned) or
`{engine_key}.*` (provider-owned, for example `codex.*`, `claude.*`,
`hermes.*`, `gemini.*`, `opencode.*`, `openclaw.*`, `rig.*`, `mimo-code.*`).
The Agents reconciler reads metadata through the engine-key namespace, so a
provider must never emit bare keys or a namespace that differs from its
canonical engine key.

`CodexSdkIntegration` exposes bounded async methods for:

- thread list and read
- paginated turn list
- paginated thread item list
- SDKWork session and message projections

All list methods require a limit in `1..=200`; omission resolves to 20. Opaque
forward and backwards cursors pass through unchanged. Every projected record
also retains the complete official typed object. Messages include a
`TenantSensitive` raw typed JSON part so new upstream fields are not discarded.

`session.list` accepts the full generic SPI filter set and maps it onto
`ThreadListParams` without loss: `source_kinds` (normalized to
`ThreadSourceKind`), `section_id`, `archived`, `search_term`, `sort_key`
(normalized to `ThreadSortKey`), `sort_direction` (normalized to
`SortDirection`), and `model_providers`. Providers that cannot express a filter
leave the corresponding `ThreadListParams` field untouched, so the SPI stays
provider-neutral while the Codex surface stays complete.

Turn-level protocol fields survive the message projection: every message of a
turn carries `codex.turn.status` / `codex.turn.items_view` / optional
`codex.turn.error` (typed JSON) / `codex.turn.started_at` /
`codex.turn.completed_at` / `codex.turn.duration_ms` metadata, and the turn
start time becomes the canonical message `created_at` so the conversation
timeline is preserved end to end. Thread metadata additionally retains
`codex.path` (durable thread file) alongside section, history mode, direct
input capability, thread source, git info, and sub-agent identity fields.

Provider-produced command, file, MCP, dynamic-tool, web, and image output is
marked untrusted at the SDKWork message boundary. MCP `readOnlyHint` is retained
explicitly. The unstable upstream thread path is not copied into persistent
SDKWork metadata.

Tool outputs carry the originating item id as `codex.tool_call_id` on the
output/result part so downstream session history reconciliation can pair each
`ToolResult` with its `ToolCall` and persist the call → result parent chain.
Structured results (`McpToolCall.result`/`error`, dynamic tool content items,
web search results) are projected as explicit result parts
(`mcp_tool_result`, `tool_result`, `web_search_tool_result`) instead of being
flattened into the raw payload. The full typed item JSON remains available on
every part batch through the raw provider item part.

Every protocol field with a dedicated projection is preserved on the
structured parts, not just in the raw payload: `AgentMessage.phase` /
`memoryCitation`, `UserMessage.clientId`, `CommandExecution.cwd` /
`processId` / `exitCode` / `durationMs` / `source` / `pluginId`,
`McpToolCall.durationMs`, `DynamicToolCall.namespace` / `durationMs`,
`CollabAgentToolCall.reasoningEffort`, `WebSearch.action`, and
`ImageGeneration.revisedPrompt` / `savedPath` all surface as `codex.*`
metadata, so consumers never need to re-parse the raw item to read them.

Thread tree relationships are preserved at the session level: `Thread.parentThreadId`
maps to `parent_session_id` with `SessionKind::Subagent`, and `forkedFromId` is
retained on the session so sub-agent and forked thread topology survives the
canonical projection. Session `compression_count` is derived from
`ContextCompaction` items in the thread history. Session metadata additionally
retains the full identity of the upstream thread: `codex.forked_from_id` (fork
lineage), `codex.agent_nickname` / `codex.agent_role` (AgentControl-spawned
sub-agent identity), and `codex.session_id` (the session tree shared by threads
that belong to the same tree), so the persisted protocol topology is never
reduced to the canonical parent link alone.

Sub-agent activity is not flattened into display text only: `SubAgentActivity`
items carry structured `codex.sub_agent.kind` / `codex.sub_agent.thread_id` /
`codex.sub_agent.path` metadata on their text part, and `CollabAgentToolCall`
items project a structured `collab_agent_tool_result` result part carrying the
receiver thread ids, prompt, model, and per-agent live state (including the
final sub-agent message) so the sub-agent execution context survives the
canonical projection. The runtime host can enumerate direct sub-agent threads
through `thread/list` with `parentThreadId` (`list_provider_session_children`)
so transcript synchronization recurses into every spawned sub-agent session.

The provider never resolves or opens a Codex state SQLite file by path, queries
private Codex tables/PRAGMAs, or reads rollout JSONL. Runtime startup delegates
provider state ownership to the official app-server client; persistence remains
an upstream implementation detail behind the app-server contract.

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

### Event Frame Convention

`provider_stream_event.v1` frames forward the raw JSON-RPC notification method
name verbatim (`item/started`, `item/completed`, `item/agentMessage/delta`).
Consumers must accept both the slashed method form and the legacy dotted
spelling (`item.started`); the kernel never rewrites method names. Server
initiated requests are covered explicitly: `currentTime/read` is answered with
the current epoch, the six user-mediated request types
(`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`,
`item/permissions/requestApproval`, `item/tool/requestUserInput`,
`mcpServer/elicitation/request`, `item/tool/call`) are forwarded to the host
policy interaction path, and any other server request is rejected
fail-closed instead of hanging the app-server. Interaction resolutions are
normalized onto the Codex wire shapes before resolution: the generic host
`execPolicyAmendment` object (`command` / `commandPrefix` / `prefix` token
array) becomes the transparent `ExecPolicyAmendment` token array, and the
`networkPolicyAmendment` object (`host` or `hosts` array) becomes a single
`NetworkPolicyAmendment` `{host, action: "allow"}` pair. Every resolution
shape round-trips through the official protocol response types in tests.

## Verification

```bash
cargo test -p sdkwork-agent-provider-codex
cargo clippy -p sdkwork-agent-provider-codex --all-targets -- -D warnings
node --test scripts/provider-transport-workers/codex-app-server-live.test.mjs
node scripts/check-kernel-standards.mjs
```
