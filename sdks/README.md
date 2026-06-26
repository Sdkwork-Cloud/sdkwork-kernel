# SDKWork Kernel SDK Workspace

This directory is the application-root SDK workspace for **kernel runtime** surfaces only.

It owns the internal runtime API authority, derived generator inputs, generated SDK output boundaries, and verification entrypoints for trusted in-app automation and kernel UI.

Managed agents open/app/backend SDK families (`sdkwork-agents-sdk`, `sdkwork-agents-app-sdk`, `sdkwork-agents-backend-sdk`) are owned by the sibling application repository [`../sdkwork-agents/sdks/`](../sdkwork-agents/sdks/).

This workspace follows the root SDK standard in `specs/SDK_SPEC.md`. All SDKs are generated through:

```text
..\sdkwork-sdk-generator
```

The canonical CLI entrypoint is:

```text
..\sdkwork-sdk-generator\bin\sdkgen.js
```

Do not use handwritten SDK forks, copied generated output, or raw HTTP wrappers for remote APIs. If an SDK method is missing, fix the API/OpenAPI/generator chain and regenerate.

## SDK Families (kernel-owned)

| Family | Authority | Prefix | Package | Audience |
| --- | --- | --- | --- | --- |
| `sdkwork-agent-internal-sdk` | `sdkwork-agent-internal-api` | `/internal/v3/api` | `@sdkwork/agent-internal-sdk` | Kernel UI, embedded consoles, trusted in-app automation |

## Boundaries

- Authority OpenAPI files live under each family as `openapi/<authority>.openapi.yaml`.
- Derived generator inputs live beside authority files as `openapi/<authority>.sdkgen.yaml`.
- Generated TypeScript transport output belongs under `<family>/<family>-typescript/generated/server-openapi`.
- Handwritten wrappers or composed facades must stay outside `generated/server-openapi`.
- Generated SDK output must not be hand-edited.

## Commands

Run from repository root.

```powershell
node .\sdks\materialize-agent-internal-api-openapi.mjs
node .\scripts\check-agent-sdk-workspace.mjs
```

Preview TypeScript SDK generation:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode dry-run
```

Apply TypeScript SDK generation:

```powershell
node .\sdks\workspace-agent-sdkgen.mjs --mode apply
```

Verify internal SDK family:

```powershell
node .\sdks\sdkwork-agent-internal-sdk\bin\verify-sdk.mjs
```

The generator command always uses `--standard-profile sdkwork-v3`.

## Versioning

The initial SDK package version is `0.1.0`. Generator output is reproducible from:

- authority OpenAPI
- derived `*.sdkgen.yaml`
- `sdks/_shared/agent-sdk-families.mjs`
- `sdkwork-sdk-generator`
- `--standard-profile sdkwork-v3`
