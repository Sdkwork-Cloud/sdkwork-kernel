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
- `../sdkwork-specs/PNPM_SCRIPT_SPEC.md`
- `../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`
- `../sdkwork-specs/SOURCE_CONFIG_SPEC.md`

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
- `specs/kernel-local-conventions.md`: repository-specific structure, ownership boundary, command, testing, and security notes moved out of legacy agent guidance.
- `sdks/`: SDK families, OpenAPI authorities, route manifests, and generated SDK artifacts.
- `etc/`: deployable-root source configuration for concrete environment, runtime, topology, and deployment values.
- Local directories to inspect first when relevant: `docs/`, `external/`, `scripts/`, `sdks/`, `agent-providers/`, `sdkwork-kernel-plugins/`, `sdkwork-agent-kernel/`, `sdkwork-agent-database/`, `sdkwork-code-kernel/`, `specs/`.

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

Use dynamic progressive loading: read the nearest `AGENTS.md`, app identity only when relevant, local specs only when relevant, root `sdkwork-specs/README.md`, task-specific specs, then implementation files. Do not eagerly load all language, runtime, UI, deployment, or SDK specs for unrelated work.

## Required Specs By Task Type

- Agent/workflow changes: `../sdkwork-specs/SOUL.md`, `../sdkwork-specs/AGENTS_SPEC.md`, `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`; include `../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md` when GitHub packaging is touched.
- Any code change: `../sdkwork-specs/CODE_STYLE_SPEC.md`, `../sdkwork-specs/NAMING_SPEC.md`, plus only the touched language/framework spec.
- Build scripts, dev runners, dependency preparation, and root command changes: `../sdkwork-specs/CODE_STYLE_SPEC.md`, `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`, and `../sdkwork-specs/PNPM_SCRIPT_SPEC.md`.
- Release packaging workflow changes: `../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`, `../sdkwork-specs/APP_MANIFEST_SPEC.md`, `../sdkwork-specs/RELEASE_SPEC.md`, and `../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`.
- Rust code: `../sdkwork-specs/RUST_CODE_SPEC.md` and `../sdkwork-specs/RUST_RPC_SPEC.md` when RPC is touched.
- Java/Spring code: `../sdkwork-specs/JAVA_CODE_SPEC.md` and `../sdkwork-specs/WEB_BACKEND_SPEC.md` when HTTP backend behavior is touched.
- TypeScript/Node code: `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`.
- Frontend/UI code: `../sdkwork-specs/FRONTEND_CODE_SPEC.md`, `../sdkwork-specs/FRONTEND_SPEC.md`, `../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`, and exactly one detailed UI architecture spec.
- Source configuration or `etc/` changes: `../sdkwork-specs/SOURCE_CONFIG_SPEC.md`, plus the task-matrix dependencies in `../sdkwork-specs/README.md`.
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

## Task-Specific Standards

- App SDK consumer import work loads `../sdkwork-specs/APP_SDK_INTEGRATION_SPEC.md` and runs `node ../sdkwork-specs/tools/check-app-sdk-consumer-imports.mjs --workspace .`.
- HTTP API contract work loads `../sdkwork-specs/API_SPEC.md` and runs `node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .` plus `node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .`.
- List/search work loads `../sdkwork-specs/PAGINATION_SPEC.md` and runs `node ../sdkwork-specs/tools/check-pagination.mjs --workspace .`.
- Source configuration work loads `../sdkwork-specs/SOURCE_CONFIG_SPEC.md`; concrete deployable-root values remain owned by `etc/`.

## Human Review Rules

Request human review before breaking SDKWORK standards, changing public naming, altering security/auth behavior, changing database migrations or production deployment config, deleting data/files, or changing generated SDK ownership. Surface unresolved spec paths, app identity conflicts, component ownership conflicts, and API authority ambiguity instead of guessing.

## Repository Local Conventions

Durable repository-specific guidance lives in [`specs/kernel-local-conventions.md`](specs/kernel-local-conventions.md). The remaining notes below are local context only; if they conflict with `../sdkwork-specs/` or machine contracts under `specs/`, the SDKWORK standards and local specs win.

### Project Structure & Module Organization

This repository defines the SDKWork kernel standard for agent and code-agent systems. Rust crates live at `sdkwork-agent-kernel/` (L0 SPI), `sdkwork-code-kernel/` (code-agent SPI), `sdkwork-agent-provider-spi/` (L1 provider integration), `sdkwork-agent-provider-transport-*/` (L2 transports), `agent-providers/crates/sdkwork-agent-provider-*/` (L3 per-framework implementations: codex, claude-code, opencode, gemini-cli, openclaw, hermes, mimo-code, rig), `sdkwork-agent-server/` (operational HTTP server), `sdkwork-agent-client/` (desktop/mobile bridge), `sdkwork-agent-database/` (runtime transient session/message/task state), `sdkwork-agent-session/`, `sdkwork-agent-streaming/`, `sdkwork-agent-api-bridge/`, and `sdkwork-kernel-plugins/` (plugin trait + provider-core + platform plugins). TypeScript integration ships through `sdks/sdkwork-agent-internal-sdk/` (`@sdkwork/agent-internal-sdk`). Cross-cutting contracts and schemas are in root `specs/`. Third-party source trees under `external/` are fixed-revision, read-only inputs. L3 provider crates may consume an approved upstream public facade through root workspace dependencies; provider-neutral kernel core and SPI crates must not depend on those trees or expose their types.

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
- `node scripts/check-kernel-standards.mjs`: verify required specs, schemas, crates, and SDK workspace structure.
- `node scripts/verify-kernel-audit-remediation.mjs`: run the full kernel audit remediation verification matrix.
- `pnpm --dir sdks/sdkwork-agent-internal-sdk/sdkwork-agent-internal-sdk-typescript verify`: verify the agent internal SDK family.
- `pnpm install`: install root `@sdkwork/app-topology` workspace dependency.
- `pnpm topology:validate`: validate `specs/topology.spec.json` against the shared topology schema.
- `pnpm test:topology` / `pnpm test:topology-baggage`: verify topology adoption contracts and retired vocabulary.
- `pnpm test:topology-smoke`: start `sdkwork-agent-server` with the `standalone.development` profile and wait for `/healthz`.
- `pnpm dev`: start the default `standalone.development` stack (agent server only).
- `pnpm verify`: run the merge-ready verification aggregate.
- `pnpm check`: run kernel standards, SDK workspace, and PNPM script checks.

### Coding Style & Naming Conventions

Rust uses standard `cargo fmt` style, snake_case modules, PascalCase types, and explicit contract names such as `AgentRuntimeDiagnostics`. TypeScript SDK packages use scoped names like `@sdkwork/agent-internal-sdk`; source exports should go through `src/index.ts`.

### Testing Guidelines

Prefer contract tests that document public behavior. Rust integration tests belong in each crate's `tests/` directory and typically use names ending in `_contracts.rs`. SDK workspace checks are Node scripts. When adding behavior, add the failing test first and verify the targeted test before broader checks.

### Commit & Pull Request Guidelines

History uses Conventional Commit style, for example `feat(agent-kernel): add task scheduling SPI pause/resume contracts` and `refactor(agent-server): centralize problem error category mapping`. Use a scoped subject, keep it imperative, and keep unrelated changes out of the same commit. Pull requests should summarize the contract or subsystem changed, list verification commands run, link relevant specs or issues, and include screenshots only for visible UI changes.

### Security & Architecture Notes

Kernel core, provider SPI, provider-neutral transports, server, client, and database crates must not depend on React, Vite, product UI, or provider-specific `external/` source trees. An L3 crate under `agent-providers/crates/sdkwork-agent-provider-*` may depend on a fixed, read-only external source tree only through its documented public facade and root workspace dependency declaration. It must not modify the submodule, query private provider persistence, or leak provider-specific types into L0/L1 contracts. Product applications must use typed SDK clients rather than raw kernel HTTP mutations. Generated SDK contracts, schemas, and provider manifests should be updated deliberately and reviewed as compatibility surfaces.

Agent runtime HTTP on `application.public-ingress` uses the SDKWork `internal-api` surface only (`/internal/v3/api/intelligence/runtime/*`). Authoritative OpenAPI lives under `apis/internal-api/`; SDK family materialization runs through `node sdks/materialize-agent-internal-api-openapi.mjs` before `node scripts/check-agent-sdk-workspace.mjs`. Retired application-local prefixes such as `/api/kernel/*` must not be remounted.
