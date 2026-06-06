import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { resolveAgentSdkFamily } from './_shared/agent-sdk-families.mjs';
import {
  syncAgentSdkOwnershipFamily
} from './_shared/agent-sdk-ownership.mjs';

const root = process.cwd();
const args = parseArgs(process.argv.slice(2));
const mode = args.mode ?? 'dry-run';
if (!['dry-run', 'apply'].includes(mode)) {
  throw new Error(`--mode must be dry-run or apply, received: ${mode}`);
}

const sourceFamily = resolveAgentSdkFamily('app');
const targetFamily = resolveAgentSdkFamily('open');
const sourceRoot = path.join(
  root,
  'sdks',
  sourceFamily.familyDir,
  sourceFamily.languagePackageDir,
  'generated',
  'server-openapi'
);
const targetRoot = path.join(
  root,
  'sdks',
  targetFamily.familyDir,
  targetFamily.languagePackageDir,
  'generated',
  'server-openapi'
);
if (!fs.existsSync(path.join(sourceRoot, 'src', 'index.ts'))) {
  throw new Error(
    `Open SDK derivation source is missing. Generate ${sourceFamily.familyDir} first: ${sourceRoot}`
  );
}

const files = collectSourceFiles(sourceRoot);
const nextFiles = new Map();
for (const relativePath of files) {
  const sourcePath = path.join(sourceRoot, relativePath);
  let content = fs.readFileSync(sourcePath, 'utf8');
  content = transformText(content);
  content = transformJsonByPath(relativePath, content);
  nextFiles.set(relativePath, content);
}

const derivationManifest = buildDerivationManifest(nextFiles);
nextFiles.set(
  '.sdkwork/sdkwork-open-sdk-derivation.json',
  `${JSON.stringify(derivationManifest, null, 2)}\n`
);

const changes = diffTarget(targetRoot, nextFiles);
if (mode === 'apply') {
  materializeTarget(targetRoot, nextFiles);
}

const report = {
  schemaVersion: 1,
  mode,
  sourceFamily: sourceFamily.familyDir,
  sourceAuthority: sourceFamily.authority,
  targetFamily: targetFamily.familyDir,
  targetAuthority: targetFamily.authority,
  packageName: targetFamily.packageName,
  apiPrefix: targetFamily.apiPrefix,
  sourceRoot,
  targetRoot,
  hasChanges: changes.created.length > 0 || changes.updated.length > 0 || changes.deleted.length > 0,
  changes,
  fileCount: nextFiles.size,
  generated: mode === 'apply'
};

if (args.json) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(
    `${targetFamily.familyDir} open SDK ${mode === 'apply' ? 'materialized' : 'dry-run complete'}: ` +
      `${report.fileCount} files, hasChanges=${report.hasChanges}`
  );
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--mode') {
      parsed.mode = argv[++index];
    } else if (value === '--json') {
      parsed.json = true;
    } else if (value === '--help' || value === '-h') {
      console.log('Usage: node sdks/materialize-agent-open-sdk-from-app.mjs [--mode dry-run|apply] [--json]');
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${value}`);
    }
  }
  return parsed;
}

function collectSourceFiles(directory) {
  const files = [];
  walk(directory, (filePath) => {
    const relativePath = normalizeRelative(path.relative(directory, filePath));
    if (shouldSkip(relativePath)) {
      return;
    }
    files.push(relativePath);
  });
  return files.sort();
}

function walk(directory, onFile) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      walk(entryPath, onFile);
    } else if (entry.isFile()) {
      onFile(entryPath);
    }
  }
}

function shouldSkip(relativePath) {
  const normalized = normalizeRelative(relativePath);
  return (
    normalized === '.gitkeep' ||
    normalized === 'package-lock.json' ||
    normalized.startsWith('node_modules/') ||
    normalized.startsWith('dist/') ||
    normalized.startsWith('.sdkwork/')
  );
}

function transformText(content) {
  return content
    .replaceAll('@sdkwork/agent-app-sdk', '@sdkwork/agent-sdk')
    .replaceAll('sdkwork-agent-app-sdk', 'sdkwork-agent-sdk')
    .replaceAll('/app/v3/api', '/agent/v3/api')
    .replaceAll('SdkworkAppClient', 'SdkworkAgentClient')
    .replaceAll('SdkworkAppConfig', 'SdkworkAgentConfig')
    .replaceAll('APP_API_PREFIX', 'AGENT_API_PREFIX')
    .replaceAll('appApiPath', 'agentApiPath');
}

function transformJsonByPath(relativePath, content) {
  if (relativePath === 'package.json') {
    const packageJson = JSON.parse(content);
    packageJson.name = targetFamily.packageName;
    packageJson.description = 'SDKWork agent developer Open API TypeScript SDK.';
    packageJson.keywords = ['sdk', 'api', 'agent', 'open', 'sdkwork'];
    delete packageJson.sdkwork;
    return `${JSON.stringify(packageJson, null, 2)}\n`;
  }
  if (relativePath === 'sdkwork-sdk.json') {
    const sdkworkJson = JSON.parse(content);
    deleteGeneratedOwnershipStandardKeys(sdkworkJson);
    sdkworkJson.name = targetFamily.sdkName;
    sdkworkJson.sdkType = targetFamily.sdkType;
    sdkworkJson.sdkSurface = targetFamily.sdkSurface;
    sdkworkJson.packageName = targetFamily.packageName;
    sdkworkJson.authority = targetFamily.authority;
    sdkworkJson.apiPrefix = targetFamily.apiPrefix;
    sdkworkJson.derivedFrom = {
      family: sourceFamily.familyDir,
      authority: sourceFamily.authority,
      reason: targetFamily.externalSdkgenProfileGap
    };
    return `${JSON.stringify(sdkworkJson, null, 2)}\n`;
  }
  return content;
}

function deleteGeneratedOwnershipStandardKeys(metadata) {
  for (const key of [
    'sdkOwner',
    'apiAuthority',
    'sdkFamily',
    'generationInputSpec',
    'sdkDependencies',
    'ownerOnlyOperationCount',
    'standardProfile',
    'standardVersion'
  ]) {
    delete metadata[key];
  }
}

function buildDerivationManifest(nextFiles) {
  return {
    schemaVersion: 1,
    generator: 'sdkwork-agent-open-sdk-derivation',
    source: {
      family: sourceFamily.familyDir,
      authority: sourceFamily.authority,
      packageName: sourceFamily.packageName,
      apiPrefix: sourceFamily.apiPrefix
    },
    target: {
      family: targetFamily.familyDir,
      authority: targetFamily.authority,
      packageName: targetFamily.packageName,
      apiPrefix: targetFamily.apiPrefix,
      sdkType: targetFamily.sdkType,
      sdkSurface: targetFamily.sdkSurface
    },
    externalSdkgenProfileGap: targetFamily.externalSdkgenProfileGap,
    generatedFiles: Array.from(nextFiles.entries()).map(([filePath, content]) => ({
      path: filePath,
      sha256: sha256(content)
    }))
  };
}

function diffTarget(targetDirectory, nextFiles) {
  const existingFiles = fs.existsSync(targetDirectory)
    ? collectExistingTargetFiles(targetDirectory)
    : [];
  const existingSet = new Set(existingFiles);
  const nextSet = new Set(nextFiles.keys());
  const created = [];
  const updated = [];
  const deleted = [];

  for (const [relativePath, content] of nextFiles.entries()) {
    const targetPath = path.join(targetDirectory, relativePath);
    if (!fs.existsSync(targetPath)) {
      created.push(relativePath);
      continue;
    }
    if (fs.readFileSync(targetPath, 'utf8') !== content) {
      updated.push(relativePath);
    }
  }
  for (const relativePath of existingSet) {
    if (!nextSet.has(relativePath)) {
      deleted.push(relativePath);
    }
  }
  return {
    created: created.sort(),
    updated: updated.sort(),
    deleted: deleted.sort()
  };
}

function collectExistingTargetFiles(directory) {
  const files = [];
  walk(directory, (filePath) => {
    const relativePath = normalizeRelative(path.relative(directory, filePath));
    if (
      !shouldSkip(relativePath) &&
      !relativePath.startsWith('node_modules/') &&
      !relativePath.startsWith('dist/')
    ) {
      files.push(relativePath);
    }
  });
  return files.sort();
}

function materializeTarget(targetDirectory, nextFiles) {
  fs.mkdirSync(targetDirectory, { recursive: true });
  const targetRootResolved = path.resolve(targetDirectory);
  if (!targetRootResolved.startsWith(path.resolve(root))) {
    throw new Error(`Refusing to materialize outside workspace: ${targetRootResolved}`);
  }

  for (const relativePath of collectExistingTargetFiles(targetDirectory)) {
    if (!nextFiles.has(relativePath)) {
      fs.rmSync(path.join(targetDirectory, relativePath), { force: true });
    }
  }
  for (const [relativePath, content] of nextFiles.entries()) {
    const targetPath = path.join(targetDirectory, relativePath);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.writeFileSync(targetPath, content, 'utf8');
  }
  syncAgentSdkOwnershipFamily(root, targetFamily);
}

function normalizeRelative(value) {
  return value.replace(/\\/g, '/');
}

function sha256(content) {
  return crypto.createHash('sha256').update(content).digest('hex');
}
