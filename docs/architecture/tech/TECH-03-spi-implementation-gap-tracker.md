# SDKWork Kernel — SPI Implementation And Gap Tracker

Status: active
Owner: SDKWork kernel maintainers
Updated: 2026-07-28
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
| sandbox (legacy host command) | Legacy `SandboxProvider` | `SANDBOX_PROVIDER_SPEC.md` | `sandbox.rs`, `host_sandbox.rs` | Existing one-shot HostProvider wrapper only; not the `sdkwork-sandbox` lifecycle or a production isolation claim |
| secret | `SecretProvider` | `SECRET_PROVIDER_SPEC.md` | `secret.rs`, `secret_env.rs`, `secret_composite.rs` | Implementation present; target secret-manager integration and rotation evidence required |
| rate_limit | `RateLimitProvider` | `AGENT_KERNEL_SPEC.md` §3.4 | `rate_limit.rs`, `ingress_rate_limit.rs` | Implementation present; Redis-backed cluster failure drills required |
| cancellation | `CancellationProvider` | `AGENT_KERNEL_SPEC.md` §3.4 | `cancellation.rs` | Implementation present; request-scoped transport cancellation and stress evidence required |
| model_stream | `ModelStreamProvider` | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` | `model_stream.rs`, `model.rs` | Implementation present; live-binding slow-consumer/disconnect/soak evidence required |
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
| G-01 | Production Sandbox Runtime composition is incomplete | P0 | `sdkwork-sandbox` PRD/technical architecture | Kernel `SandboxSessionLifecycleAdapter` is implemented; durable lifecycle host, constrained Provider, attachment persistence and cross-platform conformance remain pending | Codex-class production execution isolation | Deliver reviewed `sdkwork-sandbox` Provider profiles and common lifecycle/security conformance |
| G-02 | A2A protocol adapter incomplete | P0 | `A2A_PROTOCOL_ADAPTER_SPEC.md` | Local types/registry adapter implemented; remote HTTP conformance pending | External HTTP A2A interop | Remote HTTP adapter + conformance suite |
| G-03 | Orchestration not connected to planning loop | P0 | `MULTI_AGENT_ORCHESTRATION_SPEC.md` | Conversion helper implemented; caller integration evidence pending | Multi-agent workflows | Wire planning provider callers to orchestration bridge |
| G-04 | Backend health monitor not spawned in all server profiles | P0 | `BACKEND_HEALTH_MONITOR_SPEC.md` | Worker bootstrap implemented; composed production readiness pending | Stale backend selection | Add required provider/Redis readiness and diagnostics evidence |
| G-05 | Model streaming not universal across providers | P1 | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` | Incremental bridge/IPC/SSE implemented; per-binding live soak pending | BirdCoder live transcript | Run cancellation, disconnect, slow-consumer, and live SDK soak per binding |
| G-06 | Cancellation not propagated to subprocess transports | P1 | `cancellation.rs` | Transport cancellation implementation present; isolation/stress evidence pending | Long tool runs | Verify request-scoped cancellation without collateral worker turns |
| G-07 | Secret vault backends (HashiCorp, cloud SM) | P1 | `SECRET_PROVIDER_SPEC.md` | Env/file/Vault implementation present; production secret-manager and rotation evidence pending | Enterprise deploy | Add target cloud SM adapters and rotation drills |
| G-08 | Rate limit provider vs server middleware duplication | P1 | `rate_limit.rs` | Shared provider and Redis path implemented; cluster failure semantics require release drills | Quota consistency | Verify fail-closed Redis outage and metric parity in staging |
| G-09 | Provider fallback chains in binding manifests | P2 | `AGENT_PROVIDER_BINDING_SPEC.md` | Single-backend selection | Resilience | Add `fallback_backends` to manifest schema |
| G-10 | Context compression provider | P2 | `AGENT_CONTEXT_MEMORY_SPEC.md` | `ContextProvider::trim` only | Long sessions | Add optional `ContextCompressionProvider` or extend context SPI |
| G-11 | Artifact durable storage provider | P2 | `AGENT_KERNEL_SPEC.md` | `AgentArtifact` object only | Task outputs | Add `ArtifactStorageProvider` or delegate to `sdkwork-drive` via agents |
| G-12 | MiMo Code facade/live SDK proof pending | P1 | `AGENT_PROVIDER_BINDING_SPEC.md` | Binding/provider implementation present; agents facade and live proof pending | BirdCoder engine parity | Add agents facade registration and staging live SDK proof |
| G-13 | Live official SDK staging gate | P1 | `AGENT_PROVIDER_INTEGRATION_SPEC.md` | Workflow/scripts present; target credentials and release-run evidence required | Commercial confidence | Populate protected staging inputs and record release-train invokes |
| G-14 | Kernel mock fail-closed in release builds | P0 | `AGENT_RUNTIME_SPEC.md` | Policy/preflight/test implementation present; exact-release gate required | Production safety | Keep the release step mandatory in CI/promotion |
| G-15 | Multimodal interaction contract runtime wiring | P0 | `AGENT_KERNEL_SPEC.md` §6.4 | Runtime wiring present; live provider conformance pending | Voice/vision agents | Live SDK multimodal wire in bindings |
| G-16 | Model providers consume structured `input_messages` | P1 | `AGENT_MODEL_PROVIDER_SPI_SPEC.md` §4.1 | Structured wire implementation present; per-package live conformance pending | Native OpenAI/Anthropic multimodal | Per-package live multimodal conformance |
| G-17 | Protocol adapter structured ingress (non-RPC) | P1 | `AGENT_PROTOCOL_ADAPTER_SPEC.md` | Chat RPC parsing present; remote HTTP ingress pending | HTTP multimodal upload | Remote A2A HTTP ingress |
| G-18 | Developer-friendly Agent SPI (`api` module) | P1 | `AGENT_KERNEL_SPEC.md` §3.5 | Authored API types present; live provider adoption evidence pending | Provider-native multimodal egress | Live provider adoption |

| G-19 | Durable runtime task execution | P0 | `AGENT_RUNTIME_SPEC.md` section 6 | v5 task/run/step persistence, async submit, lease/fencing, controls, retry creation, and a bounded single-model-step worker are implemented; planner, multi-step tool reconciliation, automatic retry/backoff, request-id persistence, and real cancellation are pending | Complex submitted tasks cannot yet complete safely | Continue [ADR-20260716](../decisions/ADR-20260716-durable-runtime-execution.md) with multi-step execution and target PostgreSQL/failure/load proof |
| G-20 | Permission approval execution resume | P0 release gate | `AGENT_RUNTIME_SPEC.md` section 8 | SQLite/PostgreSQL encrypted operations, atomic decisions/expiry, `SKIP LOCKED`/SQLite claims, fencing, policy/revision revalidation, original tool-call id, worker resume, and terminal crypto-erasure are implemented with local contracts | Target-environment correctness and operability are not yet evidenced | Run live PostgreSQL contention, provider idempotency/restart injection, key rotation, load/soak, metrics/dashboard, and rollout drills for the exact revision |
| G-21 | Cross-provider live session control parity | P1 | `AGENT_PROVIDER_INTEGRATION_SPEC.md` section 6.1.1 | Provider-neutral L0 extension, runtime registry, L1 operation allowlists, OpenCode official-SDK control, and Codex resident app-server `interrupt`/`compact`/`fork` with same-worker affinity are implemented; Claude streaming-query control is not wired | Users cannot yet control every provider session through one commercial runtime surface | Add the Claude adapter, context usage/settings/resource refresh actions, Codex/OpenCode cancellation stress, and staging live proof |
| G-22 | OpenCode source authority mismatch | P0 release gate | `AGENT_PROVIDER_INTEGRATION_SPEC.md` sections 3 and 6.1.1 | The pinned `external/opencode` gitlink at `73ee493` is the archived legacy Go project and does not contain `@opencode-ai/sdk@1.18.11`; exact npm tarball types were audited for the current adapter, but the repository source mirror remains non-authoritative | Source review, provenance checks, and reproducible staging builds cannot use the declared external tree | Human-review and pin the current official OpenCode source authority, then add package-source mapping and live session-control conformance against that revision |

## 3. sdkwork-agents Alignment Gaps

Kernel SPI alone does not deliver product value — agents must expose operations.

| Product need | Agents API | Kernel bridge | Status |
| --- | --- | --- | --- |
| Agent CRUD + composition | `/app/v3/api/ai/agents/*` | N/A | Implemented; sibling release gate required |
| Chat sessions + messages | `/app/v3/api/ai/agents/{id}/sessions/*` | Runtime facade turn | Implemented; sibling contract gate required |
| Code engine bootstrap | `sdkwork-agents-runtime-facade` | Provider negotiation | Implemented; cross-repository verification required |
| Memory tier binding | composition slot `memory` | `MemoryProvider` at runtime | Composition implemented; backend variety and integration evidence remain in `sdkwork-memory` |
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
| B-04 | Live SDK backend (not inner mock) | Kernel | **Closed** - `engine-sdk-live-staging.mjs` + Hermes-specific `hermes-gateway-staging.mjs` + staging workflow; merge pipeline stays credential-free |
| B-05 | Agents API for agent-managed chat vs coding session | BirdCoder + Agents | Verify UX uses agents SDK where product requires managed agents |

## 5. Commercial Readiness Scorecard

| Dimension | Score | Blockers |
| --- | --- | --- |
| Architecture / layering | **Implemented; gate required** | Dependency rules require cross-repository verification |
| SPI completeness (spec) | **Implemented with open gaps** | Extension specs exist; remote A2A and orchestration caller wiring remain open |
| SPI completeness (runtime wiring) | **Implemented; target evidence required** | Secret-manager, provider readiness, and live binding evidence remain required |
| Provider catalog | **Release gate** | Protected staging inputs and Hermes-specific gateway proof are required |
| Agents API + SDK | **Release gate** | Pagination/search and sibling facade conformance remain required |
| Security / fail-closed | **Release gate** | Dedicated secrets, Redis outage drills, provider cancellation, and exact-release tests remain required |
| Operability (HA, observability) | **Target-environment gate** | Managed HA services, NetworkPolicy, image digest, restore/failover/load evidence required |
| Go-to-market artifacts | **Blocked** | Published registry, immutable container evidence, and release promotion records remain open |

**Commercial landing verdict:** **Not approved for production or GA.** A controlled
beta/pilot can proceed only after the exact release revision passes repository
gates and the target environment supplies managed HA data services, protected
secrets, immutable artifacts, NetworkPolicy, restore/failover/load evidence,
provider live proof, and sibling-repository approval. Remaining gaps are release
blockers, not documentation-only debt.

Improvement plan owner: [REQ-2026-0001](../../product/requirements/REQ-2026-0001-commercial-hardening.md).

## 6. Closure Checklist (Kernel Spec System)

Cross-reference `specs/README.md` standard closure checklist. Outstanding:

- [ ] Every compatibility claim backed by conformance profile run in CI
- [ ] Remote A2A HTTP adapter conformance suite
- [ ] `sdkwork-sandbox` production Provider isolation and lifecycle conformance on supported OS/CI profiles
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
