# sdkwork-agent-provider-core

External agent protocol adapter for the SDKWork agent kernel.

## Purpose

Maps an external agent runtime contract to SDKWork kernel SPI types through `sdkwork-agent-provider-core` seams.

Provider session snapshots pass through `finalize_provider_session_snapshot` before entering
shared lifecycle storage. Non-empty timestamps are parsed with `sdkwork-utils-rust`, converted to
UTC, and serialized as compact RFC3339 (`Z`) values: whole-second inputs do not gain a synthetic
nanosecond suffix, while meaningful sub-second precision is retained.

## Provider Session Activity

`ProviderSessionActivityAdapter` maps provider status and event facts to the
kernel `SessionActivitySnapshot`. `InMemoryProviderSessionActivityProvider`
implements the provider-neutral ingestion and query contracts for runtime
facades. Its default TTL is 30 seconds and every read recalculates freshness;
unknown ids are `Unsupported`, and expired observations are `Stale`.

This crate does not infer activity from JSONL, SQLite history, transcript file
timestamps, or last-message time. A provider runtime host must explicitly
ingest a proven live status/event observation. Without that collector wiring,
queries remain `Unsupported`.

## Verification

```bash
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-provider-core/Cargo.toml
```
