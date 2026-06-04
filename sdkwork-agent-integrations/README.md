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
