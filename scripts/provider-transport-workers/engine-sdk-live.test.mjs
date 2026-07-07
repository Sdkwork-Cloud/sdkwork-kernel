import assert from 'node:assert/strict';
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

process.env.SDKWORK_AGENT_SDK_WORKSPACE_ROOT = birdcoderRoot;
delete process.env.SDKWORK_KERNEL_PROFILE_ID;
delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;

assert.equal(mockProviderInvocationAllowed(), true, 'dev profile should allow mock fallback');

const codexPath = resolvePackageSpecifier('@openai/codex-sdk');
assert.ok(codexPath, 'codex sdk mirror should resolve from birdcoder workspace');

const opencodePath = resolvePackageSpecifier('@opencode-ai/sdk');
assert.ok(opencodePath, 'opencode sdk mirror should resolve from birdcoder workspace');

const stub = buildStubModelChatResult(
  '@openai/codex-sdk',
  { model_request_id: 'req-1', messages: ['hello'] },
  probePackage('@openai/codex-sdk'),
);
assert.equal(stub.mode, 'sdk_probe');

const wirePrompt = resolveModelChatPrompt({
  messages: ['legacy'],
  wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'structured' }] }],
});
assert.equal(wirePrompt, 'structured', 'wire_messages should drive live prompt resolution');

process.env.SDKWORK_KERNEL_PROFILE_ID = 'cloud.split-services.production';
process.env.SDKWORK_KERNEL_ENVIRONMENT = 'production';
delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;
assert.equal(mockProviderInvocationAllowed(), false, 'production topology profile should block mock fallback');

console.log('engine-sdk-live contract passed.');
