# SDKWork Agent SDK SPI

Rust SPI crate for binding external agent native SDKs to the SDKWork Agent Kernel.

## Standards

- [`../specs/AGENT_SDK_SPI_SPEC.md`](../specs/AGENT_SDK_SPI_SPEC.md)
- [`../specs/AGENT_SDK_BINDING_SPEC.md`](../specs/AGENT_SDK_BINDING_SPEC.md)
- Binding catalog: [`../bindings/agent-providers/`](../bindings/agent-providers/)

## Verification

```bash
cargo test --manifest-path sdkwork-agent-provider-spi/Cargo.toml
```

## Layering

- Kernel semantics: `sdkwork-agent-kernel`
- SDK adaptation SPI: this crate
- Object mapping traits: re-exported from `sdkwork-agent-provider-core`
- Agent plugins: `sdkwork-agent-adapter-*`

## Stream Completion Contract

`SdkRuntimeBackedModelProvider::stream_into_with_completion` is an additive
runtime-backed extension; it does not change the stable kernel `ModelProvider`
SPI. It forwards `ModelStreamChunk` values through the normal sink and returns
`SdkRuntimeStreamCompletion` only after a correlated `stream.done` frame.

The terminal frame must carry the active `model_request_id`. A
`native_session_id` is optional and is accepted only when the provider runtime
actually created or resumed that native session. Product facades must keep an
initial turn invoke-only unless the completion proves a non-empty native
session id. No adapter may invent a native id to make streaming appear
resumable.
