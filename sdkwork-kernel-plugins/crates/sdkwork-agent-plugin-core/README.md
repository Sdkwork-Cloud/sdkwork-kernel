# SDKWork Agent Plugin Core

Shared SDKWork-owned plugin contracts. Its public API exposes canonical plugin
names such as `KernelPluginManifest`,
`KernelProviderBinding`, `StandardPluginIds`, and `SdkworkKernelPlugin`.

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent-plugin-core
Package type: rust-crate
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

Only canonical plugin API names are exported.

### Required SDK Surface

- None declared in `specs/component.spec.json`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `cargo test`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
