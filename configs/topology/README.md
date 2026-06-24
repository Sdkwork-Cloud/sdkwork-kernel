# Kernel Topology Profiles

Machine contract: `../../specs/topology.spec.json`
Human summary: `../../docs/topology-standard.md`

## Default profile

`standalone.split-services.development`

## Commands

```bash
pnpm dev
pnpm dev:server:postgres:split-services:standalone
pnpm dev:server:postgres:unified-process:standalone
pnpm dev:server:postgres:split-services:cloud
pnpm topology:validate
pnpm test:topology-baggage
```

Profile env files in this directory are the only authoritative source for local bind addresses and public URLs. Do not hardcode ports in Rust crates or UI packages.
