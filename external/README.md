# External Agent References

This directory contains third-party agent and agent-framework repositories as
Git submodules. They are reference implementations for SDKWork capability
mapping, provider design, runtime adapter behavior, and conformance work.

These repositories must not become direct dependencies of the kernel core.
SDKWork integration should happen through the existing kernel boundaries:

- `sdkwork-agent-kernel` provider SPI, runtime, policy, event, manifest, MCP,
  skill, collaboration, host, and protocol adapter contracts.
- `sdkwork-code-kernel` workspace, VCS, patch, terminal, verification,
  language, review, artifact, knowledge, and safety provider SPI.
- `sdkwork-agents` managed lifecycle and backend orchestration when an agent is
  installed, configured, audited, or exposed as a managed resource.

## Submodules

| Path | Upstream | Primary mapping focus |
| --- | --- | --- |
| `external/hermes-agent` | `https://github.com/NousResearch/hermes-agent.git` | General agent runtime, tool use, memory/context, skill-style behavior |
| `external/openclaw` | `https://github.com/openclaw/openclaw.git` | General agent application/runtime patterns and integration surfaces |
| `external/codex` | `https://github.com/openai/codex.git` | Code-agent runtime, workspace editing, terminal execution, patch/review flow |
| `external/claude-code` | `https://github.com/anthropics/claude-code.git` | Code-agent CLI behavior, task lifecycle, tool orchestration, permission flow |
| `external/opencode` | `https://github.com/opencode-ai/opencode.git` | Code-agent runtime, provider abstraction, terminal/workspace orchestration |
| `external/gemini-cli` | `https://github.com/google-gemini/gemini-cli.git` | Code-agent CLI behavior, model/tool integration, command workflow |
| `external/rig` | `https://github.com/0xPlaygrounds/rig.git` | Rust-native agent framework patterns for model/tool/provider composition |
| `external/mimo-code` | `https://github.com/XiaomiMiMo/MiMo-Code.git` | MiMo Code agent, code-agent runtime, tool orchestration, workspace management |

## Usage

Initialize submodules after cloning this repository:

```bash
git submodule update --init --recursive
```

Update all external references intentionally:

```bash
git submodule update --remote --merge external/hermes-agent
git submodule update --remote --merge external/openclaw
git submodule update --remote --merge external/codex
git submodule update --remote --merge external/claude-code
git submodule update --remote --merge external/opencode
git submodule update --remote --merge external/gemini-cli
git submodule update --remote --merge external/rig
git submodule update --remote --merge external/mimo-code
```

When a submodule is updated, review the changed upstream code and record the
SDKWork mapping impact before committing the new gitlink.

Submodule versions are pinned source references, not assertions that the
corresponding npm, PyPI, or Cargo package is the latest registry release. When a
provider package is intentionally upgraded, update the gitlink, binding
manifest version metadata, mapping document, and staging live SDK proof
together.

Runtime workers must not treat these source mirrors as installed SDK packages
unless the mirror exposes the package's own importable entry files or an
explicit package path is injected through the worker configuration. For example,
an unbuilt TypeScript source tree with only `package.json` does not satisfy the
live SDK resolver contract.
