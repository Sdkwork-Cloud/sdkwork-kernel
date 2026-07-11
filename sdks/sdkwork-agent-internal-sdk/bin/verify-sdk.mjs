import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolveAgentSdkFamily } from '../../_shared/agent-sdk-families.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const family = resolveAgentSdkFamily('internal');

verify(family);
verifyTypeScriptPackage(family);
console.log(`${family.familyDir} SDK boundary check passed.`);

function verify(candidate) {
  const familyRoot = path.join(root, 'sdks', candidate.familyDir);
  const authority = path.join(familyRoot, 'openapi', `${candidate.authority}.openapi.yaml`);
  const sdkgen = path.join(familyRoot, 'openapi', `${candidate.authority}.sdkgen.yaml`);
  const output = path.join(
    familyRoot,
    candidate.languagePackageDir,
    'generated',
    'server-openapi'
  );

  for (const filePath of [authority, sdkgen]) {
    if (!fs.existsSync(filePath)) {
      throw new Error(`missing OpenAPI file: ${filePath}`);
    }
  }
  if (!fs.existsSync(output) || !fs.statSync(output).isDirectory()) {
    throw new Error(`missing generated output directory: ${output}`);
  }

  const authorityText = fs.readFileSync(authority, 'utf8');
  const sdkgenText = fs.readFileSync(sdkgen, 'utf8');
  for (const required of [
    candidate.apiPrefix,
    candidate.title,
    'operationId: runtime.snapshot.load',
    'operationId: runtime.sessions.create',
    'operationId: runtime.sessions.events.stream',
    'x-sdkwork-api-surface: internal-api',
    'ApiKey'
  ]) {
    if (!authorityText.includes(required) || !sdkgenText.includes(required)) {
      throw new Error(`${candidate.familyDir} OpenAPI boundary missing ${required}`);
    }
  }
  if (sdkgenText.includes("$ref: '#/components/responses/Problem'")) {
    throw new Error(`${candidate.familyDir} sdkgen input must inline explicit problem responses`);
  }
}

function verifyTypeScriptPackage(candidate) {
  const packageRoot = path.join(
    root,
    'sdks',
    candidate.familyDir,
    candidate.languagePackageDir
  );
  for (const relativePath of [
    'tsconfig.json',
    'src/index.ts',
    'src/sse-parser.ts',
    'src/streaming.ts',
    'tests/sse-parser.test.mjs',
    'tests/streaming.test.mjs'
  ]) {
    const filePath = path.join(packageRoot, relativePath);
    if (!fs.existsSync(filePath)) {
      throw new Error(`missing TypeScript SDK verification source: ${filePath}`);
    }
  }

  run(process.execPath, [
    path.join(packageRoot, 'node_modules', 'typescript', 'bin', 'tsc'),
    '--noEmit',
    '-p',
    path.join(packageRoot, 'tsconfig.json')
  ], packageRoot);
  const testFiles = fs
    .readdirSync(path.join(packageRoot, 'tests'))
    .filter((fileName) => fileName.endsWith('.test.mjs'))
    .sort()
    .map((fileName) => path.join(packageRoot, 'tests', fileName));
  run(process.execPath, ['--experimental-strip-types', '--test', ...testFiles], packageRoot);
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: 'pipe'
  });
  if (result.status === 0) {
    return;
  }

  const output = [result.stdout, result.stderr]
    .filter(Boolean)
    .join('\n')
    .trim();
  throw new Error(
    `${command} ${args.join(' ')} failed in ${cwd}${output ? `\n${output}` : ''}`
  );
}
