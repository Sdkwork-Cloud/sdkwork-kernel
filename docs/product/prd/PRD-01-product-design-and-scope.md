# SDKWork Kernel — Product Design And Scope

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-06-26
Parent: [PRD.md](PRD.md)
Specs: [REQUIREMENTS_SPEC.md](../../../../sdkwork-specs/REQUIREMENTS_SPEC.md)

## 1. Product Positioning

SDKWork Kernel is the **shared intelligence mechanism layer** for the SDKWork
product family. It is not an end-user application. It provides:

- Agent runtime semantics (session, model, tool, skill, policy, events).
- Multi-framework provider integration (Codex, Claude Code, Gemini CLI, OpenCode,
  OpenClaw, Hermes, Rig, and future frameworks).
- Operational hosted runtime and internal API for sibling applications.
- Code-agent SPI for BirdCoder-class products.
- Client bridge for local/hybrid/remote agent sessions on desktop and mobile hosts.

Products consume kernel capabilities through **`sdkwork-agents`** (HTTP/SDK and
runtime facade), not by depending on `sdkwork-agent-provider-*` crates directly.

## 2. Target Users

| Persona | Primary need |
| --- | --- |
| Platform engineer | Add a framework once; all products inherit via agents facade |
| Product engineer | Bootstrap engines/runtime without per-vendor wiring |
| Agents application team | Compose kernel server router with business routes |
| SRE / release engineer | Deploy with topology profiles and fail-closed defaults |
| Security / compliance | Audit transport health, mock policy, ingress, secrets |

## 3. Product Goals

### 3.1 Mechanism goals

- Single agent object model across all integrated frameworks.
- Negotiated capability routing with explicit transport health and driver resolution.
- Fail-closed when required capabilities cannot be served in production.
- Stable internal runtime API independent of upstream SDK churn.
- Extensible provider onboarding: binding manifest + provider crate + agents facade hook.

### 3.2 Non-functional goals

| Attribute | Requirement |
| --- | --- |
| Cohesion | Kernel aggregates runtime + provider mechanisms; no product business logic |
| Coupling | Products depend on agents application layer, not provider crates |
| Performance | In-process Rust path preferred; subprocess transports only when required |
| Security | Mock/stub blocked in production; ingress auth on hosted runtime |
| Maintainability | One crate per framework; no duplicate plugin/adapter pairs |
| Operability | Topology-driven env; verifiable binding catalog and standards gates |

## 4. In-Scope Capabilities (Kernel-Owned)

| Domain | Deliverables |
| --- | --- |
| Agent SPI | `sdkwork-agent-kernel`, execution loop specs |
| Provider integration | `sdkwork-agent-provider-spi`, transport crates, `agent-providers/crates/*` |
| Binding catalog | `bindings/agent-providers/*/provider-binding.manifest.json` |
| Hosted runtime | `sdkwork-agent-server`, session DB, internal API |
| Client bridge | `sdkwork-agent-client`, builtin bridge plugins |
| Code kernel | `sdkwork-code-kernel` |
| Platform plugins | Drive, knowledgebase kernel plugins |
| Release | `sdkwork.app.config.json`, topology profiles, workflow manifest |

## 5. Out-of-Scope (Sibling Repositories)

| Domain | Owner |
| --- | --- |
| Managed agents CRUD, marketplace | `sdkwork-agents` |
| Open/app/backend agent HTTP + SDK families | `sdkwork-agents` |
| Code-engine runtime facade for products | `sdkwork-agents-runtime-facade` |
| IM messaging, social graph | `sdkwork-im` |
| RTC media runtime | `sdkwork-rtc` |
| Identity, tenant IAM | `sdkwork-iam` |

## 6. Core Design Principles

1. **Kernel object models are immutable for vendor fields** — external metadata lives in bindings and diagnostics.
2. **Mechanism vs policy** — kernel provides drivers and transports; agents owns business policy.
3. **Official SDK first** — raw HTTP bypass only when binding declares OpenAPI authority.
4. **Health-aware selection** — transports attach only after `prepare()` succeeds.
5. **Products fail closed** — missing provider capability is an explicit error, not a silent stub.
6. **Open extension** — new framework = manifest + provider crate + conformance tests.

## 7. Key Product Objects

| Object | Description |
| --- | --- |
| `AgentSession` | Hosted or client-local session with lifecycle and persistence |
| `ProviderBinding` | Catalog manifest (`binding.agent-provider.*`) |
| `CapabilityDriver` | Registered handler for `sdk.*` capability ids |
| `TransportHost` | Language/runtime probe (`typescript_node`, `rust_native`, …) |
| `SdkRuntimeRouter` | Routes runtime requests to healthy negotiated transports |
| `KernelEvent` | Telemetry/audit envelope; may project to product events |
| `CodeSession` | Code-kernel workspace/task context for BirdCoder |

## 8. User Scenarios

Canonical step-by-step scenarios: [PRD.md §5](PRD.md#5-user-scenarios).

## 9. Evaluation Criteria

Success metrics and release gates: [PRD-03 §4](PRD-03-commercial-readiness-baseline.md#4-success-metrics).
