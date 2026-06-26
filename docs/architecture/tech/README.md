# Technical Architecture Directory

This directory owns the technical architecture Canon for the repository.

## Fixed Entry

- [TECH_ARCHITECTURE.md](TECH_ARCHITECTURE.md) — required entry document. Keep summary, status, and links here.

## Active Shards

- [TECH-01-kernel-module-reference.md](TECH-01-kernel-module-reference.md) — crate/module reference and bootstrap sequence
- [TECH-2026-06-14-multi-mode-agent-system.md](TECH-2026-06-14-multi-mode-agent-system.md) — server plugins, client bridge, providers
- [TECH-2026-06-10-agent-execution-loop.md](TECH-2026-06-10-agent-execution-loop.md) — turn loop and tool execution
- [TECH-2026-06-10-sdkwork-kernel-plugin-system.md](TECH-2026-06-10-sdkwork-kernel-plugin-system.md) — kernel plugin manifests
- [TECH-2026-06-12-agent-implementation-type.md](TECH-2026-06-12-agent-implementation-type.md) — implementation typing
- [TECH-topology-standard.md](TECH-topology-standard.md) — deployment topology profiles

## Splitting Rules

- Split large architecture content into sibling shards named `TECH-<kebab-topic>.md`.
- Every shard `MUST` be linked from `TECH_ARCHITECTURE.md`.
- Do not create competing architecture roots such as `docs/architecture/TECH_ARCHITECTURE.md`; that path is retired and redirect-only.

See [DOCUMENTATION_SPEC.md](../../../sdkwork-specs/DOCUMENTATION_SPEC.md) section 2.2.
