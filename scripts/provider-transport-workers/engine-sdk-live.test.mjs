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
  VERIFIED_PROVIDER_SESSION_ID,
} from './engine-sdk-live.mjs';

const workerDir = path.dirname(fileURLToPath(import.meta.url));
const birdcoderRoot = path.resolve(workerDir, '../../../sdkwork-birdcoder');
const kernelRoot = path.resolve(workerDir, '../..');
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-engine-sdk-live-'));
const opencodeSdkMirror = path.join(tempRoot, 'opencode-sdk');
const claudeSdkMirror = path.join(tempRoot, 'claude-sdk');
const geminiSdkMirror = path.join(tempRoot, 'gemini-sdk');
const invalidSdkMirror = path.join(tempRoot, 'invalid-sdk');
const codexSdkMirror = path.join(tempRoot, 'codex-sdk');
const openaiSdkMirror = path.join(tempRoot, 'openai-sdk');
const claudeCapturePath = path.join(tempRoot, 'claude-sdk-capture.json');
const codexCapturePath = path.join(tempRoot, 'codex-sdk-capture.json');
const geminiCapturePath = path.join(tempRoot, 'gemini-sdk-capture.json');
const openaiCapturePath = path.join(tempRoot, 'openai-sdk-capture.json');
const opencodeCapturePath = path.join(tempRoot, 'opencode-sdk-capture.json');
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
fs.mkdirSync(claudeSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(claudeSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@anthropic-ai/claude-agent-sdk',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(
  path.join(claudeSdkMirror, 'index.js'),
  `import fs from 'node:fs';

const capturePath = ${JSON.stringify(claudeCapturePath)};

function capture(prompt, options) {
  fs.writeFileSync(capturePath, JSON.stringify({
    prompt,
    options: {
      cwd: options.cwd,
      has_abort_controller: Boolean(options.abortController),
      model: options.model,
      permission_mode: options.permissionMode,
      allow_dangerously_skip_permissions: options.allowDangerouslySkipPermissions,
      resume: options.resume,
    },
  }), 'utf8');
}

export function query({ prompt, options = {} }) {
  capture(prompt, options);
  const sessionId = prompt === 'mismatched Claude session'
    ? 'claude-sdk-unexpected'
    : options.resume ?? 'claude-sdk-created';
  return (async function* () {
    yield { type: 'system', subtype: 'init', session_id: sessionId };
    yield { type: 'permission_request', session_id: sessionId };
    yield {
      type: 'assistant',
      session_id: sessionId,
      message: { content: [{ type: 'text', text: 'assistant:' + prompt }] },
    };
    yield {
      type: 'result',
      subtype: 'success',
      is_error: false,
      result: 'claude sdk:' + prompt,
      session_id: sessionId,
    };
  })();
}
`,
  'utf8',
);
fs.mkdirSync(geminiSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(geminiSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@google/gemini-cli-sdk',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(
  path.join(geminiSdkMirror, 'index.js'),
  `import fs from 'node:fs';

const capturePath = ${JSON.stringify(geminiCapturePath)};

function capture(record) {
  fs.writeFileSync(capturePath, JSON.stringify(record), 'utf8');
}

class FakeSession {
  constructor(id, record) {
    this.id = id;
    this.record = record;
  }

  async *sendStream(prompt, signal) {
    this.record.send_stream = { prompt, signal_present: Boolean(signal) };
    capture(this.record);
    yield { type: 'content', value: 'gemini sdk:' + prompt };
    yield { type: 'finished', value: { reason: 'stop' } };
  }
}

export class GeminiCliAgent {
  constructor(options) {
    this.record = { constructor_options: options };
  }

  session(options) {
    this.record.session_options = options ?? null;
    capture(this.record);
    return new FakeSession('gemini-sdk-created', this.record);
  }

  async resumeSession(id) {
    this.record.resume_session_id = id;
    capture(this.record);
    return new FakeSession(id, this.record);
  }
}
`,
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
fs.writeFileSync(
  path.join(opencodeSdkMirror, 'index.js'),
  `import fs from 'node:fs';

const capturePath = ${JSON.stringify(opencodeCapturePath)};

function capture(record) {
  fs.writeFileSync(capturePath, JSON.stringify(record), 'utf8');
}

export async function createOpencodeServer(options = {}) {
  return {
    url: 'http://127.0.0.1:4096',
    async close() {},
  };
}

export function createOpencodeClient(options = {}) {
  const record = {
    client_options: {
      base_url: options.baseUrl,
      directory: options.directory,
    },
  };
  capture(record);
  return {
    session: {
      create: async ({ body, signal } = {}) => {
        record.session_create = { body, signal_present: Boolean(signal) };
        capture(record);
        return { data: { id: 'opencode-sdk-created' } };
      },
      get: async ({ path, signal } = {}) => {
        record.session_get = { path, signal_present: Boolean(signal) };
        capture(record);
        return {
          data: {
            id: path.id === 'opencode-sdk-mismatch-request'
              ? 'opencode-sdk-different'
              : path.id,
          },
        };
      },
      update: async ({ path, body, signal } = {}) => {
        record.session_update = { path, body, signal_present: Boolean(signal) };
        capture(record);
        return { data: { id: path.id } };
      },
      prompt: async ({ path, body, signal } = {}) => {
        record.session_prompt = { path, body, signal_present: Boolean(signal) };
        capture(record);
        return {
          data: {
            parts: [{ type: 'text', text: 'opencode sdk:' + body.parts[0].text }],
          },
        };
      },
    },
  };
}
`,
  'utf8',
);
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
    const thread = this;
    capture(this.record);
    return {
      events: (async function* () {
        if (!thread.id) {
          thread.id = 'thread-sdk-streamed';
        }
        yield { type: 'thread.started', thread_id: thread.id };
        yield { type: 'item.updated', item: { id: 'message-1', type: 'agent_message', text: 'official' } };
        if (prompt === 'stream emits fatal error') {
          yield { type: 'error', message: 'stream transport failed' };
          return;
        }
        if (prompt === 'stream ends incomplete') {
          return;
        }
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
  '@anthropic-ai/claude-agent-sdk': claudeSdkMirror,
});
const claudeActivity = [];
const claudeResult = await invokeModelChatLive(
  '@anthropic-ai/claude-agent-sdk',
  {
    model_request_id: 'req-claude-sdk-new',
    model_id: 'claude-sonnet-4-6',
    working_directory: 'C:/sdkwork/claude-workspace',
    timeout_ms: 2_000,
    messages: ['Claude prompt'],
    execution_options: { approval_policy: 'accept-edits' },
  },
  {
    onActivity: async (event) => claudeActivity.push(event),
  },
);
assert.equal(claudeResult.provider_session_id, 'claude-sdk-created');
assert.equal(claudeResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.deepEqual(claudeResult.messages, ['claude sdk:Claude prompt']);
assert.deepEqual(
  claudeActivity.map((event) => event.phase),
  ['started', 'working', 'waiting', 'working', 'idle', 'terminal'],
);
assert.equal(claudeActivity[2].interaction_hint, 'approval_required');
const claudeCapture = JSON.parse(fs.readFileSync(claudeCapturePath, 'utf8'));
assert.deepEqual(claudeCapture, {
  prompt: 'Claude prompt',
  options: {
    cwd: 'C:/sdkwork/claude-workspace',
      has_abort_controller: true,
      model: 'claude-sonnet-4-6',
      permission_mode: 'acceptEdits',
    },
  });
const resumedClaudeResult = await invokeModelChatLive('@anthropic-ai/claude-agent-sdk', {
  model_request_id: 'req-claude-sdk-resume',
  session_id: 'claude-sdk-existing',
  messages: ['Resume Claude'],
});
assert.equal(resumedClaudeResult.provider_session_id, 'claude-sdk-existing');
assert.equal(resumedClaudeResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.equal(
  JSON.parse(fs.readFileSync(claudeCapturePath, 'utf8')).options.resume,
  'claude-sdk-existing',
  'Claude resume must use the official query resume option.',
);
await invokeModelChatLive('@anthropic-ai/claude-agent-sdk', {
  model_request_id: 'req-claude-sdk-bypass',
  messages: ['Bypass Claude permissions'],
  execution_options: { approval_policy: 'bypass-permissions' },
});
const bypassClaudeCapture = JSON.parse(fs.readFileSync(claudeCapturePath, 'utf8'));
assert.equal(bypassClaudeCapture.options.permission_mode, 'bypassPermissions');
assert.equal(bypassClaudeCapture.options.allow_dangerously_skip_permissions, true);
const mismatchedClaudeActivity = [];
await assert.rejects(
  invokeModelChatLive(
    '@anthropic-ai/claude-agent-sdk',
    {
      model_request_id: 'req-claude-sdk-mismatch',
      session_id: 'claude-sdk-existing',
      messages: ['mismatched Claude session'],
    },
    { onActivity: async (event) => mismatchedClaudeActivity.push(event) },
  ),
  /resumed a different provider session/,
);
assert.deepEqual(
  mismatchedClaudeActivity,
  [],
  'an unverified request session id must not be published as provider activity',
);

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({
  '@google/gemini-cli-sdk': geminiSdkMirror,
});
const geminiResult = await invokeModelChatLive('@google/gemini-cli-sdk', {
  model_request_id: 'req-gemini-sdk-new',
  model_id: 'gemini-2.5-pro',
  working_directory: 'C:/sdkwork/gemini-workspace',
  timeout_ms: 2_000,
  messages: ['Gemini prompt'],
});
assert.equal(geminiResult.provider_session_id, 'gemini-sdk-created');
assert.equal(geminiResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.deepEqual(geminiResult.messages, ['gemini sdk:Gemini prompt']);
const geminiCapture = JSON.parse(fs.readFileSync(geminiCapturePath, 'utf8'));
assert.deepEqual(geminiCapture.constructor_options, {
  instructions: '',
  cwd: 'C:/sdkwork/gemini-workspace',
  model: 'gemini-2.5-pro',
});
assert.equal(geminiCapture.session_options, null);
assert.deepEqual(geminiCapture.send_stream, {
  prompt: 'Gemini prompt',
  signal_present: true,
});
const resumedGeminiResult = await invokeModelChatLive('@google/gemini-cli-sdk', {
  model_request_id: 'req-gemini-sdk-resume',
  session_id: 'gemini-sdk-existing',
  messages: ['Resume Gemini'],
});
assert.equal(resumedGeminiResult.provider_session_id, 'gemini-sdk-existing');
assert.equal(resumedGeminiResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.equal(
  JSON.parse(fs.readFileSync(geminiCapturePath, 'utf8')).resume_session_id,
  'gemini-sdk-existing',
  'Gemini resume must use the official agent.resumeSession API.',
);

process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({
  '@opencode-ai/sdk': opencodeSdkMirror,
});
const opencodeResult = await invokeModelChatLive('@opencode-ai/sdk', {
  model_request_id: 'req-opencode-sdk-new',
  model_id: 'opencode/big-pickle',
  working_directory: 'C:/sdkwork/opencode-workspace',
  timeout_ms: 2_000,
  messages: ['OpenCode prompt'],
  execution_options: { approval_policy: 'allow-edits' },
});
assert.equal(opencodeResult.provider_session_id, 'opencode-sdk-created');
assert.equal(opencodeResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.deepEqual(opencodeResult.messages, ['opencode sdk:OpenCode prompt']);
const opencodeCapture = JSON.parse(fs.readFileSync(opencodeCapturePath, 'utf8'));
assert.deepEqual(opencodeCapture.client_options, {
  base_url: 'http://127.0.0.1:4096',
  directory: 'C:/sdkwork/opencode-workspace',
});
assert.deepEqual(opencodeCapture.session_create, {
  body: {
    permission: [
      { permission: '*', pattern: '*', action: 'ask' },
      { permission: 'read', pattern: '*', action: 'allow' },
      { permission: 'edit', pattern: '*', action: 'allow' },
      { permission: 'glob', pattern: '*', action: 'allow' },
      { permission: 'grep', pattern: '*', action: 'allow' },
      { permission: 'list', pattern: '*', action: 'allow' },
    ],
  },
  signal_present: true,
});
assert.deepEqual(opencodeCapture.session_prompt, {
  path: { id: 'opencode-sdk-created' },
  body: {
    parts: [{ type: 'text', text: 'OpenCode prompt' }],
    model: { providerID: 'opencode', modelID: 'big-pickle' },
  },
  signal_present: true,
});
const resumedOpencodeActivity = [];
const resumedOpencodeResult = await invokeModelChatLive(
  '@opencode-ai/sdk',
  {
    model_request_id: 'req-opencode-sdk-resume',
    session_id: 'opencode-sdk-existing',
    messages: ['Resume OpenCode'],
    execution_options: { approval_policy: 'allow-all' },
  },
  { onActivity: async (event) => resumedOpencodeActivity.push(event) },
);
assert.equal(resumedOpencodeResult.provider_session_id, 'opencode-sdk-existing');
assert.equal(resumedOpencodeResult[VERIFIED_PROVIDER_SESSION_ID], true);
assert.deepEqual(
  resumedOpencodeActivity.map((event) => event.phase),
  ['started', 'working', 'idle', 'terminal'],
);
const resumedOpencodeCapture = JSON.parse(fs.readFileSync(opencodeCapturePath, 'utf8'));
assert.equal(
  Object.hasOwn(resumedOpencodeCapture, 'session_create'),
  false,
  'OpenCode resume must not create another provider session.',
);
assert.deepEqual(resumedOpencodeCapture.session_get, {
  path: { id: 'opencode-sdk-existing' },
  signal_present: true,
});
assert.deepEqual(resumedOpencodeCapture.session_update, {
  path: { id: 'opencode-sdk-existing' },
  body: { permission: [{ permission: '*', pattern: '*', action: 'allow' }] },
  signal_present: true,
});
assert.equal(resumedOpencodeCapture.session_prompt.path.id, 'opencode-sdk-existing');

const mismatchedOpencodeActivity = [];
await assert.rejects(
  invokeModelChatLive(
    '@opencode-ai/sdk',
    {
      model_request_id: 'req-opencode-sdk-mismatch',
      session_id: 'opencode-sdk-mismatch-request',
      messages: ['Do not invoke the mismatched session'],
    },
    { onActivity: async (event) => mismatchedOpencodeActivity.push(event) },
  ),
  /resumed a different provider session/,
);
assert.deepEqual(mismatchedOpencodeActivity, []);

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
    approvals_reviewer: 'auto_review',
    full_auto: false,
    skip_git_repo_check: true,
  },
};
const codexResult = await invokeModelChatLive('@openai/codex-sdk', codexOperation);
assert.equal(codexResult.ok, true);
assert.equal(codexResult.mode, 'sdk_live');
assert.equal(codexResult.provider_session_id, 'thread-sdk-existing');
assert.deepEqual(codexResult.messages, ['official sdk:official sdk prompt']);
const codexCapture = JSON.parse(fs.readFileSync(codexCapturePath, 'utf8'));
assert.deepEqual(codexCapture.constructor_options, {
  config: { approvals_reviewer: 'auto_review' },
});
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
assert.equal(codexStreamResult.provider_session_id, 'thread-sdk-streamed');
assert.deepEqual(codexStreamResult.chunks, [
  { sequence: 0, content: 'official' },
  { sequence: 1, content: ' sdk' },
  { sequence: 2, content: ' stream' },
]);

const deliveredCodexChunks = [];
const callbackCodexStreamResult = await invokeModelChatStreamLive(
  '@openai/codex-sdk',
  {
    model_request_id: 'req-codex-sdk-stream-callback',
    messages: ['stream callback prompt'],
    timeout_ms: 2_000,
  },
  {
    onChunk: async (chunk) => {
      deliveredCodexChunks.push(chunk);
    },
  },
);
assert.deepEqual(deliveredCodexChunks, [
  { sequence: 0, content: 'official' },
  { sequence: 1, content: ' sdk' },
  { sequence: 2, content: ' stream' },
]);
assert.deepEqual(
  callbackCodexStreamResult.chunks,
  [],
  'callback delivery must not retain every Codex chunk in the worker result',
);
assert.equal(callbackCodexStreamResult.provider_session_id, 'thread-sdk-streamed');

const failedCodexChunks = [];
await assert.rejects(
  invokeModelChatStreamLive(
    '@openai/codex-sdk',
    {
      model_request_id: 'req-codex-sdk-stream-error',
      messages: ['stream emits fatal error'],
      timeout_ms: 2_000,
    },
    {
      onChunk: async (chunk) => {
        failedCodexChunks.push(chunk);
      },
    },
  ),
  /stream transport failed/,
);
assert.deepEqual(failedCodexChunks, [{ sequence: 0, content: 'official' }]);
await assert.rejects(
  invokeModelChatStreamLive('@openai/codex-sdk', {
    model_request_id: 'req-codex-sdk-stream-incomplete',
    messages: ['stream ends incomplete'],
    timeout_ms: 2_000,
  }),
  /missing turn\.completed event/,
);

const newThreadResult = await invokeModelChatLive('@openai/codex-sdk', {
  model_request_id: 'req-codex-sdk-new',
  messages: ['new thread'],
  execution_options: { full_auto: true },
});
assert.equal(newThreadResult.provider_session_id, 'thread-sdk-started');
const newThreadCapture = JSON.parse(fs.readFileSync(codexCapturePath, 'utf8'));
assert.deepEqual(newThreadCapture.start_thread_options, {
  sandboxMode: 'workspace-write',
  approvalPolicy: 'on-failure',
});
await invokeModelChatLive('@openai/codex-sdk', {
  model_request_id: 'req-codex-sdk-full-access',
  messages: ['allow configured full access'],
  execution_options: {
    sandbox_mode: 'danger-full-access',
    approval_policy: 'never',
  },
});
const fullAccessCapture = JSON.parse(fs.readFileSync(codexCapturePath, 'utf8'));
assert.equal(fullAccessCapture.start_thread_options.sandboxMode, 'danger-full-access');
assert.equal(fullAccessCapture.start_thread_options.approvalPolicy, 'never');

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
