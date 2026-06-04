# Repository Guidelines

## Project Structure & Module Organization

This repository defines the SDKWork kernel standard for agent and code-agent systems. Rust crates live at `sdkwork-agent-kernel/`, `sdkwork-code-kernel/`, and `sdkwork-agent-business/`, each with `src/`, `tests/`, and local `specs/` where applicable. `sdkwork-kernel-ui/` is a pnpm TypeScript/Vite/React workspace with reusable packages under `packages/`. Cross-cutting contracts and schemas are in root `specs/`. Third-party reference source trees are under `external/` and must remain inspection and mapping inputs, not direct kernel-core dependencies.

## Build, Test, and Development Commands

- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`: run agent-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-code-kernel/Cargo.toml`: run code-kernel Rust contracts.
- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`: run managed-agent business contracts.
- `pnpm --dir sdkwork-kernel-ui install --frozen-lockfile`: install UI workspace dependencies.
- `pnpm --dir sdkwork-kernel-ui build`: build the kernel UI shell and packages.
- `pnpm --dir sdkwork-kernel-ui typecheck`: run TypeScript checks.
- `node scripts/check-kernel-standards.mjs`: verify required specs, schemas, crates, and UI package structure.
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`: enforce UI package layering.

## Coding Style & Naming Conventions

Rust uses standard `cargo fmt` style, snake_case modules, PascalCase types, and explicit contract names such as `AgentRuntimeDiagnostics`. TypeScript packages use scoped names like `@sdkwork/kernel-ui-agent`; source exports should go through `src/index.ts` or `src/index.tsx`. Keep provider and manifest IDs stable, lowercase, and dot-delimited when they represent kernel capabilities or events, for example `agent.business.created`.

## Testing Guidelines

Prefer contract tests that document public behavior. Rust integration tests belong in each crate's `tests/` directory and typically use names ending in `_contracts.rs`. UI architecture checks are Node scripts, while TypeScript validation runs through `pnpm --dir sdkwork-kernel-ui typecheck`. When adding behavior, add the failing test first and verify the targeted test before broader checks.

## Commit & Pull Request Guidelines

History uses Conventional Commit style, for example `feat(agent-business): add expectedVersion optimistic concurrency contracts` and `refactor(agent-business): centralize problem error category mapping`. Use a scoped subject, keep it imperative, and keep unrelated changes out of the same commit. Pull requests should summarize the contract or subsystem changed, list verification commands run, link relevant specs or issues, and include screenshots only for visible UI changes.

## Security & Architecture Notes

Kernel crates must not depend on React, Vite, product UI, or `external/` source trees. UI packages must use typed service adapters rather than raw kernel mutations. Generated SDK contracts, schemas, and provider manifests should be updated deliberately and reviewed as compatibility surfaces.
