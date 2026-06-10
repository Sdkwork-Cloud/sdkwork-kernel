import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const appApiFiles = [
  'sdkwork-agent-business/specs/openapi/agent-business-app-openapi-3.1.2.yaml',
  'sdks/sdkwork-agent-app-sdk/openapi/sdkwork-agent-app-api.openapi.yaml',
  'sdks/sdkwork-agent-app-sdk/openapi/sdkwork-agent-app-api.sdkgen.yaml'
];

const generatedApiFile =
  'sdks/sdkwork-agent-app-sdk/sdkwork-agent-app-sdk-typescript/generated/server-openapi/src/api/ai.ts';
const generatedTypeDir =
  'sdks/sdkwork-agent-app-sdk/sdkwork-agent-app-sdk-typescript/generated/server-openapi/src/types';
const backendHttpFile = 'sdkwork-agent-business/src/http.rs';

const errors = [];

for (const relativePath of appApiFiles) {
  const content = read(relativePath);
  assertNoClientScopeQueryParameters(relativePath, content);
  assertNoClientScopeRequestSchemas(relativePath, content);
}

assertGeneratedAppSdkDoesNotExposeScopeParams(
  generatedApiFile,
  read(generatedApiFile)
);
assertGeneratedRequestTypesDoNotExposeScope(generatedTypeDir);
assertAppHandlersConsumeTypedContext(backendHttpFile, read(backendHttpFile));

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log('Agent app-api context scope contract passed.');

function assertNoClientScopeQueryParameters(label, content) {
  for (const forbidden of [
    "$ref: '#/components/parameters/TenantId'",
    "$ref: '#/components/parameters/OrganizationId'",
    "$ref: '#/components/parameters/OwnerUserId'",
    'name: tenant_id',
    'name: organization_id',
    'name: owner_user_id'
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose client-controlled app-api scope query parameter ${forbidden}`);
    }
  }
}

function assertNoClientScopeRequestSchemas(label, content) {
  const requestBlocks = schemaBlocks(content).filter(({ name }) => /Request$/.test(name));
  for (const { name, body } of requestBlocks) {
    for (const forbidden of ['tenantId', 'organizationId', 'ownerUserId']) {
      if (new RegExp(`(^|\\n)\\s{8}${forbidden}:\\s*\\n`, 'u').test(body)) {
        errors.push(`${label} ${name} must not expose client-controlled request field ${forbidden}`);
      }
      if (new RegExp(`required:\\s*\\[[^\\]]*\\b${forbidden}\\b`, 'u').test(body)) {
        errors.push(`${label} ${name} must not require client-controlled request field ${forbidden}`);
      }
    }
  }
}

function assertGeneratedAppSdkDoesNotExposeScopeParams(label, content) {
  for (const forbidden of [
    'tenantId: Int64String',
    'organizationId?: Int64String',
    'ownerUserId?: Int64String',
    "{ name: 'tenant_id'",
    "{ name: 'organization_id'",
    "{ name: 'owner_user_id'"
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose generated app SDK scope parameter ${forbidden}`);
    }
  }
}

function assertGeneratedRequestTypesDoNotExposeScope(relativeDir) {
  const absoluteDir = path.join(root, relativeDir);
  if (!fs.existsSync(absoluteDir)) {
    errors.push(`missing generated app SDK type directory: ${relativeDir}`);
    return;
  }
  for (const entry of fs.readdirSync(absoluteDir, { withFileTypes: true })) {
    if (!entry.isFile() || !/-request\.ts$/u.test(entry.name)) {
      continue;
    }
    const relativePath = path.join(relativeDir, entry.name).replaceAll(path.sep, '/');
    const content = read(relativePath);
    for (const forbidden of [
      'tenantId',
      'organizationId',
      'ownerUserId'
    ]) {
      if (content.includes(forbidden)) {
        errors.push(`${relativePath} must not expose client-controlled request field ${forbidden}`);
      }
    }
  }
}

function assertAppHandlersConsumeTypedContext(label, content) {
  if (!content.includes('AgentRequestContext')) {
    errors.push(`${label} must define and consume AgentRequestContext projected from appbase request context`);
  }
  if (!content.includes('Extension<AgentRequestContext>')) {
    errors.push(`${label} app-api handlers must consume Extension<AgentRequestContext>`);
  }
  const appHandlerBlocks = functionBlocks(content).filter(({ name }) => name.startsWith('app_'));
  for (const { name, body } of appHandlerBlocks) {
    if (body.includes('HeaderMap')) {
      errors.push(`${label} ${name} must not accept HeaderMap for app-api credential or scope parsing`);
    }
    if (body.includes('extract_policy_subject')) {
      errors.push(`${label} ${name} must not call extract_policy_subject; use AgentRequestContext.subject()`);
    }
  }
}

function schemaBlocks(content) {
  const lines = content.split(/\r?\n/u);
  const blocks = [];
  let current = null;
  for (const line of lines) {
    const match = /^    ([A-Za-z][A-Za-z0-9]+):\s*$/u.exec(line);
    if (match) {
      if (current) {
        blocks.push(current);
      }
      current = { name: match[1], body: line };
      continue;
    }
    if (current) {
      if (/^    [A-Za-z][A-Za-z0-9]+:\s*$/u.test(line)) {
        blocks.push(current);
        current = null;
      } else {
        current.body += `\n${line}`;
      }
    }
  }
  if (current) {
    blocks.push(current);
  }
  return blocks;
}

function functionBlocks(content) {
  const matches = [...content.matchAll(/async fn (app_[A-Za-z0-9_]+)\s*\(/gu)];
  return matches.map((match) => {
    const start = match.index ?? 0;
    const openBrace = content.indexOf('{', start);
    let depth = 0;
    let end = content.length;
    for (let index = openBrace; index < content.length; index += 1) {
      const char = content[index];
      if (char === '{') {
        depth += 1;
      } else if (char === '}') {
        depth -= 1;
        if (depth === 0) {
          end = index + 1;
          break;
        }
      }
    }
    return {
      name: match[1],
      body: content.slice(start, end)
    };
  });
}

function read(relativePath) {
  const absolutePath = path.join(root, relativePath);
  if (!fs.existsSync(absolutePath)) {
    errors.push(`missing file: ${relativePath}`);
    return '';
  }
  return fs.readFileSync(absolutePath, 'utf8');
}
