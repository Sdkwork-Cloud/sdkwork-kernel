# sdkwork-agent-provider-core

External agent protocol adapter for the SDKWork agent kernel.

## Purpose

Maps an external agent runtime contract to SDKWork kernel SPI types through `sdkwork-agent-provider-core` seams.

Provider session snapshots pass through `finalize_provider_session_snapshot` before entering
shared lifecycle storage. Non-empty timestamps are parsed with `sdkwork-utils-rust`, converted to
UTC, and serialized as compact RFC3339 (`Z`) values: whole-second inputs do not gain a synthetic
nanosecond suffix, while meaningful sub-second precision is retained.

## Verification

```bash
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-provider-core/Cargo.toml
```
