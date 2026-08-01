# MIG-2026-0002: Provider Host Root Naming

```yaml
id: MIG-2026-0002
owner: SDKWork kernel maintainers
status: active
requirement: REQ-2026-0001
type: config
scope:
  producers:
    - sdkwork-birdcoder desktop provider-host staging
    - sdkwork-kernel server release packaging
    - sdkwork-kernel container packaging
  consumers:
    - sdkwork-agent-provider-transport-node
    - sdkwork-agent-provider-transport-python
    - sdkwork-agent-plugin-core process adapter installer
compatibility_window:
  starts_at: 2026-08-01
  ends_at: 2026-10-01
strategy: expand-contract
rollback:
  supported: true
  steps:
    - Restore the previous application or kernel package while keeping the legacy provider-runtime directory and environment variable available.
    - Do not publish a mixed package whose declared root differs from its bundled directory.
verification:
  - cargo test --manifest-path sdkwork-agent-provider-transport-node/Cargo.toml
  - cargo test --manifest-path sdkwork-agent-provider-transport-python/Cargo.toml
  - cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml process_adapter
  - node --test tests/kernel_deployment_release.test.mjs
```

## Scope

`provider-host` names the packaged process/tool host consumed by provider
transports. The retired `provider-runtime` name overloaded the SDKWork runtime
authority and no longer describes this sidecar's responsibility.

The canonical configuration is:

```text
SDKWORK_AGENT_PROVIDER_HOST_ROOT=<package-root>/provider-host
```

BirdCoder, kernel release archives, and container images emit only that
configuration. Kernel consumers temporarily accept
`SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT` and `provider-runtime` as read-only inputs
for packages produced before this migration.

## Resolution And Cutover

1. Explicit worker and language-binary overrides retain highest precedence.
2. `SDKWORK_AGENT_PROVIDER_HOST_ROOT` wins over the legacy environment key.
3. A packaged `provider-host` directory wins over every discovered legacy
   directory, including a legacy directory nearer the executable.
4. New release artifacts contain no legacy root variable or directory.
5. Remove the legacy readers after 2026-10-01 once supported desktop, server,
   and container packages have moved beyond the compatibility window.

An explicitly empty canonical variable fails closed in installer/configuration
flows even when the legacy key is populated. This prevents an invalid new
configuration from silently activating stale package state.

## Rollback

Rollback uses a complete pre-migration package whose executable and legacy root
layout agree. Renaming only the directory inside a published archive is not a
valid rollback because it breaks artifact integrity and provenance.
