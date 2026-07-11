# SDKWork Agent Client

Domain: `intelligence`
Capability: `agent-client`
Package type: Rust runtime crate

Typed HTTP and SSE clients for the canonical internal-api runtime, plus local SDK bridge plugins (OpenClaw, Hermes, Codex) and `AgentClient` Remote/Local/Hybrid modes.

Production runtime ingress uses `SseChatClient` at
`/internal/v3/api/intelligence/runtime/*`. The crate intentionally exposes no
WebSocket feature because the internal-api authority does not mount a WebSocket
surface.

`SseChatClient` exposes bounded page methods and typed async model/session event
streams for native async callers and implements the sync `ChatClient` trait for
bridge callers. `send_message_async` returns the created user message promised
by the `201` API response; it does not read an oldest-first history page or
invent an assistant reply. The sync compatibility layer detects an existing
Tokio runtime and runs the blocking bridge call on a dedicated runtime thread
instead of nesting `Runtime::block_on` on the executor thread.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-client/Cargo.toml
cargo check --manifest-path sdkwork-agent-client/Cargo.toml --all-features
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
