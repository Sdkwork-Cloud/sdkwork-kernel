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

No `sdkwork.app.config.json` is present at this root. If the task changes application behavior, runtime config, SDK wiring, release metadata, or app-owned capabilities, first locate the nearest application root that has this manifest or add one according to the root specs.

## Local Dictionary Structure

- `AGENTS.md`: local agent entrypoint and relative SDKWORK spec index.
- `CLAUDE.md`: Claude Code compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `GEMINI.md`: Gemini CLI compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `CODEX.md`: Codex compatibility shim that points to `AGENTS.md` and must not duplicate rules.
- `sdkwork.app.config.json`: not present here; required for application roots.
- `.sdkwork/`: reserved local dictionary folder; create only for local skills, plugins, manifests, or AI workspace metadata.
- `specs/`: local application/component contracts and narrowing rules.
- `sdks/`: SDK families, OpenAPI authorities, route manifests, and generated SDK artifacts.
- Local directories to inspect first when relevant: `docs/`, `external/`, `scripts/`, `sdks/`, `sdkwork-agent-business/`, `sdkwork-kernel-plugins/`, `sdkwork-agent-kernel/`, `sdkwork-code-kernel/`, `sdkwork-kernel-ui/`, `specs/`.

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

## Build, Test, and Verification

No standard build manifest was detected at this root. Read `README.md`, local `specs/`, and parent repository guidance before choosing commands. Record any manual verification in the task result.

## Agent Execution Rules

Use the convention dictionary instead of broad context loading. Do not hand-edit generated SDK output unless the task is explicitly about generated artifacts and the source contract is verified. Do not replace generated SDK integration with raw HTTP. Keep changes scoped to the owning module, package, crate, or app root. Record the exact verification commands and important outputs before reporting completion.

## Human Review Rules

Request human review before breaking SDKWORK standards, changing public naming, altering security/auth behavior, changing database migrations or production deployment config, deleting data/files, or changing generated SDK ownership. Surface unresolved spec paths, app identity conflicts, component ownership conflicts, and API authority ambiguity instead of guessing.

## Existing Local Guidance

The repository-specific guidance below was preserved from the previous `AGENTS.md`. If it conflicts with the SDKWORK sections above or with `../sdkwork-specs/`, the SDKWORK standards win.

### Project Structure & Module Organization

This repository defines the SDKWork kernel standard for agent and code-agent systems. Rust crates live at `sdkwork-agent-kernel/`, `sdkwork-code-kernel/`, and `sdkwork-agent-business/`, each with `src/`, `tests/`, and local `specs/` where applicable. `sdkwork-kernel-ui/` is a pnpm TypeScript/Vite/React workspace with reusable packages under `packages/`. Cross-cutting contracts and schemas are in root `specs/`. Third-party reference source trees are under `external/` and must remain inspection and mapping inputs, not direct kernel-core dependencies.

### Build, Test, and Development Commands

- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`: run agent-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-code-kernel/Cargo.toml`: run code-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`: run managed-agent business contracts.
- `pnpm --dir sdkwork-kernel-ui install --frozen-lockfile`: install UI workspace dependencies.
- `pnpm --dir sdkwork-kernel-ui build`: build the kernel UI shell and packages.
- `pnpm --dir sdkwork-kernel-ui typecheck`: run TypeScript checks.
- `node scripts/check-kernel-standards.mjs`: verify required specs, schemas, crates, and UI package structure.
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`: enforce UI package layering.

### Coding Style & Naming Conventions

Rust uses standard `cargo fmt` style, snake_case modules, PascalCase types, and explicit contract names such as `AgentRuntimeDiagnostics`. TypeScript packages use scoped names like `@sdkwork/kernel-ui-agent`; source exports should go through `src/index.ts` or `src/index.tsx`. Keep provider and manifest IDs stable, lowercase, and dot-delimited when they represent kernel capabilities or events, for example `agent.business.created`.

### Testing Guidelines

Prefer contract tests that document public behavior. Rust integration tests belong in each crate's `tests/` directory and typically use names ending in `_contracts.rs`. UI architecture checks are Node scripts, while TypeScript validation runs through `pnpm --dir sdkwork-kernel-ui typecheck`. When adding behavior, add the failing test first and verify the targeted test before broader checks.

### Commit & Pull Request Guidelines

History uses Conventional Commit style, for example `feat(agent-business): add expectedVersion optimistic concurrency contracts` and `refactor(agent-business): centralize problem error category mapping`. Use a scoped subject, keep it imperative, and keep unrelated changes out of the same commit. Pull requests should summarize the contract or subsystem changed, list verification commands run, link relevant specs or issues, and include screenshots only for visible UI changes.

### Security & Architecture Notes

Kernel crates must not depend on React, Vite, product UI, or `external/` source trees. UI packages must use typed service adapters rather than raw kernel mutations. Generated SDK contracts, schemas, and provider manifests should be updated deliberately and reviewed as compatibility surfaces.
