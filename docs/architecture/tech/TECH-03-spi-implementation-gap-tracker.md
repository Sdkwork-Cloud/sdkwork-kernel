# SDKWork Kernel — SPI Implementation And Gap Tracker

Status: active
Owner: SDKWork kernel maintainers
Updated: 2026-07-07
Parent: [TECH_ARCHITECTURE.md](TECH_ARCHITECTURE.md)
Specs: [AGENT_KERNEL_SPEC.md](../../../specs/AGENT_KERNEL_SPEC.md), [AGENT_CONFORMANCE_SPEC.md](../../../specs/AGENT_CONFORMANCE_SPEC.md)

Living tracker for SPI **spec → implementation → product** closure. Update this
shard when traits, specs, or provider wiring change.

## 1. Provider Family Inventory

### 1.1 Core families (18) — required for full runtime manifests

| Family | Trait | Spec shard | Rust module | Runtime registry |
| --- | --- | --- | --- | --- |
| model | `ModelProvider` | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` | `model.rs` | Yes |
| tool | `ToolProvider` | `AGENT_TOOL_PROVIDER_SPI_SPEC.md` | `tool.rs` | Yes |
| policy | `PolicyProvider` | `AGENT_SECURITY_POLICY_SPEC.md` | `policy.rs` | Yes |
| context | `ContextProvider` | `AGENT_CONTEXT_MEMORY_SPEC.md` | `context_memory.rs` | Yes |
| memory | `MemoryProvider` | `AGENT_CONTEXT_MEMORY_SPEC.md` | `context_memory.rs` | Yes |
| knowledge | `KnowledgeProvider` | `AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md` | `knowledge.rs` | Yes |
| planning | `PlanningProvider` | `AGENT_PLANNING_EXECUTION_SPEC.md` | `planning.rs` | Yes |
| host | `HostProvider` | `AGENT_HOST_PROVIDER_SPI_SPEC.md` | `host.rs` | Yes |
| protocol_adapter | `ProtocolAdapter` | `AGENT_PROTOCOL_ADAPTER_SPEC.md` | `protocol.rs` | Yes |
| mcp | `McpProvider` | `AGENT_MCP_PROVIDER_SPI_SPEC.md` | `mcp.rs` | Yes |
| skill | `AgentSkillProvider` | `AGENT_SKILL_PROVIDER_SPI_SPEC.md` | `skill.rs` | Yes |
| collaboration | `AgentCollaborationProvider` | `AGENT_COLLABORATION_SPI_SPEC.md` | `collaboration.rs` | Yes |
| telemetry | `TelemetryProvider` | `AGENT_EVENT_TELEMETRY_SPEC.md` | `telemetry.rs` | Yes |
| task_scheduling | `TaskSchedulingProvider` | `AGENT_PLANNING_EXECUTION_SPEC.md` | `task_scheduling.rs` | Yes |
| agent_classification | `AgentClassificationProvider` | `AGENT_KERNEL_SPEC.md` | `classification.rs` | Yes |
| message_query | `MessageQueryProvider` | `AGENT_KERNEL_SPEC.md` | `message_query.rs` | Yes |
| agent_installer | `AgentInstaller` | `AGENT_INSTALLATION_CONFIGURATION_SPEC.md` | `installation.rs` | Yes |
| agent_configuration | `AgentConfigurationProvider` | `AGENT_INSTALLATION_CONFIGURATION_SPEC.md` | `configuration.rs` | Yes |

### 1.2 Extension families (6) — production hardening and advanced orchestration

| Family | Trait | Spec shard | Rust module | Wired in default runtime |
| --- | --- | --- | --- | --- |
| sandbox | `SandboxProvider` | `SANDBOX_PROVIDER_SPEC.md` | `sandbox.rs`, `host_sandbox.rs` | Yes — `RuntimeBuilder::enable_platform_host_sandbox()` wraps host `process` |
| secret | `SecretProvider` | `SECRET_PROVIDER_SPEC.md` | `secret.rs`, `secret_env.rs`, `secret_composite.rs` | **Closed (pre-prod)** — `ChainedSecretProvider` (env/file + optional Vault feature); enterprise cloud SM deferred |
| rate_limit | `RateLimitProvider` | `AGENT_KERNEL_SPEC.md` §3.4 | `rate_limit.rs`, `ingress_rate_limit.rs` | **Closed** — HTTP ingress uses `TokenBucketRateLimitProvider`; Redis distributed path remains server-owned |
| cancellation | `CancellationProvider` | `AGENT_KERNEL_SPEC.md` §3.4 | `cancellation.rs` | **Closed** — `SdkBackendRuntime::cancel_inflight` + worker kill/respawn |
| model_stream | `ModelStreamProvider` | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` | `model_stream.rs`, `model.rs` | **Closed** — `ModelStreamSink` + IPC NDJSON + HTTP SSE incremental path; `finalize_stream` releases in-memory stream capacity |
| backend_health | `BackendHealthMonitor` | `BACKEND_HEALTH_MONITOR_SPEC.md` | `backend_health.rs` | Yes — `BackendHealthWorker` spawned in `sdkwork-agent-server` bootstrap |

### 1.3 Orchestration primitives (not a separate provider family yet)

| Component | Spec | Rust module | Status |
| --- | --- | --- | --- |
| Multi-agent plan/graph | `MULTI_AGENT_ORCHESTRATION_SPEC.md` | `orchestration.rs` | `OrchestrationPlan::into_planning_plan()` bridges to `PlanningProvider` |
| A2A object mapping | `A2A_PROTOCOL_ADAPTER_SPEC.md` | `a2a_protocol.rs` | Types + `A2AProtocolAdapter` trait; HTTP/registry adapter pending |

## 2. Gap Register

Priority aligns with [ADR-20260628](../decisions/ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md).

| ID | Gap | Priority | Spec | Implementation | Product impact | Next action |
| --- | --- | --- | --- | --- | --- | --- |
| G-01 | Sandbox not default-routed from HostProvider | P0 | `SANDBOX_PROVIDER_SPEC.md` | **Closed** — `SandboxingHostProvider` + platform sandbox on bootstrap | Codex-class prod isolation | Linux sandbox conformance CI (follow-up) |
| G-02 | A2A protocol adapter incomplete | P0 | `A2A_PROTOCOL_ADAPTER_SPEC.md` | **Closed** — structured `A2ATaskRequest.messages`, `RegistryA2AProtocolAdapter`, multimodal task tests | External HTTP A2A interop | Remote HTTP adapter + conformance suite |
| G-03 | Orchestration not connected to planning loop | P0 | `MULTI_AGENT_ORCHESTRATION_SPEC.md` | **Closed** — `OrchestrationPlan::into_planning_plan()` | Multi-agent workflows | Wire planning provider callers to orchestration bridge |
| G-04 | Backend health monitor not spawned in all server profiles | P0 | `BACKEND_HEALTH_MONITOR_SPEC.md` | **Closed** — `BackendHealthWorker` in server bootstrap | Stale backend selection | Expose monitor snapshot on runtime diagnostics API (optional) |
| G-05 | Model streaming not universal across providers | P1 | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` | **Closed** — `ModelStreamSink` + `ModelProvider::stream_into`; IPC NDJSON `stream.chunk`/`stream.done` frames; HTTP SSE pushes chunks as emitted; finalized in-memory streams release provider capacity | BirdCoder live transcript | Live SDK incremental SSE per binding (optional hardening) |
| G-06 | Cancellation not propagated to subprocess transports | P1 | `cancellation.rs` | **Closed** — `SpawnedWorker`, `SdkBackendRuntime::cancel_inflight`, Node/Python worker kill + respawn | Long tool runs | Wire `CancellationProvider` to runtime router callers |
| G-07 | Secret vault backends (HashiCorp, cloud SM) | P1 | `SECRET_PROVIDER_SPEC.md` | **Closed (pre-prod)** — `ChainedSecretProvider` + `EnvFileSecretProvider` + `VaultSecretProvider` (`secret-vault` feature) | Enterprise deploy | Cloud SM adapters behind separate feature flags |
| G-08 | Rate limit provider vs server middleware duplication | P1 | `rate_limit.rs` | **Closed** — `TokenBucketRateLimitProvider` SPI backs in-process + Redis fail-over; Redis Lua stays server transport | Quota consistency | Monitor Redis/SPI metric parity in production |
| G-09 | Provider fallback chains in binding manifests | P2 | `AGENT_PROVIDER_BINDING_SPEC.md` | Single-backend selection | Resilience | Add `fallback_backends` to manifest schema |
| G-10 | Context compression provider | P2 | `AGENT_CONTEXT_MEMORY_SPEC.md` | `ContextProvider::trim` only | Long sessions | Add optional `ContextCompressionProvider` or extend context SPI |
| G-11 | Artifact durable storage provider | P2 | `AGENT_KERNEL_SPEC.md` | `AgentArtifact` object only | Task outputs | Add `ArtifactStorageProvider` or delegate to `sdkwork-drive` via agents |
| G-12 | Mimo Code binding manifest missing | P1 | `AGENT_PROVIDER_BINDING_SPEC.md` | **Closed** — `bindings/agent-providers/mimo-code/` | BirdCoder engine parity | Add live SDK integration crate + staging gate |
| G-13 | Live official SDK staging gate | P1 | `AGENT_PROVIDER_INTEGRATION_SPEC.md` | **Closed** — `engine-sdk-live-staging.mjs` + `.github/workflows/kernel-staging-live-sdk.yml` (`workflow_dispatch`, credential-gated) | Commercial confidence | Populate staging repository secrets and schedule release train invokes |
| G-14 | Kernel mock fail-closed in release builds | P0 | `AGENT_RUNTIME_SPEC.md` | **Closed** — `mock_policy` + preflight rejects production mock override + `tests/release_mock_fail_closed.rs` | Production safety | Add release step to CI matrix |
| G-15 | Multimodal interaction contract runtime wiring | P0 | `AGENT_KERNEL_SPEC.md` §6.4 | **Closed** — `interaction_contract` on `AgentDefinition`; chat/execution/bridge → `ModelExecutionService` | Voice/vision agents | Live SDK multimodal wire in bindings |
| G-16 | Model providers consume structured `input_messages` | P1 | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` §4.1 | **Closed** — `model_wire.rs`, `SdkRuntimeRequest::from_model_request` + worker `wire_messages` | Native OpenAI/Anthropic multimodal | Per-package live multimodal conformance |
| G-17 | Protocol adapter structured ingress (non-RPC) | P1 | `AGENT_PROTOCOL_ADAPTER_SPEC.md` | **Closed** — chat RPC `parse_chat_rpc_payload` | HTTP multimodal upload | Remote A2A HTTP ingress |
| G-18 | Developer-friendly Agent SPI (`api` module) | P1 | `AGENT_KERNEL_SPEC.md` §3.5 | **Closed** — `ContentBlock`, `MessageBuilder`, `AgentInvokeRequest`, industry role mapping | Provider-native multimodal egress | Live provider adoption |

## 3. sdkwork-agents Alignment Gaps

Kernel SPI alone does not deliver product value — agents must expose operations.

| Product need | Agents API | Kernel bridge | Status |
| --- | --- | --- | --- |
| Agent CRUD + composition | `/app/v3/api/ai/agents/*` | N/A | Done |
| Chat sessions + messages | `/app/v3/api/ai/agents/{id}/sessions/*` | Runtime facade turn | Done |
| Code engine bootstrap | `sdkwork-agents-runtime-facade` | Provider negotiation | Done |
| Memory tier binding | composition slot `memory` | `MemoryProvider` at runtime | Done (composition); backend variety in `sdkwork-memory` |
| Task scheduling (business) | agents domain jobs (if exposed) | `TaskSchedulingProvider` | Verify API coverage for scheduled agent jobs |
| Message search (business) | `agents.messages.list` with `q` | `MessageQueryProvider` | Must follow `PAGINATION_SPEC.md` |

Full API inventory: `sdkwork-agents/docs/architecture/tech/TECH-api-specification.md` (70 operations).

**Rule:** If BirdCoder needs an operation not in agents APIs, add to agents OpenAPI
authority first — see `sdkwork-birdcoder/docs/architecture/tech/TECH-33-agents-birdcoder-boundariesstandard.md`.

## 4. sdkwork-birdcoder Alignment Gaps

| ID | Item | Owner | Status |
| --- | --- | --- | --- |
| B-01 | No direct `sdkwork-agent-provider-*` dependency | BirdCoder | Enforced by contract tests |
| B-02 | Engine turns via `sdkwork-agents-runtime-facade` | BirdCoder | Done |
| B-03 | Kernel event projection spec | Kernel | `KERNEL_PRODUCT_PROJECTION_SPEC.md` done |
| B-04 | Live SDK backend (not inner mock) | Kernel | **Closed** — `engine-sdk-live-staging.mjs` + staging workflow; merge pipeline stays credential-free |
| B-05 | Agents API for agent-managed chat vs coding session | BirdCoder + Agents | Verify UX uses agents SDK where product requires managed agents |

## 5. Commercial Readiness Scorecard

| Dimension | Score | Blockers |
| --- | --- | --- |
| Architecture / layering | **A** | Dependency rules enforced cross-repo |
| SPI completeness (spec) | **A-** | Extension specs exist; A2A/orchestration wiring open |
| SPI completeness (runtime wiring) | **A** | Optional cloud SM secret adapters |
| Provider catalog | **A-** | Staging secrets population for live SDK workflow |
| Agents API + SDK | **A-** | 70 ops; pagination/search conformance ongoing |
| Security / fail-closed | **A** | Optional cloud SM secret adapters |
| Operability (HA, observability) | **A-** | TECH_ARCHITECTURE §6–7 |
| Go-to-market artifacts | **C+** | Published registry, SBOM pipeline (P4) |

**Commercial landing verdict:** Suitable for **controlled beta** with Rig/Codex/Claude
paths and agents-managed composition. **Enterprise GA** requires REQ-2026-0001
artifact publishing evidence and populated staging credentials for live SDK gate.

Improvement plan owner: [REQ-2026-0001](../../product/requirements/REQ-2026-0001-commercial-hardening.md).

## 6. Closure Checklist (Kernel Spec System)

Cross-reference `specs/README.md` standard closure checklist. Outstanding:

- [ ] Every compatibility claim backed by conformance profile run in CI
- [ ] Remote A2A HTTP adapter conformance suite
- [ ] Sandbox isolation conformance suite on Linux CI
- [ ] Provider fallback manifest schema + validator

## 7. Verification

```bash
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml
cargo test --workspace
node scripts/check-agent-provider-bindings.mjs
node scripts/check-kernel-standards.mjs
```

Cross-repo:

```bash
cargo test -p sdkwork-agents-runtime-facade
cargo test -p sdkwork-birdcoder-kernel-bridge
node ../sdkwork-birdcoder/scripts/kernel-birdcoder-alignment-contract.test.mjs
```
