import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  buildStubModelChatResult,
  mockProviderInvocationAllowed,
  probePackage,
  resolveModelChatPrompt,
  resolvePackageSpecifier,
} from './engine-sdk-live.mjs';

const workerDir = path.dirname(fileURLToPath(import.meta.url));
const birdcoderRoot = path.resolve(workerDir, '../../../sdkwork-birdcoder');
const kernelRoot = path.resolve(workerDir, '../..');
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-engine-sdk-live-'));
const opencodeSdkMirror = path.join(tempRoot, 'opencode-sdk');
const invalidSdkMirror = path.join(tempRoot, 'invalid-sdk');
fs.mkdirSync(opencodeSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(opencodeSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@opencode-ai/sdk',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(path.join(opencodeSdkMirror, 'index.js'), 'export const test = true;\n', 'utf8');
fs.mkdirSync(invalidSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(invalidSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@sdkwork/invalid-sdk',
    version: '0.0.0-test',
    exports: { '.': './missing.js' },
  }),
  'utf8',
);

process.env.SDKWORK_AGENT_SDK_WORKSPACE_ROOT = birdcoderRoot;
delete process.env.SDKWORK_KERNEL_PROFILE_ID;
delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;
delete process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS;

assert.doesNotMatch(
  fs.readFileSync(path.join(workerDir, 'engine-sdk-live.mjs'), 'utf8'),
  /external\/opencode\/packages\/sdk\/js/,
  'OpenCode SDK resolver must not assume external/opencode is a TypeScript SDK workspace',
);

assert.equal(mockProviderInvocationAllowed(), true, 'dev profile should allow mock fallback');

const appTopologyPath = resolvePackageSpecifier('@sdkwork/app-topology');
assert.ok(
  appTopologyPath?.startsWith('file:'),
  'native package resolution should return an importable file URL, not a Windows absolute path',
);
await import(appTopologyPath);

const codexPath = resolvePackageSpecifier('@openai/codex-sdk');
assert.equal(
  codexPath,
  null,
  'unbuilt source mirrors must not be treated as importable live SDK packages',
);

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({
  '@opencode-ai/sdk': opencodeSdkMirror,
});
const opencodePath = resolvePackageSpecifier('@opencode-ai/sdk');
assert.ok(opencodePath, 'opencode sdk should resolve through explicit package path injection');
assert.ok(
  opencodePath.endsWith('/index.js') && opencodePath.includes('opencode-sdk'),
  'opencode resolver should return an importable package entry file',
);

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({
  '@sdkwork/invalid-sdk': invalidSdkMirror,
});
assert.equal(
  resolvePackageSpecifier('@sdkwork/invalid-sdk'),
  null,
  'local package mirrors with missing entry files must not be marked resolved',
);

process.env.SDKWORK_AGENT_SDK_WORKSPACE_ROOT = kernelRoot;
delete process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS;
const geminiPath = resolvePackageSpecifier('@google/gemini-cli-sdk');
assert.equal(
  resolvePackageSpecifier('@anthropic-ai/claude-agent-sdk'),
  null,
  'Claude source-tree mirror is not the official SDK package unless it exposes an importable matching package',
);
assert.equal(
  geminiPath,
  null,
  'missing Gemini SDK package mirror must not be marked resolved',
);

const stub = buildStubModelChatResult(
  '@openai/codex-sdk',
  { model_request_id: 'req-1', messages: ['hello'] },
  probePackage('@openai/codex-sdk'),
);
assert.equal(stub.mode, 'stub');

const wirePrompt = resolveModelChatPrompt({
  messages: ['legacy'],
  wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'structured' }] }],
});
assert.equal(wirePrompt, 'structured', 'wire_messages should drive live prompt resolution');

process.env.SDKWORK_KERNEL_PROFILE_ID = 'cloud.production';
process.env.SDKWORK_KERNEL_ENVIRONMENT = 'production';
delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;
assert.equal(mockProviderInvocationAllowed(), false, 'production topology profile should block mock fallback');

console.log('engine-sdk-live contract passed.');
