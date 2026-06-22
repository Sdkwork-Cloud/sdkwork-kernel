import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { AGENT_SDK_FAMILIES, resolveAgentSdkFamily } from './_shared/agent-sdk-families.mjs';
import { syncAgentSdkOwnershipWorkspace } from './_shared/agent-sdk-ownership.mjs';
import {
  SDKWORK_SDKGEN_STANDARD,
  resolveSdkgenEntrypoint
} from './_shared/sdkgen-standard.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = parseArgs(process.argv.slice(2));
const mode = args.mode ?? 'dry-run';
if (!['dry-run', 'apply'].includes(mode)) {
  throw new Error(`--mode must be dry-run or apply, received: ${mode}`);
}

const requestedFamily = args.family;
const families = requestedFamily
  ? [resolveAgentSdkFamily(args.family)]
  : AGENT_SDK_FAMILIES;
const sdkgenPath = resolveSdkgenEntrypoint();
const sdkgenReportPath = toReportPath(sdkgenPath);

if (!fs.existsSync(sdkgenPath)) {
  throw new Error(
    `sdkgen entrypoint not found: ${sdkgenPath}. Set ${SDKWORK_SDKGEN_STANDARD.envOverride} only to another sdkwork-sdk-generator entrypoint.`
  );
}

runNodeScript(path.join(root, 'sdks', 'materialize-agent-v3-openapi-boundaries.mjs'), []);
runNodeScript(path.join(root, 'sdks', 'materialize-agent-internal-api-openapi.mjs'), []);

const report = {
  schemaVersion: 1,
  app: 'agent',
  mode,
  standardProfile: SDKWORK_SDKGEN_STANDARD.standardProfile,
  sdkgenPath: sdkgenReportPath,
  startedAt: new Date().toISOString(),
  families: []
};

let appFamilyWasProcessed = false;
for (const family of families) {
  const input = path.join(
    root,
    'sdks',
    family.familyDir,
    'openapi',
    `${family.authority}.sdkgen.yaml`
  );
  const output = path.join(
    root,
    'sdks',
    family.familyDir,
    family.languagePackageDir,
    'generated',
    'server-openapi'
  );
  fs.mkdirSync(output, { recursive: true });

  if (family.externalSdkgenProfileSupported === false) {
    report.families.push({
      key: family.key,
      familyDir: family.familyDir,
      authority: family.authority,
      input: toReportPath(input),
      output: toReportPath(output),
      sdkName: family.sdkName,
      sdkType: family.sdkType,
      sdkSurface: family.sdkSurface,
      packageName: family.packageName,
      apiPrefix: family.apiPrefix,
      generated: false,
      skipped: true,
      skipReason: family.externalSdkgenProfileGap,
      derivedFrom: 'sdkwork-agent-app-sdk'
    });
    continue;
  }

  const baseArgs = [
    sdkgenPath,
    'generate',
    '-i',
    input,
    '-o',
    output,
    '-n',
    family.sdkName,
    '-t',
    family.sdkType,
    '-l',
    'typescript',
    '--base-url',
    'http://localhost:8080',
    '--api-prefix',
    family.apiPrefix,
    '--package-name',
    family.packageName,
    '--npm-package-name',
    family.npmPackageName,
    '--sdk-root',
    path.join(root, 'sdks', family.familyDir),
    '--sdk-name',
    family.sdkName,
    '--standard-profile',
    SDKWORK_SDKGEN_STANDARD.standardProfile,
    '--no-sync-published-version'
  ];

  const dryRun = runNodeForJson([
    ...baseArgs,
    '--fixed-sdk-version',
    '0.1.0',
    '--dry-run',
    '--json'
  ]);

  const familyReport = {
    key: family.key,
    familyDir: family.familyDir,
    authority: family.authority,
    input: toReportPath(input),
    output: toReportPath(output),
    sdkName: family.sdkName,
    sdkType: family.sdkType,
    sdkOwner: family.sdkOwner,
    packageName: family.packageName,
    apiPrefix: family.apiPrefix,
    sdkDependencies: family.sdkDependencies,
    version: dryRun.sdk?.version ?? '0.1.0',
    fingerprint: dryRun.changeFingerprint,
    hasChanges: Boolean(dryRun.hasChanges),
    riskLevel: dryRun.executionDecision?.riskLevel ?? 'unknown'
  };

  if (mode === 'apply') {
    if (!dryRun.changeFingerprint && dryRun.hasChanges) {
      throw new Error(`${family.familyDir} dry-run did not return a change fingerprint`);
    }
    if (dryRun.hasChanges) {
      runNodeScript(sdkgenPath, [
        'generate',
        '-i',
        input,
        '-o',
        output,
        '-n',
        family.sdkName,
        '-t',
        family.sdkType,
        '-l',
        'typescript',
        '--base-url',
        'http://localhost:8080',
        '--api-prefix',
        family.apiPrefix,
        '--package-name',
        family.packageName,
        '--npm-package-name',
        family.npmPackageName,
        '--sdk-root',
        path.join(root, 'sdks', family.familyDir),
        '--sdk-name',
        family.sdkName,
        '--standard-profile',
        SDKWORK_SDKGEN_STANDARD.standardProfile,
        '--no-sync-published-version',
        '--fixed-sdk-version',
        familyReport.version,
        '--expected-change-fingerprint',
        familyReport.fingerprint,
        '--license',
        'MIT'
      ]);
    }
    familyReport.generated = Boolean(dryRun.hasChanges);
  } else {
    familyReport.generated = false;
  }

  report.families.push(familyReport);
  if (family.key === 'app') {
    appFamilyWasProcessed = true;
  }
}

if (!requestedFamily || requestedFamily === 'open') {
  const openDerivationMode = mode;
  if (!appFamilyWasProcessed && !fs.existsSync(path.join(
    root,
    'sdks',
    'sdkwork-agent-app-sdk',
    'sdkwork-agent-app-sdk-typescript',
    'generated',
    'server-openapi',
    'src',
    'index.ts'
  ))) {
    throw new Error('Open SDK derivation requires sdkwork-agent-app-sdk generated source.');
  }
  const derivation = runNodeForJson([
    path.join(root, 'sdks', 'materialize-agent-open-sdk-from-app.mjs'),
    '--mode',
    openDerivationMode,
    '--json'
  ]);
  report.openSdkDerivation = derivation;
  for (const familyReport of report.families) {
    if (familyReport.key === 'open') {
      familyReport.derivedGenerated = mode === 'apply';
      familyReport.derivedHasChanges = derivation.hasChanges;
      familyReport.derivationFileCount = derivation.fileCount;
    }
  }
}

syncAgentSdkOwnershipWorkspace(root, AGENT_SDK_FAMILIES);
report.finishedAt = new Date().toISOString();
writeJson(path.join(root, 'sdks', '.sdkgen-agent-workspace-report.json'), report);
console.log(JSON.stringify(report, null, 2));

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--mode') {
      parsed.mode = argv[++index];
    } else if (value === '--family') {
      parsed.family = argv[++index];
    } else if (value === '--help' || value === '-h') {
      printHelpAndExit();
    } else {
      throw new Error(`Unknown argument: ${value}`);
    }
  }
  return parsed;
}

function printHelpAndExit() {
  console.log(`Usage: node sdks/workspace-agent-sdkgen.mjs [--mode dry-run|apply] [--family open|app|backend|internal]

Generates the SDKWork agent SDK families with --standard-profile ${SDKWORK_SDKGEN_STANDARD.standardProfile}.
`);
  process.exit(0);
}

function runNodeForJson(nodeArgs) {
  const result = spawnSync('node', nodeArgs, {
    cwd: root,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  });
  if (result.status !== 0) {
    throw new Error(
      `Command failed: node ${nodeArgs.join(' ')}\n${result.stdout}\n${result.stderr}`
    );
  }
  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(`Failed to parse sdkgen JSON output: ${error.message}\n${result.stdout}`);
  }
}

function runNodeScript(script, scriptArgs) {
  const result = spawnSync('node', [script, ...scriptArgs], {
    cwd: root,
    encoding: 'utf8',
    stdio: 'inherit'
  });
  if (result.status !== 0) {
    throw new Error(`Command failed: node ${script} ${scriptArgs.join(' ')}`);
  }
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function toReportPath(filePath) {
  const relative = path.relative(root, path.resolve(root, filePath));
  if (relative && !relative.startsWith('..') && !path.isAbsolute(relative)) {
    return normalizeReportPath(relative);
  }
  return normalizeReportPath(filePath);
}

function normalizeReportPath(filePath) {
  return String(filePath).replace(/\\/g, '/');
}
