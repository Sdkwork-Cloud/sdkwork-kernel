# SDKWork Agent SDK SPI

Rust SPI crate for binding external agent native SDKs to the SDKWork Agent Kernel.

## Standards

- [`../specs/AGENT_SDK_SPI_SPEC.md`](../specs/AGENT_SDK_SPI_SPEC.md)
- [`../specs/AGENT_SDK_BINDING_SPEC.md`](../specs/AGENT_SDK_BINDING_SPEC.md)
- Binding catalog: [`../sdks/external-agent-sdks/`](../sdks/external-agent-sdks/)

## Verification

```bash
cargo test --manifest-path sdkwork-agent-sdk-spi/Cargo.toml
```

## Layering

- Kernel semantics: `sdkwork-agent-kernel`
- SDK adaptation SPI: this crate
- Object mapping traits: re-exported from `sdkwork-agent-adapter-core`
- Agent plugins: `sdkwork-agent-adapter-*`
