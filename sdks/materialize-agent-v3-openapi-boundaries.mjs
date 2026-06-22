import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {
  AGENT_SDK_FAMILIES,
  forbiddenAgentApiPrefixesFor
} from './_shared/agent-sdk-families.mjs';
import {
  annotateAgentOpenApiOwnership,
  syncAgentSdkOwnershipWorkspace
} from './_shared/agent-sdk-ownership.mjs';

const root = process.cwd();
const problemRef = "          $ref: '#/components/responses/Problem'";
const explicitProblemResponse = `          description: RFC 9457 problem detail response
          content:
            application/problem+json:
              schema:
                $ref: '#/components/schemas/ProblemDetail'`;

for (const family of AGENT_SDK_FAMILIES) {
  if (!family.sourceOpenApi) {
    continue;
  }
  const sourcePath = path.join(root, family.sourceOpenApi);
  if (!fs.existsSync(sourcePath)) {
    throw new Error(`OpenAPI source not found for ${family.familyDir}: ${family.sourceOpenApi}`);
  }

  const source = fs.readFileSync(sourcePath, 'utf8');
  validateOpenApiStructure(source, family, 'source');
  const authority = materializeAuthority(source, family);
  const sdkgen = materializeSdkgen(authority, family);

  validateMaterializedOpenApi(authority, family, 'authority');
  validateMaterializedOpenApi(sdkgen, family, 'sdkgen');
  if (sdkgen.includes(problemRef)) {
    throw new Error(`${family.authority}.sdkgen.yaml still contains response $ref shorthands`);
  }

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

  if (family.moduleOpenApi) {
    writeTextIfChanged(path.join(root, family.moduleOpenApi), authority);
  }
}

syncAgentSdkOwnershipWorkspace(root, AGENT_SDK_FAMILIES);
console.log('Agent SDK OpenAPI boundaries materialized.');

function materializeAuthority(source, family) {
  let output = source;
  output = replaceOnce(output, /^  title: .+$/m, `  title: ${family.title}`);
  output = replaceOnce(output, /^  description: .+$/m, `  description: ${family.description}`);
  output = output.replaceAll(family.sourcePrefix, family.apiPrefix);
  output = annotateAgentOpenApiOwnership(output, family);
  return ensureTrailingNewline(output);
}

function materializeSdkgen(authority, family) {
  let output = authority.replaceAll(problemRef, explicitProblemResponse);
  output = ensureTrailingNewline(output);
  validateNoAuthorityExpansion(authority, output, family);
  return output;
}

function validateNoAuthorityExpansion(authority, sdkgen, family) {
  const authorityPaths = collectPathKeys(authority);
  const sdkgenPaths = collectPathKeys(sdkgen);
  const missing = authorityPaths.filter((entry) => !sdkgenPaths.includes(entry));
  const extra = sdkgenPaths.filter((entry) => !authorityPaths.includes(entry));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${family.authority} sdkgen path set differs from authority. missing=${missing.join(', ')} extra=${extra.join(', ')}`
    );
  }
}

function collectPathKeys(openapi) {
  return openapi
    .split(/\r?\n/)
    .filter((line) => /^  \/.+:$/.test(line))
    .map((line) => line.trim());
}

function validateMaterializedOpenApi(openapi, family, layer) {
  validateOpenApiStructure(openapi, family, layer);

  for (const required of [
    'openapi: 3.1.2',
    `title: ${family.title}`,
    family.apiPrefix,
    'operationId: agents.list',
    'operationId: agents.create',
    'operationId: agents.providerBindings.create',
    'operationId: agents.deployments.create',
    'components:',
    'application/problem+json',
    'Access-Token'
  ]) {
    if (!openapi.includes(required)) {
      throw new Error(`${family.authority} ${layer} OpenAPI missing ${required}`);
    }
  }

  for (const forbiddenPrefix of forbiddenAgentApiPrefixesFor(family)) {
    if (openapi.includes(forbiddenPrefix)) {
      throw new Error(`${family.authority} ${layer} OpenAPI contains ${forbiddenPrefix}`);
    }
  }

  if (openapi.includes('X-Request-Id')) {
    throw new Error(`${family.authority} ${layer} OpenAPI must not expose X-Request-Id`);
  }
}

function validateOpenApiStructure(openapi, family, layer) {
  const lineCount = openapi.split(/\r?\n/).length;
  if (lineCount > 25_000) {
    throw new Error(
      `${family.authority} ${layer} OpenAPI has ${lineCount} lines; expected a bounded contract under 25000 lines`
    );
  }

  for (const [section, pattern] of [
    ['openapi', /^openapi:\s*3\.1\.2$/],
    ['info', /^info:$/],
    ['paths', /^paths:$/],
    ['components', /^components:$/],
    ['components.parameters', /^  parameters:$/],
    ['components.responses', /^  responses:$/],
    ['components.schemas', /^  schemas:$/]
  ]) {
    const actual = countMatchingLines(openapi, pattern);
    if (actual !== 1) {
      throw new Error(
        `${family.authority} ${layer} OpenAPI must contain exactly one ${section} section, found ${actual}`
      );
    }
  }

  for (const forbidden of [
    /pattern: '[^\r\n]*(?:[ \t]{6,}type: object|components:|paths:)/,
    /pattern: '[^\r\n]{512,}/
  ]) {
    if (forbidden.test(openapi)) {
      throw new Error(`${family.authority} ${layer} OpenAPI contains a corrupted schema pattern`);
    }
  }
}

function countMatchingLines(content, pattern) {
  return content
    .split(/\r?\n/)
    .filter((line) => pattern.test(line))
    .length;
}

function writeTextIfChanged(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  if (fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8') === content) {
    return;
  }
  fs.writeFileSync(filePath, content, 'utf8');
}

function replaceOnce(content, pattern, replacement) {
  if (!pattern.test(content)) {
    throw new Error(`Pattern not found while materializing OpenAPI: ${pattern}`);
  }
  return content.replace(pattern, replacement);
}

function ensureTrailingNewline(content) {
  return content.endsWith('\n') ? content : `${content}\n`;
}
