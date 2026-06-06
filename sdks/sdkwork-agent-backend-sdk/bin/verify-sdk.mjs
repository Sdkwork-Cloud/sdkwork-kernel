import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { resolveAgentSdkFamily } from '../../_shared/agent-sdk-families.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const family = resolveAgentSdkFamily('backend');

verify(family);
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
    'operationId: agents.list',
    'operationId: agents.providerBindings.create',
    'operationId: agents.deployments.create',
    'Access-Token'
  ]) {
    if (!authorityText.includes(required) || !sdkgenText.includes(required)) {
      throw new Error(`${candidate.familyDir} OpenAPI boundary missing ${required}`);
    }
  }
  if (sdkgenText.includes("$ref: '#/components/responses/Problem'")) {
    throw new Error(`${candidate.familyDir} sdkgen input must inline explicit problem responses`);
  }
}
