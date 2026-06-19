import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const catalogRoot = path.join(root, 'sdks', 'external-agent-sdks');
const schemaPath = path.join(root, 'specs', 'schemas', 'agent-sdk-binding.schema.json');

const errors = [];

function ensureDirectory(relativePath) {
  const absolutePath = path.join(root, relativePath);
  if (!fs.existsSync(absolutePath) || !fs.statSync(absolutePath).isDirectory()) {
    errors.push(`missing directory: ${relativePath}`);
  }
}

function readJson(relativePath) {
  const absolutePath = path.join(root, relativePath);
  try {
    return JSON.parse(fs.readFileSync(absolutePath, 'utf8'));
  } catch (error) {
    errors.push(`invalid json: ${relativePath}: ${error.message}`);
    return null;
  }
}

function listBindingManifests(directory) {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(directory, entry.name, 'sdk-binding.manifest.json'))
    .filter((manifestPath) => fs.existsSync(manifestPath));
}

function validateManifestShape(manifestPath, manifest) {
  const relativePath = path.relative(root, manifestPath).replaceAll('\\', '/');
  const required = [
    'schema_version',
    'manifest_type',
    'binding_id',
    'agent_id',
    'display_name',
    'description',
    'version',
    'sdk_owner',
    'capabilities',
    'status'
  ];

  for (const field of required) {
    if (!(field in manifest)) {
      errors.push(`${relativePath} missing required field: ${field}`);
    }
  }

  if (manifest.manifest_type !== 'agent_sdk_binding') {
    errors.push(`${relativePath} manifest_type must be agent_sdk_binding`);
  }

  if (!Array.isArray(manifest.capabilities) || manifest.capabilities.length === 0) {
    errors.push(`${relativePath} must declare at least one capability`);
  }

  for (const capability of manifest.capabilities ?? []) {
    if (!capability.capability_id?.startsWith('sdk.')) {
      errors.push(`${relativePath} capability_id must use sdk.* namespace`);
    }
    if (!Array.isArray(capability.backends) || capability.backends.length === 0) {
      errors.push(`${relativePath} capability ${capability.capability_id} must declare backends`);
    }
    for (const backend of capability.backends ?? []) {
      if (!backend.driver_id?.startsWith('driver.')) {
        errors.push(`${relativePath} backend driver_id must use driver.* namespace`);
      }
    }
  }
}

ensureDirectory('sdks/external-agent-sdks');
if (!fs.existsSync(schemaPath)) {
  errors.push('missing schema: specs/schemas/agent-sdk-binding.schema.json');
}

const manifests = listBindingManifests(catalogRoot);
if (manifests.length === 0) {
  errors.push('external agent sdk catalog must contain at least one binding manifest');
}

for (const manifestPath of manifests) {
  const manifest = readJson(path.relative(root, manifestPath));
  if (manifest) {
    validateManifestShape(manifestPath, manifest);
  }
}

const rustParseCheck = spawnSync(
  'cargo',
  ['test', '--manifest-path', 'sdkwork-agent-sdk-spi/Cargo.toml', '-q'],
  {
    cwd: root,
    encoding: 'utf8',
    shell: process.platform === 'win32'
  }
);

if (rustParseCheck.status !== 0) {
  errors.push(
    `sdk-spi binding parse tests failed:\n${rustParseCheck.stdout}${rustParseCheck.stderr}`
  );
}

const backendCrates = [
  'sdkwork-agent-sdk-backend-ipc',
  'sdkwork-agent-sdk-backend-rust',
  'sdkwork-agent-sdk-backend-node',
  'sdkwork-agent-sdk-backend-python'
];

for (const crateDir of backendCrates) {
  const backendCheck = spawnSync(
    'cargo',
    ['test', '--manifest-path', `${crateDir}/Cargo.toml`, '-q'],
    {
      cwd: root,
      encoding: 'utf8',
      shell: process.platform === 'win32'
    }
  );

  if (backendCheck.status !== 0) {
    errors.push(
      `${crateDir} tests failed:\n${backendCheck.stdout}${backendCheck.stderr}`
    );
  }
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log(`External agent SDK binding check passed (${manifests.length} manifests).`);
