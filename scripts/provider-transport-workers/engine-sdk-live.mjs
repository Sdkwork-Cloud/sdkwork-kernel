#!/usr/bin/env node
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const workerDir = path.dirname(fileURLToPath(import.meta.url));
const kernelRoot = path.resolve(workerDir, '../..');

const PROFILE_ENV = 'SDKWORK_KERNEL_PROFILE_ID';
const ENVIRONMENT_ENV = 'SDKWORK_KERNEL_ENVIRONMENT';
const ALLOW_MOCK_ENV = 'SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS';
const PACKAGE_PATHS_ENV = 'SDKWORK_AGENT_SDK_PACKAGE_PATHS';
const WORKSPACE_ROOT_ENV = 'SDKWORK_AGENT_SDK_WORKSPACE_ROOT';

export function isProductionKernelProfile() {
  const environment = (process.env[ENVIRONMENT_ENV] ?? '').trim().toLowerCase();
  if (environment === 'production' || environment === 'prod') {
    return true;
  }
  const profile = (process.env[PROFILE_ENV] ?? '').trim().toLowerCase();
  return profile.endsWith('.production');
}

export function mockProviderInvocationAllowed() {
  if (isProductionKernelProfile()) {
    return explicitMockOverrideEnabled();
  }
  return !explicitMockOverrideDisabled() || explicitMockOverrideEnabled();
}

function explicitMockOverrideEnabled() {
  const value = process.env[ALLOW_MOCK_ENV];
  return value ? matchesAllowTruthy(value) : false;
}

function explicitMockOverrideDisabled() {
  const value = process.env[ALLOW_MOCK_ENV];
  return value ? matchesDenyFalsy(value) : false;
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

  if (existsSync(path.join(kernelRoot, 'external'))) {
    return kernelRoot;
  }

  return null;
}

function defaultPackagePaths(root) {
  const paths = {
    '@openai/codex-sdk': path.join(root, 'external/codex/sdk/typescript'),
    '@openai/codex': path.join(root, 'external/codex/sdk/typescript'),
    '@opencode-ai/sdk': path.join(root, 'external/opencode/packages/sdk/js'),
    '@google/gemini-cli-sdk': path.join(root, 'external/gemini-cli/packages/sdk'),
    '@anthropic-ai/claude-agent-sdk': path.join(root, 'external/claude-code'),
    openclaw: path.join(root, 'external/openclaw'),
  };

  const kernelOpenClaw = path.join(kernelRoot, 'external/openclaw');
  if (existsSync(path.join(kernelOpenClaw, 'package.json'))) {
    paths.openclaw = kernelOpenClaw;
  }

  return paths;
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

export function resolveModelChatPrompt(operation) {
  const wire = operation.wire_messages;
  if (Array.isArray(wire) && wire.length > 0) {
    const lastUser = [...wire].reverse().find((entry) => entry?.role === 'user') ?? wire.at(-1);
    const content = lastUser?.content;
    if (typeof content === 'string') {
      return content;
    }
    if (Array.isArray(content)) {
      return extractTextParts(content);
    }
  }
  return (operation.messages ?? []).join('\n');
}

export function resolveOpencodePromptParts(operation) {
  const wire = operation.wire_messages;
  if (!Array.isArray(wire) || wire.length === 0) {
    return [{ type: 'text', text: resolveModelChatPrompt(operation) }];
  }

  const lastUser = [...wire].reverse().find((entry) => entry?.role === 'user') ?? wire.at(-1);
  const content = lastUser?.content;
  if (!Array.isArray(content)) {
    return [{ type: 'text', text: resolveModelChatPrompt(operation) }];
  }

  return content
    .map((part) => {
      if (!part || typeof part !== 'object') {
        return null;
      }
      if (part.type === 'text' && typeof part.text === 'string') {
        return { type: 'text', text: part.text };
      }
      if (part.type === 'image_url' && part.image_url?.url) {
        return { type: 'image', url: part.image_url.url };
      }
      return { type: 'text', text: JSON.stringify(part) };
    })
    .filter(Boolean);
}

export function resolveOpenClawWireMessages(operation) {
  const wire = operation.wire_messages;
  if (Array.isArray(wire) && wire.length > 0) {
    return wire.map((entry) => ({
      role: entry?.role ?? 'user',
      content: entry?.content ?? '',
    }));
  }
  return (operation.messages ?? [resolveModelChatPrompt(operation)]).map((entry) => ({
    role: 'user',
    content: String(entry ?? ''),
  }));
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
          parts: resolveOpencodePromptParts(operation),
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
        parts: resolveOpencodePromptParts(operation),
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

async function invokeOpenClawModelChat(prompt, operation, packageName) {
  const gatewayUrl =
    process.env.OPENCLAW_GATEWAY_URL?.trim() || process.env.OPENCLAW_HTTP_URL?.trim();
  if (gatewayUrl) {
    const base = gatewayUrl.replace(/\/$/, '');
    const url = `${base}/v1/chat/completions`;
    const headers = { 'Content-Type': 'application/json' };
    const token = process.env.OPENCLAW_GATEWAY_TOKEN?.trim();
    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    const messages = resolveOpenClawWireMessages(operation);
    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        model: operation.model_id ?? 'default',
        messages,
      }),
    });
    if (!response.ok) {
      throw new Error(`openclaw gateway chat failed: HTTP ${response.status}`);
    }
    const payload = await response.json();
    const text =
      payload?.choices?.[0]?.message?.content ??
      payload?.choices?.[0]?.text ??
      payload?.message?.content ??
      '';
    return liveSuccess(text, operation, {
      package: packageName,
      gateway_url: base,
    });
  }

  throw new Error(
    'openclaw live invoke requires OPENCLAW_GATEWAY_URL or a gateway-compatible HTTP endpoint'
  );
}

const LIVE_MODEL_CHAT_HANDLERS = {
  '@openai/codex-sdk': invokeCodexModelChat,
  '@openai/codex': invokeCodexModelChat,
  '@anthropic-ai/claude-agent-sdk': invokeClaudeModelChat,
  '@google/gemini-cli-sdk': invokeGeminiModelChat,
  '@opencode-ai/sdk': invokeOpencodeModelChat,
  openclaw: invokeOpenClawModelChat,
};

export async function invokeModelChatLive(packageName, operation) {
  const handler = LIVE_MODEL_CHAT_HANDLERS[packageName];
  if (!handler) {
    throw new Error(`no live model_chat handler for package ${packageName}`);
  }

  const prompt = resolveModelChatPrompt(operation);
  return handler(prompt, operation, packageName);
}

export function buildModelChatStreamResult(baseResult) {
  const text = Array.isArray(baseResult.messages)
    ? baseResult.messages.map((entry) => String(entry ?? '')).join('\n')
    : String(baseResult.messages ?? '');
  return {
    ...baseResult,
    chunks: [{ sequence: 0, content: text }],
  };
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
