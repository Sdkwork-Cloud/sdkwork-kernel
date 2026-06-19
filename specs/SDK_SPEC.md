# SDKWork SDK Standard

This specification defines the SDK generation standard for SDKWork kernel,
agent, code-agent, business, and application SDKs.

## Canonical Generator

All SDKWork SDKs `MUST` be generated through the SDKWork SDK generator located
at:

```text
..\sdkwork-sdk-generator
```

The canonical CLI entrypoint is:

```text
..\sdkwork-sdk-generator\bin\sdkgen.js
```

Repository automation `MUST` default to that entrypoint. `SDKWORK_SDKGEN_PATH`
`MAY` override the entrypoint only when the same `sdkwork-sdk-generator`
product is checked out or installed at a different local path. The override
must not point to a different generator implementation.

The following SDK production modes are forbidden:

- `sdkwork-code-generator` for SDK generation.
- Handwritten SDKs that duplicate generated transport clients.
- App-local SDK forks copied from generated output.
- Manual patches inside generated output directories.
- Raw HTTP wrappers used because a generated SDK method is missing.

If a generated SDK surface is incomplete, the source API contract, OpenAPI
authority document, or generator input `MUST` be fixed first, then the SDK
`MUST` be regenerated through `sdkwork-sdk-generator`.

## Source Of Truth

SDK generation follows this source chain:

1. Runtime or business API contract defines the real capability.
2. Authority OpenAPI 3.x document describes the public API surface.
3. Derived generator input adapts the authority document to the generator
   without changing authority ownership, path prefixes, operation IDs, auth
   semantics, or problem response semantics.
4. `sdkwork-sdk-generator` generates language SDK transport output.
5. Handwritten service facades, wrappers, or adapters compose the generated SDK
   outside generated output directories.

Generated SDK files are compatibility surfaces. Generated SDK output must not be hand-edited.
It must be reproducible from the API contract, authority OpenAPI, derived
generator input, generator version, generator command, standard profile, and
package metadata.

## Workspace Boundaries

SDK workspaces `MUST` separate these files:

- Authority OpenAPI: `openapi/<authority>.openapi.yaml`
- Generator input: `openapi/<authority>.sdkgen.yaml`
- Generated output: a generator-owned directory such as
  `generated/server-openapi`
- Handwritten composition: wrapper, adapter, custom, or service directories
  outside generated output
- Verification: workspace-local scripts that validate package metadata,
  authority ownership, generated boundaries, and forbidden raw bypasses

Generated output directories are generator-owned. Developers must not manually
edit files under those directories. When generated output is wrong, fix the
contract or generator chain and regenerate.

## Standard Profile

SDKWork v3 SDKs `MUST` use:

```text
--standard-profile sdkwork-v3
```

The standard profile validates SDKWork API conventions, including path prefix
ownership, operation IDs, token headers, problem responses, and generated
metadata. Where the current generator profile does not yet support a specific
public prefix, a repository-local materialization step may derive the SDK from
a strict-profile generated source only when the authority OpenAPI remains
complete, the derivation is deterministic, and the generated result remains
traceable to `sdkwork-sdk-generator`.

## Agent SDK Families

For `app=agent`, the root SDK workspace is `sdks/`. It defines these SDK
families:

| Family | Authority | Prefix | Package | Audience |
| --- | --- | --- | --- | --- |
| `sdkwork-agent-sdk` | `sdkwork-agent-open-api` | `/agent/v3/api` | `@sdkwork/agent-sdk` | Developer and integration authors |
| `sdkwork-agent-app-sdk` | `sdkwork-agent-app-api` | `/app/v3/api` | `@sdkwork/agent-app-sdk` | App, desktop, mobile, H5, and user-facing clients |
| `sdkwork-agent-backend-sdk` | `sdkwork-agent-backend-api` | `/backend/v3/api` | `@sdkwork/agent-backend-sdk` | Backend console, operators, automation, and control-plane integrations |

All three families are owned by the SDK standard in this document and by the
root `sdks/` workspace. No consumer may bypass these SDK families with local
HTTP implementations for remote agent business APIs.

External third-party agent native SDK bindings are cataloged separately under
`sdks/external-agent-sdks/` and governed by `AGENT_SDK_BINDING_SPEC.md`. Those
bindings describe how SDKWork integrates with agent products such as Codex,
Hermes, and OpenClaw; they do not replace SDKWork-owned SDK families above.

## Regeneration Contract

SDK generation commands `MUST` be idempotent. A dry run must report the planned
change fingerprint before apply mode writes files. Apply mode must require the
expected fingerprint when supported by the generator.

SDK generation reports should include:

- Application domain.
- SDK family.
- Authority.
- Input OpenAPI path.
- Output generated path.
- Package name.
- API prefix.
- Generator entrypoint.
- Standard profile.
- Version.
- Change fingerprint.

Reports are audit artifacts; generated transport code remains the generator
artifact.

## Conformance

A repository conforms to this SDK standard when:

- `specs/SDK_SPEC.md` is present and referenced by the relevant kernel specs.
- SDK automation defaults to
  `..\sdkwork-sdk-generator\bin\sdkgen.js`.
- SDK docs state that all SDKs are generated through
  `..\sdkwork-sdk-generator`.
- SDK generation commands use `--standard-profile sdkwork-v3` where the API is
  part of the SDKWork v3 contract.
- Generated output is isolated from handwritten composition.
- Missing SDK methods are closed through API/OpenAPI/generator changes, not
  through app-local raw HTTP or copied SDK forks.
- Static checks reject the old business-local generator location as a
  canonical generator path.
