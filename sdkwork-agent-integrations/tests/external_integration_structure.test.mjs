import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');
const integrationRoot = path.join(root, 'sdkwork-agent-integrations');

const upstreams = [
  'hermes-agent',
  'openclaw',
  'codex',
  'claude-code',
  'opencode',
  'gemini-cli',
  'rig'
];

const requiredFiles = [
  'README.md',
  'specs/README.md',
  'specs/component.spec.json',
  'specs/EXTERNAL_AGENT_INTEGRATION_SPEC.md',
  'specs/conformance/manifest-profile.md',
  'specs/conformance/local-runtime-profile.md',
  'specs/conformance/process-adapter-profile.md',
  'specs/manifests/agents/external-code-agent-runtime.agent.json',
  'specs/manifests/agents/external-general-agent-runtime.agent.json',
  'specs/manifests/providers/codex-process.provider.json',
  'specs/manifests/providers/rig-rust.provider.json',
  'specs/manifests/protocol-adapters/external-process.protocol-adapter.json',
  'crates/sdkwork-agent-integration-core/Cargo.toml',
  'crates/sdkwork-agent-integration-core/README.md',
  'crates/sdkwork-agent-integration-core/src/lib.rs',
  'crates/sdkwork-agent-integration-rig/Cargo.toml',
  'crates/sdkwork-agent-integration-rig/README.md',
  'crates/sdkwork-agent-integration-rig/src/lib.rs',
  'scripts/check-external-integrations.mjs'
];

for (const upstream of upstreams) {
  requiredFiles.push(`specs/mappings/${upstream}.md`);
}

test('external integration standards assets are present', () => {
  for (const relativePath of requiredFiles) {
    const absolutePath = path.join(integrationRoot, relativePath);
    assert.equal(fs.existsSync(absolutePath), true, `${relativePath} should exist`);
    assert.equal(fs.statSync(absolutePath).isFile(), true, `${relativePath} should be a file`);
  }
});

test('external submodules are present for every mapped upstream', () => {
  for (const upstream of upstreams) {
    const submodulePath = path.join(root, 'external', upstream);
    assert.equal(fs.existsSync(submodulePath), true, `external/${upstream} should exist`);
    assert.equal(fs.statSync(submodulePath).isDirectory(), true, `external/${upstream} should be a directory`);
  }
});

test('JSON manifests parse and keep expected manifest types', () => {
  const jsonExpectations = new Map([
    ['specs/component.spec.json', 'sdkwork.component.spec'],
    ['specs/manifests/agents/external-code-agent-runtime.agent.json', 'agent'],
    ['specs/manifests/agents/external-general-agent-runtime.agent.json', 'agent'],
    ['specs/manifests/providers/codex-process.provider.json', 'provider'],
    ['specs/manifests/providers/rig-rust.provider.json', 'provider'],
    ['specs/manifests/protocol-adapters/external-process.protocol-adapter.json', 'provider']
  ]);

  for (const [relativePath, expectedKind] of jsonExpectations) {
    const content = fs.readFileSync(path.join(integrationRoot, relativePath), 'utf8');
    const parsed = JSON.parse(content);
    const actualKind = parsed.kind ?? parsed.manifest_type;
    assert.equal(actualKind, expectedKind, `${relativePath} should use ${expectedKind}`);
  }
});

test('experimental manifests use schema-compatible naming and fail-closed security', () => {
  const manifestPaths = [
    'specs/manifests/agents/external-code-agent-runtime.agent.json',
    'specs/manifests/agents/external-general-agent-runtime.agent.json',
    'specs/manifests/providers/codex-process.provider.json',
    'specs/manifests/providers/rig-rust.provider.json',
    'specs/manifests/protocol-adapters/external-process.protocol-adapter.json'
  ];
  const extensionKeyPattern = /^(sdkwork|[a-z0-9-]+\.[a-z0-9.-]+)\.[a-z0-9.-]+$/;
  const capabilityPattern = /^[a-z0-9_.-]+$/;

  for (const relativePath of manifestPaths) {
    const manifest = JSON.parse(
      fs.readFileSync(path.join(integrationRoot, relativePath), 'utf8')
    );
    const security = manifest.security_profile ?? manifest.security_requirements;

    assert.equal(manifest.status, 'experimental', `${relativePath} should be experimental`);
    assert.equal(security.fail_closed, true, `${relativePath} should fail closed`);

    for (const key of Object.keys(manifest.extensions ?? {})) {
      assert.match(key, extensionKeyPattern, `${relativePath} extension key ${key} should be schema-compatible`);
    }

    for (const capability of [
      ...(manifest.capabilities ?? []),
      ...(manifest.required_capabilities ?? []).map((entry) => entry.capability_id),
      ...(manifest.optional_capabilities ?? []).map((entry) => entry.capability_id)
    ]) {
      assert.match(capability, capabilityPattern, `${relativePath} capability ${capability} should be namespaced`);
    }
  }
});

test('mapping docs declare SDKWork surface and current integration mode', () => {
  for (const upstream of upstreams) {
    const mapping = fs.readFileSync(
      path.join(integrationRoot, 'specs', 'mappings', `${upstream}.md`),
      'utf8'
    );
    assert.match(mapping, /SDKWork Surface/);
    assert.match(mapping, /Initial Registration Mode/);
    assert.match(mapping, /Policy Boundaries/);
    assert.match(mapping, /Conformance/);
  }
});
