# SDKWork Agent Plugin Specs

This directory is the local standards index for the canonical
`sdkwork-kernel-plugins` package root. The canonical architecture name is
`plugin`; adapter and external-system work is modeled as plugin contributions.

Root SDKWork Agent and Code Kernel specifications remain authoritative. Local
plugin specs can narrow external mapping behavior, but they must not
contradict `../../specs/AGENT_KERNEL_SPEC.md`, `../../specs/CODE_KERNEL_SPEC.md`, or
their provider and security companion specs.

## Component

| Field | Value |
| --- | --- |
| Name | `sdkwork-kernel-plugins` |
| Type | `standard-assets` |
| Root | `sdkwork-kernel/sdkwork-kernel-plugins` |
| Domain | `intelligence` |
| Capability | `kernel.plugin` |
| Languages | `markdown`, `json`, `javascript` |
| Status | `standardizing` |

## Contract Manifest

- [component.spec.json](./component.spec.json) is the machine-readable component
  contract.
- [../../specs/KERNEL_PLUGIN_SPEC.md](../../specs/KERNEL_PLUGIN_SPEC.md)
  defines canonical SDKWork plugin rules.
- [EXTERNAL_AGENT_PLUGIN_SPEC.md](./EXTERNAL_AGENT_PLUGIN_SPEC.md)
  defines external agent plugin rules.
- [mappings/](./mappings/) records how each upstream project maps to SDKWork
  kernel surfaces.
- [manifests/](./manifests/) contains experimental SDKWork manifest examples.
- [conformance/](./conformance/) defines profile expectations before runtime
  code is introduced.

## Naming Policy

Standards, examples, directories, crates, and public APIs use `plugin` naming.
Legacy extension package names are not valid public surfaces in this package
root.

## Verification

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
```
