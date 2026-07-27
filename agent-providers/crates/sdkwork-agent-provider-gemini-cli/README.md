# sdkwork-agent-provider-gemini-cli

External agent protocol adapter for the SDKWork agent kernel.

## Purpose

Maps an external agent runtime contract to SDKWork kernel SPI types through `sdkwork-agent-provider-core` seams.

## Provider Session Activity

`GeminiCliSdkIntegration::record_provider_session_activity` accepts live Gemini
CLI `AgentEvent` observations. Agent/tool start maps to working, elicitation to
user-input waiting, agent end to idle, and fatal error to failed.

The managed Node transport forwards official Gemini SDK `AgentEvent` values and
incremental CLI JSONL events into the same activity store for operations run by
this integration. Independently running Gemini CLI processes remain
`Unsupported` unless a runtime host attaches an authoritative event consumer;
persisted conversation timestamps are not live evidence.

## Verification

```bash
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-gemini-cli/Cargo.toml
```
