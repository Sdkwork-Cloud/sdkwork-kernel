import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

import { collectManifestValidationErrors } from './check-agent-provider-bindings.mjs';
import { migrateManifest } from './migrate-provider-binding-manifests.mjs';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, '..');

test('binding schema exposes runtime operations as a reusable top-level definition', () => {
  const schemaPath = path.join(
    root,
    'specs',
    'schemas',
    'agent-sdk-binding.schema.json'
  );
  const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));

  assert.deepEqual(schema.$defs.runtimeOperation.enum, [
    'ping',
    'session_create',
    'model_chat',
    'model_chat_stream',
    'tool_invoke',
    'skill_invoke'
  ]);
  assert.equal(
    schema.$defs.backendCandidate.properties.runtime_operations.items.$ref,
    '#/$defs/runtimeOperation'
  );
  assert.equal(
    schema.$defs.integrationSource.properties.runtimeOperation,
    undefined,
    'runtimeOperation is a reusable $defs entry, not an integration source field'
  );
});

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

test('agent-internal tools are not declared as independently invocable SDK capabilities', () => {
  for (const provider of ['codex', 'claude-code', 'opencode', 'openclaw', 'hermes']) {
    const manifestPath = path.join(
      root,
      'bindings',
      'agent-providers',
      provider,
      'provider-binding.manifest.json'
    );
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    const capabilityIds = manifest.capabilities.map((capability) => capability.capability_id);
    assert.ok(!capabilityIds.includes('sdk.tool.invoke'), `${provider} must not claim sdk.tool.invoke`);
    assert.ok(!capabilityIds.includes('sdk.skill.invoke'), `${provider} must not claim sdk.skill.invoke`);
  }
});

test('OpenCode binding must not reuse the OpenClaw gateway OpenAPI authority', () => {
  const manifestPath = path.join(
    root,
    'bindings',
    'agent-providers',
    'opencode',
    'provider-binding.manifest.json'
  );
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const serializedManifest = JSON.stringify(manifest);

  assert.doesNotMatch(
    serializedManifest,
    /openclaw-gateway-open-api/,
    'OpenCode binding must not reference the OpenClaw gateway authority'
  );
});

test('http_openapi integration sources must be backed by matching capability backends', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.opencode',
    agent_id: 'agent.intelligence.opencode',
    display_name: 'OpenCode',
    description: 'Synthetic binding used to prove unbacked OpenAPI sources are rejected.',
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
        mode: 'http_openapi',
        transport: 'opencode-open-api'
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
    /http_openapi source transport opencode-open-api must match at least one http_openapi backend openapi_authority/
  );
});

test('backend candidates must declare executable runtime operations', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'Synthetic binding used to prove runtime operation declarations are required.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    integration_sources: [
      {
        mode: 'rust_crate',
        crate: 'codex-core'
      }
    ],
    capabilities: [
      {
        capability_id: 'sdk.session.lifecycle',
        required: true,
        backends: [
          {
            kind: 'rust_native',
            driver_id: 'driver.codex.session.lifecycle.rust',
            crate: 'codex-core'
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/codex/provider-binding.manifest.json'
  ).join('\n');

  assert.match(
    errors,
    /backend driver\.codex\.session\.lifecycle\.rust must declare runtime_operations/
  );
});

test('rust native lifecycle bindings must not claim runtime session creation', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'Synthetic binding used to prove unsupported Rust runtime operations are rejected.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    integration_sources: [
      {
        mode: 'rust_crate',
        crate: 'codex-core'
      }
    ],
    capabilities: [
      {
        capability_id: 'sdk.session.lifecycle',
        required: true,
        backends: [
          {
            kind: 'rust_native',
            driver_id: 'driver.codex.session.lifecycle.rust',
            crate: 'codex-core',
            runtime_operations: ['session_create']
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/codex/provider-binding.manifest.json'
  ).join('\n');

  assert.match(
    errors,
    /rust_native backend driver\.codex\.session\.lifecycle\.rust must not declare unsupported runtime operation session_create/
  );
});

test('integration sources reject fields outside the binding schema contract', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'Synthetic binding used to prove unknown source fields are rejected.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    integration_sources: [
      {
        mode: 'official_sdk',
        package: '@openai/codex-sdk',
        url: 'https://example.invalid/not-a-contract-field'
      }
    ],
    capabilities: [
      {
        capability_id: 'sdk.model.chat',
        required: true,
        backends: [
          {
            kind: 'typescript_node',
            driver_id: 'driver.codex.model.chat.ts',
            package: '@openai/codex-sdk'
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/codex/provider-binding.manifest.json'
  );

  assert.match(
    errors.join('\n'),
    /integration_sources\[0\] field url is not allowed/
  );
});

test('integration source modes require their authoritative locator fields', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.hermes',
    agent_id: 'agent.intelligence.hermes',
    display_name: 'Hermes Agent',
    description: 'Synthetic binding used to prove source locator requirements.',
    version: '0.1.0',
    sdk_owner: 'nousresearch',
    status: 'experimental',
    integration_sources: [
      { mode: 'official_sdk' },
      { mode: 'rust_crate' },
      { mode: 'source_tree' },
      { mode: 'npm_package' },
      { mode: 'python_module' },
      { mode: 'http_openapi' },
      { mode: 'ipc_protocol' }
    ],
    capabilities: [
      {
        capability_id: 'sdk.model.chat',
        required: true,
        backends: [
          {
            kind: 'python_process',
            driver_id: 'driver.hermes.model.chat.python',
            python_module: 'run_agent'
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/hermes/provider-binding.manifest.json'
  ).join('\n');

  assert.match(errors, /integration_sources\[0\] official_sdk source must declare package/);
  assert.match(errors, /integration_sources\[1\] rust_crate source must declare crate/);
  assert.match(errors, /integration_sources\[2\] source_tree source must declare path/);
  assert.match(errors, /integration_sources\[3\] npm_package source must declare package/);
  assert.match(errors, /integration_sources\[4\] python_module source must declare module/);
  assert.match(errors, /integration_sources\[5\] http_openapi source must declare transport/);
  assert.match(errors, /integration_sources\[6\] ipc_protocol source must declare transport/);
});

test('binding manifests reject unknown fields on closed contract objects', () => {
  const manifest = {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'Synthetic binding used to prove closed object validation.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    unexpected_root: true,
    selection_policy: {
      default_backend_priority: ['typescript_node'],
      unexpected_policy: true
    },
    language_packages: {
      typescript: {
        package: '@openai/codex-sdk',
        optional: false,
        unexpected_package_field: true
      },
      java: {
        package: 'not-supported'
      }
    },
    integration_sources: [
      {
        mode: 'official_sdk',
        package: '@openai/codex-sdk'
      }
    ],
    capabilities: [
      {
        capability_id: 'sdk.model.chat',
        required: true,
        unexpected_capability: true,
        backends: [
          {
            kind: 'typescript_node',
            driver_id: 'driver.codex.model.chat.ts',
            package: '@openai/codex-sdk',
            unexpected_backend: true
          }
        ]
      }
    ]
  };

  const errors = collectManifestValidationErrors(
    manifest,
    'bindings/agent-providers/codex/provider-binding.manifest.json'
  ).join('\n');

  assert.match(errors, /field unexpected_root is not allowed/);
  assert.match(errors, /selection_policy field unexpected_policy is not allowed/);
  assert.match(errors, /language_packages field java is not allowed/);
  assert.match(errors, /language_packages\.typescript field unexpected_package_field is not allowed/);
  assert.match(errors, /capabilities\[0\] field unexpected_capability is not allowed/);
  assert.match(errors, /capabilities\[0\]\.backends\[0\] field unexpected_backend is not allowed/);
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

test('migration fills capability execution scope and backend runtime operations', () => {
  const migrated = migrateManifest('codex', {
    schema_version: '0.1.0',
    manifest_type: 'agent_sdk_binding',
    binding_id: 'binding.agent-sdk.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'External agent SDK binding for migration coverage.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    capabilities: [
      {
        capability_id: 'sdk.session.lifecycle',
        required: true,
        backends: [
          {
            kind: 'rust_native',
            driver_id: 'driver.codex.session.lifecycle.rust',
            crate: 'codex-core'
          }
        ]
      },
      {
        capability_id: 'sdk.model.chat',
        required: true,
        backends: [
          {
            kind: 'typescript_node',
            driver_id: 'driver.codex.model.chat.ts',
            package: '@openai/codex-sdk'
          }
        ]
      }
    ]
  });

  assert.equal(migrated.manifest_type, 'agent_provider_binding');
  assert.equal(migrated.binding_id, 'binding.agent-provider.codex');
  assert.equal(
    migrated.description,
    'External agent provider binding for migration coverage.'
  );

  const lifecycle = migrated.capabilities.find(
    (capability) => capability.capability_id === 'sdk.session.lifecycle'
  );
  assert.equal(lifecycle.execution_scope, 'provider_local');
  assert.deepEqual(lifecycle.backends[0].runtime_operations, ['ping']);

  const chat = migrated.capabilities.find(
    (capability) => capability.capability_id === 'sdk.model.chat'
  );
  assert.equal(chat.execution_scope, 'transport_runtime');
  assert.deepEqual(chat.backends[0].runtime_operations, [
    'ping',
    'model_chat',
    'model_chat_stream'
  ]);
});

test('migration does not synthesize unsupported rust native runtime operations', () => {
  const migrated = migrateManifest('codex', {
    schema_version: '0.1.0',
    manifest_type: 'agent_provider_binding',
    binding_id: 'binding.agent-provider.codex',
    agent_id: 'agent.intelligence.codex',
    display_name: 'Codex',
    description: 'Synthetic binding used to prove rust operation filtering.',
    version: '0.1.0',
    sdk_owner: 'openai',
    status: 'standardizing',
    capabilities: [
      {
        capability_id: 'sdk.skill.invoke',
        required: false,
        backends: [
          {
            kind: 'rust_native',
            driver_id: 'driver.codex.skill.invoke.rust',
            crate: 'codex-core'
          }
        ]
      }
    ]
  });

  assert.deepEqual(migrated.capabilities[0].backends[0].runtime_operations, ['ping']);
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
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md',
    'docs/architecture/tech/TECH-03-spi-implementation-gap-tracker.md'
  ]) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.doesNotMatch(
      content,
      /Mimo Code[^\n]*(?:binding manifest pending|binding manifest missing|TBD|pending binding)/i,
      `${relativePath} should reflect the current Mimo binding manifest`
    );
  }
});

test('provider documentation uses canonical MiMo Code display name', () => {
  const canonicalNamePaths = [
    'bindings/agent-providers/README.md',
    'docs/product/prd/PRD-02-provider-integration-requirements.md',
    'docs/product/prd/PRD-03-commercial-readiness-baseline.md',
    'docs/product/prd/PRD-04-ecosystem-architecture.md',
    'docs/product/requirements/REQ-2026-0001-commercial-hardening.md',
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md',
    'docs/architecture/tech/TECH-03-spi-implementation-gap-tracker.md',
    'specs/kernel-local-conventions.md'
  ];

  for (const relativePath of canonicalNamePaths) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.doesNotMatch(
      content,
      /\bMimo Code\b/,
      `${relativePath} should use canonical display name MiMo Code`
    );
  }
});

test('provider documentation distinguishes credential-free probes from staging live SDK gates', () => {
  for (const relativePath of [
    'docs/product/prd/PRD-02-provider-integration-requirements.md',
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md',
    'docs/architecture/tech/TECH_ARCHITECTURE.md',
    'docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md'
  ]) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.doesNotMatch(
      content,
      /live (?:SDK )?proof[^\n]*engine-sdk-live\.test\.mjs/i,
      `${relativePath} should not describe engine-sdk-live.test.mjs as the staging live proof`
    );
    assert.match(
      content,
      /engine-sdk-live-staging\.mjs/,
      `${relativePath} should reference the staging live SDK gate`
    );
  }
});

test('OpenCode documentation distinguishes source mirror from runtime SDK package resolution', () => {
  const content = fs.readFileSync(
    path.join(root, 'sdkwork-kernel-plugins', 'specs', 'mappings', 'opencode.md'),
    'utf8'
  );

  assert.match(
    content,
    /external\/opencode[^\n]*source reference/i,
    'OpenCode mapping should state that external/opencode is a source reference'
  );
  assert.match(
    content,
    /SDKWORK_AGENT_SDK_PACKAGE_PATHS/,
    'OpenCode mapping should document explicit SDK package path injection'
  );
});

test('provider binding checker validates source-tree mapping metadata boundaries', async () => {
  const checker = await import('./check-agent-provider-bindings.mjs');

  assert.equal(
    typeof checker.collectSourceTreeDocumentationErrors,
    'function',
    'binding checker should expose source-tree documentation validation'
  );

  assert.deepEqual(
    checker.collectSourceTreeDocumentationErrors({ workspaceRoot: root }),
    []
  );
});

test('source-tree mapping validation rejects undocumented SDK package subpaths', async () => {
  const checker = await import('./check-agent-provider-bindings.mjs');
  assert.equal(
    typeof checker.collectSourceTreeDocumentationErrors,
    'function',
    'binding checker should expose source-tree documentation validation'
  );

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-binding-docs-'));
  try {
    fs.mkdirSync(
      path.join(tempRoot, 'bindings', 'agent-providers', 'demo-agent'),
      { recursive: true }
    );
    fs.mkdirSync(
      path.join(tempRoot, 'external', 'demo-agent', 'packages', 'sdk'),
      { recursive: true }
    );
    fs.mkdirSync(
      path.join(tempRoot, 'sdkwork-kernel-plugins', 'specs', 'mappings'),
      { recursive: true }
    );

    fs.writeFileSync(
      path.join(tempRoot, 'bindings', 'agent-providers', 'demo-agent', 'provider-binding.manifest.json'),
      JSON.stringify({
        manifest_type: 'agent_provider_binding',
        language_packages: {
          typescript: {
            package: '@demo/agent-sdk'
          }
        },
        integration_sources: [
          {
            mode: 'source_tree',
            path: 'external/demo-agent'
          }
        ],
        capabilities: []
      })
    );
    fs.writeFileSync(
      path.join(tempRoot, 'external', 'demo-agent', 'packages', 'sdk', 'package.json'),
      JSON.stringify({
        name: '@demo/agent-sdk'
      })
    );
    fs.writeFileSync(
      path.join(tempRoot, 'sdkwork-kernel-plugins', 'specs', 'mappings', 'demo-agent.md'),
      '# Demo Agent Mapping\n\n- Local path: `external/demo-agent`\n'
    );

    assert.match(
      checker.collectSourceTreeDocumentationErrors({ workspaceRoot: tempRoot }).join('\n'),
      /external\/demo-agent\/packages\/sdk/
    );
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test('Hermes documentation identifies the Python worker fail-closed contract', () => {
  for (const relativePath of [
    'docs/product/prd/PRD-02-provider-integration-requirements.md',
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md',
    'docs/architecture/tech/TECH_ARCHITECTURE.md',
    'docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md',
    'sdkwork-kernel-plugins/specs/mappings/hermes-agent.md'
  ]) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.match(
      content,
      /generic-python-sdk-worker\.test\.mjs/,
      `${relativePath} should reference the Hermes Python worker fail-closed contract`
    );
  }
});

test('target provider mappings record runtime proof boundaries', () => {
  const nodeStagingProviders = [
    'codex',
    'claude-code',
    'opencode',
    'openclaw'
  ];

  for (const provider of nodeStagingProviders) {
    const relativePath = `sdkwork-kernel-plugins/specs/mappings/${provider}.md`;
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.match(
      content,
      /engine-sdk-live\.test\.mjs/,
      `${relativePath} should name the credential-free resolver and fail-closed merge contract`
    );
    assert.match(
      content,
      /engine-sdk-live-staging\.mjs/,
      `${relativePath} should name the staging live SDK gate for release proof`
    );
  }

  const hermesPath = 'sdkwork-kernel-plugins/specs/mappings/hermes-agent.md';
  const hermesContent = fs.readFileSync(path.join(root, hermesPath), 'utf8');
  assert.match(
    hermesContent,
    /generic-python-sdk-worker\.test\.mjs/,
    `${hermesPath} should name the Python worker fail-closed merge contract`
  );
  assert.match(
    hermesContent,
    /Hermes-specific staging gateway proof/i,
    `${hermesPath} should state the separate Hermes staging proof required for release`
  );
});

test('provider mappings document operation allowlists and provider-local lifecycle scope', () => {
  const mappingDocs = [
    'codex',
    'claude-code',
    'gemini-cli',
    'opencode',
    'openclaw',
    'hermes-agent'
  ];

  for (const mapping of mappingDocs) {
    const relativePath = `sdkwork-kernel-plugins/specs/mappings/${mapping}.md`;
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.doesNotMatch(
      content,
      /fail closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`/i,
      `${relativePath} should not retain the pre-operation-allowlist safety statement`
    );
    assert.match(
      content,
      /execution_scope/i,
      `${relativePath} should document the binding execution scope boundary`
    );
    assert.match(
      content,
      /runtime_operations/i,
      `${relativePath} should document the selected backend operation allowlist`
    );
    assert.match(
      content,
      /provider-local lifecycle|provider_local lifecycle/i,
      `${relativePath} should document that session lifecycle is provider-local`
    );
  }
});

test('OpenClaw mapping separates gateway staging proof from local SDK package health', () => {
  const relativePath = 'sdkwork-kernel-plugins/specs/mappings/openclaw.md';
  const content = fs.readFileSync(path.join(root, relativePath), 'utf8');

  assert.match(
    content,
    /OPENCLAW_GATEWAY_URL/i,
    `${relativePath} should document the OpenClaw gateway URL requirement`
  );
  assert.match(
    content,
    /does not satisfy local Node runtime SDK package health/i,
    `${relativePath} should state gateway proof does not prove local SDK importability`
  );
});

test('provider integration spec records operation-level runtime dispatch semantics', () => {
  const content = fs.readFileSync(
    path.join(root, 'specs', 'AGENT_PROVIDER_INTEGRATION_SPEC.md'),
    'utf8'
  );

  assert.match(content, /runtime_operations/i);
  assert.match(content, /operation_not_supported/i);
  assert.match(content, /Ping[^\n]*not proof|ping[^\n]*not proof/i);
});

test('PRD and architecture docs record operation allowlist runtime boundaries', () => {
  for (const relativePath of [
    'docs/product/prd/PRD-02-provider-integration-requirements.md',
    'docs/product/prd/PRD-03-commercial-readiness-baseline.md',
    'docs/architecture/tech/TECH-02-provider-framework-matrix.md',
    'docs/architecture/tech/TECH_ARCHITECTURE.md',
    'docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md'
  ]) {
    const content = fs.readFileSync(path.join(root, relativePath), 'utf8');
    assert.match(
      content,
      /runtime_operations/i,
      `${relativePath} should document runtime operation allowlists`
    );
    assert.match(
      content,
      /execution_scope/i,
      `${relativePath} should document provider capability execution scope`
    );
    assert.match(
      content,
      /provider-local lifecycle|provider_local lifecycle/i,
      `${relativePath} should document provider-local lifecycle handling`
    );
  }
});
