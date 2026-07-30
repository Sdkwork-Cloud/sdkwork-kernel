# SDKWork Kernel — Module Reference (As-Built)

Status: active
Owner: SDKWork kernel maintainers
Updated: 2026-06-26
Parent: [TECH_ARCHITECTURE.md](TECH_ARCHITECTURE.md)

Dense module reference for implementers. Normative behavior remains in `specs/`.

## 1. Workspace Members (Cargo)

| Crate | Layer | Purpose |
| --- | --- | --- |
| `sdkwork-agent-kernel` | L0 | Core SPI traits and types |
| `sdkwork-agent-provider-spi` | L1 | Bindings, drivers, negotiation, runtime router |
| `sdkwork-agent-provider-transport-core` | L2 | Registry, bootstrap, transport host traits |
| `sdkwork-agent-provider-transport-{ipc,rust,node,python}` | L2 | Transport implementations + workers |
| `sdkwork-agent-provider-{framework}` | L3 | Per-framework integration (under `agent-providers/crates/`) |
| `sdkwork-agent-provider-core` | L1 helper | Mapping, mock policy (in `sdkwork-kernel-plugins/`) |
| `sdkwork-agent-plugin-core` | Plugin | `SdkworkKernelPlugin` trait |
| `sdkwork-agent-server` | Runtime | Hosted HTTP server, plugin bootstrap |
| `sdkwork-agent-client` | Client | Bridge plugins, local SQLite, remote SSE |
| `sdkwork-agent-database` | Runtime | SQLx pool bootstrap for sessions |
| `sdkwork-agent-session` | Runtime | Session persistence adapters |
| `sdkwork-agent-streaming` | Runtime | Stream/SSE helpers |
| `sdkwork-agent-api-bridge` | Runtime | Internal API wiring helpers |
| `sdkwork-code-kernel` | Code | Workspace/VCS/patch/terminal SPI |
| `sdkwork-routes-agent-internal-*` | API | Internal route manifests + axum handlers |
| `sdkwork-kernel-plugin-drive` | Plugin | Drive host integration |
| `sdkwork-kernel-plugin-knowledgebase` | Plugin | KB contract integration |

## 2. Key Source Entrypoints

| Concern | File |
| --- | --- |
| Server plugin bootstrap | `sdkwork-agent-server/src/runtime_bootstrap.rs` |
| Hosted agent registry | `sdkwork-agent-server/src/agent_registry.rs` |
| Client bridge modes | `sdkwork-agent-client/src/bridge/client.rs` |
| Ingress auth | `sdkwork-agent-client/src/ingress_auth.rs` |
| Transport registry | `sdkwork-agent-provider-transport-core/src/registry.rs` |
| Transport bootstrap | `sdkwork-agent-provider-transport-core/src/bootstrap.rs` |
| Node worker path | `sdkwork-agent-provider-transport-node/src/worker_runtime.rs` |
| Python worker path | `sdkwork-agent-provider-transport-python/src/worker_runtime.rs` |
| Mock policy | `sdkwork-kernel-plugins/crates/sdkwork-agent-provider-core/src/mock_policy.rs` |
| Provider sdk_integration | `agent-providers/crates/sdkwork-agent-provider-*/src/sdk_integration.rs` |

## 3. Binding Catalog Layout

```text
bindings/agent-providers/
  codex/provider-binding.manifest.json
  claude-code/provider-binding.manifest.json
  gemini-cli/provider-binding.manifest.json
  opencode/provider-binding.manifest.json
  openclaw/provider-binding.manifest.json
  hermes/provider-binding.manifest.json
  rig/provider-binding.manifest.json
```

Schema: `specs/schemas/agent-sdk-binding.schema.json`

## 4. Provider Bootstrap Sequence

```text
AgentSdkBindingManifest::from_json(...)
        → bootstrap_binding() / bindings.negotiate()
        → ProviderTransportBootstrap::new()
        → register_host(*TransportHost)
        → with_*_runtime(SdkBackendRuntime)
        → finalize_pair(negotiation)
        → SdkRuntimeBackedModelProvider / ToolProvider
```

## 5. Client Bridge Builtins

| Plugin id | Provider integration type |
| --- | --- |
| `builtin.codex` | `CodexSdkIntegration` |
| `builtin.openclaw` | `OpenClawSdkIntegration` |
| `builtin.hermes` | `HermesSdkIntegration` |
| `builtin.zeroclaw` | Fail-closed placeholder |

Local streaming: **not supported** on SDK bridges; use Remote + SSE.

## 6. Environment Variables (Common)

| Variable | Scope | Purpose |
| --- | --- | --- |
| `SDKWORK_KERNEL_AGENT_PLUGIN` | Server | Active hosted provider |
| `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` | Server/dev | Allow mock model paths |
| `SDKWORK_KERNEL_INGRESS_AUTH_MODE` | Server | Ingress auth profile |
| `SDKWORK_DATABASE_FILE` | Client | Local SQLite sessions |
| `SDKWORK_HERMES_USE_TUI_GATEWAY` | Hermes | Prefer IPC over Python module |
| `SDKWORK_DATABASE_*` | Server | Workspace PostgreSQL runtime persistence |

Full list: `specs/topology.spec.json` → `envKeys`.

## 7. Related Specifications

| Spec | Topic |
| --- | --- |
| `AGENT_KERNEL_SPEC.md` | Object model |
| `AGENT_PROVIDER_INTEGRATION_SPEC.md` | Integration rules |
| `AGENT_PROVIDER_BINDING_SPEC.md` | Manifest contract |
| `AGENT_RUNTIME_SPEC.md` | Bootstrap and lifecycle |
| `CODE_KERNEL_SPEC.md` | Code-agent SPI |
| `KERNEL_PLUGIN_SPEC.md` | Plugin manifests |
| `KERNEL_PRODUCT_PROJECTION_SPEC.md` | BirdCoder event mapping |
