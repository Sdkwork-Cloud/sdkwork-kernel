# SDKWork Agent Skill Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: agent skill discovery, descriptors, invocation, cancellation, health,
  policy, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.skill-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_MODEL_PROVIDER_SPI_SPEC.md`
  - `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`

The Agent Skill Provider SPI models reusable agent capability packs. A skill can
be model-invocable, tool-backed, workflow-backed, or host-provided. The kernel
does not treat skills as prompt text or product-specific plugins; it exposes
them as typed, discoverable, policy-aware runtime capabilities.

## 1. Provider Family

Skill providers use `provider_family: skill`.

Standard capabilities:

- `skill.discover`
- `skill.invoke`

Rules:

- Skills `MUST` have stable ids.
- Skill descriptors `MUST` declare invocation mode, side-effect level, policy
  categories, optional model hints, and allowed tools where applicable.
- Skill invocation `MUST` return normalized status and stable kernel errors.
- Skill output `MUST` be treated as untrusted unless the provider declares and
  enforces a stronger trust model.
- Skills `MUST NOT` bypass model, tool, host, memory, policy, or audit hooks.

## 2. Required SPI Objects

Required objects:

- `AgentSkillDescriptor`
- `AgentSkillInvocationMode`
- `AgentSkillRequest`
- `AgentSkillResult`
- `AgentSkillStatus`
- `AgentSkillProvider`

Required operations:

- `provider_manifest`
- `health`
- `list_skills`
- `describe_skill`
- `invoke_skill`

Optional operations:

- `cancel_skill`

Rules:

- Unsupported cancellation `MUST` return `capability_missing`.
- Provider-specific skill arguments `MUST` be carried as typed or namespaced
  request metadata, not hidden global state.
- Model hints are advisory. Runtime policy decides whether a hinted model may be
  used.

## 3. Runtime Registration

Runtime builders `MUST` provide manifest-only and typed registration paths for
skill providers.

Rules:

- Manifest-only skill providers are valid for negotiation and introspection.
- Direct local SPI execution against a manifest-only skill provider `MUST`
  return `provider_unavailable`.
- Typed skill providers `MUST` appear in runtime diagnostics as typed
  registered providers.
- The runtime capability manifest `MUST` preserve `provider_family: skill`,
  provider id, version, and declared `skill.*` capabilities.

## 4. Conformance

Required cases:

- Typed skill provider registers and appears in capability manifest.
- Manifest-only skill provider negotiates capabilities but direct execution
  returns `provider_unavailable`.
- Skills can be listed, described, and invoked.
- Missing skill ids return `capability_missing`.
- Skill side effects are represented by side-effect level and policy
  categories.
- Provider health appears in runtime diagnostics.
