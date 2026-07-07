import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

import { collectManifestValidationErrors } from './check-agent-provider-bindings.mjs';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, '..');

test('official SDK source must match the declared TypeScript SDK package', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.opencode',
    agent_id: 'agent.intelligence.opencode',
    display_name: 'OpenCode',
    description: 'Synthetic binding used to prove package drift is rejected.',
    version: '0.1.0',
    sdk_owner: 'sst',
    status: 'experimental',
    language_packages: {
      typescript: {
        package: '@opencode-ai/sdk',
        optional: false
      }
    },
    integration_sources: [
      {
        mode: 'official_sdk',
        package: 'opencode-ai'
      }
    ],
    capabilities: [
      {
        capability_id: 'sdk.model.chat',
        required: true,
        backends: [
          {
            kind: 'typescript_node',
            driver_id: 'driver.opencode.model.chat.ts',
            package: '@opencode-ai/sdk'
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/opencode/provider-binding.manifest.json'
  );

  assert.match(
    errors.join('\n'),
    /official_sdk package opencode-ai must match language_packages\.typescript\.package @opencode-ai\/sdk/
  );
});

test('OpenCode binding uses the typed SDK package as the official SDK source', () => {
  const manifestPath = path.join(
    root,
    'bindings',
    'agent-providers',
    'opencode',
    'provider-binding.manifest.json'
  );
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const officialSdkSource = manifest.integration_sources.find(
    (source) => source.mode === 'official_sdk'
  );

  assert.equal(
    officialSdkSource.package,
    manifest.language_packages.typescript.package,
    'OpenCode official_sdk source should use @opencode-ai/sdk, not the opencode-ai CLI package'
  );
});
