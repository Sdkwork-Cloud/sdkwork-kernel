import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');
const pluginRoot = path.join(root, 'sdkwork-kernel-plugins');

const upstreams = [
  'hermes-agent',
  'openclaw',
  'codex',
  'claude-code',
  'opencode',
  'gemini-cli',
  'rig',
  'mimo-code'
];

const adapterCrates = [
  'sdkwork-agent-adapter-core',
  'sdkwork-agent-adapter-hermes',
  'sdkwork-agent-adapter-openclaw',
  'sdkwork-agent-adapter-codex',
  'sdkwork-agent-adapter-claude-code',
  'sdkwork-agent-adapter-opencode',
  'sdkwork-agent-adapter-gemini-cli',
  'sdkwork-agent-adapter-mimo-code'
];

const requiredFiles = [
  'README.md',
  'specs/README.md',
  'specs/component.spec.json',
  'specs/EXTERNAL_AGENT_PLUGIN_SPEC.md',
  'specs/conformance/manifest-profile.md',
  'specs/conformance/local-runtime-profile.md',
  'specs/conformance/process-adapter-profile.md',
  'specs/manifests/agents/external-code-agent-runtime.agent.json',
  'specs/manifests/agents/external-general-agent-runtime.agent.json',
  'specs/manifests/providers/codex-process.provider.json',
  'specs/manifests/providers/rig-rust.provider.json',
  'specs/manifests/protocol-adapters/agent-chat-rpc.protocol-adapter.json',
  'specs/manifests/protocol-adapters/external-process.protocol-adapter.json',
  'crates/sdkwork-agent-plugin-core/Cargo.toml',
  'crates/sdkwork-agent-plugin-core/README.md',
  'crates/sdkwork-agent-plugin-core/specs/component.spec.json',
  'crates/sdkwork-agent-plugin-core/src/lib.rs',
  'crates/sdkwork-agent-plugin-rig/Cargo.toml',
  'crates/sdkwork-agent-plugin-rig/README.md',
  'crates/sdkwork-agent-plugin-rig/specs/component.spec.json',
  'crates/sdkwork-agent-plugin-rig/src/lib.rs',
  'crates/sdkwork-kernel-plugin-drive/Cargo.toml',
  'crates/sdkwork-kernel-plugin-drive/README.md',
  'crates/sdkwork-kernel-plugin-drive/specs/component.spec.json',
  'crates/sdkwork-kernel-plugin-drive/src/lib.rs',
  'crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml',
  'crates/sdkwork-kernel-plugin-knowledgebase/README.md',
  'crates/sdkwork-kernel-plugin-knowledgebase/specs/component.spec.json',
  'crates/sdkwork-kernel-plugin-knowledgebase/src/lib.rs',
  'scripts/check-kernel-plugins.mjs'
];

for (const adapterCrate of adapterCrates) {
  requiredFiles.push(`crates/${adapterCrate}/Cargo.toml`);
  requiredFiles.push(`crates/${adapterCrate}/README.md`);
  requiredFiles.push(`crates/${adapterCrate}/specs/component.spec.json`);
  requiredFiles.push(`crates/${adapterCrate}/src/lib.rs`);
}

for (const upstream of upstreams) {
  requiredFiles.push(`specs/mappings/${upstream}.md`);
}

test('kernel plugin standards assets are present', () => {
  for (const relativePath of requiredFiles) {
    const absolutePath = path.join(pluginRoot, relativePath);
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
    ['specs/manifests/protocol-adapters/agent-chat-rpc.protocol-adapter.json', 'provider'],
    ['specs/manifests/protocol-adapters/external-process.protocol-adapter.json', 'provider']
  ]);

  for (const [relativePath, expectedKind] of jsonExpectations) {
    const content = fs.readFileSync(path.join(pluginRoot, relativePath), 'utf8');
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
    'specs/manifests/protocol-adapters/agent-chat-rpc.protocol-adapter.json',
    'specs/manifests/protocol-adapters/external-process.protocol-adapter.json'
  ];
  const extensionKeyPattern = /^(sdkwork|[a-z0-9-]+\.[a-z0-9.-]+)\.[a-z0-9.-]+$/;
  const capabilityPattern = /^[a-z0-9_.-]+$/;

  for (const relativePath of manifestPaths) {
    const manifest = JSON.parse(
      fs.readFileSync(path.join(pluginRoot, relativePath), 'utf8')
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

test('rig static manifests mirror typed provider-family ownership', () => {
  const rigModelProvider = JSON.parse(
    fs.readFileSync(
      path.join(pluginRoot, 'specs/manifests/providers/rig-rust.provider.json'),
      'utf8'
    )
  );
  const chatRpcAdapter = JSON.parse(
    fs.readFileSync(
      path.join(pluginRoot, 'specs/manifests/protocol-adapters/agent-chat-rpc.protocol-adapter.json'),
      'utf8'
    )
  );

  assert.equal(rigModelProvider.provider_family, 'model');
  assert.deepEqual(
    rigModelProvider.capabilities,
    ['model.catalog', 'model.chat'],
    'Rig model provider manifest should only claim implemented model capabilities'
  );
  assert.equal(chatRpcAdapter.provider_family, 'protocol_adapter');
  assert.equal(chatRpcAdapter.adapter_id, 'adapter.rpc.agent-chat');
  assert.deepEqual(chatRpcAdapter.capabilities, ['protocol.map', 'protocol.stream']);
  assert.ok(chatRpcAdapter.exposed_capabilities.includes('knowledge.search'));
  assert.ok(chatRpcAdapter.kernel_object_mappings.includes('KnowledgeSearchRequest'));
  assert.ok(chatRpcAdapter.security_requirements.required_policy_categories.includes('knowledge.search'));
});

test('mapping docs declare SDKWork surface and current plugin mode', () => {
  for (const upstream of upstreams) {
    const mapping = fs.readFileSync(
      path.join(pluginRoot, 'specs', 'mappings', `${upstream}.md`),
      'utf8'
    );
    assert.match(mapping, /SDKWork Surface/);
    assert.match(mapping, /Initial Registration Mode/);
    assert.match(mapping, /Policy Boundaries/);
    assert.match(mapping, /Conformance/);
  }
});

test('plugin crates do not require external reference sources for default Cargo metadata', () => {
  const crateManifests = [
    'crates/sdkwork-agent-plugin-core/Cargo.toml',
    'crates/sdkwork-agent-plugin-rig/Cargo.toml',
    'crates/sdkwork-kernel-plugin-drive/Cargo.toml',
    'crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml',
    ...adapterCrates.map((crateName) => `crates/${crateName}/Cargo.toml`)
  ];

  for (const relativePath of crateManifests) {
    const content = fs.readFileSync(path.join(pluginRoot, relativePath), 'utf8');

    assert.doesNotMatch(
      content,
      /path\s*=\s*["'][^"']*external\//,
      `${relativePath} must not make external reference source a Cargo path dependency`
    );
  }
});

test('plugin packages do not retain stale extension naming', () => {
  const oldTerm = 'inte' + 'gration';
  const oldTermPlural = oldTerm + 's';
  const oldTypeTerm = 'Inte' + 'gration';
  const stalePackageRoot = 'sdkwork-agent-' + oldTermPlural;
  const stalePackagePrefix = 'sdkwork-agent-' + oldTerm + '-';
  const staleCratePrefix = 'sdkwork_agent_' + oldTerm + '_';
  const staleTraitName = 'SdkworkAgent' + oldTypeTerm + 'Plugin';
  const staleRigPluginName = 'Rig' + oldTypeTerm + 'Plugin';
  const staleManifestName = oldTypeTerm + 'PluginManifest';
  const staleBindingName = oldTypeTerm + 'ProviderBinding';
  const staleProfileName = oldTypeTerm + 'ConformanceProfile';
  const staleIdsName = 'Standard' + oldTypeTerm + 'Ids';
  const staleFacadePhrase = 'compatibility ' + 'facades';
  const staleAliasPhrase = 'compatibility ' + 'alias';
  const stalePatterns = [
    new RegExp(stalePackageRoot),
    new RegExp(stalePackagePrefix),
    new RegExp(staleCratePrefix),
    new RegExp(staleTraitName),
    new RegExp(staleRigPluginName),
    new RegExp(staleManifestName),
    new RegExp(staleBindingName),
    new RegExp(staleProfileName),
    new RegExp(staleIdsName),
    new RegExp(staleFacadePhrase),
    new RegExp(staleAliasPhrase)
  ];
  const scanRoots = [
    path.join(root, 'Cargo.toml'),
    pluginRoot,
    path.join(root, 'specs', 'KERNEL_PLUGIN_SPEC.md')
  ];

  for (const filePath of listFiles(scanRoots)) {
    const content = fs.readFileSync(filePath, 'utf8');
    for (const pattern of stalePatterns) {
      assert.doesNotMatch(content, pattern, `${filePath} must not contain stale ${pattern}`);
    }
  }
});

function listFiles(pathsToScan) {
  const files = [];
  for (const scanPath of pathsToScan) {
    if (!fs.existsSync(scanPath)) {
      continue;
    }
    const stat = fs.statSync(scanPath);
    if (stat.isFile()) {
      files.push(scanPath);
      continue;
    }
    for (const entry of fs.readdirSync(scanPath)) {
      if (entry === 'target' || entry === '.git') {
        continue;
      }
      files.push(...listFiles([path.join(scanPath, entry)]));
    }
  }
  return files;
}
