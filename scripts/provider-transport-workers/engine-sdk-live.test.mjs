import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  buildStubModelChatResult,
  invokeModelChatLive,
  invokeModelChatStreamLive,
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
const codexSdkMirror = path.join(tempRoot, 'codex-sdk');
const openaiSdkMirror = path.join(tempRoot, 'openai-sdk');
const codexCapturePath = path.join(tempRoot, 'codex-sdk-capture.json');
const openaiCapturePath = path.join(tempRoot, 'openai-sdk-capture.json');
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
fs.mkdirSync(openaiSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(openaiSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: 'openai',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(
  path.join(openaiSdkMirror, 'index.js'),
  `import fs from 'node:fs';
const capturePath = ${JSON.stringify(openaiCapturePath)};
export default class OpenAI {
  constructor(options) { this.options = options; }
  chat = { completions: { create: async (request) => {
    fs.writeFileSync(capturePath, JSON.stringify({ options: this.options, request }), 'utf8');
    return { choices: [{ message: { content: 'openclaw sdk:' + request.messages[0].content } }] };
  } } };
}
`,
  'utf8',
);
fs.writeFileSync(path.join(opencodeSdkMirror, 'index.js'), 'export const test = true;\n', 'utf8');
fs.mkdirSync(codexSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(codexSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@openai/codex-sdk',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(
  path.join(codexSdkMirror, 'index.js'),
  `import fs from 'node:fs';

const capturePath = ${JSON.stringify(codexCapturePath)};

function capture(value) {
  fs.writeFileSync(capturePath, JSON.stringify(value), 'utf8');
}

class FakeThread {
  constructor(id, record) {
    this.id = id;
    this.record = record;
  }

  async run(prompt, turnOptions = {}) {
    this.record.run = {
      prompt,
      signal_present: Boolean(turnOptions.signal),
    };
    if (!this.id) {
      this.id = 'thread-sdk-started';
    }
    capture(this.record);
    return {
      finalResponse: 'official sdk:' + prompt,
      items: [{ type: 'agent_message', text: 'official sdk:' + prompt }],
    };
  }

  async runStreamed(prompt, turnOptions = {}) {
    this.record.stream = {
      prompt,
      signal_present: Boolean(turnOptions.signal),
    };
    if (!this.id) {
      this.id = 'thread-sdk-streamed';
    }
    capture(this.record);
    return {
      events: (async function* () {
        yield { type: 'item.updated', item: { id: 'message-1', type: 'agent_message', text: 'official' } };
        yield { type: 'item.updated', item: { id: 'message-1', type: 'agent_message', text: 'official sdk' } };
        yield { type: 'item.completed', item: { id: 'message-1', type: 'agent_message', text: 'official sdk stream' } };
        yield { type: 'turn.completed', usage: {} };
      })(),
    };
  }
}

export class Codex {
  constructor(options = {}) {
    this.record = { constructor_options: options };
  }

  startThread(options = {}) {
    this.record.start_thread_options = options;
    return new FakeThread(null, this.record);
  }

  resumeThread(id, options = {}) {
    this.record.resume_thread_id = id;
    this.record.resume_thread_options = options;
    return new FakeThread(id, this.record);
  }
}
`,
  'utf8',
);
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

assert.match(
  fs.readFileSync(path.join(workerDir, 'engine-sdk-live.mjs'), 'utf8'),
  /external\/opencode\/packages\/sdk\/js/,
  'OpenCode SDK resolver should include the canonical SDK mirror path',
);
assert.match(
  fs.readFileSync(path.join(workerDir, 'engine-sdk-live.mjs'), 'utf8'),
  /external\/gemini\/packages\/sdk/,
  'Gemini SDK resolver should include the canonical SDK mirror path',
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

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({
  '@openai/codex-sdk': codexSdkMirror,
});
const codexOperation = {
  model_request_id: 'req-codex-sdk',
  model_id: 'gpt-5.4',
  session_id: 'thread-sdk-existing',
  working_directory: 'C:/sdkwork/workspace',
  timeout_ms: 2_000,
  messages: ['legacy prompt'],
  wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'official sdk prompt' }] }],
  execution_options: {
    approval_policy: 'onrequest',
    sandbox_mode: 'workspace_write',
    full_auto: false,
    skip_git_repo_check: true,
  },
};
const codexResult = await invokeModelChatLive('@openai/codex-sdk', codexOperation);
assert.equal(codexResult.ok, true);
assert.equal(codexResult.mode, 'sdk_live');
assert.equal(codexResult.native_session_id, 'thread-sdk-existing');
assert.deepEqual(codexResult.messages, ['official sdk:official sdk prompt']);
const codexCapture = JSON.parse(fs.readFileSync(codexCapturePath, 'utf8'));
assert.deepEqual(codexCapture.constructor_options, {});
assert.equal(codexCapture.resume_thread_id, 'thread-sdk-existing');
assert.deepEqual(codexCapture.resume_thread_options, {
  model: 'gpt-5.4',
  workingDirectory: 'C:/sdkwork/workspace',
  sandboxMode: 'workspace-write',
  approvalPolicy: 'on-request',
  skipGitRepoCheck: true,
});
assert.equal(codexCapture.run.prompt, 'official sdk prompt');
assert.equal(codexCapture.run.signal_present, true);

const codexStreamResult = await invokeModelChatStreamLive('@openai/codex-sdk', {
  model_request_id: 'req-codex-sdk-stream',
  messages: ['stream prompt'],
  timeout_ms: 2_000,
});
assert.equal(codexStreamResult.native_session_id, 'thread-sdk-streamed');
assert.deepEqual(codexStreamResult.chunks, [
  { sequence: 0, content: 'official' },
  { sequence: 1, content: ' sdk' },
  { sequence: 2, content: ' stream' },
]);

const newThreadResult = await invokeModelChatLive('@openai/codex-sdk', {
  model_request_id: 'req-codex-sdk-new',
  messages: ['new thread'],
  execution_options: { full_auto: true },
});
assert.equal(newThreadResult.native_session_id, 'thread-sdk-started');
const newThreadCapture = JSON.parse(fs.readFileSync(codexCapturePath, 'utf8'));
assert.deepEqual(newThreadCapture.start_thread_options, {
  sandboxMode: 'workspace-write',
  approvalPolicy: 'on-failure',
});
await assert.rejects(
  invokeModelChatLive('@openai/codex-sdk', {
    model_request_id: 'req-codex-sdk-dangerous',
    messages: ['reject unsafe sandbox'],
    execution_options: { sandbox_mode: 'danger-full-access' },
  }),
  /danger-full-access is prohibited/,
);

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({ openai: openaiSdkMirror });
process.env.OPENCLAW_GATEWAY_URL = 'http://127.0.0.1:18789';
process.env.OPENCLAW_GATEWAY_TOKEN = 'gateway-test-token';
const openclawResult = await invokeModelChatLive('openai', {
  model_request_id: 'req-openclaw-sdk',
  model_id: 'default',
  messages: ['gateway prompt'],
});
assert.equal(openclawResult.ok, true);
assert.equal(openclawResult.messages[0], 'openclaw sdk:gateway prompt');
const openaiCapture = JSON.parse(fs.readFileSync(openaiCapturePath, 'utf8'));
assert.equal(openaiCapture.options.baseURL, 'http://127.0.0.1:18789/v1');
assert.equal(openaiCapture.options.apiKey, 'gateway-test-token');
assert.equal(openaiCapture.request.model, 'default');
delete process.env.OPENCLAW_GATEWAY_URL;
delete process.env.OPENCLAW_GATEWAY_TOKEN;

process.env.SDKWORK_KERNEL_PROFILE_ID = 'cloud.production';
process.env.SDKWORK_KERNEL_ENVIRONMENT = 'production';
delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;
assert.equal(mockProviderInvocationAllowed(), false, 'production topology profile should block mock fallback');

console.log('engine-sdk-live contract passed.');
