# External Plugin Local Runtime Profile

## Purpose

The local-runtime profile applies after a plugin crate implements typed
SDKWork Rust SPI for an upstream project.

## Required Evidence

- Manifest profile passes.
- A typed local provider crate exists.
- Provider registration uses `RuntimeBuilder` or `CodeKernelRuntimeBuilder`.
- Diagnostics report the provider as typed.
- Provider health is available.
- Unsupported upstream operations return `capability_missing`.
- Manifest-only operations without typed providers return `provider_unavailable`.
- No provider bypasses SDKWork host, policy, redaction, event, or audit SPI.

## Required Cases

| Case Id | Description |
| --- | --- |
| `external.local.provider.typed` | A SDKWork trait implementation is registered. |
| `external.local.provider.health_available` | Health is visible in diagnostics. |
| `external.local.policy.fail_closed` | Protected operations fail closed without policy. |
| `external.local.errors.normalized` | Upstream errors map to stable `KernelErrorKind` values. |
| `external.local.events.emitted` | Runtime activity emits SDKWork kernel events. |

## Initial Candidate

Rig is the preferred first local-runtime candidate because it is Rust-native and
can map to `ModelProvider`, `ToolProvider`, and `PlanningProvider` without
starting an external CLI process.
