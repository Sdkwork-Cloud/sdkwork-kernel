# Kernel Topology Profiles

Machine contract: `../../specs/topology.spec.json`
Human summary: `../../docs/topology-standard.md`

## Default profile

`self-hosted.split-services.development`

## Commands

```bash
pnpm kernel:dev
pnpm kernel:dev:unified
pnpm kernel:dev:cloud
pnpm topology:validate
pnpm test:topology-baggage
```

Profile env files in this directory are the only authoritative source for local bind addresses and public URLs. Do not hardcode ports in Rust crates or UI packages.
