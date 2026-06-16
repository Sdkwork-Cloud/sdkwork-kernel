# SDKWork Agent Database

Domain: `intelligence`
Capability: `agent-database`
Package type: Rust runtime crate

Repository traits and SQLite, PostgreSQL, and in-memory adapters for agent session persistence.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-database/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
