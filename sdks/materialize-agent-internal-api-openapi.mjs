import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {
  AGENT_SDK_FAMILIES,
  resolveAgentSdkFamily
} from './_shared/agent-sdk-families.mjs';
import {
  annotateAgentOpenApiOwnership,
  syncAgentSdkOwnershipWorkspace
} from './_shared/agent-sdk-ownership.mjs';

const root = process.cwd();
const family = resolveAgentSdkFamily('internal');
const problemRef = "          $ref: '#/components/responses/Problem'";
const explicitProblemResponse = `          description: RFC 9457 problem detail response
          content:
            application/problem+json:
              schema:
                $ref: '#/components/schemas/ProblemDetail'`;

const sourcePath = path.join(root, family.authorityOpenApi);
if (!fs.existsSync(sourcePath)) {
  throw new Error(`Internal API authority source not found: ${family.authorityOpenApi}`);
}

const source = fs.readFileSync(sourcePath, 'utf8');
const authority = annotateAgentOpenApiOwnership(source, family);
const sdkgen = materializeSdkgen(authority);

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

function materializeSdkgen(authorityYaml) {
  let output = authorityYaml.replaceAll(problemRef, explicitProblemResponse);
  if (output.includes(problemRef)) {
    throw new Error(`${family.authority}.sdkgen.yaml still contains response $ref shorthands`);
  }
  return ensureTrailingNewline(output);
}

function writeTextIfChanged(filePath, content) {
  const normalized = ensureTrailingNewline(content);
  if (fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8') === normalized) {
    return;
  }
  fs.writeFileSync(filePath, normalized, 'utf8');
}

function ensureTrailingNewline(content) {
  return content.endsWith('\n') ? content : `${content}\n`;
}
