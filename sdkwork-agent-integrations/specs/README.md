# SDKWork Agent Integrations Specs

This directory is the local standards index for `sdkwork-agent-integrations`.

Root SDKWork Agent and Code Kernel specifications remain authoritative. Local
integration specs can narrow external mapping behavior, but they must not
contradict `../../specs/AGENT_KERNEL_SPEC.md`, `../../specs/CODE_KERNEL_SPEC.md`, or
their provider and security companion specs.

## Component

| Field | Value |
| --- | --- |
| Name | `sdkwork-agent-integrations` |
| Type | `standard-assets` |
| Root | `sdkwork-birdcoder/kernel/sdkwork-agent-integrations` |
| Domain | `intelligence` |
| Capability | `external-agent-integrations` |
| Languages | `markdown`, `json`, `javascript` |
| Status | `standardizing` |

## Contract Manifest

- [component.spec.json](./component.spec.json) is the machine-readable component
  contract.
- [EXTERNAL_AGENT_INTEGRATION_SPEC.md](./EXTERNAL_AGENT_INTEGRATION_SPEC.md)
  defines external agent integration rules.
- [mappings/](./mappings/) records how each upstream project maps to SDKWork
  kernel surfaces.
- [manifests/](./manifests/) contains experimental SDKWork manifest examples.
- [conformance/](./conformance/) defines profile expectations before runtime
  code is introduced.

## Verification

```bash
node --test sdkwork-agent-integrations/tests/external_integration_structure.test.mjs
node sdkwork-agent-integrations/scripts/check-external-integrations.mjs
```
