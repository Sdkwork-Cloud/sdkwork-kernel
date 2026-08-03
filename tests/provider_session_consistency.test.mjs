import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const providers = [
  {
    id: 'claude-code',
    adapter: 'src/lib.rs',
    integration: 'src/sdk_integration.rs',
    officialSdkHistory: true,
  },
  {
    id: 'codex',
    adapter: 'src/lib.rs',
    integration: 'src/sdk_integration.rs',
    officialSdkHistory: true,
    // Codex routes inventory/transcript through the official app-server client
    // protocol instead of the shared AgentSdk runtime helpers.
    sessionListMarker: /ClientRequest::ThreadList/u,
    sessionHistoryMarker: /ClientRequest::ThreadTurnsList/u,
  },
  { id: 'gemini-cli', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  {
    id: 'hermes',
    adapter: 'src/lib.rs',
    integration: 'src/sdk_integration.rs',
    officialSdkHistory: true,
    // Hermes routes inventory/transcript through the TUI gateway runtime
    // (hermes_state python backend), mirroring the desktop app.
    sessionListMarker: /SdkRuntimeOperation::SessionList/u,
    sessionHistoryMarker: /SdkRuntimeOperation::SessionHistory/u,
  },
  { id: 'mimo-code', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  { id: 'openclaw', adapter: 'src/lib.rs', integration: 'src/sdk_integration.rs' },
  {
    id: 'opencode',
    adapter: 'src/lib.rs',
    integration: 'src/sdk_integration.rs',
    officialSdkHistory: true,
  },
  { id: 'rig', adapter: 'src/session.rs', integration: 'src/sdk_integration.rs' },
];

async function read(relativePath) {
  return readFile(path.join(root, relativePath), 'utf8');
}

async function readRustSources(relativeDirectory) {
  const directory = path.join(root, relativeDirectory);
  const sources = [];
  const pending = [directory];
  while (pending.length > 0) {
    const currentDirectory = pending.pop();
    const entries = await readdir(currentDirectory, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(currentDirectory, entry.name);
      if (entry.isDirectory()) {
        pending.push(entryPath);
      } else if (entry.isFile() && entry.name.endsWith('.rs')) {
        sources.push(await readFile(entryPath, 'utf8'));
      }
    }
  }
  return sources.join('\n');
}

function assertUnifiedSnapshotInvariant(source, providerId) {
  const directProviderCall = new RegExp(
    `finalize_provider_session_snapshot\\(\\s*"${providerId}"\\s*,\\s*session\\s*\\)`,
    'u',
  );
  if (directProviderCall.test(source)) {
    return;
  }

  const providerConstant = source.match(
    new RegExp(`const\\s+([A-Z][A-Z0-9_]*)\\s*:\\s*&str\\s*=\\s*"${providerId}"\\s*;`, 'u'),
  );
  assert.ok(providerConstant, `${providerId} must declare its provider id when using a constant`);
  assert.match(
    source,
    new RegExp(
      `finalize_provider_session_snapshot\\(\\s*${providerConstant[1]}\\s*,\\s*session\\s*\\)`,
      'u',
    ),
    `${providerId} SessionAdapter must apply unified snapshot invariants`,
  );
}

test('every shipped provider exposes one complete unified session surface', async () => {
  for (const provider of providers) {
    const crateRoot = `agent-providers/crates/sdkwork-agent-provider-${provider.id}`;
    const [bindingText, adapterSource, integrationSource, cargoManifest] = await Promise.all([
      read(`bindings/agent-providers/${provider.id}/provider-binding.manifest.json`),
      readRustSources(`${crateRoot}/src`),
      read(`${crateRoot}/${provider.integration}`),
      read(`${crateRoot}/Cargo.toml`),
    ]);
    const binding = JSON.parse(bindingText);
    const lifecycle = binding.capabilities.find(
      (capability) => capability.capability_id === 'sdk.session.lifecycle',
    );

    assert.ok(lifecycle, `${provider.id} must declare sdk.session.lifecycle`);
    assert.equal(lifecycle.required, true, `${provider.id} session lifecycle must be required`);
    if (provider.officialSdkHistory) {
      assert.equal(
        lifecycle.execution_scope,
        'transport_runtime',
        `${provider.id} history must execute through its official SDK runtime`,
      );
      assert.ok(
        lifecycle.backends.some((backend) =>
          backend.runtime_operations.includes('session_list')
          && backend.runtime_operations.includes('session_history')),
        `${provider.id} official SDK backend must expose list and history`,
      );
      assert.match(
        `${integrationSource}\n${adapterSource}`,
        provider.sessionListMarker ?? /list_all_provider_sessions_from_runtime/u,
        `${provider.id} inventory must route through the official SDK runtime`,
      );
      assert.match(
        `${integrationSource}\n${adapterSource}`,
        provider.sessionHistoryMarker ?? /load_all_provider_messages_from_runtime/u,
        `${provider.id} transcript must route through the official SDK runtime`,
      );
      assert.doesNotMatch(
        `${adapterSource}\n${cargoManifest}`,
        /rusqlite|libsqlite3-sys|opencode\.db|\.claude[\\/]projects|JSONL transcript|jsonl transcript/u,
        `${provider.id} must not interpret provider-private persistence`,
      );
    } else {
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
    }
    assert.match(adapterSource, /impl SessionAdapter for \w+/u, `${provider.id} needs SessionAdapter`);
    assertUnifiedSnapshotInvariant(adapterSource, provider.id);
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
