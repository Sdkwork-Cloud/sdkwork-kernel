# SDKWork Agent API Bridge

Domain: `intelligence`
Capability: `agent-api-bridge`
Package type: Rust runtime crate

Bridge layer connecting AgentKernel runtime contracts to AgentBusiness HTTP APIs.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-api-bridge/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
