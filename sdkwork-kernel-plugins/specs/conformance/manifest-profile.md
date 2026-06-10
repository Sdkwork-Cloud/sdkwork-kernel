# External Plugin Manifest Profile

## Purpose

The manifest profile verifies that an external plugin can be discovered,
described, and negotiated without executing third-party code.

## Required Evidence

- External submodule path exists.
- Mapping document exists.
- Agent or provider manifest parses as JSON.
- Manifest uses stable SDKWork ids.
- Manifest status is `experimental`, `candidate`, `stable`, `deprecated`, or
  `removed`.
- Required capabilities use SDKWork capability ids.
- Security profile or security requirements set `fail_closed` to `true`.
- Raw secrets do not appear in manifests.
- Extensions use namespaced `sdkwork.external.*` keys.

## Required Cases

| Case Id | Description |
| --- | --- |
| `external.manifest.source.present` | The mapped `external/*` source path exists. |
| `external.manifest.mapping.present` | A mapping document exists for the upstream. |
| `external.manifest.json.valid` | Manifest JSON parses. |
| `external.manifest.ids.namespaced` | Agent and provider ids follow SDKWork naming rules. |
| `external.manifest.security.fail_closed` | Security metadata requires fail-closed behavior. |
| `external.manifest.execution.not_claimed` | Manifest-only entries do not claim executable local runtime behavior. |

## Non-Goals

This profile does not invoke models, tools, terminals, workspaces, networks, or
third-party processes.
