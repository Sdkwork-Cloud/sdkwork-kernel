# Repository Guidelines

<!-- SDKWORK-AGENTS-GENERATED: v1 -->

## SDKWORK Soul

Read `../sdkwork-specs/SOUL.md` before executing tasks in this root. Follow specs before memory, dictionary before context, stop on ambiguity, and evidence before completion.

## SDKWORK Standards

Canonical SDKWORK specs path from this root:

- `../sdkwork-specs/README.md`
- `../sdkwork-specs/SOUL.md`
- `../sdkwork-specs/AGENTS_SPEC.md`
- `../sdkwork-specs/CODE_STYLE_SPEC.md`
- `../sdkwork-specs/NAMING_SPEC.md`

Do not copy root standard text into this repository. If these relative paths do not resolve, stop and report the broken workspace layout.

## Application Identity

`sdkwork.app.config.json` is present at this repository root (`app.key: sdkwork-kernel`). Topology and workflow metadata are linked through `metadata.topologySpec` and `metadata.workflowManifest`.

## Local Dictionary Structure

- `AGENTS.md`: local agent entrypoint and relative SDKWORK spec index.
- `CLAUDE.md`: Claude Code compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `GEMINI.md`: Gemini CLI compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `CODEX.md`: Codex compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `sdkwork.app.config.json`: application identity, topology, and workflow metadata for this repository root (`app.key: sdkwork-kernel`).
- `.sdkwork/`: reserved local dictionary folder; create only for local skills, plugins, manifests, or AI workspace metadata.
- `specs/`: local application/component contracts and narrowing rules.
- `sdks/`: SDK families, OpenAPI authorities, route manifests, and generated SDK artifacts.
- Local directories to inspect first when relevant: `docs/`, `external/`, `scripts/`, `sdks/`, `agent-providers/`, `sdkwork-kernel-plugins/`, `sdkwork-agent-kernel/`, `sdkwork-agent-database/`, `sdkwork-code-kernel/`, `sdkwork-kernel-ui/`, `specs/`.

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Spec Resolution Order

1. Read this `AGENTS.md` and any nearer component-level `AGENTS.md`.
2. Read `sdkwork.app.config.json` when present.
3. Read local `specs/README.md` and `specs/component.spec.json` when present.
4. Read local `.sdkwork/README.md`, `.sdkwork/skills/`, and `.sdkwork/plugins/` when relevant.
5. Read `../sdkwork-specs/README.md` and the task-specific root specs.
6. Inspect implementation files only after the relevant dictionary entries are clear.

## Required Specs By Task Type

- Agent/workflow changes: `../sdkwork-specs/SOUL.md`, `../sdkwork-specs/AGENTS_SPEC.md`, `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`.
- Any code change: `../sdkwork-specs/CODE_STYLE_SPEC.md`, `../sdkwork-specs/NAMING_SPEC.md`, plus only the touched language/framework spec.
- Rust code: `../sdkwork-specs/RUST_CODE_SPEC.md` and `../sdkwork-specs/RUST_RPC_SPEC.md` when RPC is touched.
- Java/Spring code: `../sdkwork-specs/JAVA_CODE_SPEC.md` and `../sdkwork-specs/WEB_BACKEND_SPEC.md` when HTTP backend behavior is touched.
- TypeScript/Node code: `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`.
- Frontend/UI code: `../sdkwork-specs/FRONTEND_CODE_SPEC.md`, `../sdkwork-specs/FRONTEND_SPEC.md`, `../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`, and exactly one detailed UI architecture spec.
- API, SDK, database, runtime, security, and deployment changes must follow the task matrix in `../sdkwork-specs/README.md`.

Language-specific specs are on-demand; do not load Rust, Java, TypeScript, and frontend specs for unrelated tasks.

## Code Style Rules

Read `../sdkwork-specs/CODE_STYLE_SPEC.md` and `../sdkwork-specs/NAMING_SPEC.md` before code changes.

Load language specs only when touched: Rust uses `RUST_CODE_SPEC.md`, Java/Spring uses `JAVA_CODE_SPEC.md`, TypeScript/Node uses `TYPESCRIPT_CODE_SPEC.md`, and frontend/UI uses `FRONTEND_CODE_SPEC.md`.


Build scripts, dev runners, and `pnpm clean` must follow `CODE_STYLE_SPEC.md` §7 (Build Source Integrity And Self-Healing). Git-tracked build-critical source files must be verified before builds and self-healed from git when missing; `clean` must not delete them.

## Build, Test, and Verification

No standard build manifest was detected at this root. Read `README.md`, local `specs/`, and parent repository guidance before choosing commands. Record any manual verification in the task result.

## Agent Execution Rules

Use the convention dictionary instead of broad context loading. Do not hand-edit generated SDK output unless the task is explicitly about generated artifacts and the source contract is verified. Do not replace generated SDK integration with raw HTTP. Keep changes scoped to the owning module, package, crate, or app root. Record the exact verification commands and important outputs before reporting completion.

## HTTP API Response Envelope

All L2+ `app-api`, `backend-api`, and SDKWork-owned business `open-api` HTTP contracts `MUST` follow `API_SPEC.md` section 4.5, section 14, and section 15:

- **Input:** typed request bodies, section 14.1 list/search/command input, `SdkWorkListQuery`, and `q` for free-text search.
- **Success output:** `SdkWorkApiResponse` with `{ "code": 0, "data": <payload>, "traceId": "<server-uuid>" }`.
- **Error output:** HTTP 4xx/5xx `application/problem+json` (`ProblemDetail`) with numeric `code` and `traceId`.
- Success `code` is numeric `int32`; HTTP 2xx JSON bodies `MUST` use `0` only. REST semantics remain on HTTP status (`201`, `202`, etc.).
- Platform error codes are numeric non-zero values per section 15.3 (`40001`, `40101`, `40401`, …).
- Single resource: `data.item`
- Lists: `data.items` + `data.pageInfo` (`PageInfo.mode` is `offset` or `cursor`)
- Commands: `data.accepted` plus optional `resourceId` / `status`
- Async accept (`202`): `data.operationId`, `data.status`, optional `pollUrl`

Vendor compatibility `open-api` routes that mirror upstream tool or provider wire (for example OpenAI `/v1/*`, Claude Code, Codex) `MAY` opt out only when every exempt operation declares `x-sdkwork-wire-protocol: external` and `x-sdkwork-external-protocol-id` per `API_SPEC.md` section 4.5.2. SDKWork-owned business `open-api` operations `MUST NOT` opt out.

Errors `MUST` use HTTP 4xx/5xx with `application/problem+json` (`ProblemDetail`) including required numeric `code` and `traceId`. Business failures `MUST NOT` use HTTP 2xx with non-zero `code`, string wire codes, `success`, or human `message`.

Forbidden legacy envelopes and fields: `PlusApiResult`, `AppbaseApiResult`, `StoreApiResult`, `SdkWorkResponse`, per-domain `*ApiResult`, wire field `requestId`, bare domain DTOs at the HTTP root, and top-level `{ items, pageInfo, traceId }` without `data`.

Handlers `MUST` serialize success and map errors through `sdkwork-web-framework` response mapping. Generated HTTP SDKs (`--standard-profile sdkwork-v3`) unwrap `data` by default and expose typed numeric `ProblemDetail.code` / `traceId` on errors; use `.raw` when the full envelope is required.

Before completing API contract, SDK generation, or frontend service work, run:

```bash
node <sdkwork-specs>/tools/check-api-response-envelope.mjs --workspace <workspace-root>
```

Authority: `sdkwork-specs/API_SPEC.md` section 4.5 and sections 14–16, `SDK_SPEC.md` section 4.2, `FRONTEND_SPEC.md`, `MIGRATION_SPEC.md` section 4.2.

## Human Review Rules

Request human review before breaking SDKWORK standards, changing public naming, altering security/auth behavior, changing database migrations or production deployment config, deleting data/files, or changing generated SDK ownership. Surface unresolved spec paths, app identity conflicts, component ownership conflicts, and API authority ambiguity instead of guessing.

## Existing Local Guidance

The repository-specific guidance below was preserved from the previous `AGENTS.md`. If it conflicts with the SDKWORK sections above or with `../sdkwork-specs/`, the SDKWORK standards win.

### Project Structure & Module Organization

This repository defines the SDKWork kernel standard for agent and code-agent systems. Rust crates live at `sdkwork-agent-kernel/` (L0 SPI), `sdkwork-code-kernel/` (code-agent SPI), `sdkwork-agent-provider-spi/` (L1 provider integration), `sdkwork-agent-provider-transport-*/` (L2 transports), `agent-providers/crates/sdkwork-agent-provider-*/` (L3 per-framework implementations: codex, claude-code, opencode, gemini-cli, openclaw, hermes, mimo-code, rig), `sdkwork-agent-server/` (operational HTTP server), `sdkwork-agent-client/` (desktop/mobile bridge), `sdkwork-agent-database/` (runtime transient session/message/task state), `sdkwork-agent-session/`, `sdkwork-agent-streaming/`, `sdkwork-agent-api-bridge/`, and `sdkwork-kernel-plugins/` (plugin trait + provider-core + platform plugins). `sdkwork-kernel-ui/` is a pnpm TypeScript/Vite/React workspace with reusable packages under `packages/`. Cross-cutting contracts and schemas are in root `specs/`. Third-party reference source trees are under `external/` and must remain inspection and mapping inputs, not direct kernel-core dependencies.

### Kernel ↔ Agents Responsibility Boundary

SDKWork follows a Linux-kernel-style split. This repository (`sdkwork-kernel`) owns **SPI definitions, runtime mechanisms, and transient runtime state only**. Business persistence and application surfaces belong to the sibling `sdkwork-agents` repository.

| Concern | Owner | Location |
| --- | --- | --- |
| Agent SPI (18 provider families: model, tool, policy, context, memory, knowledge, planning, host, protocol_adapter, mcp, skill, collaboration, telemetry, task_scheduling, agent_classification, message_query, agent_installer, agent_configuration) | kernel | `sdkwork-agent-kernel/src/` |
| Provider integration SPI + transports (Rust SDK in-process, Node/Python subprocess, IPC) | kernel | `sdkwork-agent-provider-spi/`, `sdkwork-agent-provider-transport-*/` |
| Per-framework provider implementations (pluggable, open-closed) | kernel | `agent-providers/crates/sdkwork-agent-provider-*/` |
| Runtime transient state (active sessions, streaming buffers, in-flight tasks, SSE cursors) | kernel | `sdkwork-agent-database/` (SessionRepository/MessageRepository/TaskRepository/EventRepository traits + sqlite/postgres/memory impls) |
| Operational HTTP server (`/internal/v3/api/intelligence/runtime/*`) | kernel | `sdkwork-agent-server/` |
| Client bridge (desktop/mobile local + hybrid + remote) | kernel | `sdkwork-agent-client/` |
| Business database (agent catalog, agent configuration profiles, long-term session archive, task history, scheduled job registry) | agents | `../sdkwork-agents/` |
| Agent classification catalog, app-api / backend-api / open-api + SDK generation | agents | `../sdkwork-agents/` |
| Integration of sdkwork-knowledge, sdkwork-drive, sdkwork-skills, sdkwork-prompts, sdkwork-memory | agents | `../sdkwork-agents/` |
| Memory provider implementations (permanent / user / growth-tier backends) | agents | `../sdkwork-agents/` (kernel defines `MemoryProvider` SPI + `MemoryTier`/`MemoryScope` model only) |

The kernel MUST NOT own business persistence (agent config catalog, long-term archives) or application HTTP surfaces (app-api/backend-api/open-api). The agents application MUST NOT depend on `sdkwork-agent-provider-*` crates directly; it consumes the kernel via `sdkwork-agent-internal-sdk` and the runtime facade.

### Build, Test, and Development Commands

- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`: run agent-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-code-kernel/Cargo.toml`: run code-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-agent-database/Cargo.toml`: run runtime session/message/task persistence contracts.
- `cargo test --manifest-path sdkwork-agent-server/Cargo.toml`: run operational server HTTP contracts.
- `pnpm --dir sdkwork-kernel-ui install --frozen-lockfile`: install UI workspace dependencies.
- `pnpm --dir sdkwork-kernel-ui build`: build the kernel UI shell and packages.
- `pnpm --dir sdkwork-kernel-ui typecheck`: run TypeScript checks.
- `node scripts/check-kernel-standards.mjs`: verify required specs, schemas, crates, and UI package structure.
- `node scripts/verify-kernel-audit-remediation.mjs`: run the full kernel audit remediation verification matrix.
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`: enforce UI package layering.
- `pnpm install`: install root `@sdkwork/app-topology` workspace dependency.
- `pnpm topology:validate`: validate `specs/topology.spec.json` against the shared topology schema.
- `pnpm test:topology` / `pnpm test:topology-baggage`: verify topology adoption contracts and retired vocabulary.
- `pnpm test:topology-smoke`: start `sdkwork-agent-server` with the standalone split-services dev profile and wait for `/health`.
- `pnpm dev`: start the default standalone split-services development stack (agent server + kernel UI).
- `pnpm verify`: run the merge-ready verification aggregate.
- `pnpm check`: run kernel standards, SDK workspace, UI architecture, and PNPM script checks.

### Coding Style & Naming Conventions

Rust uses standard `cargo fmt` style, snake_case modules, PascalCase types, and explicit contract names such as `AgentRuntimeDiagnostics`. TypeScript packages use scoped names like `@sdkwork/kernel-ui-agent`; source exports should go through `src/index.ts` or `src/index.tsx`. Keep provider and manifest IDs stable, lowercase, and dot-delimited when they represent kernel capabilities or events, for example `agent.business.created`.

### Testing Guidelines

Prefer contract tests that document public behavior. Rust integration tests belong in each crate's `tests/` directory and typically use names ending in `_contracts.rs`. UI architecture checks are Node scripts, while TypeScript validation runs through `pnpm --dir sdkwork-kernel-ui typecheck`. When adding behavior, add the failing test first and verify the targeted test before broader checks.

### Commit & Pull Request Guidelines

History uses Conventional Commit style, for example `feat(agent-kernel): add task scheduling SPI pause/resume contracts` and `refactor(agent-server): centralize problem error category mapping`. Use a scoped subject, keep it imperative, and keep unrelated changes out of the same commit. Pull requests should summarize the contract or subsystem changed, list verification commands run, link relevant specs or issues, and include screenshots only for visible UI changes.

### Security & Architecture Notes

Kernel crates must not depend on React, Vite, product UI, or `external/` source trees. UI packages must use typed service adapters rather than raw kernel mutations. Generated SDK contracts, schemas, and provider manifests should be updated deliberately and reviewed as compatibility surfaces.

Agent runtime HTTP on `application.public-ingress` uses the SDKWork `internal-api` surface only (`/internal/v3/api/intelligence/runtime/*`). Authoritative OpenAPI lives under `apis/internal-api/`; SDK family materialization runs through `node sdks/materialize-agent-internal-api-openapi.mjs` before `node scripts/check-agent-sdk-workspace.mjs`. Retired application-local prefixes such as `/api/kernel/*` must not be remounted.
