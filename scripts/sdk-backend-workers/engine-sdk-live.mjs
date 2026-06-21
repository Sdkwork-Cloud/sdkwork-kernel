#!/usr/bin/env node
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const workerDir = path.dirname(fileURLToPath(import.meta.url));
const kernelRoot = path.resolve(workerDir, '../..');

const PROFILE_ENV = 'SDKWORK_KERNEL_PROFILE_ID';
const ALLOW_MOCK_ENV = 'SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS';
const LEGACY_ALLOW_MOCK_ENV = 'SDKWORK_KERNEL_ALLOW_MOCK_FALLBACK';
const PACKAGE_PATHS_ENV = 'SDKWORK_AGENT_SDK_PACKAGE_PATHS';
const WORKSPACE_ROOT_ENV = 'SDKWORK_AGENT_SDK_WORKSPACE_ROOT';

const PRODUCTION_PROFILES = new Set(['prod', 'production', 'release']);

export function mockProviderInvocationAllowed() {
  const profile = (process.env[PROFILE_ENV] ?? '').trim().toLowerCase();
  if (PRODUCTION_PROFILES.has(profile)) {
    return explicitMockOverrideEnabled();
  }
  return !explicitMockOverrideDisabled() || explicitMockOverrideEnabled();
}

function explicitMockOverrideEnabled() {
  return [ALLOW_MOCK_ENV, LEGACY_ALLOW_MOCK_ENV]
    .map((key) => process.env[key])
    .filter(Boolean)
    .some((value) => matchesAllowTruthy(value));
}

function explicitMockOverrideDisabled() {
  return [ALLOW_MOCK_ENV, LEGACY_ALLOW_MOCK_ENV]
    .map((key) => process.env[key])
    .filter(Boolean)
    .some((value) => matchesDenyFalsy(value));
}

function matchesAllowTruthy(value) {
  return ['1', 'true', 'yes', 'on'].includes(value.trim().toLowerCase());
}

function matchesDenyFalsy(value) {
  return ['0', 'false', 'no', 'off'].includes(value.trim().toLowerCase());
}

function workspaceRoot() {
  const configured = process.env[WORKSPACE_ROOT_ENV]?.trim();
  if (configured && existsSync(configured)) {
    return path.resolve(configured);
  }

  const birdcoderSibling = path.resolve(kernelRoot, '../sdkwork-birdcoder');
  if (existsSync(birdcoderSibling)) {
    return birdcoderSibling;
  }

  return null;
}

function defaultPackagePaths(root) {
  return {
    '@openai/codex-sdk': path.join(root, 'external/codex/sdk/typescript'),
    '@openai/codex': path.join(root, 'external/codex/sdk/typescript'),
    '@opencode-ai/sdk': path.join(root, 'external/opencode/packages/sdk/js'),
    '@google/gemini-cli-sdk': path.join(root, 'external/gemini/packages/sdk'),
    '@anthropic-ai/claude-agent-sdk': path.join(root, 'external/claude-code'),
  };
}

function configuredPackagePaths() {
  const raw = process.env[PACKAGE_PATHS_ENV]?.trim();
  if (!raw) {
    return {};
  }

  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

export function resolvePackageSpecifier(packageName) {
  const require = createRequire(import.meta.url);
  try {
    return require.resolve(packageName);
  } catch {
    const paths = {
      ...defaultPackagePaths(workspaceRoot() ?? ''),
      ...configuredPackagePaths(),
    };
    const localPath = paths[packageName];
    if (localPath && existsSync(localPath)) {
      const entry = path.join(localPath, 'package.json');
      if (existsSync(entry)) {
        return pathToFileURL(entry).href;
      }
      return pathToFileURL(localPath).href;
    }
    return null;
  }
}

export function probePackage(packageName) {
  return {
    resolved: Boolean(resolvePackageSpecifier(packageName)),
  };
}

async function loadPackage(packageName) {
  const specifier = resolvePackageSpecifier(packageName);
  if (!specifier) {
    throw new Error(`package not resolved: ${packageName}`);
  }
  return import(specifier);
}

function liveSuccess(messages, operation, extra = {}) {
  const normalized = Array.isArray(messages)
    ? messages.map((entry) => String(entry))
    : [String(messages ?? '')];
  return {
    ok: true,
    mode: 'sdk_live',
    messages: normalized,
    finish_reason: 'stop',
    model_request_id: operation.model_request_id ?? null,
    ...extra,
  };
}

function extractTextParts(parts) {
  if (!Array.isArray(parts)) {
    return '';
  }

  return parts
    .map((part) => {
      if (!part || typeof part !== 'object') {
        return '';
      }
      if (part.type === 'text' && typeof part.text === 'string') {
        return part.text;
      }
      if (typeof part.content === 'string') {
        return part.content;
      }
      return '';
    })
    .join('')
    .trim();
}

async function invokeCodexModelChat(prompt, operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);
  const Codex = moduleNamespace.Codex;
  if (typeof Codex !== 'function') {
    throw new Error('Codex class is unavailable in @openai/codex-sdk');
  }

  const codex = new Codex({ cwd: process.cwd() });
  const thread = codex.startThread();
  const turn = await thread.run(prompt);
  const text = turn?.finalResponse ?? turn?.items?.map((item) => item?.text ?? '').join('\n') ?? '';
  return liveSuccess(text, operation, { package: packageName });
}

async function invokeClaudeModelChat(prompt, operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);

  if (typeof moduleNamespace.unstable_v2_prompt === 'function') {
    const result = await moduleNamespace.unstable_v2_prompt(prompt, {
      cwd: process.cwd(),
    });
    return liveSuccess(String(result ?? ''), operation, { package: packageName });
  }

  if (typeof moduleNamespace.query === 'function') {
    let text = '';
    for await (const event of moduleNamespace.query({
      prompt,
      options: { cwd: process.cwd() },
    })) {
      const content = event?.message?.content ?? event?.content;
      if (typeof content === 'string') {
        text += content;
      } else if (Array.isArray(content)) {
        text += extractTextParts(content);
      }
    }
    return liveSuccess(text, operation, { package: packageName });
  }

  throw new Error('claude agent sdk missing query entrypoints');
}

async function invokeGeminiModelChat(prompt, operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);
  const Agent = moduleNamespace.GeminiCliAgent;
  if (typeof Agent !== 'function') {
    throw new Error('GeminiCliAgent is unavailable in @google/gemini-cli-sdk');
  }

  const agent = new Agent({ cwd: process.cwd() });
  const session = agent.session();
  let text = '';

  if (typeof session.sendStream === 'function') {
    for await (const event of session.sendStream(prompt)) {
      const message = event?.message ?? event;
      if (message?.role === 'assistant') {
        if (typeof message.content === 'string') {
          text += message.content;
        } else if (Array.isArray(message.content)) {
          text += extractTextParts(message.content);
        }
      } else if (typeof event?.text === 'string') {
        text += event.text;
      }
    }
  } else if (typeof session.send === 'function') {
    const response = await session.send(prompt);
    text = typeof response === 'string' ? response : String(response?.text ?? '');
  } else {
    throw new Error('gemini cli sdk session missing send/sendStream');
  }

  return liveSuccess(text, operation, { package: packageName });
}

async function invokeOpencodeModelChat(prompt, operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);
  const createOpencode = moduleNamespace.createOpencode;
  const createOpencodeClient = moduleNamespace.createOpencodeClient;

  if (typeof createOpencode === 'function') {
    const { client, server } = await createOpencode();
    try {
      const created = await client.session.create({ body: {} });
      const sessionId = created?.data?.id ?? created?.id;
      if (!sessionId) {
        throw new Error('opencode session.create did not return a session id');
      }

      const response = await client.session.prompt({
        path: { id: sessionId },
        body: {
          parts: [{ type: 'text', text: prompt }],
        },
      });
      const text =
        extractTextParts(response?.data?.parts) ||
        extractTextParts(response?.parts) ||
        String(response?.data?.content ?? response?.content ?? '');
      return liveSuccess(text, operation, { package: packageName, session_id: sessionId });
    } finally {
      await server?.close?.();
    }
  }

  if (typeof createOpencodeClient === 'function') {
    const baseUrl = process.env.OPENCODE_SERVER_URL?.trim();
    if (!baseUrl) {
      throw new Error('OPENCODE_SERVER_URL is required for createOpencodeClient live invoke');
    }
    const client = createOpencodeClient({ baseUrl });
    const created = await client.session.create({ body: {} });
    const sessionId = created?.data?.id ?? created?.id;
    const response = await client.session.prompt({
      path: { id: sessionId },
      body: {
        parts: [{ type: 'text', text: prompt }],
      },
    });
    const text =
      extractTextParts(response?.data?.parts) ||
      extractTextParts(response?.parts) ||
      String(response?.data?.content ?? response?.content ?? '');
    return liveSuccess(text, operation, { package: packageName, session_id: sessionId });
  }

  throw new Error('opencode sdk missing createOpencode/createOpencodeClient');
}

const LIVE_MODEL_CHAT_HANDLERS = {
  '@openai/codex-sdk': invokeCodexModelChat,
  '@openai/codex': invokeCodexModelChat,
  '@anthropic-ai/claude-agent-sdk': invokeClaudeModelChat,
  '@google/gemini-cli-sdk': invokeGeminiModelChat,
  '@opencode-ai/sdk': invokeOpencodeModelChat,
};

export async function invokeModelChatLive(packageName, operation) {
  const handler = LIVE_MODEL_CHAT_HANDLERS[packageName];
  if (!handler) {
    throw new Error(`no live model_chat handler for package ${packageName}`);
  }

  const prompt = (operation.messages ?? []).join('\n');
  return handler(prompt, operation, packageName);
}

export function buildStubModelChatResult(packageName, operation, packageProbe) {
  const prompt = (operation.messages ?? []).join('\n');
  const prefix = packageProbe.resolved ? `[${packageName}]` : `[${packageName} stub]`;
  return {
    ok: true,
    mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
    messages: [`${prefix} ${prompt}`],
    finish_reason: 'stop',
    package: packageName,
    model_request_id: operation.model_request_id ?? null,
  };
}
