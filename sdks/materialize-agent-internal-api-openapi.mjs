import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {
  AGENT_SDK_FAMILIES,
  resolveAgentSdkFamily
} from './_shared/agent-sdk-families.mjs';
import { syncAgentSdkOwnershipWorkspace } from './_shared/agent-sdk-ownership.mjs';
import {
  ensureTrailingNewline,
  materializeInternalOpenApiAuthority,
  materializeInternalOpenApiSdkgen
} from './_shared/materialize-internal-openapi.mjs';

const root = process.cwd();
const family = resolveAgentSdkFamily('internal');

const sourcePath = path.join(root, family.authorityOpenApi);
if (!fs.existsSync(sourcePath)) {
  throw new Error(`Internal API authority source not found: ${family.authorityOpenApi}`);
}

const source = fs.readFileSync(sourcePath, 'utf8');
const authority = materializeInternalOpenApiAuthority(source, family);
const sdkgen = materializeInternalOpenApiSdkgen(authority, family.authority);

const familyOpenApiDir = path.join(root, 'sdks', family.familyDir, 'openapi');
fs.mkdirSync(familyOpenApiDir, { recursive: true });
writeTextIfChanged(
  path.join(familyOpenApiDir, `${family.authority}.openapi.yaml`),
  authority
);
writeTextIfChanged(
  path.join(familyOpenApiDir, `${family.authority}.sdkgen.yaml`),
  sdkgen
);

syncAgentSdkOwnershipWorkspace(root, AGENT_SDK_FAMILIES);
console.log('Agent internal API OpenAPI boundaries materialized.');

function writeTextIfChanged(filePath, content) {
  const normalized = ensureTrailingNewline(content);
  if (fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8') === normalized) {
    return;
  }
  fs.writeFileSync(filePath, normalized, 'utf8');
}
