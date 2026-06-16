# SDKWork Agent Client

Domain: `intelligence`
Capability: `agent-client`
Package type: Rust runtime crate

Typed HTTP, SSE, and WebSocket clients plus bridge plugin registry for external agent runtimes.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
