# Rust Crate Workspace

Purpose: standard location for Rust crate roots when kernel crates are created or migrated under the SDKWork repository dictionary.

Owner: SDKWork kernel maintainers.

Allowed content: Rust service, repository, route, host, worker, gateway, plugin, and reusable library crates with local `Cargo.toml` and component specs.

Forbidden content: generated SDK transport output, React/Vite UI packages, third-party reference source trees, live secrets, and runtime databases.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/RUST_CODE_SPEC.md`, `../sdkwork-specs/NAMING_SPEC.md`, and `../sdkwork-specs/COMPONENT_SPEC.md`.

Verification: run `cargo test --workspace` or the crate-specific commands documented in `AGENTS.md`, plus `node scripts/check-kernel-standards.mjs`.
