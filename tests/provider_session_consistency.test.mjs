import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const providers = [
  { id: 'claude-code', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'codex', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'gemini-cli', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'hermes', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'mimo-code', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'openclaw', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'opencode', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'rig', adapter: 'src/session.rs', integration: 'src/sdk_integration.rs' },
];

async function read(relativePath) {
  return readFile(path.join(root, relativePath), 'utf8');
}

test('every shipped provider exposes one complete unified session surface', async () => {
  for (const provider of providers) {
    const crateRoot = `agent-providers/crates/sdkwork-agent-provider-${provider.id}`;
    const [bindingText, adapterSource, integrationSource] = await Promise.all([
      read(`bindings/agent-providers/${provider.id}/provider-binding.manifest.json`),
      read(`${crateRoot}/${provider.adapter}`),
      read(`${crateRoot}/${provider.integration}`),
    ]);
    const binding = JSON.parse(bindingText);
    const lifecycle = binding.capabilities.find(
      (capability) => capability.capability_id === 'sdk.session.lifecycle',
    );

    assert.ok(lifecycle, `${provider.id} must declare sdk.session.lifecycle`);
    assert.equal(lifecycle.required, true, `${provider.id} session lifecycle must be required`);
    assert.equal(
      lifecycle.execution_scope,
      'provider_local',
      `${provider.id} session lifecycle must execute through provider-local SPI`,
    );
    assert.ok(
      lifecycle.backends.every(
        (backend) =>
          backend.runtime_operations.length === 1 && backend.runtime_operations[0] === 'ping',
      ),
      `${provider.id} provider-local session backends may expose ping only`,
    );
    assert.match(adapterSource, /impl SessionAdapter for \w+/u, `${provider.id} needs SessionAdapter`);
    assert.match(
      adapterSource,
      new RegExp(`finalize_provider_session_snapshot\\("${provider.id}",\\s*session\\)`),
      `${provider.id} SessionAdapter must apply unified snapshot invariants`,
    );
    assert.doesNotMatch(
      adapterSource,
      /token_usage\.total_tokens\s*=\s*external\.[^;]+\+\s*external\./u,
      `${provider.id} token aggregation must not overflow`,
    );
    assert.doesNotMatch(
      adapterSource,
      /message_count\s*=\s*[^;]+\bas\s+u32/u,
      `${provider.id} message count conversion must not truncate`,
    );
    assert.match(
      adapterSource,
      /define_provider_lifecycle_provider!/u,
      `${provider.id} needs the unified lifecycle provider`,
    );
    assert.match(
      integrationSource,
      /pub lifecycle: \w+LifecycleProvider/u,
      `${provider.id} SDK integration must expose lifecycle`,
    );
    assert.match(
      integrationSource,
      /pub session_adapter: \w+Adapter/u,
      `${provider.id} SDK integration must expose the session adapter`,
    );
  }
});
