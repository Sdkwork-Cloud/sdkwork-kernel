import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { AGENT_SDK_FAMILIES } from '../../../sdks/_shared/agent-sdk-families.mjs';
import { SDKWORK_SDKGEN_STANDARD } from '../../../sdks/_shared/sdkgen-standard.mjs';
import { validateGeneratedAgentApi } from './generated-typescript-api-surface-checks.mjs';
import { validateOpenApi } from './openapi-checks.mjs';
import { validateSdkFamilyMetadata } from './sdk-family-metadata-checks.mjs';
import { validateSdkgenStandard } from './sdkgen-standard-checks.mjs';


export function runAgentSdkWorkspaceCheck() {
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const sdksRoot = path.join(root, 'sdks');
const families = AGENT_SDK_FAMILIES;

const errors = [];

validateSdkgenStandard({ root, errors, ensureFile, readIfExists, readJsonIfExists, families });

const agentBusinessApi = readIfExists(path.join(root, 'sdkwork-agent-business', 'src', 'api.rs'));
for (const required of [
  'AGENT_OPEN_API_PREFIX',
  '"/agent/v3/api"',
  'AGENT_OPEN_API_OPERATIONS'
]) {
  if (!agentBusinessApi.includes(required)) {
    errors.push(`sdkwork-agent-business/src/api.rs must include ${required}`);
  }
}

const agentBusinessHttp = readIfExists(path.join(root, 'sdkwork-agent-business', 'src', 'http.rs'));
for (const required of ['build_open_router', '"/agent/v3/api/ai/agents"']) {
  if (!agentBusinessHttp.includes(required)) {
    errors.push(`sdkwork-agent-business/src/http.rs must include ${required}`);
  }
}

for (const family of families) {
  const familyRoot = path.join(sdksRoot, family.familyDir);
  const authorityPath = path.join(
    familyRoot,
    'openapi',
    `${family.authority}.openapi.yaml`
  );
  const sdkgenPath = path.join(
    familyRoot,
    'openapi',
    `${family.authority}.sdkgen.yaml`
  );
  const packageRoot = path.join(familyRoot, `${family.familyDir}-typescript`);

  ensureFile(path.join('sdks', family.familyDir, 'README.md'));
  ensureFile(path.join('sdks', family.familyDir, '.sdkwork-assembly.json'));
  ensureFile(path.join('sdks', family.familyDir, 'sdk-manifest.json'));
  ensureFile(path.join('sdks', family.familyDir, 'specs', 'README.md'));
  ensureFile(path.join('sdks', family.familyDir, 'specs', 'component.spec.json'));
  ensureFile(path.join('sdks', family.familyDir, 'bin', 'verify-sdk.mjs'));
  ensureFile(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'README.md')
  );
  ensureFile(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'package.json')
  );
  ensureDirectory(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'generated', 'server-openapi')
  );
  ensureFile(path.relative(root, authorityPath));
  ensureFile(path.relative(root, sdkgenPath));
  const generatedApiPath = path.join(
    packageRoot,
    SDKWORK_SDKGEN_STANDARD.generatedOutput,
    'src',
    'api',
    'ai.ts'
  );
  ensureFile(path.relative(root, generatedApiPath));

  validateSdkFamilyMetadata({ family, familyRoot, packageRoot, errors, readIfExists, readJsonIfExists });

  const authority = readIfExists(authorityPath);
  const sdkgen = readIfExists(sdkgenPath);
  validateOpenApi({ label: `${family.familyDir} authority`, content: authority, family, errors });
  validateOpenApi({ label: `${family.familyDir} sdkgen`, content: sdkgen, family, errors });
  if (sdkgen.includes("$ref: '#/components/responses/Problem'")) {
    errors.push(`${family.familyDir} sdkgen input must inline explicit problem responses`);
  }

  validateGeneratedAgentApi({
    label: `${family.familyDir} generated TypeScript API`,
    content: readIfExists(generatedApiPath),
    family,
    errors
  });
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log('Agent SDK workspace check passed.');

function ensureFile(relativePath) {
  const filePath = path.join(root, relativePath);
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    errors.push(`missing file: ${relativePath}`);
  }
}

function ensureDirectory(relativePath) {
  const directoryPath = path.join(root, relativePath);
  if (!fs.existsSync(directoryPath) || !fs.statSync(directoryPath).isDirectory()) {
    errors.push(`missing directory: ${relativePath}`);
  }
}

function readIfExists(filePath) {
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    return '';
  }
  return fs.readFileSync(filePath, 'utf8');
}

function readJsonIfExists(filePath) {
  const content = readIfExists(filePath);
  if (!content) {
    return null;
  }
  try {
    return JSON.parse(content);
  } catch (error) {
    errors.push(`invalid json: ${path.relative(root, filePath)}: ${error.message}`);
    return null;
  }
}
}

