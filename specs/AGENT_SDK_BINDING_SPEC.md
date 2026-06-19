# SDKWork Agent SDK Binding Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: external agent SDK binding manifests, language package metadata,
  capability backend candidates, driver references, and catalog layout
- Domain: `intelligence`
- Capability: `agent-kernel.agent-sdk-binding`
- Related:
  - `AGENT_SDK_SPI_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `SDK_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`

An Agent SDK binding manifest declares how SDKWork integrates with one external
agent product's native SDK surface.

## 1. Catalog Location

External agent SDK bindings live under:

```text
sdks/external-agent-sdks/<agent>/sdk-binding.manifest.json
```

Rules:

- This catalog is separate from SDKWork-owned SDK families under `sdks/`.
- Binding manifests are authored metadata, not generated SDK transport output.
- OpenAPI authorities referenced by bindings `MUST` follow `SDK_SPEC.md`
  regeneration rules when HTTP backends are used.

## 2. Manifest Schema

Machine-readable schema:

[`schemas/agent-sdk-binding.schema.json`](./schemas/agent-sdk-binding.schema.json)

Rust parsing type: `AgentSdkBindingManifest` in `sdkwork-agent-sdk-spi`.

## 3. Required Fields

- `schema_version`
- `manifest_type` = `agent_sdk_binding`
- `binding_id`
- `agent_id`
- `display_name`
- `description`
- `version`
- `sdk_owner`
- `capabilities`
- `status`

## 4. Capability Entries

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

## 5. Language Packages

Optional `language_packages` object maps language keys to package metadata:

| Key | Meaning |
| --- | --- |
| `rust` | crate name and version requirement |
| `typescript` | npm package name and version requirement |
| `python` | PyPI distribution or module name |

Rules:

- Language package metadata is declarative documentation for installers and
  backend hosts; it does not replace driver registration.
- `optional: true` marks a language SDK that may be absent on the host.

## 6. Selection Policy

Optional `selection_policy`:

- `default_backend_priority[]` overrides the global backend order for the
  entire binding.
- Per-capability backend arrays still take precedence over the global default.

## 7. Status And Compatibility

`status` values:

- `experimental`
- `standardizing`
- `stable`
- `deprecated`

Optional `kernel_compatibility` declares supported Agent Kernel semver ranges.

## 8. Agent Onboarding Checklist

- [ ] Add `sdks/external-agent-sdks/<agent>/sdk-binding.manifest.json`
- [ ] Register drivers in `sdkwork-agent-adapter-<agent>`
- [ ] Link binding id from plugin manifest
- [ ] Validate manifest against schema in CI
- [ ] Add negotiation contract tests with fake drivers
- [ ] Document supported language packages in agent adapter README
