# SDKWork Agent Streaming

Domain: `intelligence`
Capability: `agent-streaming`
Package type: Rust runtime crate

SSE and WebSocket protocol adapters plus stream lifecycle management for agent events.

## Resource Bounds

`StreamManager` applies bounded backpressure at every in-process queue boundary:

- 4,096 concurrent connections per process.
- 1,024 queued updates per connection.
- 256 KiB per queued update, including owned string capacities and trace metadata.
- 4 MiB of queued update data per connection.
- 64 MiB of queued update data across the process.

Capacity failures use the kernel `resource_exhausted` error kind. Pop, drain,
disconnect, and same-id reconnect paths release their byte accounting under the
same mutex as the queue mutation; empty queues also release retained `VecDeque`
storage.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-streaming/Cargo.toml
```

## Canonical Specifications

- Component spec: [`specs/component.spec.json`](specs/component.spec.json)
- Agent kernel spec: [`../specs/AGENT_KERNEL_SPEC.md`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [`../specs/AGENT_RUNTIME_SPEC.md`](../specs/AGENT_RUNTIME_SPEC.md)
