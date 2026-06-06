# @sdkwork/agent-backend-sdk

TypeScript package boundary for the SDKWork agent backend API SDK.

Generated transport output belongs under `generated/server-openapi` and is
created by:

```powershell
node ..\..\workspace-agent-sdkgen.mjs --family backend --mode apply
```

Do not hand-edit files under `generated/server-openapi`.

## SDKWork Documentation Contract

Domain: intelligence
Capability: agent-backend-sdk
Package type: node-package
Status: standard

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

- `powershell -NoProfile -Command "Get-Content specs/component.spec.json -Raw | ConvertFrom-Json | Out-Null"`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
