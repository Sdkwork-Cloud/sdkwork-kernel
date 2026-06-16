# SDKWork Agent Server

Domain: `intelligence`
Capability: `agent-server`
Package type: Rust runtime crate

Runnable Axum server binary with configuration, preflight checks, health endpoints, and chat APIs.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
