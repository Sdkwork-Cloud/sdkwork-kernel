# SDKWork Agent Host Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: filesystem, process, network, secrets, storage, time, environment,
  executor providers, sandbox policy, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.host-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

Host providers expose platform capabilities to the Agent Kernel. They are the
only standard path for filesystem, process, network, secret, storage, time,
environment, and executor operations.

## 1. Host Provider Families

Families:

- `filesystem`
- `process`
- `network`
- `secrets`
- `storage`
- `time`
- `environment`
- `executor`

Rules:

- Host providers `MUST` declare capabilities in provider manifests.
- Host operations with side effects or sensitive data `MUST` pass policy.
- Kernel core `MUST NOT` perform direct host side effects outside host SPI.
- Host providers `MUST` provide deterministic fake implementations for tests.
- Runtime registries `MUST` support multiple typed host providers in one agent
  runtime. The default provider is the deterministic first registered provider;
  callers that require a specific local, remote, sandboxed, containerized, or
  platform host implementation `MUST` select it by provider id.
- Host provider diagnostics `MUST` report health per registered provider id
  without collapsing multiple host implementations into one manifest entry.

## 2. Filesystem Provider

Required operations:

- `read`
- `write`
- `list`
- `stat`
- `delete`
- `watch` when supported

Rules:

- Paths `MUST` be validated against allowed roots.
- Path traversal `MUST` be denied.
- Writes and deletes `MUST` require policy.
- File payload telemetry `MUST` respect redaction.

## 3. Process Provider

Required operations:

- `spawn`
- `stream_output`
- `cancel`
- `status`

Rules:

- Process execution `MUST` require policy.
- Commands `MUST` declare working directory, env policy, timeout, and output
  redaction.
- Cancellation `MUST` be best-effort and observable.
- Exit status `MUST` be normalized.

## 4. Network Provider

Required operations:

- `request`
- `stream`
- `cancel`

Rules:

- Network access `MUST` require policy unless explicitly classified as safe by
  host configuration.
- Host/protocol allowlists `SHOULD` be supported.
- Request/response payload telemetry `MUST` respect redaction.

## 5. Secrets Provider

Required operations:

- `resolve_secret_ref`
- `list_secret_refs` when allowed
- `health`

Rules:

- Raw secrets `MUST NOT` be stored in manifests.
- Secret reads `MUST` require policy.
- Secret values `MUST` not appear in logs, events, traces, or model prompts
  unless explicit policy allows.

## 6. Storage, Time, Environment, Executor

Rules:

- Storage provider `MUST` enforce scope and retention.
- Time provider `SHOULD` be injectable for deterministic tests.
- Environment provider `MUST` classify sensitive environment values.
- Executor provider `MUST` support cancellation where possible.

## 7. Conformance

Required cases:

- Path traversal is denied.
- Filesystem write requires policy.
- Process execution requires policy and enforces timeout.
- Process cancellation is observable.
- Network request respects policy.
- Raw secret is never logged.
- Time provider can be faked for tests.
- Storage provider enforces scope.

## 8. Acceptance Checklist

- [ ] Host operations go through host SPI.
- [ ] Filesystem roots and path traversal are enforced.
- [ ] Process execution is policy-controlled and cancellable.
- [ ] Network access is policy-controlled.
- [ ] Secrets use references and redaction.
- [ ] Storage/time/environment/executor are injectable.
- [ ] Conformance tests cover sandbox and deterministic fakes.
