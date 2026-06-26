# Product PRD Directory

This directory owns the product Canon for the repository.

## Fixed Entry

- [PRD.md](PRD.md) — required entry document. Keep summary, status, and links here.

## Shards

- [PRD-01-product-design-and-scope.md](PRD-01-product-design-and-scope.md) — positioning, users, goals, scope
- [PRD-02-provider-integration-requirements.md](PRD-02-provider-integration-requirements.md) — provider integration product acceptance (normative rules in `specs/`)
- [PRD-03-commercial-readiness-baseline.md](PRD-03-commercial-readiness-baseline.md) — phases, readiness matrix, deployment

## Splitting Rules

- Split large PRD content into sibling shards named `PRD-<kebab-topic>.md`.
- Every shard `MUST` be linked from `PRD.md`.
- Do not create competing product roots such as `docs/product/PRD.md`; that path is retired and redirect-only.

See [DOCUMENTATION_SPEC.md](../../../sdkwork-specs/DOCUMENTATION_SPEC.md) section 2.2.
