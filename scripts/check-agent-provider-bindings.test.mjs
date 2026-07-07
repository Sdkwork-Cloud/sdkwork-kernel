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

test('all checked binding manifests pass authored metadata validation', () => {
  const catalogRoot = path.join(root, 'bindings', 'agent-providers');
  const errors = [];

  for (const entry of fs.readdirSync(catalogRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }
    const manifestPath = path.join(catalogRoot, entry.name, 'provider-binding.manifest.json');
    if (!fs.existsSync(manifestPath)) {
      continue;
    }
    const relativePath = path.relative(root, manifestPath).replaceAll('\\', '/');
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    errors.push(...collectManifestValidationErrors(manifest, relativePath));
  }

  assert.deepEqual(errors, []);
});

test('migration seed uses the OpenCode SDK package, not the CLI package', () => {
  const migrationScript = fs.readFileSync(
    path.join(root, 'scripts', 'migrate-provider-binding-manifests.mjs'),
    'utf8'
  );

  assert.match(migrationScript, /package: '@opencode-ai\/sdk'/);
  assert.doesNotMatch(migrationScript, /package: 'opencode-ai'/);
});

test('provider documentation does not retain stale unimplemented or pending binding status', () => {
  const mappingDocs = [
    'claude-code',
    'opencode',
    'mimo-code'
  ];

  for (const mapping of mappingDocs) {
    const content = fs.readFileSync(
      path.join(root, 'sdkwork-kernel-plugins', 'specs', 'mappings', `${mapping}.md`),
      'utf8'
    );
    assert.doesNotMatch(
      content,
      /SDKWork adapter code is not implemented|binding manifest pending/i,
      `${mapping} mapping should not retain stale implementation status`
    );
  }

  for (const relativePath of [
    'docs/product/prd/PRD-02-provider-integration-requirements.md',
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md'
  ]) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.doesNotMatch(
      content,
      /Mimo Code[^\n]*(?:binding manifest pending|TBD|pending binding)/i,
      `${relativePath} should reflect the current Mimo binding manifest`
    );
  }
});
