# Kernel Topology Profiles

Machine contract: `../../specs/topology.spec.json`
Human summary: `../../docs/topology-standard.md`

## Default profile

`standalone.development`

## Commands

```bash
pnpm dev
pnpm dev:server:postgres:standalone
pnpm dev:server:postgres:cloud
pnpm topology:validate
pnpm test:topology-baggage
```

Profile env files in this directory are the only authoritative source for local bind addresses and public URLs. Do not hardcode ports in Rust crates or UI packages.

## Agent runtime plugin

Set `SDKWORK_KERNEL_AGENT_PLUGIN` to select the kernel agent plugin loaded by `sdkwork-agent-server`:

| Value | Hosted agent id |
| --- | --- |
| `rig` (production default) | `agent.intelligence.rig-general` |
| `openclaw` | `agent.intelligence.openclaw` |
| `hermes` | `agent.intelligence.hermes` |
| `codex` | `agent.intelligence.codex` |

Development profiles may override this value for SDK integration testing. Production profiles keep `rig` unless a deliberate rollout changes the topology env and hosted agent registry together.

See `docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md`.
