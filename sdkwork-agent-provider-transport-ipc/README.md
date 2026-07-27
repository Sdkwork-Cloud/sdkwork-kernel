# SDKWork Agent Provider Transport IPC

JSON-RPC stdio transport and development-only stubs for external agent provider workers.

Managed Node and Python runtimes lease one worker process to one active request. This permits
bounded concurrency and request-scoped cancellation without terminating unrelated tenant work.
`SDKWORK_PROVIDER_WORKER_MAX_CONCURRENCY` defaults to `8` and is clamped to `1..=64`.

The wire contract rejects JSON-RPC frames larger than 8 MiB, more than 4096 stream chunks,
individual chunks larger than 256 KiB, and aggregate stream content larger than 16 MiB. A worker
that violates framing is discarded instead of being reused.

For `model_chat_stream`, workers terminate with a `stream.done` frame. The frame always carries
the active `model_request_id`; it may carry `provider_session_id` only when the provider runtime can
prove that identity. Buffered transport expansion preserves both fields. Consumers must correlate
the terminal request id before accepting completion metadata and must not synthesize a session id
for providers that cannot provide one.

Verification:

```bash
cargo test --manifest-path sdkwork-agent-provider-transport-ipc/Cargo.toml
```
