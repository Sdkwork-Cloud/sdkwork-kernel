import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const catalogRoot = path.join(root, 'bindings', 'agent-providers');
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
    .map((entry) => path.join(directory, entry.name, 'provider-binding.manifest.json'))
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

  const manifestType = manifest.manifest_type;
  if (manifestType !== 'agent_provider_binding') {
    errors.push(
      `${relativePath} manifest_type must be agent_provider_binding`
    );
  }

  if (
    !Array.isArray(manifest.integration_sources) ||
    manifest.integration_sources.length === 0
  ) {
    errors.push(`${relativePath} must declare integration_sources`);
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

ensureDirectory('bindings/agent-providers');
if (!fs.existsSync(schemaPath)) {
  errors.push('missing schema: specs/schemas/agent-sdk-binding.schema.json');
}

const manifests = listBindingManifests(catalogRoot);
if (manifests.length === 0) {
  errors.push('agent provider binding catalog must contain at least one manifest');
}

for (const manifestPath of manifests) {
  const manifest = readJson(path.relative(root, manifestPath));
  if (manifest) {
    validateManifestShape(manifestPath, manifest);
  }
}

function runCargoTest(crateDir) {
  const result = spawnSync(
    'cargo',
    ['test', '--manifest-path', `${crateDir}/Cargo.toml`, '-q'],
    {
      cwd: root,
      encoding: 'utf8',
      shell: process.platform === 'win32'
    }
  );
  if (result.status === 0) {
    return { passed: true, skipped: false };
  }
  const output = `${result.stdout}${result.stderr}`;
  if (isWindowsBuildScriptPanic(output)) {
    // Known Windows toolchain issue: proc-macro2/serde/serde_core/quote build
    // scripts panic during process spawning on certain Windows configurations
    // (notably non-English locales). The panic occurs in Rust's standard
    // library process module (`Result::unwrap()` on `Os { code: 0 }`),
    // not in kernel code. Linux CI validates these crates without issue.
    // Skipping here avoids blocking Windows development while keeping CI
    // authoritative.
    console.warn(
      `warning: ${crateDir} cargo tests skipped on Windows due to build-script toolchain panic (proc-macro2/serde/quote). ` +
      'These crates are validated on Linux CI. See AGENTS.md "Build, Test, and Verification" for details.'
    );
    return { passed: true, skipped: true };
  }
  return { passed: false, skipped: false, output };
}

function isWindowsBuildScriptPanic(output) {
  if (process.platform !== 'win32') {
    return false;
  }
  // Detect the known Windows build-script panic pattern. The build scripts
  // for proc-macro2, serde, serde_core, and quote panic with an Os error
  // code 0 ("操作成功完成。" / "The operation completed successfully")
  // during process spawning. This is a Rust standard library issue on
  // Windows, not a kernel code defect.
  const buildScriptPanicCrates = [
    'proc-macro2',
    'serde_core',
    'serde v',
    'quote v'
  ];
  const hasBuildScriptFailure = buildScriptPanicCrates.some((crate) =>
    output.includes(`failed to run custom build command for \`${crate}`)
  );
  const hasProcessPanic = output.includes("called `Result::unwrap()` on an `Err` value: Os { code: 0");
  return hasBuildScriptFailure && hasProcessPanic;
}

const rustParseCheck = runCargoTest('sdkwork-agent-provider-spi');
if (!rustParseCheck.passed) {
  errors.push(
    `provider-spi binding parse tests failed:\n${rustParseCheck.output}`
  );
}

const transportCrates = [
  'sdkwork-agent-provider-transport-ipc',
  'sdkwork-agent-provider-transport-rust',
  'sdkwork-agent-provider-transport-node',
  'sdkwork-agent-provider-transport-python',
  'sdkwork-agent-provider-transport-core'
];

for (const crateDir of transportCrates) {
  const transportCheck = runCargoTest(crateDir);
  if (!transportCheck.passed) {
    errors.push(
      `${crateDir} tests failed:\n${transportCheck.output}`
    );
  }
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log(`Agent provider binding check passed (${manifests.length} manifests).`);
