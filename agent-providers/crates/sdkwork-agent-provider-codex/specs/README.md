# Codex Provider Contracts

`component.spec.json` is the component dictionary for the L3 Codex provider.

The provider may consume the pinned, read-only `external/codex` public Rust
facades declared in the Kernel root `Cargo.toml`. It must keep all Codex types
inside this crate and map them into Kernel-neutral session, message, activity,
and provider contracts.

Session and history production access is exclusively through
`codex-app-server-client` requests and `codex-app-server-protocol` models.
Direct provider state SQLite, private SQL/schema inspection, and rollout file
parsing are forbidden. `node scripts/check-kernel-standards.mjs` enforces this
boundary.

List operations preserve upstream opaque cursors and enforce the SDKWork page
size range `1..=200`, defaulting to 20. Provider output trust and redaction are
governed by `SECURITY_SPEC.md`; complete raw typed ThreadItem JSON is retained
as tenant-sensitive data.
