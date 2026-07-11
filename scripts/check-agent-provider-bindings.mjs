import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), '..');
const catalogRoot = path.join(root, 'bindings', 'agent-providers');
const schemaPath = path.join(root, 'specs', 'schemas', 'agent-sdk-binding.schema.json');

const integrationSourceFields = new Set([
  'mode',
  'package',
  'crate',
  'module',
  'transport',
  'repository',
  'path',
  'feature',
  'optional'
]);

const sourceLocatorByMode = new Map([
  ['official_sdk', 'package'],
  ['rust_crate', 'crate'],
  ['source_tree', 'path'],
  ['npm_package', 'package'],
  ['python_module', 'module'],
  ['http_openapi', 'transport'],
  ['ipc_protocol', 'transport']
]);

const manifestFields = new Set([
  'schema_version',
  'manifest_type',
  'binding_id',
  'agent_id',
  'display_name',
  'description',
  'version',
  'sdk_owner',
  'status',
  'kernel_compatibility',
  'selection_policy',
  'language_packages',
  'integration_sources',
  'capabilities'
]);

const selectionPolicyFields = new Set(['default_backend_priority']);
const languagePackageFields = new Set(['rust', 'typescript', 'python']);
const rustPackageFields = new Set(['crate', 'version', 'optional']);
const npmPackageFields = new Set(['package', 'version', 'optional']);
const pythonPackageFields = new Set(['module', 'version', 'optional']);
const capabilityFields = new Set([
  'capability_id',
  'required',
  'execution_scope',
  'backends'
]);
const backendFields = new Set([
  'kind',
  'driver_id',
  'runtime_operations',
  'crate',
  'package',
  'python_module',
  'openapi_authority',
  'transport'
]);

const capabilityExecutionScopes = new Set(['transport_runtime', 'provider_local']);
const runtimeOperations = new Set([
  'ping',
  'session_create',
  'model_chat',
  'model_chat_stream',
  'tool_invoke',
  'skill_invoke'
]);
const unsupportedRustRuntimeOperations = new Set(['session_create', 'skill_invoke']);
const sourceTreeScanSkipDirectories = new Set([
  '.git',
  '.next',
  '.pnpm',
  '.turbo',
  'coverage',
  'dist',
  'node_modules',
  'target'
]);
const sourceTreeScanMaxDepth = 6;

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

function validateAllowedFields(objectPath, value, allowedFields, errors) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return;
  }

  for (const field of Object.keys(value)) {
    if (!allowedFields.has(field)) {
      errors.push(`${objectPath} field ${field} is not allowed`);
    }
  }
}

function listBindingManifests(directory) {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(directory, entry.name, 'provider-binding.manifest.json'))
    .filter((manifestPath) => fs.existsSync(manifestPath));
}

function toRepositoryPath(filePath) {
  return filePath.split(path.sep).join('/');
}

function readJsonFile(absolutePath) {
  try {
    return JSON.parse(fs.readFileSync(absolutePath, 'utf8'));
  } catch {
    return null;
  }
}

function readTextFile(absolutePath) {
  try {
    return fs.readFileSync(absolutePath, 'utf8');
  } catch {
    return null;
  }
}

function listMetadataFiles(sourceRoot, fileName) {
  const matches = [];
  if (!fs.existsSync(sourceRoot) || !fs.statSync(sourceRoot).isDirectory()) {
    return matches;
  }

  const queue = [{ directory: sourceRoot, depth: 0 }];
  for (let cursor = 0; cursor < queue.length; cursor += 1) {
    const current = queue[cursor];
    const metadataPath = path.join(current.directory, fileName);
    if (fs.existsSync(metadataPath) && fs.statSync(metadataPath).isFile()) {
      matches.push(metadataPath);
    }

    if (current.depth >= sourceTreeScanMaxDepth) {
      continue;
    }

    let entries;
    try {
      entries = fs.readdirSync(current.directory, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of entries) {
      if (!entry.isDirectory() || sourceTreeScanSkipDirectories.has(entry.name)) {
        continue;
      }
      queue.push({
        directory: path.join(current.directory, entry.name),
        depth: current.depth + 1
      });
    }
  }

  return matches;
}

function findTypeScriptPackageDirectories(sourceRoot, packageName) {
  return listMetadataFiles(sourceRoot, 'package.json')
    .filter((packagePath) => readJsonFile(packagePath)?.name === packageName)
    .map((packagePath) => path.dirname(packagePath));
}

function findRustCrateDirectories(sourceRoot, crateName) {
  return listMetadataFiles(sourceRoot, 'Cargo.toml')
    .filter((cargoPath) => {
      const content = readTextFile(cargoPath);
      return Boolean(content?.match(new RegExp(`^name\\s*=\\s*"${escapeRegExp(crateName)}"`, 'm')));
    })
    .map((cargoPath) => path.dirname(cargoPath));
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function findMappingDocumentPath(workspaceRoot, providerName) {
  const mappingRoot = path.join(
    workspaceRoot,
    'sdkwork-kernel-plugins',
    'specs',
    'mappings'
  );
  const candidates = [
    `${providerName}.md`,
    providerName === 'hermes' ? 'hermes-agent.md' : null
  ].filter(Boolean);

  for (const candidate of candidates) {
    const candidatePath = path.join(mappingRoot, candidate);
    if (fs.existsSync(candidatePath)) {
      return candidatePath;
    }
  }

  return path.join(mappingRoot, `${providerName}.md`);
}

function collectRustCrateNames(manifest) {
  const crates = new Set();
  if (manifest.language_packages?.rust?.crate) {
    crates.add(manifest.language_packages.rust.crate);
  }
  for (const source of manifest.integration_sources ?? []) {
    if (source.mode === 'rust_crate' && source.crate) {
      crates.add(source.crate);
    }
  }
  for (const capability of manifest.capabilities ?? []) {
    for (const backend of capability.backends ?? []) {
      if (backend.kind === 'rust_native' && backend.crate) {
        crates.add(backend.crate);
      }
    }
  }
  return [...crates];
}

export function collectSourceTreeDocumentationErrors(options = {}) {
  const workspaceRoot = options.workspaceRoot ?? root;
  const providerCatalogRoot = path.join(workspaceRoot, 'bindings', 'agent-providers');
  const errors = [];

  if (!fs.existsSync(providerCatalogRoot)) {
    return errors;
  }

  for (const manifestPath of listBindingManifests(providerCatalogRoot)) {
    const manifest = readJsonFile(manifestPath);
    if (!manifest) {
      continue;
    }

    const providerName = path.basename(path.dirname(manifestPath));
    const mappingPath = findMappingDocumentPath(workspaceRoot, providerName);
    const mappingContent = readTextFile(mappingPath);
    const mappingRelativePath = toRepositoryPath(path.relative(workspaceRoot, mappingPath));

    for (const source of manifest.integration_sources ?? []) {
      if (source.mode !== 'source_tree' || !source.path) {
        continue;
      }

      const sourceRoot = path.join(workspaceRoot, source.path);
      if (!fs.existsSync(sourceRoot)) {
        continue;
      }

      if (!mappingContent) {
        errors.push(
          `${mappingRelativePath} missing mapping document for source_tree ${source.path}`
        );
        continue;
      }

      const typeScriptPackageName = manifest.language_packages?.typescript?.package;
      if (typeScriptPackageName) {
        const packageDirectories = findTypeScriptPackageDirectories(
          sourceRoot,
          typeScriptPackageName
        );
        for (const packageDirectory of packageDirectories) {
          const packageRelativePath = toRepositoryPath(path.relative(workspaceRoot, packageDirectory));
          if (!mappingContent.includes(packageRelativePath)) {
            errors.push(
              `${mappingRelativePath} must document TypeScript SDK package source path ${packageRelativePath} for ${typeScriptPackageName}`
            );
          }
          if (
            packageRelativePath !== source.path &&
            !/(source reference|reference source|inspection input)/i.test(mappingContent)
          ) {
            errors.push(
              `${mappingRelativePath} must state that ${source.path} is a source reference or inspection input, not the runtime SDK package root`
            );
          }
        }
      }

      for (const crateName of collectRustCrateNames(manifest)) {
        const crateDirectories = findRustCrateDirectories(sourceRoot, crateName);
        for (const crateDirectory of crateDirectories) {
          const crateRelativePath = toRepositoryPath(path.relative(workspaceRoot, crateDirectory));
          if (!mappingContent.includes(crateRelativePath)) {
            errors.push(
              `${mappingRelativePath} must document Rust crate source path ${crateRelativePath} for ${crateName}`
            );
          }
          if (
            crateRelativePath !== source.path &&
            !/(source reference|reference source|inspection input)/i.test(mappingContent)
          ) {
            errors.push(
              `${mappingRelativePath} must state that ${source.path} is a source reference or inspection input, not the runtime crate root`
            );
          }
        }
      }
    }
  }

  return errors;
}

function validateManifestShape(relativePath, manifest, errors) {
  validateAllowedFields(relativePath, manifest, manifestFields, errors);

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

  validateAllowedFields(
    `${relativePath} selection_policy`,
    manifest.selection_policy,
    selectionPolicyFields,
    errors
  );

  validateAllowedFields(
    `${relativePath} language_packages`,
    manifest.language_packages,
    languagePackageFields,
    errors
  );
  validateAllowedFields(
    `${relativePath} language_packages.rust`,
    manifest.language_packages?.rust,
    rustPackageFields,
    errors
  );
  validateAllowedFields(
    `${relativePath} language_packages.typescript`,
    manifest.language_packages?.typescript,
    npmPackageFields,
    errors
  );
  validateAllowedFields(
    `${relativePath} language_packages.python`,
    manifest.language_packages?.python,
    pythonPackageFields,
    errors
  );

  for (const [index, capability] of (manifest.capabilities ?? []).entries()) {
    validateAllowedFields(
      `${relativePath} capabilities[${index}]`,
      capability,
      capabilityFields,
      errors
    );
  }

  for (const capability of manifest.capabilities ?? []) {
    if (!capability.capability_id?.startsWith('sdk.')) {
      errors.push(`${relativePath} capability_id must use sdk.* namespace`);
    }
    if (!Array.isArray(capability.backends) || capability.backends.length === 0) {
      errors.push(`${relativePath} capability ${capability.capability_id} must declare backends`);
    }
    if (!capability.execution_scope) {
      errors.push(
        `${relativePath} capability ${capability.capability_id} must declare execution_scope`
      );
    } else if (!capabilityExecutionScopes.has(capability.execution_scope)) {
      errors.push(
        `${relativePath} capability ${capability.capability_id} execution_scope ${capability.execution_scope} is not supported`
      );
    }
    for (const [backendIndex, backend] of (capability.backends ?? []).entries()) {
      const capabilityIndex = (manifest.capabilities ?? []).indexOf(capability);
      validateAllowedFields(
        `${relativePath} capabilities[${capabilityIndex}].backends[${backendIndex}]`,
        backend,
        backendFields,
        errors
      );
      if (!backend.driver_id?.startsWith('driver.')) {
        errors.push(`${relativePath} backend driver_id must use driver.* namespace`);
      }
      if (!Array.isArray(backend.runtime_operations) || backend.runtime_operations.length === 0) {
        errors.push(
          `${relativePath} backend ${backend.driver_id} must declare runtime_operations`
        );
        continue;
      }
      for (const operation of backend.runtime_operations) {
        if (!runtimeOperations.has(operation)) {
          errors.push(
            `${relativePath} backend ${backend.driver_id} runtime operation ${operation} is not supported`
          );
        }
        if (backend.kind === 'rust_native' && unsupportedRustRuntimeOperations.has(operation)) {
          errors.push(
            `${relativePath} rust_native backend ${backend.driver_id} must not declare unsupported runtime operation ${operation}`
          );
        }
      }
      if (
        capability.execution_scope === 'provider_local' &&
        backend.runtime_operations.some((operation) => operation !== 'ping')
      ) {
        errors.push(
          `${relativePath} provider_local capability ${capability.capability_id} backend ${backend.driver_id} may only declare ping runtime operation`
        );
      }
    }
  }
}

function validateIntegrationSources(relativePath, manifest, errors) {
  for (const [index, source] of (manifest.integration_sources ?? []).entries()) {
    for (const field of Object.keys(source)) {
      if (!integrationSourceFields.has(field)) {
        errors.push(`${relativePath} integration_sources[${index}] field ${field} is not allowed`);
      }
    }

    const requiredLocator = sourceLocatorByMode.get(source.mode);
    if (!requiredLocator) {
      errors.push(
        `${relativePath} integration_sources[${index}] mode ${source.mode} is not a supported integration source mode`
      );
      continue;
    }
    if (!source[requiredLocator]) {
      errors.push(
        `${relativePath} integration_sources[${index}] ${source.mode} source must declare ${requiredLocator}`
      );
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

function validateHttpOpenApiConsistency(relativePath, manifest, errors) {
  const httpSources = (manifest.integration_sources ?? [])
    .filter((source) => source.mode === 'http_openapi');
  const httpBackends = (manifest.capabilities ?? [])
    .flatMap((capability) => capability.backends ?? [])
    .filter((backend) => backend.kind === 'http_openapi');

  for (const source of httpSources) {
    if (!source.transport) {
      errors.push(`${relativePath} http_openapi source must declare transport`);
      continue;
    }
    const backed = httpBackends.some(
      (backend) => backend.openapi_authority === source.transport
    );
    if (!backed) {
      errors.push(
        `${relativePath} http_openapi source transport ${source.transport} must match at least one http_openapi backend openapi_authority`
      );
    }
  }

  for (const backend of httpBackends) {
    if (!backend.openapi_authority) {
      errors.push(
        `${relativePath} http_openapi backend ${backend.driver_id} must declare openapi_authority`
      );
      continue;
    }
    const sourced = httpSources.some(
      (source) => source.transport === backend.openapi_authority
    );
    if (!sourced) {
      errors.push(
        `${relativePath} http_openapi backend ${backend.driver_id} authority ${backend.openapi_authority} must match a http_openapi integration source`
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
  validateIntegrationSources(relativePath, manifest, errors);
  validateTypeScriptPackageConsistency(relativePath, manifest, errors);
  validateHttpOpenApiConsistency(relativePath, manifest, errors);
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

  errors.push(...collectSourceTreeDocumentationErrors({ workspaceRoot: root }));

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
