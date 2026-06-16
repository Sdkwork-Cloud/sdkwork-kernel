# SDKWork Agent Streaming

Domain: `intelligence`
Capability: `agent-streaming`
Package type: Rust runtime crate

SSE and WebSocket protocol adapters plus stream lifecycle management for agent events.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-streaming/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
