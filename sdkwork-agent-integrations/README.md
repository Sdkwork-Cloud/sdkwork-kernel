# SDKWork Agent Integrations

Domain: `intelligence`
Capability: `external-agent-integrations`
Package type: standard assets and future adapter boundary
Status: standard candidate

`sdkwork-agent-integrations` owns the SDKWork-facing integration boundary for
external agent runtimes, code-agent CLIs, and agent frameworks referenced under
`external/`.

This component is intentionally separate from `sdkwork-agent-kernel` and
`sdkwork-code-kernel`. The kernel crates define stable object models and SPI;
this component maps third-party behavior into those contracts.

## Scope

This phase adds:

- External source mapping documents.
- Experimental Agent and Provider manifest examples.
- Conformance profile documents for manifest-only, local-runtime, and
  process-adapter integration modes.
- A lightweight structure verification script and Node test.

This phase does not add:

- Real third-party runtime execution.
- Rust provider crates.
- UI implementation.
- Direct dependencies from kernel core crates to `external/`.

## Directory Model

```text
sdkwork-agent-integrations/
|-- README.md
|-- crates/
|-- scripts/
|   `-- check-external-integrations.mjs
|-- specs/
|   |-- README.md
|   |-- component.spec.json
|   |-- EXTERNAL_AGENT_INTEGRATION_SPEC.md
|   |-- mappings/
|   |-- manifests/
|   |   |-- agents/
|   |   |-- providers/
|   |   `-- protocol-adapters/
|   `-- conformance/
`-- tests/
    `-- external_integration_structure.test.mjs
```

`crates/` is reserved for future implementation crates such as process
adapters, shared mapping helpers, and typed provider implementations. It stays
empty in this phase so the standard boundary can be reviewed before runtime
code exists.

## Integration Rule

External implementations are reference inputs. SDKWork compatibility is only
claimed through SDKWork manifests, typed provider SPI, policy-aware host
execution, kernel events, diagnostics, and conformance reports.

## Verification

```bash
node --test sdkwork-agent-integrations/tests/external_integration_structure.test.mjs
node sdkwork-agent-integrations/scripts/check-external-integrations.mjs
```

## SDKWork Documentation Contract

Domain: intelligence
Capability: external-agent-integrations
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

- `node --test sdkwork-agent-integrations/tests/external_integration_structure.test.mjs`
- `node sdkwork-agent-integrations/scripts/check-external-integrations.mjs`
- `cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/Cargo.toml`
- `cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
