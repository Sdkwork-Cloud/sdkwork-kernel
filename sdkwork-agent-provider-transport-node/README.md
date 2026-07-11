# SDKWork Agent Provider Transport Node

Node/TypeScript JSON-RPC worker runtime for external agent provider capabilities. Managed calls use
the bounded request-isolated worker pool from `sdkwork-agent-provider-transport-ipc`; configure its
concurrency with `SDKWORK_PROVIDER_WORKER_MAX_CONCURRENCY` (`8` by default, maximum `64`).

Verification:

```bash
cargo test --manifest-path sdkwork-agent-provider-transport-node/Cargo.toml
```
