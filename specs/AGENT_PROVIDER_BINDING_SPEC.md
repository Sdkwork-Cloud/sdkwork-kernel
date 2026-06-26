# SDKWork Agent Provider Binding Specification

- Version: 0.2.0
- Status: standard candidate
- Scope: external agent provider binding manifests, integration sources,
  language package metadata, capability transport candidates, driver references,
  and catalog layout
- Domain: `intelligence`
- Capability: `agent-kernel.agent-provider-binding`
- Supersedes: `AGENT_SDK_BINDING_SPEC.md` (archival alias retained one release cycle)
- Related:
  - `AGENT_PROVIDER_INTEGRATION_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `SDK_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`

A provider binding manifest declares how SDKWork integrates with one external
agent framework through official SDKs, Rust crates, source trees, HTTP/OpenAPI,
or IPC transports.

## 1. Catalog Location

External agent provider bindings live under:

```text
bindings/agent-providers/<framework>/provider-binding.manifest.json
```

Rules:

- This catalog is separate from SDKWork-owned SDK families under `sdkwork-agents/sdks/`.
- Binding manifests are authored metadata, not generated SDK transport output.
- OpenAPI authorities referenced by bindings `MUST` follow `SDK_SPEC.md`
  regeneration rules when HTTP transports are used.

## 2. Manifest Schema

Machine-readable schema:

[`schemas/agent-sdk-binding.schema.json`](./schemas/agent-sdk-binding.schema.json)

Rust parsing type: `AgentSdkBindingManifest` in `sdkwork-agent-provider-spi`.

## 3. Required Fields

- `schema_version`
- `manifest_type` = `agent_provider_binding`
- `binding_id` (`binding.agent-provider.<framework>`)
- `agent_id`
- `display_name`
- `description`
- `version`
- `sdk_owner`
- `integration_sources[]`
- `capabilities`
- `status`

## 4. Integration Sources

Each `integration_sources[]` entry `MUST` include `mode`:

| Mode | Meaning |
| --- | --- |
| `official_sdk` | Published npm/PyPI SDK |
| `rust_crate` | Cargo crate |
| `source_tree` | Vendored or submodule source |
| `npm_package` | npm package without official SDK branding |
| `python_module` | Importable Python module |
| `http_openapi` | HTTP/OpenAPI authority |
| `ipc_protocol` | stdio/socket IPC transport |

## 5. Capability Entries

Each capability entry `MUST` include:

- `capability_id`
- `required`
- `backends[]`

Each backend candidate `MUST` include:

- `kind`
- `driver_id`

Optional backend fields:

- `crate` for `rust_native`
- `package` for `typescript_node`
- `python_module` for `python_process`
- `openapi_authority` for `http_openapi`
- `transport` for `ipc_protocol`

Rules:

- Backend arrays are priority-ordered unless `selection_policy` overrides them.
- Every `driver_id` `MUST` match a contributed driver at runtime.
- Required capabilities `MUST` declare at least one backend candidate.

## 6. Language Packages

Optional `language_packages` object maps language keys to package metadata:

| Key | Meaning |
| --- | --- |
| `rust` | crate name and version requirement |
| `typescript` | npm package name and version requirement |
| `python` | PyPI distribution or module name |

## 7. Selection Policy

Optional `selection_policy.default_backend_priority[]` overrides the global
transport order for the entire binding.

## 8. Provider Onboarding Checklist

- [ ] Add `bindings/agent-providers/<framework>/provider-binding.manifest.json`
- [ ] Implement `agent-providers/crates/sdkwork-agent-provider-<framework>`
- [ ] Register drivers and wire `ProviderTransportBootstrap`
- [ ] Validate manifest against schema in CI
- [ ] Add negotiation and transport contract tests
- [ ] Expose bootstrap through `sdkwork-agents-runtime-facade` when product-facing
