#!/usr/bin/env node
import { createRequire } from 'node:module';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  invokeCodexCliModelChat,
  isCodexPackage,
  probeCodexCli,
} from './codex-cli-live.mjs';
import {
  invokeProviderCliModelChat,
  isProviderCliPackage,
  probeProviderCli,
} from './provider-cli-live.mjs';

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
    '@google/gemini-cli-sdk': [
      path.join(root, 'external/gemini/packages/sdk'),
      path.join(root, 'external/gemini-cli/packages/sdk'),
    ],
    '@anthropic-ai/claude-agent-sdk': path.join(root, 'external/claude-code'),
    '@opencode-ai/sdk': path.join(root, 'external/opencode/packages/sdk/js'),
  };

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

function fileExists(filePath) {
  try {
    return existsSync(filePath) && statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function packageEntryCandidates(packageJson) {
  const candidates = [];
  const rootExport = packageJson.exports?.['.'] ?? packageJson.exports;

  appendExportCandidates(candidates, rootExport);
  for (const field of ['module', 'main']) {
    if (typeof packageJson[field] === 'string') {
      candidates.push(packageJson[field]);
    }
  }
  return candidates;
}

function appendExportCandidates(candidates, value) {
  if (typeof value === 'string') {
    candidates.push(value);
    return;
  }

  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return;
  }

  for (const condition of ['import', 'default', 'node', 'module', 'require']) {
    const next = value[condition];
    if (typeof next === 'string') {
      candidates.push(next);
    } else if (next && typeof next === 'object') {
      appendExportCandidates(candidates, next);
    }
  }
}

function localPackageNameMatches(requestedPackageName, actualPackageName) {
  if (!actualPackageName) {
    return false;
  }
  if (requestedPackageName === actualPackageName) {
    return true;
  }
  return requestedPackageName === '@openai/codex' && actualPackageName === '@openai/codex-sdk';
}

function resolveLocalPackageSpecifier(packageName, localPath) {
  if (Array.isArray(localPath)) {
    for (const candidate of localPath) {
      const resolved = resolveLocalPackageSpecifier(packageName, candidate);
      if (resolved) {
        return resolved;
      }
    }
    return null;
  }
  if (!localPath || !existsSync(localPath)) {
    return null;
  }

  if (fileExists(localPath)) {
    const extension = path.extname(localPath).toLowerCase();
    return ['.js', '.mjs', '.cjs'].includes(extension) ? pathToFileURL(localPath).href : null;
  }

  const packageJsonPath = path.join(localPath, 'package.json');
  if (!fileExists(packageJsonPath)) {
    return null;
  }

  let packageJson;
  try {
    packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
  } catch {
    return null;
  }

  if (!localPackageNameMatches(packageName, packageJson.name)) {
    return null;
  }

  for (const candidate of packageEntryCandidates(packageJson)) {
    if (!candidate || !candidate.startsWith('.')) {
      continue;
    }
    const entryPath = path.resolve(localPath, candidate);
    if (fileExists(entryPath)) {
      return pathToFileURL(entryPath).href;
    }
  }

  return null;
}

export function resolvePackageSpecifier(packageName) {
  const configuredPaths = configuredPackagePaths();
  if (Object.hasOwn(configuredPaths, packageName)) {
    return resolveLocalPackageSpecifier(packageName, configuredPaths[packageName]);
  }

  const require = createRequire(import.meta.url);
  try {
    const resolved = require.resolve(packageName);
    return path.isAbsolute(resolved) ? pathToFileURL(resolved).href : resolved;
  } catch {
    const paths = defaultPackagePaths(workspaceRoot() ?? '');
    const localPath = paths[packageName];
    return resolveLocalPackageSpecifier(packageName, localPath);
  }
}

export function probePackage(packageName) {
  return {
    resolved: Boolean(resolvePackageSpecifier(packageName)),
  };
}

export function probeModelChatRuntime(packageName) {
  const packageProbe = probePackage(packageName);
  const cliProbe = isCodexPackage(packageName)
    ? probeCodexCli()
    : isProviderCliPackage(packageName)
      ? probeProviderCli(packageName)
      : null;
  const cliAvailable = Boolean(cliProbe?.available);
  return {
    ...packageProbe,
    cli_available: cliAvailable,
    runtime_available: packageProbe.resolved || cliAvailable,
    runtime_mode: cliAvailable ? 'sdk_cli' : packageProbe.resolved ? 'sdk_live' : null,
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
  const { sessionId, thread } = await createCodexThread(operation, packageName);
  const turn = await runCodexThread(thread, prompt, operation.timeout_ms);
  const text = turn?.finalResponse ?? turn?.items?.map((item) => item?.text ?? '').join('\n') ?? '';
  return liveSuccess(text, operation, {
    package: packageName,
    native_session_id: thread?.id ?? sessionId,
  });
}

async function createCodexThread(operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);
  const Codex = moduleNamespace.Codex;
  if (typeof Codex !== 'function') {
    throw new Error('Codex class is unavailable in @openai/codex-sdk');
  }

  const codex = new Codex();
  const threadOptions = buildCodexThreadOptions(operation);
  const sessionId = optionalOperationString(operation.session_id, 'session_id');
  const thread = sessionId
    ? codex.resumeThread(sessionId, threadOptions)
    : codex.startThread(threadOptions);
  return { sessionId, thread };
}

async function invokeCodexModelChatStream(prompt, operation, packageName) {
  const { sessionId, thread } = await createCodexThread(operation, packageName);
  if (typeof thread.runStreamed !== 'function') {
    throw new Error('Codex thread is missing runStreamed() in @openai/codex-sdk');
  }

  const streamed = await runCodexOperation(operation.timeout_ms, (turnOptions) =>
    thread.runStreamed(prompt, turnOptions),
  );
  const chunks = [];
  const itemText = new Map();
  let sequence = 0;

  for await (const event of streamed.events) {
    if (event?.type === 'turn.failed') {
      throw new Error(event?.error?.message ?? 'Codex streamed turn failed');
    }
    if (
      (event?.type !== 'item.updated' && event?.type !== 'item.completed') ||
      event?.item?.type !== 'agent_message' ||
      typeof event.item.text !== 'string'
    ) {
      continue;
    }
    const previous = itemText.get(event.item.id) ?? '';
    const current = event.item.text;
    const delta = current.startsWith(previous) ? current.slice(previous.length) : current;
    itemText.set(event.item.id, current);
    if (delta) {
      chunks.push({ sequence, content: delta });
      sequence += 1;
    }
  }

  if (chunks.length === 0) {
    throw new Error('Codex streamed turn completed without an agent message');
  }
  return {
    ...liveSuccess(chunks.map((chunk) => chunk.content), operation, {
      package: packageName,
      native_session_id: thread?.id ?? sessionId,
    }),
    chunks,
  };
}

function buildCodexThreadOptions(operation) {
  const executionOptions = readCodexExecutionOptions(operation);
  const fullAuto =
    executionOptions.full_auto == null
      ? false
      : optionalOperationBoolean(executionOptions.full_auto, 'full_auto');
  const sandboxMode = normalizeCodexSandboxMode(executionOptions.sandbox_mode);
  const approvalPolicy = normalizeCodexApprovalPolicy(executionOptions.approval_policy);
  const threadOptions = {};
  const modelId = optionalOperationString(operation.model_id, 'model_id');
  const workingDirectory = optionalOperationString(operation.working_directory, 'working_directory');

  if (modelId) {
    threadOptions.model = modelId;
  }
  if (workingDirectory) {
    threadOptions.workingDirectory = workingDirectory;
  }
  if (sandboxMode ?? fullAuto) {
    threadOptions.sandboxMode = sandboxMode ?? 'workspace-write';
  }
  if (approvalPolicy ?? fullAuto) {
    threadOptions.approvalPolicy = approvalPolicy ?? 'on-failure';
  }
  if (executionOptions.skip_git_repo_check != null) {
    threadOptions.skipGitRepoCheck = optionalOperationBoolean(
      executionOptions.skip_git_repo_check,
      'skip_git_repo_check',
    );
  }

  return threadOptions;
}

async function runCodexThread(thread, prompt, timeoutMs) {
  return runCodexOperation(timeoutMs, (turnOptions) => thread.run(prompt, turnOptions));
}

async function runCodexOperation(timeoutMs, invoke) {
  if (timeoutMs == null) {
    return invoke({});
  }
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
    throw new Error('timeout_ms must be a positive safe integer');
  }

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await invoke({ signal: controller.signal });
  } catch (error) {
    if (controller.signal.aborted) {
      throw new Error(`codex_sdk_timeout: exceeded ${timeoutMs} ms`);
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function readCodexExecutionOptions(operation) {
  const options = operation?.execution_options;
  if (options == null) {
    return {};
  }
  if (typeof options !== 'object' || Array.isArray(options)) {
    throw new Error('execution_options must be an object');
  }
  return options;
}

function optionalOperationString(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value !== 'string') {
    throw new Error(`${fieldName} must be a string`);
  }
  const normalized = value.trim();
  return normalized || null;
}

function optionalOperationBoolean(value, fieldName) {
  if (typeof value !== 'boolean') {
    throw new Error(`execution_options.${fieldName} must be a boolean`);
  }
  return value;
}

function normalizeCodexSandboxMode(value) {
  const normalized = optionalOperationString(value, 'execution_options.sandbox_mode');
  if (!normalized) {
    return null;
  }
  const compact = normalized.toLowerCase().replace(/[_\s]/gu, '-');
  if (compact === 'read-only' || compact === 'readonly') {
    return 'read-only';
  }
  if (compact === 'workspace-write' || compact === 'workspacewrite') {
    return 'workspace-write';
  }
  if (compact === 'danger-full-access' || compact === 'dangerfullaccess') {
    throw new Error('danger-full-access is prohibited for kernel-owned Codex SDK execution');
  }
  throw new Error(`unsupported Codex sandbox mode: ${normalized}`);
}

function normalizeCodexApprovalPolicy(value) {
  const normalized = optionalOperationString(value, 'execution_options.approval_policy');
  if (!normalized) {
    return null;
  }
  const compact = normalized.toLowerCase().replace(/[-_\s]/gu, '');
  const aliases = new Map([
    ['onrequest', 'on-request'],
    ['untrusted', 'untrusted'],
    ['restricted', 'untrusted'],
    ['unlesstrusted', 'untrusted'],
    ['onfailure', 'on-failure'],
    ['releaseonly', 'on-failure'],
    ['autoallow', 'on-failure'],
  ]);
  const mapped = aliases.get(compact);
  if (!mapped) {
    throw new Error(`unsupported Codex approval policy: ${normalized}`);
  }
  return mapped;
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
    const base = gatewayUrl.replace(/\/$/, '').replace(/\/v1$/, '');
    const token = process.env.OPENCLAW_GATEWAY_TOKEN?.trim();
    if (!token) {
      throw new Error('OPENCLAW_GATEWAY_TOKEN is required for the OpenAI SDK gateway adapter');
    }
    const moduleNamespace = await loadPackage(packageName);
    const OpenAI = moduleNamespace.default ?? moduleNamespace.OpenAI;
    if (typeof OpenAI !== 'function') {
      throw new Error('official openai sdk missing OpenAI constructor');
    }
    const client = new OpenAI({
      apiKey: token,
      baseURL: `${base}/v1`,
      timeout: operation.timeout_ms ?? 120_000,
    });
    const messages = resolveOpenClawWireMessages(operation);
    const payload = await client.chat.completions.create({
      model: operation.model_id ?? 'default',
      messages,
    });
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
  openai: invokeOpenClawModelChat,
};

export async function invokeModelChatLive(packageName, operation) {
  const handler = LIVE_MODEL_CHAT_HANDLERS[packageName];
  if (!handler) {
    throw new Error(`no live model_chat handler for package ${packageName}`);
  }

  const prompt = resolveModelChatPrompt(operation);
  return handler(prompt, operation, packageName);
}

export async function invokeModelChatStreamLive(packageName, operation) {
  if (isCodexPackage(packageName)) {
    return invokeCodexModelChatStream(resolveModelChatPrompt(operation), operation, packageName);
  }
  return buildModelChatStreamResult(await invokeModelChatLive(packageName, operation));
}

export async function invokeModelChatRuntime(packageName, operation) {
  const packageProbe = probePackage(packageName);
  const codexPackage = isCodexPackage(packageName);
  const providerCliPackage = isProviderCliPackage(packageName);
  const cliProbe = codexPackage
    ? probeCodexCli()
    : providerCliPackage
      ? probeProviderCli(packageName)
      : null;
  let sdkError = null;
  let cliError = null;

  // The CLI transport is the production-complete Codex lane. Prefer it when
  // present so an incomplete or version-skewed SDK facade cannot shadow it.
  if (cliProbe?.available) {
    try {
      const prompt = resolveModelChatPrompt(operation);
      return codexPackage
        ? await invokeCodexCliModelChat(operation, { packageName, prompt })
        : await invokeProviderCliModelChat(packageName, operation, { prompt });
    } catch (error) {
      cliError = error;
    }
  }

  if (packageProbe.resolved) {
    try {
      return await invokeModelChatLive(packageName, operation);
    } catch (error) {
      sdkError = error;
    }
  }

  if ((codexPackage || providerCliPackage) && !cliProbe?.available) {
    cliError = new Error(`provider_cli_unavailable: no real executable was found for ${packageName}`);
  }

  if (sdkError || cliError) {
    if (sdkError && cliError) {
      throw new Error(
        `Provider CLI invoke failed (${formatError(cliError)}); Provider SDK invoke failed (${formatError(sdkError)})`,
      );
    }
    throw sdkError ?? cliError;
  }
  throw new Error(`package not resolved: ${packageName}`);
}

function formatError(error) {
  return error instanceof Error ? error.message : String(error);
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
