# SDKWork Rig Plugin

Complete SDKWork plugin implementation boundary for Rig.

The executable `AgentDefinition` binds Rig model, tool, memory, knowledge,
planning, policy, MCP, lifecycle, and protocol-adapter providers explicitly.
Knowledge and MCP are optional bindings; missing optional capabilities degrade
runtime negotiation instead of blocking agents that do not require them.

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent-plugin-rig
Package type: rust-crate
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

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
