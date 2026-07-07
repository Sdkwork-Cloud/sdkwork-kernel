# SDKWork Agent Client

Domain: `intelligence`
Capability: `agent-client`
Package type: Rust runtime crate

Typed HTTP and SSE clients for canonical internal-api runtime, plus local SDK bridge plugins (OpenClaw, Hermes, Codex) and `AgentClient` Remote/Local/Hybrid modes.

**Transport:** production runtime ingress uses `SseChatClient` → `/internal/v3/api/intelligence/runtime/*`. `WebSocketChatClient` remains a fail-closed legacy scaffold (not mounted on internal-api).

`SseChatClient` exposes async methods for native async callers and implements the
sync `ChatClient` trait for legacy bridge callers. The sync compatibility layer
detects an existing Tokio runtime and runs the blocking bridge call on a
dedicated runtime thread instead of nesting `Runtime::block_on` on the executor
thread. This prevents desktop, Tauri, and server hosts from panicking when a sync
remote bridge call is made inside an async runtime.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
