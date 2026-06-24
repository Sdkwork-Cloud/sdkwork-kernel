# SDKWork Agent Client

Domain: `intelligence`
Capability: `agent-client`
Package type: Rust runtime crate

Typed HTTP and SSE clients for canonical internal-api runtime, plus local SDK bridge plugins (OpenClaw, Hermes, Codex) and `AgentClient` Remote/Local/Hybrid modes.

**Transport:** production runtime ingress uses `SseChatClient` → `/internal/v3/api/intelligence/runtime/*`. `WebSocketChatClient` remains a fail-closed legacy scaffold (not mounted on internal-api).

## Verification

```bash
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
