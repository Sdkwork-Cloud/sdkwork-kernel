import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const requiredSpecFiles = [
  'SDK_SPEC.md',
  'KERNEL_PLUGIN_SPEC.md',
  'AGENT_KERNEL_SPEC.md',
  'AGENT_PROVIDER_INTEGRATION_SPEC.md',
  'AGENT_SDK_SPI_SPEC.md',
  'AGENT_SDK_BINDING_SPEC.md',
  'CODE_KERNEL_SPEC.md',
  'AGENT_MANIFEST_SPEC.md',
  'AGENT_INSTALLATION_CONFIGURATION_SPEC.md',
  'AGENT_RUNTIME_SPEC.md',
  'AGENT_MODEL_PROVIDER_SPI_SPEC.md',
  'AGENT_MCP_PROVIDER_SPI_SPEC.md',
  'AGENT_SKILL_PROVIDER_SPI_SPEC.md',
  'AGENT_COLLABORATION_SPI_SPEC.md',
  'AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md',
  'AGENT_TOOL_PROVIDER_SPI_SPEC.md',
  'AGENT_CONTEXT_MEMORY_SPEC.md',
  'AGENT_PLANNING_EXECUTION_SPEC.md',
  'AGENT_HOST_PROVIDER_SPI_SPEC.md',
  'AGENT_PROTOCOL_ADAPTER_SPEC.md',
  'AGENT_SECURITY_POLICY_SPEC.md',
  'AGENT_EVENT_TELEMETRY_SPEC.md',
  'AGENT_UI_CONTRACT_SPEC.md',
  'AGENT_CONFORMANCE_SPEC.md'
];

const requiredSchemas = [
  'kernel-plugin-manifest.schema.json',
  'agent-definition.schema.json',
  'agent-manifest.schema.json',
  'agent-package-manifest.schema.json',
  'agent-configuration-spec.schema.json',
  'agent-configuration-profile.schema.json',
  'agent-configuration-migration.schema.json',
  'agent-card.schema.json',
  'provider-manifest.schema.json',
  'capability-manifest.schema.json',
  'agent-runtime-diagnostics.schema.json',
  'kernel-conformance-report.schema.json',
  'agent-sdk-binding.schema.json',
  'code-capability-manifest.schema.json',
  'code-runtime-diagnostics.schema.json',
  'code-conformance-report.schema.json'
];

const requiredRustCrates = [
  ['sdkwork-agent-kernel', ['src/lib.rs', 'Cargo.toml', 'README.md']],
  ['sdkwork-agent-provider-spi', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-agent-provider-transport-core', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-agent-provider-transport-ipc', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-agent-provider-transport-rust', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-agent-provider-transport-node', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-agent-provider-transport-python', ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']],
  ['sdkwork-code-kernel', ['src/lib.rs', 'Cargo.toml', 'README.md']]
];

const requiredWorkspaceRustCrates = [
  'sdkwork-agent-api-bridge',
  'sdkwork-agent-client',
  'sdkwork-agent-database',
  'sdkwork-agent-server',
  'sdkwork-agent-session',
  'sdkwork-agent-streaming'
];

const requiredRouteRustCrates = [
  'crates/sdkwork-routes-agent-internal-manifest',
  'crates/sdkwork-routes-agent-internal-api'
];

const requiredKernelPluginFiles = [
  'README.md',
  'specs/component.spec.json',
  'scripts/check-kernel-plugins.mjs',
  'crates/sdkwork-agent-plugin-core/Cargo.toml',
  'crates/sdkwork-agent-provider-core/Cargo.toml',
  'crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml'
];

const requiredAgentProviderFiles = [
  'crates/sdkwork-agent-provider-rig/Cargo.toml'
];

const expectedKernelComponentIdentity = [
  ['sdkwork-agent-kernel', 'intelligence', 'agent-kernel'],
  ['sdkwork-code-kernel', 'intelligence', 'code-kernel']
];

export function validateKernelContracts({ kernelRoot, errors, ensureFile, readJson, readFileIfExists }) {
  for (const specFile of requiredSpecFiles) {
    ensureFile(path.join('specs', specFile));
  }

  for (const schemaFile of requiredSchemas) {
    const schema = readJson(path.join('specs', 'schemas', schemaFile));
    if (schema && schema.$schema !== 'https://json-schema.org/draft/2020-12/schema') {
      errors.push(`${schemaFile} must use JSON Schema draft 2020-12`);
    }
  }

  for (const [crateDir, files] of requiredRustCrates) {
    for (const file of files) {
      ensureFile(path.join(crateDir, file));
    }
  }

  for (const crateDir of requiredWorkspaceRustCrates) {
    for (const file of ['src/lib.rs', 'Cargo.toml', 'README.md', 'AGENTS.md', 'specs/component.spec.json']) {
      ensureFile(path.join(crateDir, file));
    }
  }

  for (const crateDir of requiredRouteRustCrates) {
    for (const file of ['src/lib.rs', 'Cargo.toml', 'AGENTS.md', 'specs/component.spec.json']) {
      ensureFile(path.join(crateDir, file));
    }
  }

  for (const pluginFile of requiredKernelPluginFiles) {
    ensureFile(path.join('sdkwork-kernel-plugins', pluginFile));
  }

  for (const providerFile of requiredAgentProviderFiles) {
    ensureFile(path.join('agent-providers', providerFile));
  }

  const kernelPluginCheck = spawnSync(
    process.execPath,
    [path.join(kernelRoot, 'sdkwork-kernel-plugins', 'scripts', 'check-kernel-plugins.mjs')],
    {
      cwd: kernelRoot,
      encoding: 'utf8'
    }
  );
  if (kernelPluginCheck.status !== 0) {
    errors.push(
      `kernel plugin structure check failed:\n${kernelPluginCheck.stdout}${kernelPluginCheck.stderr}`
    );
  }

  for (const [crateDir, expectedDomain, expectedCapability] of expectedKernelComponentIdentity) {
    const componentSpec = readJson(path.join(crateDir, 'specs', 'component.spec.json'));

    if (componentSpec) {
      const component = componentSpec.component ?? {};
      if (component.domain !== expectedDomain) {
        errors.push(`${crateDir} component domain must be ${expectedDomain}`);
      }
      if (component.capability !== expectedCapability) {
        errors.push(`${crateDir} component capability must be ${expectedCapability}`);
      }
    }
  }

  const codeCargo = readFileIfExists(path.join(kernelRoot, 'sdkwork-code-kernel', 'Cargo.toml'));
  if (codeCargo && !codeCargo.includes('sdkwork-agent-kernel')) {
    errors.push('sdkwork-code-kernel must depend on sdkwork-agent-kernel');
  }
}
