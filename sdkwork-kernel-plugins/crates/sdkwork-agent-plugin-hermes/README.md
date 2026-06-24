# SDKWork Hermes Plugin

SDKWork kernel plugin for Hermes Agent Python runtime and optional TUI gateway IPC.

Select with `SDKWORK_KERNEL_AGENT_PLUGIN=hermes` in `sdkwork-agent-server`.
Set `SDKWORK_HERMES_USE_TUI_GATEWAY=1` to prefer JSON-RPC IPC via `tui_gateway`.

## Verification

```bash
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-hermes/Cargo.toml
```
