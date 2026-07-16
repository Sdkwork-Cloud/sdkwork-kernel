# KERNEL_PRODUCT_PROJECTION_SPEC

<!-- SDKWORK-KERNEL-PROJECTION-SPEC: v1 -->

Authority for mapping **sdkwork-kernel** agent/runtime events to **BirdCoder** `coding_session_event` canonical events consumed by workbench UI and `coding-server` APIs.

## Scope

| Layer | Owns |
| --- | --- |
| **sdkwork-kernel** | `agent.*` turn execution, model/tool providers, kernel events, official SDK bindings |
| **BirdCoder** | `coding_session*`, canonical event dialect, workbench projection, native-session catalog |
| **Integration boundary** | `sdkwork-birdcoder-kernel-bridge` + `@sdkwork/birdcoder-pc-projection` |

Kernel MUST NOT emit BirdCoder OpenAPI shapes directly. BirdCoder MUST NOT execute agent turns outside the kernel bridge.

## Canonical Event Contract (BirdCoder)

BirdCoder canonical events use:

- `kind`: stable event type string (see table below)
- `sequence`: monotonic stringified integer per turn
- `runtimeStatus`: `ready` | `streaming` | `awaiting_tool` | `completed` | `failed`
- `payload`: JSON object with dialect-normalized fields

Authoritative TS projection reference: `@sdkwork/birdcoder-pc-projection` `canonicalEventsFromChatStream()`.

## Kernel → Product Projection Table

| BirdCoder `kind` | `runtimeStatus` | Kernel / bridge source | Notes |
| --- | --- | --- | --- |
| `session.started` | `ready` | Turn bootstrap / `KernelEngineSlot` selection | Includes `engineId`, `modelId`, `transportKind`, `approvalPolicy` |
| `turn.started` | `streaming` | `agent.turn.started` or bridge turn open | `messageCount`, `lastMessageRole` |
| `message.delta` | `streaming` | `agent.message.delta` / model stream chunk | `contentDelta`, `role`, `chunkId` |
| `message.completed` | `completed` | Aggregated assistant output | Full `content` after stream settles |
| `tool.call.requested` | `awaiting_tool` | `agent.tool.call.requested` | `toolCallId`, `toolName`, `toolArguments` |
| `tool.call.completed` | `completed` | `agent.tool.call.completed` | Maps tool result output |
| `approval.required` | `awaiting_tool` | Provider-neutral interaction projection | `interactionId`, `interactionKind: "approval"`, plus tool display metadata |
| `user.question.required` | `awaiting_tool` | Provider-neutral interaction projection | `interactionId`, `interactionKind: "user_question"`, plus `questions` and tool display metadata |
| `operation.updated` | `completed` / `awaiting_tool` | Checkpoint / approval / command lifecycle | `status`, optional `finishReason` |
| `turn.completed` | `completed` | `agent.turn.completed` | `contentLength`, `finishReason` |
| `turn.failed` | `failed` | `agent.turn.failed` / kernel error normalization | `errorMessage` |

## Dialect Normalization (BirdCoder-owned)

After projection, BirdCoder `codeengine.dialect` normalizes:

- tool names → `canonicalize_codeengine_tool_name`
- approval IDs → `resolve_codeengine_approval_id`
- command interaction state → `resolve_codeengine_command_interaction_state`
- runtime status strings → `normalize_codeengine_runtime_status`

Kernel event payloads remain provider-neutral. Dialect normalization happens only in BirdCoder crates.

### Interaction Projection Contract

Pending provider interactions are projected by the BirdCoder kernel bridge,
not by provider-specific UI code. The bridge accepts only kernel-neutral tool
calls/command records and emits these canonical events:

- Approval: `approval.required` with `interactionKind: "approval"`.
- User question: `user.question.required` with `interactionKind: "user_question"`.

Both payloads MUST include a non-empty immutable `interactionId` copied from
the provider-native approval, permission, question, request, or checkpoint
identifier. The bridge MUST NOT synthesize an id from a local session, turn,
database record, transport request, or counter. Invalid/ambiguous provider
interaction payloads fail closed instead of creating an unrouteable durable
event.

The canonical payload MAY retain display information under `questions`,
`tool`, `toolName`, `toolCallId`, `toolArguments`, and `metadata`, but it MUST
not publish raw provider event names or provider identifier aliases as
top-level contract fields. Provider-specific request/permission/session alias
keys are removed from projected display arguments because `interactionId` is
the only interaction authority.

Finalization inserts canonical interaction events before terminal
`message.completed` / `turn.completed` events and preserves the source
`turn_id` and `runtime_id`. Repository persistence assigns the durable event
UUID and sequence. Mutation endpoints resolve that durable UUID back to the
payload `interactionId`; bridge adapters pass the resolved value unchanged as
the provider reply target and idempotency key. A durable lease can prevent
concurrent replies, but an unknown provider outcome after process loss remains
an explicit recovery state rather than a claim of external exactly-once.

## Live Interaction (transition)

| Concern | Current owner | Target owner |
| --- | --- | --- |
| OpenCode permission / user-question replies (product API) | `sdkwork-birdcoder-kernel-bridge/live_interaction.rs` | `sdkwork-kernel` `agent.live_interaction` |
| Approval / question during **agent turn** | `sdkwork-birdcoder-kernel-bridge` | `sdkwork-kernel` unified live interaction SPI |

Native-session catalog inventory remains BirdCoder-owned. Live replies route through kernel bridge, not deprecated `pc-chat` paths.

## SDK Runtime Modes (production)

| `mode` | Production policy |
| --- | --- |
| `sdk_live` | **Allowed** — canonical official SDK invoke |
| `stub` / `sdk_probe` | **Rejected** |
| `sdk_live_failed` | **Rejected** — invoke/configuration error surfaced to caller |

## Transport Mapping

| Engine | Kernel binding | Bridge transport label | Workbench `transportKind` |
| --- | --- | --- | --- |
| codex | `binding.agent-provider.codex` | `typescript_node` / `rust_native` / `ipc_protocol` | `sdk-stream` / `cli-jsonl` / `json-rpc-v2` |
| claude-code | `binding.agent-provider.claude-code` | `typescript_node` | `sdk-stream` |
| gemini-cli | `binding.agent-provider.gemini-cli` | `typescript_node` | `sdk-stream` |
| opencode | `binding.agent-provider.opencode` | `typescript_node` / `http_openapi` | `sdk-stream` / `openapi-http` |

## Production Fail-Closed

Production kernel profiles are detected when `SDKWORK_KERNEL_ENVIRONMENT=production` **or** `SDKWORK_KERNEL_PROFILE_ID` ends with `.production` (for example `cloud.production`):

- Mock `ModelProvider` responses MUST NOT be returned.
- Runtime payloads with `mode=stub|sdk_probe|sdk_live_failed` MUST be rejected.
- Live invokes MUST return `mode=sdk_live` when official SDK execution succeeds.
- TypeScript worker bootstrap MUST NOT silently fall back to in-memory stubs.

Override for local diagnostics only: `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`.

## Verification

```bash
# kernel
cargo test -p sdkwork-agent-provider-core
cargo test -p sdkwork-agent-provider-spi
cargo test -p sdkwork-agent-provider-transport-node

# birdcoder
pnpm run check:kernel-birdcoder-alignment
cargo test -p sdkwork-birdcoder-kernel-bridge
node scripts/birdcoder-kernel-integration-contract.test.mjs
```

## Related Docs

- BirdCoder: `docs/架构/30-Kernel-BirdCoder-职责边界标准.md`
- BirdCoder: `docs/架构/31-Kernel-BirdCoder-集成实施方案.md`
- BirdCoder: `specs/kernel-birdcoder-alignment.spec.json`
