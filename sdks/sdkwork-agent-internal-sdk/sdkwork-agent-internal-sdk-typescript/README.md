# SDKWork Agent Internal SDK TypeScript

TypeScript composed client for `@sdkwork/agent-internal-sdk`.

`createClient` returns `SdkworkAgentInternalClient`. Generated resource methods
remain available under `client.intelligence`; typed named SSE streams are under
`client.streaming.model(...)` and `client.streaming.sessionEvents(...)`.
Streaming uses the generated authenticated transport and does not create a raw
HTTP fallback. Each decoded SSE event is limited to 1 MiB.

## Verification

```bash
pnpm verify
```

The verification command checks the SDK family boundary, runs strict TypeScript
compilation for the generated transport plus composed facade, and executes SSE
parser contract tests.
