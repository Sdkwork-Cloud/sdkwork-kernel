import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const kernelRoot = process.cwd().endsWith('kernel')
  ? process.cwd()
  : path.resolve(process.cwd(), 'kernel');

const requiredSpecFiles = [
  'AGENT_KERNEL_SPEC.md',
  'CODE_KERNEL_SPEC.md',
  'AGENT_MANIFEST_SPEC.md',
  'AGENT_INSTALLATION_CONFIGURATION_SPEC.md',
  'AGENT_RUNTIME_SPEC.md',
  'AGENT_MODEL_PROVIDER_SPI_SPEC.md',
  'AGENT_MCP_PROVIDER_SPI_SPEC.md',
  'AGENT_SKILL_PROVIDER_SPI_SPEC.md',
  'AGENT_COLLABORATION_SPI_SPEC.md',
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
  'code-capability-manifest.schema.json',
  'code-runtime-diagnostics.schema.json',
  'code-conformance-report.schema.json'
];

const requiredRustCrates = [
  ['sdkwork-agent-kernel', ['src/lib.rs', 'Cargo.toml', 'README.md']],
  ['sdkwork-code-kernel', ['src/lib.rs', 'Cargo.toml', 'README.md']]
];

const requiredUiPackages = [
  'sdkwork-kernel-ui-types',
  'sdkwork-kernel-ui-core',
  'sdkwork-kernel-ui-services',
  'sdkwork-kernel-ui-commons',
  'sdkwork-kernel-ui-agent',
  'sdkwork-kernel-ui-code',
  'sdkwork-kernel-ui-workspace',
  'sdkwork-kernel-ui-terminal',
  'sdkwork-kernel-ui-telemetry',
  'sdkwork-kernel-ui-permissions'
];

const errors = [];

function ensureFile(relativePath) {
  const filePath = path.join(kernelRoot, relativePath);
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    errors.push(`missing file: ${relativePath}`);
  }
}

function readJson(relativePath) {
  const filePath = path.join(kernelRoot, relativePath);
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    errors.push(`invalid json: ${relativePath}: ${error.message}`);
    return null;
  }
}

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

const agentKernelLib = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'lib.rs'));
const agentModelRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'model.rs'));
const agentRuntimeRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'runtime.rs'));
const modelProviderSpec = readFileIfExists(path.join(kernelRoot, 'specs', 'AGENT_MODEL_PROVIDER_SPI_SPEC.md'));

for (const [label, content, requiredText] of [
  ['agent lib exports ModelDescriptor', agentKernelLib, 'ModelDescriptor'],
  ['model SPI defines ModelDescriptor', agentModelRust, 'pub struct ModelDescriptor'],
  ['model request supports model_id', agentModelRust, 'pub model_id: Option<String>'],
  ['model provider exposes list_models', agentModelRust, 'fn list_models(&self) -> Vec<ModelDescriptor>'],
  ['runtime negotiates model.catalog metadata', agentRuntimeRust, '"model.catalog"'],
  ['model provider spec documents model catalog', modelProviderSpec, 'ModelDescriptor'],
  ['model provider spec documents request model_id', modelProviderSpec, 'model_id']
]) {
  if (!content.includes(requiredText)) {
    errors.push(`${label} must include ${requiredText}`);
  }
}

const codeCargo = readJsonLikeToml(path.join(kernelRoot, 'sdkwork-code-kernel', 'Cargo.toml'));
if (codeCargo && !codeCargo.includes('sdkwork-agent-kernel')) {
  errors.push('sdkwork-code-kernel must depend on sdkwork-agent-kernel');
}

const uiRoot = path.join(kernelRoot, 'sdkwork-kernel-ui');
ensureFile(path.join('sdkwork-kernel-ui', 'package.json'));
ensureFile(path.join('sdkwork-kernel-ui', 'pnpm-workspace.yaml'));
ensureFile(path.join('sdkwork-kernel-ui', 'README.md'));

for (const packageDir of requiredUiPackages) {
  const packageRoot = path.join(uiRoot, 'packages', packageDir);
  const packageJsonPath = path.join(packageRoot, 'package.json');
  const srcDir = path.join(packageRoot, 'src');

  if (!fs.existsSync(packageJsonPath)) {
    errors.push(`missing kernel UI package manifest: ${packageDir}`);
    continue;
  }

  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const expectedName = `@sdkwork/${packageDir.replace('sdkwork-', '')}`;
  if (packageJson.name !== expectedName) {
    errors.push(`${packageDir} package name must be ${expectedName}`);
  }

  if (!fs.existsSync(path.join(srcDir, 'index.ts')) && !fs.existsSync(path.join(srcDir, 'index.tsx'))) {
    errors.push(`${packageDir} must expose src/index.ts or src/index.tsx`);
  }
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log('Kernel standards conformance check passed.');

function readJsonLikeToml(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  return fs.readFileSync(filePath, 'utf8');
}

function readFileIfExists(filePath) {
  if (!fs.existsSync(filePath)) {
    return '';
  }

  return fs.readFileSync(filePath, 'utf8');
}
