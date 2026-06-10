# SDKWork Knowledgebase Plugin

Official SDKWork Kernel foundation plugin for `sdkwork-knowledgebase`.

## SDKWork Documentation Contract

Domain: intelligence
Capability: kernel-plugin-knowledgebase
Package type: rust-crate
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- `sdkwork-knowledgebase-contract`

### Configuration

This plugin does not force a knowledgebase binding into every agent. Hosts and
agent definitions opt in by registering the provided `KnowledgeProvider`.

### SaaS/Private/Local Behavior

The plugin maps SDKWork Agent Kernel knowledge SPI requests to SDKWork
Knowledgebase retrieval contracts while preserving tenant and namespace scope.

### Security

Do not add secrets, live tokens, manual auth headers, raw filesystem access, or
raw HTTP access to this plugin. Knowledgebase access flows through typed
contracts and host policy.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints,
SDK clients, events, and config keys.

### Verification

- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
