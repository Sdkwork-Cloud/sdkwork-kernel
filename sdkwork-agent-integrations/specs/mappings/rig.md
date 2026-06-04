# Rig Mapping

## Source

- Local path: `external/rig`
- Upstream: `https://github.com/0xPlaygrounds/rig.git`

## SDKWork Surface

Rig maps first to a complete SDKWork typed plugin:

- `ModelProvider`
- `ToolProvider`
- `PlanningProvider`
- `PolicyProvider` for deterministic local conformance
- `AgentInstaller`
- `AgentConfigurationProvider`
- `ContextProvider` where retrieval or context assembly is used

## Initial Registration Mode

`typed-local-provider`

Rig is Rust-native, so it is the first direct SDKWork Rust SPI adapter. The
SDKWork-owned implementation lives in
`sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig` and depends on
kernel SPI contracts, not on `sdkwork-agent-kernel` depending on Rig.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Model abstraction | `model.chat`, `model.streaming`, `model.tool_call` |
| Tool composition | `tool.invoke` |
| Agent orchestration | `planning.*` |
| Installation | `agent.install`, `agent.uninstall`, `agent.upgrade` |
| Configuration | `agent.configure`, secret-ref validation |
| Retrieval/context | `context.*`, `memory.*` when backed by durable state |

## Policy Boundaries

Model invocation with sensitive context, tool invocation, retrieval over
private documents, memory writes, and external sends must use SDKWork policy.
Rig adapters must not read secrets directly; they must consume SDKWork secret
references resolved through host providers.

## Event Mapping

Model, tool, planning, context, and policy activity should map to
`agent.model.*`, `agent.tool.*`, `agent.step.*`, `agent.context.*`, and
`agent.policy.*`.

## Error Mapping

Unknown model maps to `capability_missing`. Provider setup failure maps to
`provider_unavailable`. Invocation failure maps to `provider_error`.

## Conformance

Implemented target: local-runtime profile with typed model, tool, planning,
policy, installer, and configuration providers. Live upstream Rig execution
remains fail-closed until a feature-gated backend is deliberately configured.

## Status

Reference source is present. SDKWork adapter code is implemented as a
fail-closed local plugin with manifests, package lifecycle, configuration,
deployment snapshots, diagnostics, and conformance contract tests.
