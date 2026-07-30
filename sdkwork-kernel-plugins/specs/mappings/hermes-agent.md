# Hermes Agent Mapping

## Source

- Local path: `external/hermes-agent` (source reference and inspection input, not a Python runtime dependency)
- Upstream: `https://github.com/NousResearch/hermes-agent.git`
- PyPI distribution: `hermes-agent` (import probe module: `run_agent`)
- Python entry module: `run_agent`
- TUI gateway entry: `tui_gateway.entry`
- TUI gateway protocol implementation: `external/hermes-agent/tui_gateway/server.py`

The binding references an installed, importable Python package. The checked-out
source tree is evidence for capability mapping only; kernel crates and workers
must not treat `external/hermes-agent` as an installed package unless the host
explicitly exposes that tree through its Python environment.

## SDKWork Surface

Hermes Agent maps first to the general Agent Kernel process-adapter surface:

- `AgentRuntime` and provider-local session lifecycle state
- `ModelProvider` for one-shot model execution through a negotiated transport
- `ToolProvider` and `AgentSkillProvider` only after a policy-aware gateway
  adapter can invoke those surfaces independently
- `ProtocolAdapter` for the Hermes TUI JSON-RPC gateway
- `ContextProvider` and `MemoryProvider` only through the agents composition
  boundary, preserving provenance, user/session scope, and redaction metadata
- `AgentCollaborationProvider` and `TaskSchedulingProvider` only after the
  gateway's delegation and cron contracts are translated into typed kernel SPI

The current upstream gateway publishes `session.create`, `session.resume`,
`session.history`, `prompt.submit`, `approval.respond`, `tools.list`,
`skills.manage`, `llm.oneshot`, and observable tool/approval events. Those are
source mappings, not a license to call the gateway without the kernel policy,
audit, cancellation, and event translation boundary.

## Initial Registration Mode

`process-adapter`

`sdkwork-agent-provider-hermes` supplies the agent/package/plugin manifests,
message adapters, provider-local lifecycle provider, negotiated transport
bootstrap, and standard policy/installation/configuration providers. Direct
in-process model calls fail closed with `ProviderUnavailable`; Hermes-internal
tool activity is an event surface, not a standalone provider.

The optional `SDKWORK_HERMES_USE_TUI_GATEWAY=1` selection path chooses the
`jsonrpc_stdio` backend for `sdk.model.chat`. The Python worker starts the
installed `tui_gateway.entry` process and invokes its `llm.oneshot` JSON-RPC
method for a model request. This stateless upstream method selects the Hermes
configured auxiliary model; it intentionally rejects a kernel request that
requires a per-request `model_id`, because the upstream method has no matching
parameter.

The model bridge is not proof that the full Hermes gateway protocol is mapped.
Session, approval, progress, cancellation, tool, skill, and error objects must
still be translated before it can claim protocol-adapter or local-runtime
conformance. `llm.oneshot` also has no matching incremental stream contract,
so the binding does not declare `model_chat_stream`.

## Capability Mapping

| Upstream area | SDKWork capability family | Current boundary |
| --- | --- | --- |
| TUI session create/resume/history/close | `sdk.session.lifecycle`, `agent.session.*` | Provider-local lifecycle state; no fake remote lifecycle RPC |
| `llm.oneshot` and agent chat | `sdk.model.chat` | TUI JSON-RPC model call when `SDKWORK_HERMES_USE_TUI_GATEWAY=1` and the installed gateway is healthy; no per-request model override |
| Streaming prompt and tool progress events | `agent.model.*`, `agent.tool.*`, `agent.runtime.*` | Mapping deferred until the gateway stream preserves IDs and ordering |
| Tool registry and agent tool execution | `agent.tool.*`, `tool.*` | Observed within Hermes agent execution; no independent kernel invocation claim |
| Skills manager and preloaded skills | `agent.skill.*`, `skill.*` | Observed within Hermes agent execution; no independent kernel invocation claim |
| `approval.request` / `approval.respond` | `agent.policy.*` | Must translate to `PolicyRequest` and `PolicyDecision`; upstream approval must not bypass kernel policy |
| Delegation and subagent events | `agent.collaboration.*` | Deferred |
| Hermes cron management | `agent.task_scheduling.*` | Deferred |

Binding execution is explicit:

- `sdk.session.lifecycle` is provider-local lifecycle state with
  `execution_scope: provider_local` and `runtime_operations: ["ping"]`.
  Create, resume, close, and history must use the typed provider-local
  lifecycle implementation rather than pretending that a transport ping is a
  session API.
- Model capability uses
  `execution_scope: transport_runtime`. The selected backend's
  `runtime_operations` array is the runtime operation allowlist. The router
  rejects an operation before worker invocation when it is not listed.
- A successful ping proves only transport health. It does not prove model,
  tool, skill, approval, or session execution.

## Policy Boundaries

Hermes tools can execute commands, modify files, read secrets, send network
requests, delegate work, and interact with external channels. Before such
operations execute, the adapter must create SDKWork policy requests for
`tool.invoke`, `host.filesystem.*`, `host.process.execute`,
`host.network.connect`, `host.secrets.read`, and `protocol.send`; audit-required
decisions must remain observable.

`hermes -z` / `hermes_cli.oneshot.run_oneshot` deliberately sets
`HERMES_YOLO_MODE=1` and `HERMES_ACCEPT_HOOKS=1`. It is therefore not an
approved production transport for SDKWork: using it would bypass the required
kernel policy and approval boundary. The standalone `llm.oneshot` gateway
method is model-only, does not mutate session history, and still requires a
typed transport adapter plus model invocation policy before it can become the
production model lane.

Third-party tool output, upstream events, session transcripts, and retrieved
memory remain untrusted input until kernel policy assigns a narrower trust
classification. Raw provider keys, gateway tokens, and session secrets must
never appear in manifests, diagnostics, mappings, or event payloads.

## Event And Error Mapping

Gateway session, prompt, model, tool, approval, and subagent events should map
to `agent.session.*`, `agent.model.*`, `agent.tool.*`, `agent.policy.*`, and
`agent.runtime.*`. Tool output needs untrusted provenance and redaction.

- Missing Python module, worker, or gateway: `provider_unavailable`
- Gateway timeout: `timeout`
- Gateway cancellation/interrupt: `cancelled`
- Policy denial or missing approval: `policy_denied` / `permission_required`
- Gateway protocol or provider failure: `provider_error`
- Unsupported independent tool or skill dispatch: `capability_missing`

## Conformance

Current evidence:

- Binding manifest parses and negotiates.
- Provider crate contract tests cover manifests, adapters, typed lifecycle, and
  direct fail-closed providers.
- `node scripts/provider-transport-workers/generic-python-sdk-worker.test.mjs`
  proves the Python worker's production fail-closed contract and a TUI
  `llm.oneshot` JSON-RPC model call using an isolated gateway fixture.
- `node scripts/provider-transport-workers/hermes-gateway-staging.test.mjs`
  proves the opt-in staging-gate configuration, not a live model or tool turn.

Release requires a Hermes-specific staging gateway proof with an installed
Hermes runtime, a configured non-test model credential, gateway health, a
model request, typed event/trace correlation, cancellation, and a policy
denial/approval assertion. The Hermes-specific staging gateway proof is
separate from `engine-sdk-live-staging.mjs`; the Node SDK gate must not be used
as Hermes coverage.

## Status

- Provider crate: `agent-providers/crates/sdkwork-agent-provider-hermes`
- Binding: `bindings/agent-providers/hermes/provider-binding.manifest.json`
- Client bridge: `sdkwork-agent-client` `builtin.hermes` routes local chat
  through `HermesSdkIntegration`; remote mode uses internal-api `SseChatClient`
- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=hermes`
- Local source pin (2026-06-24, not a latest-registry claim):
  `external/hermes-agent` @ `a4a74ca9e` with source `pyproject.toml` version
  `0.17.0`
- Managed installer registry pin (verified 2026-07-30):
  `hermes-agent==0.19.0`
- Default runtime worker: `PythonSdkBackendRuntime` through
  `scripts/provider-transport-workers/generic_python_sdk_worker.py`, probing
  `run_agent`
- Optional IPC selection: `SDKWORK_HERMES_USE_TUI_GATEWAY=1` prefers the
  `jsonrpc_stdio` backend via the `tui_gateway` module
- Merge proof: `node scripts/provider-transport-workers/generic-python-sdk-worker.test.mjs`
- Release proof: Hermes-specific staging gateway proof through
  `scripts/provider-transport-workers/hermes-gateway-staging.mjs`

The binding is structurally integrated and fails closed in production. It is
not a complete Hermes runtime adapter yet: no production proof currently
exercises a real model turn, and tool/skill/delegation/cron/TUI approval event
translation remains incomplete. Those gaps must remain visible in diagnostics
and conformance reporting rather than being hidden by development mock output.
