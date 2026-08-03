# SDKWork Agent Installation And Configuration Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: agent package installation, uninstall, upgrade, configuration schema,
  validation, secret references, login authentication settings, LLM API key
  settings, machine-readable configuration profiles, migration plans, policy
  hooks, events, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.installation`, `agent-kernel.configuration`
- Implementation baseline: Rust kernel SPI
- Related:
  - [`AGENT_KERNEL_SPEC.md`](./AGENT_KERNEL_SPEC.md)
  - [`AGENT_MANIFEST_SPEC.md`](./AGENT_MANIFEST_SPEC.md)
  - [`AGENT_RUNTIME_SPEC.md`](./AGENT_RUNTIME_SPEC.md)
  - [`AGENT_SECURITY_POLICY_SPEC.md`](./AGENT_SECURITY_POLICY_SPEC.md)
  - [`AGENT_EVENT_TELEMETRY_SPEC.md`](./AGENT_EVENT_TELEMETRY_SPEC.md)
  - [`AGENT_CONFORMANCE_SPEC.md`](./AGENT_CONFORMANCE_SPEC.md)

This specification defines the standard installation and configuration surface
for every SDKWork-compatible agent. Code agents, workflow agents, operations
agents, research agents, and product assistants all share these mechanics.
Product applications may provide installers or registries, but the kernel-owned
contract is provider-neutral.

## 1. Design Goals

- Every agent package can declare how it is installed, upgraded, uninstalled,
  configured, validated, and audited.
- Installation mechanics are separate from product policy and concrete package
  registries.
- Agent configuration is typed and redaction-aware.
- Secret material such as login passwords and LLM API keys is represented by
  host secret references, not raw values.
- Configuration profiles and migration plans have machine-readable JSON
  contracts so installers, runtimes, UI packages, registries, and conformance
  suites can exchange them without product-specific DTOs.
- Installation, upgrade, uninstall, and configuration changes flow through
  policy hooks and event streams.
- Deterministic fake installers and configuration providers can prove
  conformance without touching a real registry or filesystem.

## 2. Agent Package Manifest

The Rust baseline exposes `AgentPackageManifest`.

Required package-level fields:

- `agent_id`
- `version`
- `source`
- `lifecycle`
- `provider_binding`
- `kernel_compatibility`
- `required_configuration_sections`
- `default_profile_id`

Rules:

- The package manifest `MUST` bind an agent version to a package source.
- The package manifest `MUST` declare whether install, uninstall, and upgrade
  lifecycle operations are supported.
- The package manifest `MUST` declare installer and configuration provider ids.
- The package manifest `MUST` declare required configuration section kinds,
  including base, login authentication, and LLM API key sections when required
  by the agent.
- The package manifest `MUST` declare compatible agent-kernel version ranges.
- Runtime and hosts `SHOULD` derive install, upgrade, and uninstall requests
  from the package manifest instead of duplicating agent id, version, source,
  and profile defaults.
- Runtime bootstrap `MUST` reject a package manifest whose `agent_id` does not
  match the loaded `AgentManifest`.
- Runtime bootstrap `MUST` reject a package manifest whose
  `kernel_compatibility` does not include the current agent-kernel version.

Rust package manifest objects:

- `AgentPackageManifest`
- `AgentPackageLifecycle`
- `AgentPackageProviderBinding`
- `AgentPackageVersionCompatibility`

Machine-readable validation schema:

- `schemas/agent-package-manifest.schema.json`

The Rust baseline supports `AgentPackageManifest::from_json(input)` for local
files, registries, remote archive metadata, and deterministic conformance
fixtures.

## 3. Installation SPI

The Rust baseline exposes `AgentInstaller`.

Required operations:

- `detect_installation(agent_id)`
- `configuration_spec(agent_id)`
- `plan_install(request)`
- `install(request)`
- `plan_upgrade(request)`
- `upgrade(request)`
- `plan_uninstall(request)`
- `uninstall(request)`
- `health()`

Rules:

- Installers `MUST` be able to plan before mutating host state.
- Installers `MUST` detect not-installed, installed, and degraded states and
  report the expected and detected versions of every managed dependency.
- Plans `MUST` list ordered steps and required policy categories.
- Installation and upgrade plans that write files, register manifests, mutate
  configuration, or replace versions `MUST` require policy.
- Upgrade plans `SHOULD` declare rollback requirements. Reports may expose an
  opaque rollback handle only when the host can actually consume it; lifecycle
  events `MUST NOT` contain that handle.
- Install, upgrade, and uninstall requests `MUST` support dry-run without
  mutating host state.
- Repeated install, upgrade, and uninstall calls `MUST` be idempotent.
- Install and upgrade `MUST` use exact package versions and verify the detected
  state after the package manager completes. Uninstall `MUST` verify that every
  owned dependency is absent before reporting success.
- Package-manager commands `MUST` be invoked without a command shell, use
  bounded output capture and timeouts, and expose only redacted safe errors.
- Output capture worker capacity `MUST` be bounded. Exhaustion `MUST` fail
  closed and terminate the package-manager process rather than create
  unbounded reader threads.
- Explicitly configured provider runtime roots and Python executable values
  `MUST NOT` be empty or non-Unicode. Invalid values `MUST` fail closed rather
  than select an implicit fallback runtime.
- Registry package names and versions `MUST` be canonical and exact. Tags,
  ranges, duplicate managed packages, and mixed package managers `MUST` fail
  validation before any package-manager process starts.
- On Windows, Node.js and `npm-cli.js` `MUST` resolve from the same canonical
  runtime directory. Installers `MUST NOT` combine executables and package
  manager scripts from separate `PATH` entries.
- npm lifecycle scripts `MUST` be disabled by default and require an explicit
  provider-level opt-in for a reviewed exact package release. Python registry
  installs `MUST` be non-interactive and wheel-only so source build hooks do
  not execute inside the host installer.
- Long-running package mutations `SHOULD` use a separate bounded timeout from
  lightweight detection and health commands so cold registry installs do not
  weaken runtime responsiveness checks.
- Commands that time out `MUST` terminate their owned process tree and bound
  output-pipe draining. Installer coordination `MUST` have a bounded wait:
  detections for one managed runtime may run concurrently, while install,
  upgrade, and uninstall hold an exclusive mutation lock across initial
  detection, mutation, and final verification. Shared and exclusive locking
  `MUST` coordinate across host processes that reference the same managed
  runtime, including path or executable aliases for that runtime, and an exited
  process `MUST` release its lock automatically. Persistent coordination
  artifacts `MUST` use a bounded lock pool rather than grow with every runtime
  identity observed by the host.
- Before mutation, package installers `MUST` capture the managed dependency
  state and validate that every detected version is an exact, safely
  restorable package version. An invalid or unrestorable snapshot `MUST` fail
  before any package-manager mutation. A package-manager failure or failed
  post-verification `MUST` trigger compensating restoration and verify the
  restored state before returning.
- Uninstall requests `MUST` distinguish removing the agent package from
  removing configuration or data.
- Package-only installers `MUST NOT` report configuration or data removal they
  did not perform. They `MUST` fail closed with an explicit host-store-required
  error when the request exceeds their ownership boundary.
- Embedded providers that cannot replace or remove themselves `MUST` return an
  explicit host-update-required error instead of reporting a false success.
- Reports `MUST` expose safe summaries and map to `agent.install.*` events.

Standard package sources:

- Local path.
- Registry package and version.
- Remote archive with checksum.

Standard install step kinds:

- Download package.
- Verify package.
- Write files.
- Register agent.
- Configure agent.
- Start agent.
- Stop agent.
- Backup current version.
- Replace version.
- Remove files.
- Remove configuration.

## 4. Configuration SPI

The Rust baseline exposes `AgentConfigurationProvider` and
`AgentConfigurationStore`.

Required operations:

- `configuration_spec(agent_id)`
- `validate_configuration(configuration)`
- `plan_configuration_upgrade(request)`
- `health()`

Model configuration materialization operations (required for external-CLI
providers whose request-time behavior is driven by their own config files):

- `materialize_model_configuration(request, application)` writes the applied
  model configuration (base URL, credential, default model) into the provider's
  native configuration surface (`~/.codex/config.toml`,
  `~/.claude/settings.json`, `~/.gemini/.env`, opencode config,
  `~/.hermes/config.yaml`, `~/.openclaw/openclaw.json`) so the external CLI
  actually uses the applied endpoint and credential at request time.
- `materialize_model_selection(request, application)` applies a model
  selection change to the same surface (no-op for providers that receive the
  model id per turn).
- `dematerialize_model_configuration(agent_id, profile_id)` restores the
  pre-materialization state (backup restore or file removal) when a profile is
  deprecated, archived, or removed.
- `read_model_configuration(agent_id, profile_id)` reads the currently
  effective model configuration back from the provider's native config surface
  and reports the materialization state so callers can detect drift and stale
  CLI state. Providers without a readable native surface (in-process
  providers) return `Unsupported`; the store profile remains the authoritative
  applied record.

Read-back state vocabulary (`ProviderModelMaterializationState`):

- `Unsupported` — the provider has no readable native config surface.
- `NotMaterialized` — the surface is absent or carries no SDKWork-managed
  entry.
- `Materialized` — the surface carries the SDKWork-managed marker and the
  materialized values.
- `Diverged` — the surface carries the SDKWork-managed marker but its values
  are missing, or the surface cannot be parsed.

Rules:

- Materialization `MUST` back up the existing provider config file before any
  mutation, write atomically, verify the read-back content, and restore the
  backup on failure or when the profile is dematerialized.
- Backups `MUST` be scoped by provider (`<file>.sdkwork.<provider_scope>.bak`)
  because multiple providers may share one config surface (Claude Code and
  Mimo Code both manage `~/.claude/settings.json`); one provider's
  dematerialization `MUST NOT` restore over or delete another provider's
  backup. When the config file did not exist before materialization, an empty
  backup marker is written so dematerialization removes only the file the
  provider itself created.
- Dematerialization `MUST` never delete a config file without a provider-scoped
  backup: the file may be user-owned or materialized by another provider, and
  deleting it would destroy user data (fail-closed).
- Materialization `MUST` merge into the existing provider config (user-defined
  relay entries, permissions, and unrelated settings survive) and `MUST` fail
  closed when the existing config cannot be parsed or when the merge target is
  not an object.
- Materialization `MUST` fail closed (return an error) when the provider
  config path cannot be resolved instead of silently skipping the write while
  the applied profile reports success.
- Read-back `MUST` determine `Materialized` from a SDKWork-managed marker
  written by materialization (for example `SDKWORK_MANAGED=true` in a settings
  env block or env file), never from the mere presence of user-configured
  relay values: a user-configured relay `MUST NOT` be reported as materialized,
  and a marker whose materialized values are missing `MUST` be reported as
  `Diverged`. Unparseable surfaces `MUST` be reported as `Diverged` with
  issues, not as `NotMaterialized`.
- Read-back `MUST NOT` expose raw credentials: only a boolean
  `credential_configured` flag may be reported.
- Raw credential values `MUST NOT` be stored inside kernel profiles; providers
  resolve secrets through the host secret surface (or the transient
  `api_key_materialization` request field, which `MUST` be redacted from
  `Debug` output and never persisted).

Profile store operations:

- `save_profile(profile)`
- `load_profile(agent_id, profile_id)`
- `list_profiles(agent_id)`
- `migrate_profile(plan, current_profile)`
- `archive_profile(request)`

Configuration specs are made of sections and fields. Standard section kinds:

- `Base`: identity/profile settings such as display name, locale, runtime
  defaults, and tenant-safe labels.
- `LoginAuth`: login username, password reference, OAuth reference, session
  token reference, or equivalent authentication settings.
- `LlmApiKey`: LLM provider API key references and model-provider credentials.
- `Runtime`: runtime options such as limits, timeouts, and feature flags.
- `Security`: sandbox, network, redaction, and audit configuration.
- `Custom`: namespaced extension configuration.

Rules:

- Agent configurations `MUST` be attachable to an `AgentConfigurationProfile`
  that records profile id, agent id, configuration schema version, lifecycle
  status, typed configuration values, and secret bindings.
- Profile statuses `MUST` distinguish draft, active, deprecated, and archived
  configuration records.
- Required fields `MUST` be explicit.
- Sensitive fields `MUST` carry redaction classification.
- API keys, passwords, tokens, and similar secrets `MUST` be represented as
  secret references.
- Login passwords, login tokens, OAuth credentials, LLM API keys, and custom
  secrets `MUST` be represented by `AgentSecretBinding` records that bind a
  configuration field key to a host secret reference and optional provider hint.
- Raw secret values `MUST` fail validation when a field requires a secret
  reference.
- Secret references in an agent profile `MUST` have matching profile secret
  bindings.
- Unknown fields `SHOULD` fail validation unless the agent declares an extension
  namespace for them.
- Configuration validation reports `MUST` list missing required fields and
  invalid fields separately.
- Configuration upgrades `MUST` be planned through
  `AgentConfigurationUpgradePlan` before mutating profile state.
- Configuration migration steps `SHOULD` declare whether a value is preserved,
  renamed, defaulted, removed, or whether a secret reference is preserved or
  rebound.
- Configuration migration plans that mutate profile data or secret bindings
  `MUST` require `agent.configure` policy.
- Profile persistence `MUST` flow through an `AgentConfigurationStore` SPI.
- Profile store implementations `MUST` be provider-neutral and must not imply a
  filesystem, database, browser storage, or product-specific persistence
  strategy.
- Profile archive requests `MUST` distinguish archiving configuration metadata
  from preserving or removing profile secret bindings.
- Installers `SHOULD` save the selected default profile after successful
  install when configuration is supplied.
- Upgrade flows `SHOULD` use configuration migration plans and then persist the
  migrated profile version.
- Uninstall flows `MUST` honor `remove_configuration` by archiving or removing
  profile records according to the host retention policy.

Machine-readable validation schemas:

- `schemas/agent-configuration-spec.schema.json`
- `schemas/agent-configuration-profile.schema.json`
- `schemas/agent-configuration-migration.schema.json`

The Rust baseline supports:

- `AgentConfigurationSpec::from_json(input)` for offline configuration schema
  files, registry-published agent configuration specs, UI form generation, and
  conformance fixtures.
- `AgentConfigurationProfile::from_json(input)` for configuration profile
  files, registry records, UI-exported profiles, and conformance fixtures.
- `AgentConfigurationUpgradePlan::from_json(input)` for versioned migration
  plans that preserve, rename, default, remove, preserve secret references, or
  rebind secret references.

JSON configuration spec rules:

- `manifest_type` `MUST` be `agent_configuration_spec`.
- Specs `MUST` declare agent id and at least one configuration section.
- Sections `MUST` declare section id, title, kind, and at least one field.
- Field definitions `MUST` declare key, label, value kind, required flag,
  secret-reference requirement, and redaction classification.
- Fields with `value_kind = secret_ref` `MUST` set `secret_ref_required = true`.
- Secret-reference fields `MUST` use a sensitive redaction classification.

JSON profile rules:

- `manifest_type` `MUST` be `agent_configuration_profile`.
- `configuration.entries` `MUST` carry `key`, `value_kind`, and `value`.
- Secret-bearing entries with `value_kind = secret_ref` `MUST` have matching
  `secret_bindings`.
- `secret_bindings` `MUST` carry `field_key`, `kind`, `secret_ref`, and an
  optional `provider_hint`.
- Profile JSON must never contain raw API keys, passwords, OAuth credentials,
  tokens, or other secret values.

JSON migration rules:

- `manifest_type` `MUST` be `agent_configuration_migration`.
- Migration plans `MUST` declare profile id, agent id, source configuration
  version, target configuration version, policy categories, and ordered steps.
- Secret-preserving and secret-rebinding steps `MUST` identify the affected
  field and secret binding kind.
- Rebinding an LLM provider API key `SHOULD` include `provider_hint` when the
  target provider matters.

## 5. Runtime Registration And Capability Manifest

Standard provider families:

- `agent_installer`
- `agent_configuration`

The Rust runtime baseline exposes explicit builder registration for these
provider families:

- `with_agent_package_manifest(package)`
- `register_agent_installer_provider(provider_id, version)`
- `register_agent_configuration_provider(provider_id, version)`
- `register_agent_installer(provider_id, version, provider)`
- `register_agent_configuration(provider_id, version, provider)`

`with_agent_package_manifest(package)` consumes an installable package manifest
and derives the standard installer/configuration provider manifests from its
provider binding. This keeps the package manifest as the deployment contract
and the capability manifest as the runtime negotiation result.

The `*_provider` forms register provider manifests for negotiation and
introspection. The typed forms register both the manifest and the concrete SPI
provider instance that runtime code can call. Hosts may use manifest-only
registration for discovery or remote providers, but local install/configuration
execution `MUST` use typed SPI registration.

Capability mapping:

| Capability | Provider family | Required operations | Side effect | Policy category |
| --- | --- | --- | --- | --- |
| `agent.install` | `agent_installer` | `detect_installation`, `configuration_spec`, `plan_install`, `install`, `health` | `side_effectful` | `agent.install` |
| `agent.uninstall` | `agent_installer` | `detect_installation`, `plan_uninstall`, `uninstall`, `health` | `destructive` | `agent.uninstall` |
| `agent.upgrade` | `agent_installer` | `detect_installation`, `plan_upgrade`, `upgrade`, `health` | `side_effectful` | `agent.upgrade` |
| `agent.configure` | `agent_configuration` | `configuration_spec`, `validate_configuration`, `health` | `side_effectful` | `agent.configure` |

Rules:

- Runtime capability negotiation `MUST` treat install, uninstall, upgrade, and
  configure as first-class kernel capabilities.
- Missing required `agent.install` or `agent.configure` capabilities `MUST`
  fail closed during bootstrap.
- Missing optional `agent.uninstall` or `agent.upgrade` capabilities `MUST`
  degrade the runtime instead of failing it.
- Capability manifests `MUST` include provider id, operations, side-effect
  classification, and policy category metadata for these capabilities.
- Runtime instances `MUST` expose typed accessors for registered installer and
  configuration providers.
- If a capability was negotiated through a manifest but no typed local provider
  instance is registered, runtime accessors `MUST` return a standard
  `provider_unavailable` error for that provider id.
- If no provider supports the requested lifecycle/configuration capability,
  runtime accessors `MUST` return `capability_missing`.
- If a typed local configuration provider is registered for a package, bootstrap
  `MUST` verify that its `AgentConfigurationSpec` declares every section kind
  required by the package manifest.
- Bootstrap `MUST` emit provider registration events for installer and
  configuration providers before the final ready/degraded/failed event.

## 6. Policy Categories

Standard policy categories:

- `agent.install`
- `agent.uninstall`
- `agent.upgrade`
- `agent.configure`

Rules:

- Installation, uninstall, upgrade, and configuration mutation `MUST` be
  policy-checkable.
- Policy requests `MUST` identify the agent id and version where relevant.
- Secret-bearing configuration validation `MUST` never put raw secret values in
  policy context, events, logs, or telemetry.

## 7. Events

Standard event families:

- `agent.install.provider.registered`
- `agent.configure.provider.registered`
- `agent.install.planned`
- `agent.install.installed`
- `agent.install.upgraded`
- `agent.install.uninstalled`
- `agent.install.failed`
- `agent.configure.validated`
- `agent.configure.failed`
- `agent.configure.profile.created`
- `agent.configure.profile.migrated`
- `agent.configure.profile.archived`

Rules:

- Events `MUST` include agent id, request id, status, and safe summary.
- Dry-run reports `MUST` emit `agent.install.planned`, `MUST NOT` emit an
  installed/upgraded/uninstalled success event, and `MUST NOT` claim a target
  version as an installed version.
- Events `MUST` use internal or stronger redaction classification when
  configuration or package metadata may be sensitive.
- Opaque rollback handles and raw package-manager output `MUST NOT` enter
  events. Untrusted event field delimiters and control characters `MUST` be
  encoded before projection into textual payloads.
- Event payloads `MUST` use schema version
  `sdkwork.agent.installation.report.v1` for installation lifecycle reports.
- Configuration profile lifecycle events `MUST` use payload schema version
  `sdkwork.agent.configuration.profile.v1`.

## 8. Conformance

Minimum tests:

- Agent package manifest declares lifecycle support, package source, provider
  bindings, kernel compatibility, default profile, and required configuration
  sections.
- Agent package manifest can derive install, upgrade, and uninstall requests.
- Runtime registers installer and configuration providers into the capability
  manifest with standard provider families.
- Runtime can derive installer/configuration provider manifests directly from
  an `AgentPackageManifest`.
- Runtime rejects package manifests for a different agent id or incompatible
  agent-kernel version.
- Runtime rejects typed configuration providers that omit package-required
  configuration sections.
- Runtime typed registry can return and invoke concrete `AgentInstaller` and
  `AgentConfigurationProvider` instances.
- Runtime reports `provider_unavailable` when a provider manifest is registered
  but no local typed SPI instance is available.
- Runtime exposes `agent.install`, `agent.uninstall`, `agent.upgrade`, and
  `agent.configure` capability metadata with operations, side-effect
  classification, and policy categories.
- Runtime fails closed when required install/configure capabilities are missing.
- Agent configuration spec declares base, login auth, and LLM API key sections.
- Agent configuration specs parse from machine-readable JSON and expose the
  standard configuration spec JSON Schema.
- Agent configuration spec JSON rejects secret reference fields that do not
  require secret references.
- Required configuration fields are enforced.
- LLM API key and login secret fields reject raw strings when secret references
  are required.
- Agent configuration profiles track profile id, agent id, configuration
  version, lifecycle status, typed configuration, and secret bindings.
- Agent configuration profiles parse from machine-readable JSON and expose the
  standard profile JSON Schema.
- Agent configuration profile JSON rejects secret references that do not have a
  matching profile secret binding.
- Agent configuration profile validation rejects secret references that lack a
  matching profile secret binding.
- Agent configuration providers can plan profile version migrations with
  preserve/rebind secret-reference steps.
- Agent configuration migration plans parse from machine-readable JSON and
  expose the standard migration JSON Schema.
- Agent configuration stores can save, load, list, migrate, and archive
  configuration profiles.
- Profile store records map to `agent.configure.profile.created`,
  `agent.configure.profile.migrated`, and
  `agent.configure.profile.archived` events.
- Installer can generate a side-effectful install plan with `agent.install`
  policy category.
- Installer detects absent, exact-version, and degraded installations.
- Installer can install and emit `agent.install.installed`.
- Installer can plan and execute upgrade with rollback metadata.
- Installer can plan and execute uninstall while explicitly removing or
  preserving configuration.
- Installer dry-run paths do not execute package-manager commands.
- Installer repeats install, upgrade, and uninstall idempotently and verifies
  the final detected state.
- Installer detection timeouts fail closed and package-only installers never
  claim host configuration or data removal.
- Installer permits concurrent detection for one runtime but serializes all
  package mutations across host processes, bounds lock waits, releases locks
  when a process exits, terminates timed-out process trees, and automatically
  restores the verified pre-mutation dependency state on error.

## 9. Acceptance Checklist

- [ ] Every installable agent package has a package manifest.
- [ ] Package manifests bind lifecycle support, provider ids, package source,
      compatible kernel versions, and required configuration sections.
- [ ] Runtime bootstrap can consume package manifests and fail closed on package
      identity, compatibility, lifecycle, provider-binding, or configuration
      section violations.
- [ ] Agent installers are provider-neutral.
- [ ] Installer and configuration providers are registered as first-class
      runtime provider families.
- [ ] Capability manifests expose lifecycle capability operations,
      side-effect levels, and policy categories.
- [ ] Installation, uninstall, upgrade, and configuration are policy-checkable.
- [ ] Installation detection, uninstall planning, dry-run, idempotency, exact
      versions, and post-mutation verification are covered by contract tests.
- [ ] Configuration specs are typed, sectioned, and redaction-aware.
- [ ] Configuration specs have a standard machine-readable JSON Schema and Rust
      parser.
- [ ] Configuration profiles are versioned and have explicit lifecycle status.
- [ ] Configuration profiles have a standard machine-readable JSON Schema and
      Rust parser.
- [ ] LLM API keys and login secrets use secret references.
- [ ] Secret references are bound through profile secret bindings.
- [ ] Configuration upgrades produce policy-checkable migration plans.
- [ ] Configuration migration plans have a standard machine-readable JSON
      Schema and Rust parser.
- [ ] Configuration stores persist, migrate, and archive profile records without
      owning concrete storage technology.
- [ ] Profile lifecycle records map to standard `agent.configure.profile.*`
      events.
- [ ] Reports map to kernel events.
- [ ] Deterministic fake installers can prove conformance.
