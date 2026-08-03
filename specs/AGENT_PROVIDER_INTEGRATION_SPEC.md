# SDKWork Agent Provider Integration Specification

- Version: 0.2.0
- Status: standard candidate
- Scope: external agent framework and provider integration into the SDKWork Agent
  Kernel, including official SDK, Rust crate, source tree, HTTP/OpenAPI, and IPC
  transports; capability drivers; binding negotiation; mapping; registry;
  conformance; and extension rules
- Domain: `intelligence`
- Capability: `agent-kernel.agent-provider-integration`
- Supersedes: `AGENT_SDK_SPI_SPEC.md` (archival alias retained one release cycle)
- Related:
  - `AGENT_PROVIDER_BINDING_SPEC.md`
  - `AGENT_KERNEL_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `SDK_SPEC.md`

Agent provider integration connects third-party agent frameworks (Codex, Claude
Code, Gemini CLI, OpenCode, OpenClaw, Hermes, Rig, and future providers) to the
SDKWork Agent Kernel. Kernel provider SPI remains the semantic center. External
frameworks are implementation details selected and invoked through typed
capability drivers and transport hosts.

## 1. Principle

Rules:

- Kernel object models `MUST NOT` be mutated for provider-specific fields.
- External framework metadata `MUST` be namespaced in provider binding manifests
  and driver diagnostics.
- Required capabilities `MUST` fail closed when no healthy transport can serve
  them.
- Raw HTTP bypasses `MUST NOT` replace official or generated SDK clients when a
  binding declares an SDK authority.
- Mapping from external types to kernel types `MUST` flow through
  `sdkwork-agent-provider-core`, re-exported by `sdkwork-agent-provider-spi`.
- Provider integration code `MUST NOT` depend on product UI, React, or
  application business crates; bindings declare integration sources instead.
- Application-facing business HTTP and SDK families `MUST` be owned by
  `sdkwork-agents`, not `sdkwork-kernel`.

## 2. Layering

| Layer | Owner | Responsibility |
| --- | --- | --- |
| L0 Kernel SPI | `sdkwork-agent-kernel` | Model, tool, skill, session, policy semantics |
| L1 Provider integration SPI | `sdkwork-agent-provider-spi` | Capability drivers, binding negotiation, transport selection |
| L2 Provider transport | `sdkwork-agent-provider-transport-*` | Language/runtime transport to native surfaces |
| L3 Provider implementation | `sdkwork-agent-provider-{name}` | Manifest wiring, driver registration, plugin contribution |
| L4 Application domain | `sdkwork-agents` | Managed agents, marketplace, app/open/backend SDK families |

Dependency direction:

```text
sdkwork-agent-kernel
        ↑
sdkwork-agent-provider-spi
        ↑
sdkwork-agent-provider-transport-*
        ↑
sdkwork-agent-provider-{name}
        ↑
sdkwork-agents (composition + application SDK only)
        ↑
product applications (BirdCoder, IM PC, ...)
```

Product applications `MUST` consume agent runtime through `sdkwork-agents` SDK or
HTTP surfaces. They `MUST NOT` depend on `sdkwork-agent-provider-*` crates
directly.

## 3. Integration Modes

Standard integration modes for `integration_sources[]` in provider bindings:

| Mode | Typical source | Notes |
| --- | --- | --- |
| `official_sdk` | npm / PyPI published SDK | Preferred when an official SDK exists |
| `rust_crate` | Cargo crate in workspace or registry | Preferred for in-process Rust providers |
| `source_tree` | `external/<framework>` package/crate path | Used when integration requires vendored source |
| `npm_package` | Node package without typed SDK wrapper | Worker or subprocess bootstrap |
| `python_module` | Python package / module | Subprocess or JSON-RPC worker |
| `http_openapi` | OpenAPI authority + generated transport | When only HTTP contract exists |
| `ipc_protocol` | stdio / WebSocket / JSON-RPC | Last-resort structured IPC |

Rules:

- A binding `MAY` declare multiple integration sources; transport selection
  chooses the first healthy candidate per capability.
- Integration mode is independent from transport kind; e.g. `official_sdk` often
  maps to `typescript_node` transport.
- Rig-style `source_tree` + `rust_crate` integrations use the same provider crate
  layout as Codex-style SDK integrations.
- `source_tree` entries point at the concrete package or crate path when the
  upstream checkout has one, such as `external/gemini-cli/packages/sdk`,
  `external/mimo-code/packages/sdk/js`, or `external/rig/crates/rig-core`.
  Broader upstream roots remain mapping references only and must not satisfy
  runtime SDK package health.
- Native source dependencies are permitted only in the owning L3 provider,
  declared once by the root workspace build manifest, pinned by gitlink, and
  kept read-only. Provider-neutral L0/L1/L2 crates must not inherit the
  upstream dependency or expose upstream types.
- A source dependency must use the upstream public SDK/client/protocol facade.
  Private databases, tables, caches, logs, transcripts, and file layouts are
  not supported integration surfaces.

## 4. Transport Kinds And Priority

Standard transport kinds (formerly "backend kinds"):

| Kind | Typical integration | Notes |
| --- | --- | --- |
| `rust_native` | Rust crate | Preferred when an official Rust integration exists |
| `typescript_node` | npm package via Node/Bun worker | Second preference |
| `python_process` | Python package via subprocess/JSON-RPC | Third preference |
| `http_openapi` | OpenAPI authority + generated transport | When only HTTP contract exists |
| `ipc_protocol` | stdio/WebSocket/JSON-RPC without in-process SDK | Last-resort structured IPC |

Default global priority when a binding does not override transport order:

1. `rust_native`
2. `typescript_node`
3. `python_process`
4. `http_openapi`
5. `ipc_protocol`

### 4.1 Packaged Provider Host Discovery

Desktop, server-archive, and container releases may package Node/Python workers,
provider CLIs, and their supporting binaries in one provider host root.

Rules:

- The canonical directory name is `provider-host` and the canonical explicit
  root variable is `SDKWORK_AGENT_PROVIDER_HOST_ROOT`.
- Release producers `MUST` emit only the canonical directory and variable.
  `provider-runtime` and `SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT` are read-only
  compatibility inputs governed by
  [`MIG-2026-0002-provider-host-root`](../docs/migrations/MIG-2026-0002-provider-host-root.md).
- Consumers resolve an explicit worker or language binary first, then the
  canonical host-root variable, the legacy variable during its compatibility
  window, a canonical packaged directory, a legacy packaged directory during
  that window, and finally a repository source path only in debug/test builds.
- Canonical packaged directories take precedence across the complete executable
  ancestor search. A nearer legacy directory `MUST NOT` shadow a canonical
  directory at a higher application-bundle level.
- Empty explicit root variables fail closed at installer/configuration
  boundaries. A canonical variable that is present but empty `MUST NOT` activate
  the legacy alias.
- Release verification `MUST` prove that the packaged worker inventory and
  language binaries resolve without a sibling source checkout.

## 5. Provider Crate Layout

Each external framework `MUST` ship as one crate:

`sdkwork-agent-provider-<framework>`

The crate `MUST` contain:

- `provider-binding.manifest.json` (or include path to catalog copy)
- kernel plugin manifest contribution
- capability drivers
- runtime bootstrap (`RuntimeBuilder` and/or `ProviderTransportRouter`)
- mapping adapters for supported capabilities

Plugins `MUST NOT` be split into separate `plugin-*` and `adapter-*` crates for
the same framework.

## 6. Registry, Negotiation, And Transport Health

`ProviderTransportRegistry` holds transport hosts keyed by transport kind.
`ProviderTransportRouter` routes `ProviderRuntimeRequest` values to negotiated
transports. Transport `prepare()` health `MUST` influence router selection.

Negotiation steps:

1. Load provider binding manifest.
2. For each required capability, select the first healthy transport candidate.
3. Resolve the declared `driver_id` from `DriverRegistry`.
4. Record selected, missing, and degraded capabilities.
5. Fail closed when any required capability is missing.

Operation dispatch rules:

### 6.1 Canonical Session And Provider Session Identity

The SDKWork Session and a provider-owned resumable Session are separate
identities across every provider transport.

Rules:

- `session_id` is the canonical SDKWork `AgentSession` identity. It remains the
  Session ownership key on kernel events, persistence, activity correlation,
  and product APIs.
- `provider_session_id` is the provider-returned opaque continuation identity.
  Provider-specific names such as Codex `threadId` are wire-adapter or raw
  evidence fields only; shared contracts and normalized payloads use the
  `providerSessionId` family.
- When `provider_session_id` is absent, an SDK or CLI adapter `MUST` use the
  provider's create/start path. It `MUST NOT` pass `session_id` to a provider
  resume API, `--resume`, `--session`, or an equivalent continuation option.
- When `provider_session_id` is present, the adapter `MUST` pass that exact
  value to the provider continuation path and `MUST` reject a different or
  missing provider-emitted identity before publishing verified activity or
  terminal continuation metadata.
- An adapter `MUST NOT` synthesize `provider_session_id` from `session_id`,
  `model_request_id`, a local database id, or any transport-generated id.
- A normalized `KernelEvent.session_id` retains canonical SDKWork Session
  identity. Provider continuation identity belongs in normalized payload
  metadata as `providerSessionId`; raw provider payload may retain original
  wire field names for evidence.
- `turn_id` is the canonical SDKWork Turn identity when the caller has already
  established one. `provider_turn_id` is the provider-returned opaque Turn
  identity; Codex `turnId` remains private to its adapter and raw evidence.
- When a provider request is built from the kernel `ModelRequest`, the canonical
  `turn_id` is carried by `ModelRequest.step_id` and serialized as the explicit
  runtime operation field `turn_id`. This is a compatibility name at the kernel
  SPI boundary only: it must never be rendered or documented as a provider
  `step`, and a missing value must remain omitted rather than inferred from a
  Session, request, or provider Turn identity.
- A normalized `KernelEvent.step_id` uses canonical `turn_id` when available.
  Provider Turn identity belongs in normalized payload metadata as
  `providerTurnId` and must not replace the canonical Turn identity.
- Turn control and Interaction responses must validate canonical `turn_id` and
  opaque `provider_turn_id` independently. Neither identity may be accepted as
  an alias for the other.
- Conformance tests `MUST` cover canonical-only creation, independent
  canonical/provider-id continuation, provider-id mismatch rejection, and the
  absence of synthesized provider terminal identity for each SDK and CLI lane.
- A resident transport that installs listeners before `turn/start` completes
  `MUST NOT` bind the new execution to the first same-Session event. It buffers
  at most 1,024 ordered events carrying a provider Turn id until the
  `turn/start` response supplies the authoritative id, replays only matching
  events, and discards late events from previous Turns. Turnless host requests
  may continue immediately under provider Session affinity.

- The selected backend's `runtime_operations[]` is the executable operation
  allowlist for that negotiated capability.
- `Ping` is a health probe, not proof that model, tool, skill, or session
  operations are executable.
- `ProviderTransportRouter` and `SdkRuntimeRouter` `MUST` reject a request with
  `operation_not_supported` before invoking a runtime when the requested
  operation is absent from the selected backend `runtime_operations[]`.
- Capabilities with `execution_scope: provider_local` are implemented through
  typed provider-core or local SPI paths. They may expose only `ping` through
  runtime routing; lifecycle create/get/update/resume/close/delete/list behavior
  must use the provider-local lifecycle provider rather than a fake transport
  operation.
- Provider-local lifecycle implementations must expose ordered incremental
  changes through a monotonically increasing cursor. Change retention must be
  bounded, expired cursors must fail explicitly, and synchronization into
  runtime persistence must collapse repeated changes for one session to the
  latest snapshot before writing.
- External session identities such as Codex threads, OpenCode sessions,
  OpenClaw gateway sessions, Hermes sessions, and Rig executions must map to
  `AgentSession` before they enter shared persistence or event streaming.
- A `model_chat_stream` worker terminal frame (`event: stream.done`) `MUST`
  carry the active `model_request_id`. A `provider_session_id` is optional and
  may be emitted only when the provider runtime has actually established that
  provider session; adapters must never synthesize one from a request, local
  database id, or transport id.
- Runtime-backed stream completion adapters `MUST` reject terminal metadata
  whose `model_request_id` does not match the active model request. Product
  facades may enable first-turn streaming only when a verified provider session
  id is present in that correlated completion. Providers without this proof
  remain invoke-only for initial turns while retaining normal resumed-stream
  support.
- Capabilities with `execution_scope: transport_runtime` may execute through the
  selected transport only when the backend runtime is healthy and the requested
  operation is explicitly declared.

#### 6.1.1 Provider Session Control

Provider-session control is an optional L0 extension, not a new core provider
family and not a copy of any provider's complete RPC catalog. The standard
capability is `sdk.session.control`; its executable actions are the independent
`session_interrupt`, `session_compact`, and `session_fork` runtime operations.

Rules:

- Every request `MUST` carry a unique `control_request_id`, canonical
  `session_id`, opaque `provider_session_id`, and `policy_decision_id`.
- Control adapters `MUST` validate the provider session through the official
  SDK or protocol before mutation. They `MUST NOT` substitute the canonical
  SDKWork Session id for the provider identity.
- Session-control actions are side-effectful and `MUST` be policy-gated before
  transport dispatch. Missing policy evidence fails validation.
- `session_interrupt` is idempotent; interrupting an idle provider session may
  return `no_op` but is not a generic provider failure.
- `session_compact` preserves canonical and provider session identity. An
  adapter `MUST` reject action parameters that its upstream API cannot preserve
  instead of silently dropping them.
- `session_fork` returns a new opaque `forked_provider_session_id` which `MUST`
  differ from the source `provider_session_id`. The kernel creates or updates
  canonical session state separately; a provider id never becomes a kernel id.
- Control operations require a live Provider runtime and `MUST NOT` use mock or
  synthetic fallback in development or production.
- The initial executable reference lane is OpenCode: `interrupt` and `compact`
  use `client.v2.session` from the official `@opencode-ai/sdk/v2` export against
  the owning `OPENCODE_SERVER_URL`; `fork` uses the same package's official
  root client. Adapters `MUST NOT` treat the v2 export's legacy
  `client.session` projection as the `/api/session/*` control surface. Chat
  uses the durable `/api/session/{id}/prompt` route with `delivery: steer` and
  `resume: true`, consumes the `session.next.*` runner event family (the
  durable runner does not emit `session.idle`), and gates turn completion by
  polling `v2.session.active` until the drain ends; the requested model is
  applied at session creation and confirmed through `v2.session.switchModel`
  because the durable runner resolves models from the server's built-in
  catalog rather than config-file providers. In-process servers bind an
  ephemeral OS-assigned port instead of the SDK default so concurrent turns
  cannot collide on one shared port.
- The Codex executable reference lane uses one resident app-server connection.
  An active canonical Session is bound to its exact Node worker and model
  request before the Turn starts; control is multiplexed to that worker through
  `sdkwork/session.control`. `interrupt` maps to `turn/interrupt`, `compact`
  maps to `thread/compact/start`, and `fork` maps to `thread/fork` only after
  `thread/read` validates the opaque provider Session. Idle interrupt returns
  `no_op` after that validation. Codex rejects `focus` and
  `before_message_id`: neither can be represented by the stable upstream
  methods, and a canonical message id is never guessed to be a Codex Turn id.
  All three operations use their request-scoped timeout and have no mock lane.
- The Claude Code executable reference lane uses the official
  `@anthropic-ai/claude-agent-sdk`. `session_interrupt` aborts the active
  streaming query of the exact canonical Session through the same-worker
  `sdkwork/session.control` channel (in-process abort registry keyed by
  `model_request_id`, with provider-session affinity validation); interrupting
  an idle session returns `no_op`. `session_fork` calls the official
  `forkSession()` API and returns the new provider session id. `session_compact`
  stays undeclared: the official SDK exposes no compact trigger, and the
  adapter must not invent one outside the SDK contract.

#### 6.1.2 Official SDK Session Discovery

Provider Session inventory and transcript discovery are read-only transport
operations under `sdk.session.lifecycle`. `session_list` enumerates provider
Sessions and `session_history` reads messages for one exact opaque provider
Session. They do not create canonical SDKWork identities.

Rules:

- A binding may declare `session_list` or `session_history` only when the
  selected runtime uses the provider's official public SDK. Private provider
  databases, transcript files, caches, logs, and source-tree storage layouts
  are forbidden discovery authorities.
- A discovery request carries optional `working_directory`, optional opaque
  `cursor`, and a positive bounded `limit`. `session_history` additionally
  carries the exact `provider_session_id`; it never accepts canonical
  `session_id` as an alias.
- The normalized Session page projection contains `items`, optional
  `next_cursor`, and optional `previous_cursor`. The normalized history page
  contains the same fields plus the exact `provider_session_id` whose messages
  were requested.
- Session items use `provider_session_id` and optional
  `parent_provider_session_id`. Message items use `provider_message_id`,
  `provider_session_id`, and optional `parent_provider_message_id`. Discovery
  items `MUST NOT` claim canonical `session_id`, `message_id`, or parent ids.
  Canonical identity is assigned only by an explicit runtime adoption mapping
  before shared persistence or event publication.
- Every history page and every message item must match the requested
  `provider_session_id`. Empty identities, self-parent lineage, unsupported
  roles, empty message part sets, duplicate part ids, and cross-Session records
  fail closed.
- Message parts use the provider-neutral `AgentPartKind` vocabulary. Text,
  JSON, content reference, artifact, tool call, policy decision, and error
  kinds must carry their required typed field. File and media references must
  carry a non-empty MIME type; adapters use `application/octet-stream` only
  when the official SDK omits a more precise file MIME type.
- The L1 drain treats every cursor as opaque. It requests at most 200 records
  per page, rejects a repeated cursor, rejects provider Session or message ids
  repeated across pages, and fails once a single drain exceeds 10,000 unique
  records. It never silently truncates, deduplicates, or interprets a
  provider-native cursor.
- Claude Code discovery uses
  `@anthropic-ai/claude-agent-sdk` `listSessions()` and
  `getSessionMessages()` with `includeSystemMessages: true`. Because that SDK
  exposes offset pagination, its adapter-owned Base64URL cursor is bound to the
  package, operation, provider Session, working directory, and limit; changing
  any bound value invalidates the cursor.
- OpenCode discovery uses the official `@opencode-ai/sdk/v2` client
  `client.v2.session.list()` and `client.v2.session.messages()` methods. Its
  SDK-native cursor is forwarded and returned unchanged; the adapter does not
  decode or replace it.
- Codex discovery uses the public `codex-app-server-client` and
  `codex-app-server-protocol` crates with `thread/list`, `thread/read`, and
  `thread/turns/list`. History requests force chronological `sortDirection:
  "asc"` and `itemsView: "full"`; the lower-level `thread/items/list` method
  remains a typed provider helper, not the discovery authority. The Rust
  adapter preserves both opaque app-server cursors, projects every returned
  thread and item into provider-owned records, and retains each complete typed
  `ThreadItem` as a tenant-sensitive JSON part whose metadata identifies the
  upstream schema. Its compatibility
  `SessionAdapter` and `MessageAdapter` mappings are not discovery identities
  and must not leak canonical `session_id` or `message_id` fields into these
  pages.
- Runtime absence, malformed pages, cursor cycles, identity mismatches, and SDK
  errors are provider failures. Production and development paths have no mock,
  private-storage, or synthetic discovery fallback.
- Conformance tests must cover multi-page draining, cursor advancement and
  cycles, page and item Session affinity, duplicate identities, the 10,000-item
  bound, invalid part shapes, working-directory propagation, and deterministic
  loading of the declared official package.

### 6.2 User-Mediated Server Requests

A long-lived provider transport may receive a request that pauses the active
Turn until the user responds. The transport adapter owns the provider wire
method and exact JSON-RPC request-id type; shared runtime events use a canonical
Session Interaction envelope.

The Codex app-server adapter maps the supported request family as follows:

| Canonical category | Canonical kind | Provider request |
| --- | --- | --- |
| `approval` | `command_execution` | command execution approval |
| `approval` | `file_change` | file-change approval |
| `approval` | `permission_profile` | permission-profile approval |
| `user_input` | `question_set` | one or more questions keyed by stable question id |
| `user_input` | `onboarding_question_set` | desktop onboarding dynamic tool with one to three structured questions |
| `user_input` | `option_picker` | direct or dynamic single/multiple option picker |
| `user_input` | `context_source_picker` | direct or dynamic context-source picker |
| `setup` | `setup_step` | incomplete role, task, or context setup step |
| `elicitation` | `mcp_elicitation` | MCP form, OpenAI form, or URL elicitation |

Rules:

- The envelope `MUST` carry canonical `sessionId`, category, kind, prompt,
  allowed actions, typed request data, and provider correlation metadata.
- Correlation `MUST` preserve `providerSessionId`, provider Turn and item ids,
  the exact `string | number` provider request id and its wire type, and the
  adapter-private protocol method. Provider-specific Session names remain raw
  wire evidence only.
- Command responses preserve six distinct actions, including Session scope,
  exec-policy amendment and network-policy amendment. File-change responses
  preserve four distinct actions.
- Question responses preserve an answer-array map keyed by question id and the
  request preserves `autoResolutionMs`, headers, other/secret flags and nullable
  options.
- MCP elicitation preserves mode, schema or URL, elicitation id, structured
  content and metadata. Permission approval preserves requested/granted
  profiles, Turn-or-Session scope and optional strict automatic review.
- Desktop onboarding preserves its one-to-three-question and minimum-two-option
  constraints. Option-picker requests preserve multiple-selection mode and
  nullable submit/skip labels. Context-source and setup-step requests preserve
  their distinct action sets.
- Dynamic onboarding, option-picker, context-source, and setup-step resolutions
  compile to `success: true` with one `inputText` content item whose text is the
  JSON-encoded typed payload. The two direct desktop request methods return the
  typed payload without that dynamic-tool envelope.
- Malformed `request_option_picker`, `request_onboarding_input`, and
  `setup_codex_step` arguments receive the desktop-compatible failed dynamic
  tool result and never become a canonical Interaction.
- Canonical resolutions are compiled back to the exact provider response only
  inside the provider adapter. Unknown request methods, actions, question ids,
  malformed amendments, or invalid permission scopes fail closed.
- A response is sent at most once for one exact provider request on one
  connection. A provider resolution notification clears runtime state but is
  not proof that this client sent the response.
- Business persistence and App SDK exposure remain owned by `sdkwork-agents`;
  products do not persist raw provider requests or call this transport directly.

Executable evidence:

```bash
node --test scripts/provider-transport-workers/codex-app-server-interactions.test.mjs
node --test scripts/provider-transport-workers/generic-ts-sdk-worker-app-server.test.mjs
node --test scripts/provider-transport-workers/codex-app-server-live.test.mjs
```

### 6.3 Host Auto-Responses

Provider requests owned entirely by the Kernel host do not become product
Interactions. The Codex app-server `currentTime/read` request and the valid
`setup_codex_step` completion signal are handled inside the resident transport
runtime.

Rules:

- The adapter `MUST` validate the exact string-or-number request id and the
  provider Session affinity before responding.
- The response `MUST` contain `currentTimeAt` as whole Unix seconds. The clock
  is injectable for contract tests and `MUST` fail closed when it returns a
  negative, fractional, non-finite, or unsafe millisecond timestamp.
- The provider Session wire field remains adapter-private. The response does
  not create a canonical Session item or enter Agents business persistence.
- A valid setup completion signal returns a successful dynamic-tool envelope
  containing `{ "completed": true }`. Role, task, and context setup steps remain
  user-mediated Interactions.
- Dynamic tools, token refresh, and attestation require separate typed host
  ports. Setup completion is the only registered setup-tool exception; ordinary
  dynamic tools `MUST NOT` be routed through the clock handler or exposed as raw
  product requests.

Executable evidence:

```bash
node --test scripts/provider-transport-workers/codex-app-server-host-requests.test.mjs
node --test scripts/provider-transport-workers/generic-ts-sdk-worker-app-server.test.mjs
```

## 7. Extension Rules

Adding a new external framework `SHOULD` require only:

1. `bindings/agent-providers/<framework>/provider-binding.manifest.json`
2. `agent-providers/crates/sdkwork-agent-provider-<framework>`
3. conformance tests in the provider crate

## 8. Verification

```bash
cargo test -p sdkwork-agent-provider-spi
cargo test -p sdkwork-agent-provider-transport-core
cargo test -p sdkwork-agent-provider-codex
node scripts/check-agent-provider-bindings.mjs
node scripts/check-kernel-standards.mjs
```
