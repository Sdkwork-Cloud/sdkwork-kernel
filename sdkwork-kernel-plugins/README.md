# SDKWork Agent Plugins

Domain: `intelligence`
Capability: `kernel.plugin`
Package type: standard assets and plugin contracts
Status: standard candidate

`sdkwork-kernel-plugins` is the canonical SDKWork Kernel plugin package root.
The canonical architecture name is `plugin`. External agent runtimes,
code-agent CLIs, SDKWork foundations, and agent frameworks referenced under
`external/` enter the kernel through plugin provider and adapter contributions.

This component is intentionally separate from `sdkwork-agent-kernel` and
`sdkwork-code-kernel`. The kernel crates define stable object models and SPI;
plugin crates map external or optional behavior into those contracts.

## Scope

This package owns:

- External source mapping documents (`specs/mappings/`).
- Experimental Agent and Provider manifest examples (`specs/manifests/`).
- Conformance profile documents for manifest-only, local-runtime, and
  process-adapter plugin modes.
- SDKWork-owned plugin crates: shared plugin contracts, Rig typed providers,
  OpenClaw/Hermes/Codex adapter + kernel plugins, and optional Drive/Knowledgebase
  foundation providers.
- Structure verification (`tests/kernel_plugin_structure.test.mjs`,
  `scripts/check-kernel-plugins.mjs`).

Runtime integration (server bootstrap, client SDK bridges, topology env) is
documented in `docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md`.

Out of scope here:

- Kernel core dependencies on `external/` (forbidden).
- Mapping-only upstreams without shipped adapters (see mapping status sections).
- UI product surfaces (owned by `sdkwork-kernel-ui`).

## Directory Model

```text
sdkwork-kernel-plugins/
|-- README.md
|-- crates/
|-- scripts/
|   `-- check-kernel-plugins.mjs
|-- specs/
|   |-- README.md
|   |-- component.spec.json
|   |-- EXTERNAL_AGENT_PLUGIN_SPEC.md
|   |-- mappings/
|   |-- manifests/
|   |   |-- agents/
|   |   |-- providers/
|   |   `-- protocol-adapters/
|   `-- conformance/
`-- tests/
    `-- kernel_plugin_structure.test.mjs
```

`crates/` contains SDKWork-owned plugin crates such as shared plugin contracts,
typed providers, protocol adapters, process adapters, and official foundation
plugins.

## Plugin Rule

External implementations are reference inputs. SDKWork compatibility is claimed
only through `KernelPluginManifest`, SDKWork manifests, typed provider SPI,
policy-aware host execution, kernel events, diagnostics, and conformance
reports.

## Verification

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
```

## SDKWork Documentation Contract

Domain: intelligence
Capability: kernel.plugin
Package type: node-package
Status: standardizing

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- None declared in `specs/component.spec.json`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`
- `node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml`
- `cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-rig/Cargo.toml`
- `cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-openclaw/Cargo.toml`
- `cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-hermes/Cargo.toml`
- `cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-drive/Cargo.toml`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml`

### Naming

- Canonical standard: `plugin`.
- Package roots: `sdkwork-kernel-plugins` (plugin core, foundation plugins) and `agent-providers` (framework provider integrations).
- Implemented crates: `sdkwork-agent-plugin-core`, `sdkwork-agent-provider-core`,
  `sdkwork-agent-provider-rig`, `sdkwork-agent-provider-openclaw`,
  `sdkwork-agent-provider-hermes`, `sdkwork-agent-provider-codex`,
  `sdkwork-kernel-plugin-drive`, and `sdkwork-kernel-plugin-knowledgebase`.

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
