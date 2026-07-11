import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');
const pluginRoot = path.join(root, 'sdkwork-kernel-plugins');
const providerRoot = path.join(root, 'agent-providers');

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

const pluginCoreCrates = ['sdkwork-agent-plugin-core', 'sdkwork-agent-provider-core'];

const providerFrameworkCrates = [
  'sdkwork-agent-provider-hermes',
  'sdkwork-agent-provider-openclaw',
  'sdkwork-agent-provider-codex',
  'sdkwork-agent-provider-claude-code',
  'sdkwork-agent-provider-opencode',
  'sdkwork-agent-provider-gemini-cli',
  'sdkwork-agent-provider-rig',
  'sdkwork-agent-provider-mimo-code'
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
  'specs/manifests/providers/codex-model.provider.json',
  'specs/manifests/providers/rig-rust.provider.json',
  'specs/manifests/protocol-adapters/agent-chat-rpc.protocol-adapter.json',
  'specs/manifests/protocol-adapters/external-process.protocol-adapter.json',
  'crates/sdkwork-kernel-plugin-drive/Cargo.toml',
  'crates/sdkwork-kernel-plugin-drive/README.md',
  'crates/sdkwork-kernel-plugin-drive/specs/component.spec.json',
  'crates/sdkwork-kernel-plugin-drive/src/lib.rs',
  'crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml',
  'crates/sdkwork-kernel-plugin-knowledgebase/README.md',
  'crates/sdkwork-kernel-plugin-knowledgebase/specs/component.spec.json',
  'crates/sdkwork-kernel-plugin-knowledgebase/src/lib.rs',
  'scripts/check-kernel-plugins.mjs',
  'specs/mappings/zeroclaw.md'
];

for (const crateName of pluginCoreCrates) {
  requiredFiles.push(`crates/${crateName}/Cargo.toml`);
  requiredFiles.push(`crates/${crateName}/README.md`);
  requiredFiles.push(`crates/${crateName}/specs/component.spec.json`);
  requiredFiles.push(`crates/${crateName}/src/lib.rs`);
}

for (const crateName of providerFrameworkCrates) {
  requiredFiles.push(`crates/${crateName}/Cargo.toml`);
  requiredFiles.push(`crates/${crateName}/README.md`);
  requiredFiles.push(`crates/${crateName}/specs/component.spec.json`);
  requiredFiles.push(`crates/${crateName}/src/lib.rs`);
}

for (const upstream of upstreams) {
  requiredFiles.push(`specs/mappings/${upstream}.md`);
}

test('kernel plugin standards assets are present', () => {
  for (const relativePath of requiredFiles) {
    const baseRoot = providerFrameworkCrates.some((crateName) =>
      relativePath.startsWith(`crates/${crateName}/`)
    )
      ? providerRoot
      : pluginRoot;
    const absolutePath = path.join(baseRoot, relativePath);
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

test('provider framework matrix documents every shipped provider crate boundary', () => {
  const matrix = fs.readFileSync(
    path.join(root, 'docs', 'architecture', 'tech', 'TECH-02-provider-framework-matrix.md'),
    'utf8'
  );

  for (const crateName of providerFrameworkCrates) {
    assert.match(
      matrix,
      new RegExp(`\\| \`${escapeRegExp(crateName)}\``),
      `${crateName} should be documented in the provider framework matrix`
    );
  }

  const runtimePluginBoundaries = new Map([
    ['sdkwork-agent-provider-codex', 'CodexKernelPlugin::configure_runtime'],
    ['sdkwork-agent-provider-claude-code', 'ClaudeCodeKernelPlugin::configure_runtime'],
    ['sdkwork-agent-provider-opencode', 'OpenCodeKernelPlugin::configure_runtime'],
    ['sdkwork-agent-provider-openclaw', 'OpenClawKernelPlugin::configure_runtime'],
    ['sdkwork-agent-provider-hermes', 'HermesKernelPlugin::configure_runtime'],
    ['sdkwork-agent-provider-rig', 'RigKernelPlugin::configure_runtime']
  ]);

  for (const [crateName, boundary] of runtimePluginBoundaries) {
    assert.match(
      matrix,
      new RegExp(`\\| \`${escapeRegExp(crateName)}\` .* \`${escapeRegExp(boundary)}\``),
      `${crateName} should document its kernel plugin runtime entrypoint`
    );
  }

  assert.match(matrix, /GeminiCliSdkIntegration::bootstrap/);
  assert.match(matrix, /MiMoCodeAdapter/);
  assert.match(matrix, /staging live SDK proof remain required before product GA/);
  assert.match(
    matrix,
    /\| Upstream strength \| Codex \| Claude Code \| Gemini CLI \| OpenCode \| MiMo Code \| OpenClaw \| Hermes \| Rig \| Kernel SPI owner \|/,
    'industry feature mapping should cover every shipped provider framework column'
  );
  assert.match(
    matrix,
    /\| Capability id \| Codex \| Claude Code \| Gemini CLI \| OpenCode \| MiMo Code \| OpenClaw \| Hermes \| Rig \|/,
    'binding capability coverage should cover every shipped provider framework column'
  );

  for (const crateName of providerFrameworkCrates) {
    assert.match(
      matrix,
      new RegExp(`cargo test --manifest-path agent-providers/crates/${escapeRegExp(crateName)}/Cargo\\.toml`),
      `${crateName} should have a documented cargo verification command`
    );
  }
});

test('canon indexes describe every shipped provider framework in the matrix summary', () => {
  const expectedSummary = 'Codex, Claude Code, Gemini CLI, OpenCode, MiMo Code, OpenClaw, Hermes, Rig';
  const canonPaths = [
    'docs/product/prd/PRD.md',
    'docs/architecture/tech/TECH_ARCHITECTURE.md'
  ];

  for (const relativePath of canonPaths) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.match(
      content,
      new RegExp(escapeRegExp(expectedSummary)),
      `${relativePath} should summarize all shipped provider frameworks`
    );
  }
});

test('deferred mapping docs declare SDKWork surface and policy boundaries', () => {
  const deferredMappings = ['zeroclaw'];
  for (const upstream of deferredMappings) {
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
    ...pluginCoreCrates.map((crateName) => ({
      root: pluginRoot,
      relativePath: `crates/${crateName}/Cargo.toml`
    })),
    ...providerFrameworkCrates.map((crateName) => ({
      root: providerRoot,
      relativePath: `crates/${crateName}/Cargo.toml`
    })),
    { root: pluginRoot, relativePath: 'crates/sdkwork-kernel-plugin-drive/Cargo.toml' },
    { root: pluginRoot, relativePath: 'crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml' }
  ];

  for (const { root: crateRoot, relativePath } of crateManifests) {
    const content = fs.readFileSync(path.join(crateRoot, relativePath), 'utf8');

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

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
