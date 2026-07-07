import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), '..');
const catalogRoot = path.join(root, 'bindings', 'agent-providers');
const schemaPath = path.join(root, 'specs', 'schemas', 'agent-sdk-binding.schema.json');

function ensureDirectory(relativePath, errors) {
  const absolutePath = path.join(root, relativePath);
  if (!fs.existsSync(absolutePath) || !fs.statSync(absolutePath).isDirectory()) {
    errors.push(`missing directory: ${relativePath}`);
  }
}

function readJson(relativePath, errors) {
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

function validateManifestShape(relativePath, manifest, errors) {
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

function validateTypeScriptPackageConsistency(relativePath, manifest, errors) {
  const typeScriptPackage = manifest.language_packages?.typescript?.package;
  const typeScriptBackends = (manifest.capabilities ?? [])
    .flatMap((capability) => capability.backends ?? [])
    .filter((backend) => backend.kind === 'typescript_node');

  for (const backend of typeScriptBackends) {
    if (!backend.package) {
      errors.push(
        `${relativePath} typescript_node backend ${backend.driver_id} must declare package`
      );
      continue;
    }
    if (!typeScriptPackage) {
      errors.push(
        `${relativePath} typescript_node backend ${backend.driver_id} declares package ${backend.package} but language_packages.typescript.package is missing`
      );
      continue;
    }
    if (backend.package !== typeScriptPackage) {
      errors.push(
        `${relativePath} typescript_node backend ${backend.driver_id} package ${backend.package} must match language_packages.typescript.package ${typeScriptPackage}`
      );
    }
  }

  if (!typeScriptPackage || typeScriptBackends.length === 0) {
    return;
  }

  for (const source of manifest.integration_sources ?? []) {
    if (source.mode !== 'official_sdk' || !source.package) {
      continue;
    }
    if (source.package !== typeScriptPackage) {
      errors.push(
        `${relativePath} official_sdk package ${source.package} must match language_packages.typescript.package ${typeScriptPackage}`
      );
    }
  }
}

export function collectManifestValidationErrors(
  manifest,
  relativePath = 'provider-binding.manifest.json'
) {
  const errors = [];
  validateManifestShape(relativePath, manifest, errors);
  validateTypeScriptPackageConsistency(relativePath, manifest, errors);
  return errors;
}

function runCargoTest(crateDir) {
  const result = spawnSync(
    'cargo',
    ['test', '--manifest-path', `${crateDir}/Cargo.toml`, '-q'],
    {
      cwd: root,
      encoding: 'utf8',
      shell: process.platform === 'win32',
      maxBuffer: 64 * 1024 * 1024
    }
  );
  if (result.status === 0) {
    return { passed: true, skipped: false };
  }
  const output = `${result.stdout}${result.stderr}`;
  if (isWindowsBuildScriptPanic(output)) {
    // Known Windows toolchain issue: proc-macro2/serde/serde_core/quote build
    // scripts panic during process spawning on certain Windows configurations.
    // Linux CI remains authoritative for these crates.
    console.warn(
      `warning: ${crateDir} cargo tests skipped on Windows due to build-script toolchain panic (proc-macro2/serde/quote). ` +
      'These crates are validated on Linux CI. See AGENTS.md "Build, Test, and Verification" for details.'
    );
    return { passed: true, skipped: true };
  }
  return {
    passed: false,
    skipped: false,
    output: `${result.error?.message ?? ''}\n${output}`.trim()
  };
}

function isWindowsBuildScriptPanic(output) {
  if (process.platform !== 'win32') {
    return false;
  }
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

export function runAgentProviderBindingCheck() {
  const errors = [];

  ensureDirectory('bindings/agent-providers', errors);
  if (!fs.existsSync(schemaPath)) {
    errors.push('missing schema: specs/schemas/agent-sdk-binding.schema.json');
  }

  const manifests = listBindingManifests(catalogRoot);
  if (manifests.length === 0) {
    errors.push('agent provider binding catalog must contain at least one manifest');
  }

  for (const manifestPath of manifests) {
    const relativePath = path.relative(root, manifestPath).replaceAll('\\', '/');
    const manifest = readJson(relativePath, errors);
    if (manifest) {
      errors.push(...collectManifestValidationErrors(manifest, relativePath));
    }
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
    return { passed: false, errors };
  }

  console.log(`Agent provider binding check passed (${manifests.length} manifests).`);
  return { passed: true, errors: [] };
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  const result = runAgentProviderBindingCheck();
  if (!result.passed) {
    process.exitCode = 1;
  }
}
