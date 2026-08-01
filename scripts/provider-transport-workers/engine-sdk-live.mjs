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
  closeCodexAppServerRuntime,
  interruptCodexAppServerTurn,
  invokeCodexAppServerModelChat,
  isCodexAppServerFallbackSafe,
  probeCodexAppServerRuntime,
  respondToCodexAppServerRequest,
} from './codex-app-server-runtime.mjs';
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
const CLAUDE_CLI_FALLBACK_ENV = 'SDKWORK_CLAUDE_CODE_ALLOW_CLI_FALLBACK';

// This marker stays inside the Node worker process. A provider must set it only
// after it has received the provider session id from its own runtime.
export const VERIFIED_PROVIDER_SESSION_ID = Symbol('sdkwork.verifiedProviderSessionId');

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

function packageEntryCandidates(packageJson, exportKey = '.') {
  const candidates = [];
  const rootExport =
    packageJson.exports && typeof packageJson.exports === 'object'
      ? packageJson.exports[exportKey] ?? (exportKey === '.' ? packageJson.exports : null)
      : exportKey === '.'
        ? packageJson.exports
        : null;

  appendExportCandidates(candidates, rootExport);
  if (exportKey === '.') {
    for (const field of ['module', 'main']) {
      if (typeof packageJson[field] === 'string') {
        candidates.push(packageJson[field]);
      }
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

function resolveLocalPackageSpecifier(packageName, localPath, exportKey = '.') {
  if (Array.isArray(localPath)) {
    for (const candidate of localPath) {
      const resolved = resolveLocalPackageSpecifier(packageName, candidate, exportKey);
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
    if (exportKey !== '.') {
      return null;
    }
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

  for (const candidate of packageEntryCandidates(packageJson, exportKey)) {
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
  return resolvePackageExportSpecifier(packageName, '.');
}

export function resolvePackageExportSpecifier(packageName, exportKey) {
  if (exportKey !== '.' && !/^\.\/[a-zA-Z0-9._/-]+$/u.test(exportKey)) {
    return null;
  }
  const configuredPaths = configuredPackagePaths();
  if (Object.hasOwn(configuredPaths, packageName)) {
    return resolveLocalPackageSpecifier(packageName, configuredPaths[packageName], exportKey);
  }

  const require = createRequire(import.meta.url);
  const requestedSpecifier =
    exportKey === '.' ? packageName : `${packageName}/${exportKey.slice(2)}`;
  try {
    const resolved = require.resolve(requestedSpecifier);
    return path.isAbsolute(resolved) ? pathToFileURL(resolved).href : resolved;
  } catch {
    const paths = defaultPackagePaths(workspaceRoot() ?? '');
    const localPath = paths[packageName];
    return resolveLocalPackageSpecifier(packageName, localPath, exportKey);
  }
}

export function probePackage(packageName) {
  return {
    resolved: Boolean(resolvePackageSpecifier(packageName)),
  };
}

export function probeModelChatRuntime(packageName) {
  const packageProbe = probePackage(packageName);
  const appServerProbe = isCodexPackage(packageName)
    ? probeCodexAppServerRuntime()
    : null;
  const cliProbe = isCodexPackage(packageName)
    ? probeCodexCli()
    : isProviderCliPackage(packageName)
      ? probeProviderCli(packageName)
      : null;
  const cliAvailable = Boolean(cliProbe?.available);
  const cliFallbackAllowed = !isClaudeAgentSdkPackage(packageName) || claudeCliFallbackEnabled();
  return {
    ...packageProbe,
    app_server_available: Boolean(appServerProbe?.app_server_available),
    cli_available: cliAvailable,
    runtime_available: packageProbe.resolved || (cliAvailable && cliFallbackAllowed),
    runtime_mode: appServerProbe?.app_server_available
      ? 'app_server'
      : packageProbe.resolved
        ? 'sdk_live'
        : cliAvailable && cliFallbackAllowed
          ? 'sdk_cli'
          : null,
  };
}

function isClaudeAgentSdkPackage(packageName) {
  return packageName === '@anthropic-ai/claude-agent-sdk';
}

function claudeCliFallbackEnabled(environment = process.env) {
  return matchesAllowTruthy(environment[CLAUDE_CLI_FALLBACK_ENV] ?? '');
}

async function loadPackage(packageName) {
  const specifier = resolvePackageSpecifier(packageName);
  if (!specifier) {
    throw new Error(`package not resolved: ${packageName}`);
  }
  return import(specifier);
}

async function loadPackageExport(packageName, exportKey) {
  const specifier = resolvePackageExportSpecifier(packageName, exportKey);
  if (!specifier) {
    throw new Error(`package export not resolved: ${packageName} ${exportKey}`);
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

function createRuntimeActivityReporter(operation, onActivity) {
  if (onActivity != null && typeof onActivity !== 'function') {
    throw new Error('runtime onActivity must be a function');
  }
  let providerSessionId = null;
  let started = false;
  let terminal = false;
  let lastObservedAtMillis = 0;

  const nextObservedAt = () => {
    const now = Date.now();
    lastObservedAtMillis = Math.max(now, lastObservedAtMillis + 1);
    return new Date(lastObservedAtMillis).toISOString();
  };
  const emit = async (phase, extra = {}) => {
    if (!onActivity || !providerSessionId) {
      return;
    }
    await onActivity({
      provider_session_id: providerSessionId,
      phase,
      observed_at: nextObservedAt(),
      ...extra,
    });
  };
  const establish = async (candidate) => {
    const next = readProviderSessionId(candidate);
    if (!next) {
      return;
    }
    if (providerSessionId && providerSessionId !== next) {
      throw new Error('provider emitted inconsistent provider session identities');
    }
    providerSessionId = next;
    if (!started) {
      started = true;
      await emit('started');
      await emit('working');
    }
  };
  const verifyEstablishedIdentity = (candidate) => {
    const next = readProviderSessionId(candidate);
    if (providerSessionId && next && providerSessionId !== next) {
      throw new Error('provider emitted inconsistent provider session identities');
    }
  };

  return {
    establish,
    async working(candidate) {
      await establish(candidate);
      await emit('working');
    },
    async waiting(candidate, interactionHint) {
      await establish(candidate);
      await emit('waiting', { interaction_hint: interactionHint });
    },
    async succeed(candidate) {
      if (terminal) {
        return;
      }
      verifyEstablishedIdentity(candidate);
      if (!providerSessionId) {
        return;
      }
      terminal = true;
      await emit('idle');
      await emit('terminal', { terminal_state: 'idle' });
    },
    async fail() {
      if (terminal || !providerSessionId) {
        return;
      }
      terminal = true;
      await emit('failed');
      await emit('terminal', { terminal_state: 'failed' });
    },
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

async function invokeCodexModelChat(prompt, operation, packageName, activity) {
  const { thread } = await createCodexThread(operation, packageName);
  const requestedProviderSessionId = optionalOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const initialProviderSessionId = verifiedCodexProviderSessionId(thread);
  if (initialProviderSessionId) {
    await activity.establish(
      verifyProviderSessionId(
        'codex_sdk',
        initialProviderSessionId,
        requestedProviderSessionId,
      ),
    );
  }
  const turn = await runCodexThread(thread, prompt, operation.timeout_ms);
  const text = turn?.finalResponse ?? turn?.items?.map((item) => item?.text ?? '').join('\n') ?? '';
  const providerSessionId = verifyProviderSessionId(
    'codex_sdk',
    verifiedCodexProviderSessionId(thread),
    requestedProviderSessionId,
  );
  await activity.establish(providerSessionId);
  return liveSuccess(text, operation, {
    package: packageName,
    provider_session_id: providerSessionId,
    [VERIFIED_PROVIDER_SESSION_ID]: Boolean(providerSessionId),
  });
}

async function createCodexThread(operation, packageName) {
  const moduleNamespace = await loadPackage(packageName);
  const Codex = moduleNamespace.Codex;
  if (typeof Codex !== 'function') {
    throw new Error('Codex class is unavailable in @openai/codex-sdk');
  }

  const executionOptions = readCodexExecutionOptions(operation);
  const approvalsReviewer = normalizeCodexApprovalsReviewer(
    executionOptions.approvals_reviewer,
  );
  const codex = new Codex(
    approvalsReviewer
      ? { config: { approvals_reviewer: approvalsReviewer } }
      : undefined,
  );
  const threadOptions = buildCodexThreadOptions(operation);
  const providerSessionId = optionalOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const thread = providerSessionId
    ? codex.resumeThread(providerSessionId, threadOptions)
    : codex.startThread(threadOptions);
  return { thread };
}

async function invokeCodexModelChatStream(
  prompt,
  operation,
  packageName,
  onChunk,
  activity,
  onEvent,
) {
  const { thread } = await createCodexThread(operation, packageName);
  const requestedProviderSessionId = optionalOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const initialProviderSessionId = verifiedCodexProviderSessionId(thread);
  if (initialProviderSessionId) {
    await activity.establish(
      verifyProviderSessionId(
        'codex_sdk',
        initialProviderSessionId,
        requestedProviderSessionId,
      ),
    );
  }
  if (typeof thread.runStreamed !== 'function') {
    throw new Error('Codex thread is missing runStreamed() in @openai/codex-sdk');
  }

  const collectChunks = !onChunk;
  const chunks = [];
  const itemText = new Map();
  let sequence = 0;
  let eventSequence = 0;
  let completed = false;

  await runCodexOperation(operation.timeout_ms, async (turnOptions) => {
    const streamed = await thread.runStreamed(prompt, turnOptions);
    for await (const event of streamed.events) {
      const eventProviderSessionId =
        event?.thread_id ?? event?.threadId ?? verifiedCodexProviderSessionId(thread);
      if (eventProviderSessionId) {
        await activity.establish(
          verifyProviderSessionId(
            'codex_sdk',
            eventProviderSessionId,
            requestedProviderSessionId,
          ),
        );
      }
      if (onEvent) {
        await onEvent(buildCodexKernelStreamEvent(
          event,
          operation,
          eventProviderSessionId,
          eventSequence,
        ));
      }
      eventSequence += 1;
      if (event?.type === 'turn.failed') {
        throw new Error(event?.error?.message ?? 'Codex streamed turn failed');
      }
      if (event?.type === 'error') {
        throw new Error(event?.message ?? 'Codex streamed turn failed');
      }
      if (event?.type === 'turn.completed') {
        completed = true;
        continue;
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
      if (!current.startsWith(previous)) {
        throw new Error('codex_stream_non_monotonic_agent_message');
      }
      const delta = current.slice(previous.length);
      itemText.set(event.item.id, current);
      if (!delta) {
        continue;
      }
      const chunk = { sequence, content: delta };
      sequence += 1;
      if (onChunk) {
        await onChunk(chunk);
      }
      if (collectChunks) {
        chunks.push(chunk);
      }
    }
  });

  if (!completed) {
    throw new Error('codex_stream_incomplete: missing turn.completed event');
  }
  if (sequence === 0) {
    throw new Error('Codex streamed turn completed without an agent message');
  }
  const providerSessionId = verifyProviderSessionId(
    'codex_sdk',
    verifiedCodexProviderSessionId(thread),
    requestedProviderSessionId,
  );
  return {
    ...liveSuccess(collectChunks ? chunks.map((chunk) => chunk.content) : [], operation, {
      package: packageName,
      provider_session_id: providerSessionId,
      [VERIFIED_PROVIDER_SESSION_ID]: Boolean(providerSessionId),
    }),
    chunks,
  };
}

function verifiedCodexProviderSessionId(thread) {
  const providerSessionId = thread?.id;
  if (typeof providerSessionId !== 'string') {
    return null;
  }
  const normalized = providerSessionId.trim();
  return normalized || null;
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

function buildCodexAppServerInvocationOptions(operation) {
  const threadOptions = buildCodexThreadOptions(operation);
  const executionOptions = readCodexExecutionOptions(operation);
  const approvalsReviewer = normalizeCodexApprovalsReviewer(
    executionOptions.approvals_reviewer,
  );
  const sessionOptions = {};
  const turnOptions = {};
  if (threadOptions.model) {
    sessionOptions.model = threadOptions.model;
    turnOptions.model = threadOptions.model;
  }
  if (threadOptions.workingDirectory) {
    sessionOptions.cwd = threadOptions.workingDirectory;
    turnOptions.cwd = threadOptions.workingDirectory;
  }
  if (threadOptions.sandboxMode) {
    sessionOptions.sandbox = threadOptions.sandboxMode;
  }
  if (threadOptions.approvalPolicy) {
    sessionOptions.approvalPolicy = threadOptions.approvalPolicy;
    turnOptions.approvalPolicy = threadOptions.approvalPolicy;
  }
  if (approvalsReviewer) {
    sessionOptions.approvalsReviewer = approvalsReviewer;
    turnOptions.approvalsReviewer = approvalsReviewer;
  }
  const serviceTier = optionalOperationString(operation.service_tier, 'service_tier');
  if (serviceTier) {
    sessionOptions.serviceTier = serviceTier;
    turnOptions.serviceTier = serviceTier;
  }
  const effort = optionalOperationString(
    executionOptions.reasoning_effort,
    'execution_options.reasoning_effort',
  );
  if (effort) {
    turnOptions.effort = effort;
  }
  return { sessionOptions, turnOptions };
}

async function invokeCodexAppServerModelRuntime(operation, options, activity) {
  const invocationOptions = buildCodexAppServerInvocationOptions(operation);
  const result = await invokeCodexAppServerModelChat(operation, {
    ...invocationOptions,
    activity,
    onChunk: options?.onChunk,
    onEvent: options?.onEvent,
    prompt: resolveModelChatPrompt(operation),
  });
  return {
    ...result,
    package: options?.packageName ?? '@openai/codex-sdk',
    [VERIFIED_PROVIDER_SESSION_ID]: true,
  };
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
    const result = await invoke({ signal: controller.signal });
    if (controller.signal.aborted) {
      throw new Error(`codex_sdk_timeout: exceeded ${timeoutMs} ms`);
    }
    return result;
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

function codexAppServerPreferred(operation) {
  const value = readCodexExecutionOptions(operation).prefer_app_server;
  return value == null
    ? true
    : optionalOperationBoolean(value, 'prefer_app_server');
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

function requiredOperationString(value, fieldName) {
  const normalized = optionalOperationString(value, fieldName);
  if (!normalized) {
    throw new Error(`${fieldName} must be a non-empty string`);
  }
  return normalized;
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
    return 'danger-full-access';
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
    ['never', 'never'],
  ]);
  const mapped = aliases.get(compact);
  if (!mapped) {
    throw new Error(`unsupported Codex approval policy: ${normalized}`);
  }
  return mapped;
}

function normalizeCodexApprovalsReviewer(value) {
  const normalized = optionalOperationString(
    value,
    'execution_options.approvals_reviewer',
  );
  if (!normalized) {
    return null;
  }
  const compact = normalized.toLowerCase().replace(/[-\s]/gu, '_');
  if (compact === 'user') {
    return 'user';
  }
  if (compact === 'auto_review' || compact === 'guardian_subagent') {
    return 'auto_review';
  }
  throw new Error(`unsupported Codex approvals reviewer: ${normalized}`);
}

async function invokeClaudeModelChat(prompt, operation, packageName, activity) {
  const moduleNamespace = await loadPackage(packageName);
  if (typeof moduleNamespace.query !== 'function') {
    throw new Error(
      'claude agent sdk must expose query() to establish and resume provider sessions',
    );
  }

  return runProviderOperation(operation, 'claude_agent_sdk', async (abortController) => {
    const requestedProviderSessionId = optionalOperationString(
      operation.provider_session_id,
      'provider_session_id',
    );
    const modelId = optionalOperationString(operation.model_id, 'model_id');
    const permissionSettings = resolveClaudeSdkPermissionSettings(operation);
    const options = {
      cwd: resolveProviderWorkingDirectory(operation),
      abortController,
      ...(modelId ? { model: modelId } : {}),
      ...(requestedProviderSessionId ? { resume: requestedProviderSessionId } : {}),
      ...permissionSettings,
    };
    let text = '';
    let completed = false;
    let resultText = null;
    let providerSessionId = null;

    for await (const event of moduleNamespace.query({ prompt, options })) {
      providerSessionId = collectProviderSessionId(
        'claude_agent_sdk',
        providerSessionId,
        event?.session_id ?? event?.sessionId ?? event?.message?.session_id,
      );
      if (providerSessionId) {
        verifyProviderSessionId(
          'claude_agent_sdk',
          providerSessionId,
          requestedProviderSessionId,
        );
        await activity.establish(providerSessionId);
      }
      if (event?.type === 'permission_request') {
        await activity.waiting(providerSessionId, 'approval_required');
      } else if (event?.type === 'assistant' || event?.type === 'tool_use') {
        await activity.working(providerSessionId);
      }
      if (event?.type === 'assistant') {
        text += extractClaudeAssistantText(event);
      }
      if (event?.type !== 'result') {
        continue;
      }
      if (event?.is_error === true || event?.subtype !== 'success') {
        throw new Error(
          `claude agent sdk turn failed: ${
            readProviderError(event?.error ?? event?.result) ?? event?.subtype ?? 'unknown result'
          }`,
        );
      }
      completed = true;
      if (typeof event.result === 'string' && event.result.trim()) {
        resultText = event.result;
      }
    }

    if (!completed) {
      throw new Error('claude agent sdk completed without a successful result event');
    }
    const verifiedSessionId = verifyProviderSessionId(
      'claude_agent_sdk',
      providerSessionId,
      requestedProviderSessionId,
    );
    const output = resultText ?? text;
    if (!output.trim()) {
      throw new Error('claude agent sdk completed without assistant content');
    }
    return liveSuccess(output, operation, {
      package: packageName,
      provider_session_id: verifiedSessionId,
      [VERIFIED_PROVIDER_SESSION_ID]: true,
    });
  });
}

function resolveClaudeSdkPermissionSettings(operation) {
  const executionOptions = readCodexExecutionOptions(operation);
  const requested = optionalOperationString(
    executionOptions.approval_policy,
    'execution_options.approval_policy',
  );
  const settings = {};
  if (!requested) {
    return { ...settings, ...resolveClaudeSdkSandboxSettings(executionOptions) };
  }
  const compact = requested.toLowerCase().replace(/[_\s]/gu, '-');
  if (compact === 'default' || compact === 'on-request') {
    settings.permissionMode = 'default';
  } else if (compact === 'accept-edits') {
    settings.permissionMode = 'acceptEdits';
  } else if (compact === 'bypass-permissions' || compact === 'never') {
    settings.permissionMode = 'bypassPermissions';
    settings.allowDangerouslySkipPermissions = true;
  } else {
    throw new Error(`unsupported Claude Code permission mode: ${requested}`);
  }
  return { ...settings, ...resolveClaudeSdkSandboxSettings(executionOptions) };
}

function resolveClaudeSdkSandboxSettings(executionOptions) {
  const requested = optionalOperationString(
    executionOptions.sandbox_mode,
    'execution_options.sandbox_mode',
  );
  if (!requested) {
    return {};
  }
  const compact = requested.toLowerCase().replace(/[_\s]/gu, '-');
  if (compact === 'read-only' || compact === 'readonly' || compact === 'workspace-write' || compact === 'workspacewrite') {
    return {
      sandbox: {
        enabled: true,
        autoAllowBashIfSandboxed: true,
        failIfUnavailable: true,
      },
    };
  }
  if (compact === 'danger-full-access' || compact === 'dangerfullaccess' || compact === 'none') {
    return { sandbox: { enabled: false } };
  }
  throw new Error(`unsupported Claude Code sandbox mode: ${requested}`);
}

async function invokeGeminiModelChat(prompt, operation, packageName, activity) {
  const moduleNamespace = await loadPackage(packageName);
  const Agent = moduleNamespace.GeminiCliAgent;
  if (typeof Agent !== 'function') {
    throw new Error('GeminiCliAgent is unavailable in @google/gemini-cli-sdk');
  }

  return runProviderOperation(operation, 'gemini_cli_sdk', async (abortController) => {
    const requestedProviderSessionId = optionalOperationString(
      operation.provider_session_id,
      'provider_session_id',
    );
    const agent = new Agent(buildGeminiAgentOptions(operation));
    const session = await resolveGeminiSession(agent, requestedProviderSessionId);
    const verifiedSessionId = verifyProviderSessionId(
      'gemini_cli_sdk',
      session?.id,
      requestedProviderSessionId,
    );
    await activity.establish(verifiedSessionId);
    let text = '';

    if (typeof session.sendStream === 'function') {
      let completed = false;
      for await (const event of session.sendStream(prompt, abortController.signal)) {
        if (event?.type === 'elicitation_request') {
          await activity.waiting(verifiedSessionId, 'user_input_required');
        } else if (event?.type === 'tool_request' || event?.type === 'content') {
          await activity.working(verifiedSessionId);
        }
        if (event?.type === 'error') {
          throw new Error(
            `gemini cli sdk turn failed: ${readProviderError(event?.value?.error ?? event?.error) ?? 'unknown error'}`,
          );
        }
        if (event?.type === 'user_cancelled') {
          throw new Error('gemini cli sdk turn was cancelled by the provider');
        }
        if (event?.type === 'agent_execution_stopped' || event?.type === 'agent_execution_blocked') {
          throw new Error(
            `gemini cli sdk execution stopped: ${readProviderError(event?.value?.reason) ?? event.type}`,
          );
        }
        if (event?.type === 'finished') {
          completed = true;
        }
        text += extractGeminiAssistantText(event);
      }
      if (!completed) {
        throw new Error('gemini cli sdk stream completed without a finished event');
      }
    } else if (typeof session.send === 'function') {
      const response = await session.send(prompt, abortController.signal);
      text = typeof response === 'string' ? response : String(response?.text ?? response?.content ?? '');
    } else {
      throw new Error('gemini cli sdk session missing send/sendStream');
    }

    if (!text.trim()) {
      throw new Error('gemini cli sdk completed without assistant content');
    }
    return liveSuccess(text, operation, {
      package: packageName,
      provider_session_id: verifiedSessionId,
      [VERIFIED_PROVIDER_SESSION_ID]: true,
    });
  });
}

async function invokeOpencodeModelChat(prompt, operation, packageName, activity) {
  const moduleNamespace = await loadPackage(packageName);
  const createOpencode = moduleNamespace.createOpencode;
  const createOpencodeClient = moduleNamespace.createOpencodeClient;
  const createOpencodeServer = moduleNamespace.createOpencodeServer;

  return runProviderOperation(operation, 'opencode_sdk', async (abortController) => {
    const baseUrl = process.env.OPENCODE_SERVER_URL?.trim();
    const workingDirectory = resolveProviderWorkingDirectory(operation);
    if (baseUrl) {
      if (typeof createOpencodeClient !== 'function') {
        throw new Error('opencode sdk missing createOpencodeClient() for OPENCODE_SERVER_URL');
      }
      return invokeOpencodeClient(
        createOpencodeClient({ baseUrl, directory: workingDirectory }),
        prompt,
        operation,
        packageName,
        abortController.signal,
        activity,
      );
    }

    if (typeof createOpencodeServer === 'function' && typeof createOpencodeClient === 'function') {
      const server = await createOpencodeServer({ signal: abortController.signal });
      try {
        return await invokeOpencodeClient(
          createOpencodeClient({
            baseUrl: server?.url,
            directory: workingDirectory,
          }),
          prompt,
          operation,
          packageName,
          abortController.signal,
          activity,
        );
      } finally {
        await server?.close?.();
      }
    }

    if (typeof createOpencode === 'function') {
      if (workingDirectory !== process.cwd()) {
        throw new Error(
          'opencode sdk createOpencode() fallback cannot honor working_directory; upgrade to the official server/client entrypoints',
        );
      }
      const { client, server } = await createOpencode({ signal: abortController.signal });
      try {
        return await invokeOpencodeClient(
          client,
          prompt,
          operation,
          packageName,
          abortController.signal,
          activity,
        );
      } finally {
        await server?.close?.();
      }
    }

    throw new Error('opencode sdk missing createOpencodeServer/createOpencodeClient session entrypoints');
  });
}

const OPENCODE_SESSION_CONTROL_OPERATIONS = new Set([
  'session_interrupt',
  'session_compact',
  'session_fork',
]);

export async function invokeSessionControlRuntime(packageName, operation) {
  if (packageName !== '@opencode-ai/sdk') {
    throw new Error(`no live session control handler for package ${packageName}`);
  }
  const operationName = requiredOperationString(operation?.operation, 'operation');
  if (!OPENCODE_SESSION_CONTROL_OPERATIONS.has(operationName)) {
    throw new Error(`unsupported OpenCode session control operation: ${operationName}`);
  }
  const controlRequestId = requiredOperationString(
    operation.control_request_id,
    'control_request_id',
  );
  const sessionId = requiredOperationString(operation.session_id, 'session_id');
  const providerSessionId = requiredOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const policyDecisionId = requiredOperationString(
    operation.policy_decision_id,
    'policy_decision_id',
  );
  const baseUrl = process.env.OPENCODE_SERVER_URL?.trim();
  if (!baseUrl) {
    throw new Error('OpenCode session control requires OPENCODE_SERVER_URL for the owning server');
  }

  return runProviderOperation(operation, 'opencode_session_control', async (abortController) => {
    const signal = abortController.signal;
    const workingDirectory = resolveProviderWorkingDirectory(operation);
    let forkedProviderSessionId = null;

    if (operationName === 'session_fork') {
      const moduleNamespace = await loadPackage(packageName);
      if (typeof moduleNamespace.createOpencodeClient !== 'function') {
        throw new Error('opencode sdk missing createOpencodeClient() for session fork');
      }
      const client = moduleNamespace.createOpencodeClient({
        baseUrl,
        directory: workingDirectory,
      });
      await verifyOpencodeSession(client, providerSessionId, signal);
      if (typeof client?.session?.fork !== 'function') {
        throw new Error('opencode sdk client is missing session.fork');
      }
      const beforeMessageId = optionalOperationString(
        operation.before_message_id,
        'before_message_id',
      );
      const response = await client.session.fork({
        path: { id: providerSessionId },
        body: beforeMessageId ? { messageID: beforeMessageId } : {},
        signal,
      });
      const error = readProviderError(response?.error);
      if (error) {
        throw new Error(`opencode session.fork failed: ${error}`);
      }
      forkedProviderSessionId = requireProviderSessionId(
        'opencode_sdk',
        response?.data?.id ?? response?.id,
      );
      if (forkedProviderSessionId === providerSessionId) {
        throw new Error('opencode session.fork returned the source provider session id');
      }
    } else {
      const moduleNamespace = await loadPackageExport(packageName, './v2');
      if (typeof moduleNamespace.createOpencodeClient !== 'function') {
        throw new Error('opencode v2 sdk missing createOpencodeClient()');
      }
      const client = moduleNamespace.createOpencodeClient({
        baseUrl,
        directory: workingDirectory,
      });
      const sessionClient = client?.v2?.session;
      if (!sessionClient) {
        throw new Error('opencode v2 sdk client is missing v2.session');
      }
      await verifyOpencodeV2Session(sessionClient, providerSessionId, signal);
      if (operationName === 'session_interrupt') {
        if (typeof sessionClient.interrupt !== 'function') {
          throw new Error('opencode v2 sdk client is missing v2.session.interrupt');
        }
        await invokeOpencodeV2Control(
          sessionClient.interrupt.bind(sessionClient),
          providerSessionId,
          signal,
          'interrupt',
        );
      } else {
        if (optionalOperationString(operation.focus, 'focus')) {
          throw new Error('opencode v2 session.compact does not support a focus parameter');
        }
        if (typeof sessionClient.compact !== 'function') {
          throw new Error('opencode v2 sdk client is missing v2.session.compact');
        }
        await invokeOpencodeV2Control(
          sessionClient.compact.bind(sessionClient),
          providerSessionId,
          signal,
          'compact',
        );
      }
    }

    return {
      ok: true,
      mode: 'sdk_live',
      package: packageName,
      operation: operationName,
      control_request_id: controlRequestId,
      session_id: sessionId,
      provider_session_id: providerSessionId,
      policy_decision_id: policyDecisionId,
      status: 'applied',
      ...(forkedProviderSessionId
        ? { forked_provider_session_id: forkedProviderSessionId }
        : {}),
    };
  });
}

async function verifyOpencodeV2Session(sessionClient, requestedProviderSessionId, signal) {
  if (typeof sessionClient?.get !== 'function') {
    throw new Error('opencode v2 sdk client is missing v2.session.get');
  }
  const response = await sessionClient.get(
    { sessionID: requestedProviderSessionId },
    { signal },
  );
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode v2 session.get failed: ${error}`);
  }
  const providerSessionId = requireProviderSessionId(
    'opencode_sdk',
    response?.data?.data?.id ?? response?.data?.id ?? response?.id,
  );
  return verifyProviderSessionId(
    'opencode_sdk',
    providerSessionId,
    requestedProviderSessionId,
  );
}

async function invokeOpencodeV2Control(method, providerSessionId, signal, action) {
  const response = await method({ sessionID: providerSessionId }, { signal });
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode v2 session.${action} failed: ${error}`);
  }
}

function resolveProviderWorkingDirectory(operation) {
  return optionalOperationString(operation.working_directory, 'working_directory') ?? process.cwd();
}

function buildGeminiAgentOptions(operation) {
  const modelId = optionalOperationString(operation.model_id, 'model_id');
  return {
    // The official Gemini SDK requires instructions. An empty instruction keeps
    // provider defaults intact without inventing product-owned prompt policy.
    instructions: '',
    cwd: resolveProviderWorkingDirectory(operation),
    ...(modelId ? { model: modelId } : {}),
  };
}

async function resolveGeminiSession(agent, requestedProviderSessionId) {
  if (requestedProviderSessionId) {
    if (typeof agent?.resumeSession !== 'function') {
      throw new Error('gemini cli sdk session is missing resumeSession()');
    }
    return agent.resumeSession(requestedProviderSessionId);
  }
  if (typeof agent?.session !== 'function') {
    throw new Error('gemini cli sdk agent is missing session()');
  }
  return agent.session();
}

function extractGeminiAssistantText(event) {
  if (event?.type === 'content' && typeof event.value === 'string') {
    return event.value;
  }
  const message = event?.message ?? event;
  if (message?.role === 'assistant') {
    if (typeof message.content === 'string') {
      return message.content;
    }
    if (Array.isArray(message.content)) {
      return extractTextParts(message.content);
    }
  }
  return typeof event?.text === 'string' ? event.text : '';
}

function extractClaudeAssistantText(event) {
  const content = event?.message?.content ?? event?.content;
  if (typeof content === 'string') {
    return content;
  }
  return extractTextParts(content);
}

async function invokeOpencodeClient(client, prompt, operation, packageName, signal, activity) {
  if (!client?.session?.prompt || !client?.session?.create) {
    throw new Error('opencode sdk client is missing session.create/session.prompt');
  }
  const requestedProviderSessionId = optionalOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const permission = resolveOpencodePermissionRules(operation);
  const createdSessionId = requestedProviderSessionId
    ? null
    : await createOpencodeSession(client, signal, permission);
  const sessionId = requestedProviderSessionId
    ? await verifyOpencodeSession(client, requestedProviderSessionId, signal)
    : createdSessionId;
  if (requestedProviderSessionId && permission) {
    await updateOpencodeSessionPermissions(client, sessionId, signal, permission);
  }
  await activity.establish(sessionId);
  const response = await client.session.prompt({
    signal,
    path: { id: sessionId },
    body: buildOpencodePromptBody(operation),
  });
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode session.prompt failed: ${error}`);
  }
  const text =
    extractTextParts(response?.data?.parts) ||
    extractTextParts(response?.parts) ||
    String(response?.data?.content ?? response?.content ?? '');
  if (!text.trim()) {
    throw new Error('opencode session.prompt completed without assistant content');
  }
  return liveSuccess(text, operation, {
    package: packageName,
    provider_session_id: sessionId,
    [VERIFIED_PROVIDER_SESSION_ID]: true,
  });
}

async function verifyOpencodeSession(client, requestedProviderSessionId, signal) {
  if (typeof client?.session?.get !== 'function') {
    throw new Error(
      'opencode sdk client is missing session.get required to verify a resumed provider session',
    );
  }
  const response = await client.session.get({
    signal,
    path: { id: requestedProviderSessionId },
  });
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode session.get failed: ${error}`);
  }
  const providerSessionId = requireProviderSessionId(
    'opencode_sdk',
    response?.data?.id ?? response?.id,
  );
  return verifyProviderSessionId(
    'opencode_sdk',
    providerSessionId,
    requestedProviderSessionId,
  );
}

async function createOpencodeSession(client, signal, permission) {
  const created = await client.session.create({
    body: permission ? { permission } : {},
    signal,
  });
  const error = readProviderError(created?.error);
  if (error) {
    throw new Error(`opencode session.create failed: ${error}`);
  }
  return requireProviderSessionId('opencode_sdk', created?.data?.id ?? created?.id);
}

async function updateOpencodeSessionPermissions(client, sessionId, signal, permission) {
  if (typeof client?.session?.update !== 'function') {
    throw new Error(
      'opencode sdk client is missing session.update required to apply turn permissions',
    );
  }
  const response = await client.session.update({
    path: { id: sessionId },
    body: { permission },
    signal,
  });
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode session.update failed: ${error}`);
  }
}

function resolveOpencodePermissionRules(operation) {
  const executionOptions = readCodexExecutionOptions(operation);
  const requested = optionalOperationString(
    executionOptions.approval_policy,
    'execution_options.approval_policy',
  );
  if (!requested) {
    return null;
  }
  const compact = requested.toLowerCase().replace(/[_\s]/gu, '-');
  if (compact === 'ask' || compact === 'on-request') {
    return [{ permission: '*', pattern: '*', action: 'ask' }];
  }
  if (compact === 'allow-edits') {
    return [
      { permission: '*', pattern: '*', action: 'ask' },
      ...['read', 'edit', 'glob', 'grep', 'list'].map((permission) => ({
        permission,
        pattern: '*',
        action: 'allow',
      })),
    ];
  }
  if (compact === 'allow-all' || compact === 'never') {
    return [{ permission: '*', pattern: '*', action: 'allow' }];
  }
  throw new Error(`unsupported OpenCode permission mode: ${requested}`);
}

function buildOpencodePromptBody(operation) {
  const body = {
    parts: resolveOpencodePromptParts(operation),
  };
  const model = resolveOpencodeModelSelection(operation);
  return model ? { ...body, model } : body;
}

function resolveOpencodeModelSelection(operation) {
  const modelId = optionalOperationString(operation.model_id, 'model_id');
  if (!modelId || ['opencode', 'open-code', 'open code'].includes(modelId.toLowerCase())) {
    return null;
  }
  const separator = modelId.indexOf('/');
  if (separator <= 0 || separator === modelId.length - 1) {
    throw new Error('opencode model_id must use the official providerID/modelID form');
  }
  return {
    providerID: modelId.slice(0, separator),
    modelID: modelId.slice(separator + 1),
  };
}

function collectProviderSessionId(provider, current, candidate) {
  const next = readProviderSessionId(candidate);
  if (!next) {
    return current;
  }
  if (current && current !== next) {
    throw new Error(`${provider} emitted inconsistent provider session identities`);
  }
  return next;
}

function verifyProviderSessionId(provider, candidate, requestedProviderSessionId) {
  const providerSessionId = requireProviderSessionId(provider, candidate);
  if (requestedProviderSessionId && providerSessionId !== requestedProviderSessionId) {
    throw new Error(`${provider} resumed a different provider session than requested`);
  }
  return providerSessionId;
}

function requireProviderSessionId(provider, value) {
  const providerSessionId = readProviderSessionId(value);
  if (!providerSessionId) {
    throw new Error(`${provider} completed without a provider session id`);
  }
  return providerSessionId;
}

function readProviderSessionId(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function readProviderError(value) {
  if (typeof value === 'string' && value.trim()) {
    return value.trim();
  }
  if (value && typeof value === 'object' && typeof value.message === 'string') {
    return value.message.trim() || null;
  }
  return null;
}

async function runProviderOperation(operation, provider, invoke) {
  const timeoutMs = operation.timeout_ms;
  if (timeoutMs != null && (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0)) {
    throw new Error('timeout_ms must be a positive safe integer');
  }

  const abortController = new AbortController();
  let timedOut = false;
  const timer =
    timeoutMs == null
      ? null
      : setTimeout(() => {
          timedOut = true;
          abortController.abort();
        }, timeoutMs);
  try {
    const result = await invoke(abortController);
    if (timedOut) {
      throw new Error(`${provider}_timeout: exceeded ${timeoutMs} ms`);
    }
    return result;
  } catch (error) {
    if (timedOut) {
      throw new Error(`${provider}_timeout: exceeded ${timeoutMs} ms`);
    }
    throw error;
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
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

export async function invokeModelChatLive(packageName, operation, options = {}) {
  const handler = LIVE_MODEL_CHAT_HANDLERS[packageName];
  if (!handler) {
    throw new Error(`no live model_chat handler for package ${packageName}`);
  }

  const prompt = resolveModelChatPrompt(operation);
  const activity = options.activityReporter ?? createRuntimeActivityReporter(operation, options.onActivity);
  try {
    const result = await handler(prompt, operation, packageName, activity);
    await activity.succeed(result?.provider_session_id);
    return result;
  } catch (error) {
    await activity.fail();
    throw error;
  }
}

export async function invokeModelChatStreamLive(packageName, operation, options = {}) {
  const onChunk = options?.onChunk;
  if (onChunk != null && typeof onChunk !== 'function') {
    throw new Error('stream onChunk must be a function');
  }
  const onEvent = options?.onEvent;
  if (onEvent != null && typeof onEvent !== 'function') {
    throw new Error('stream onEvent must be a function');
  }
  if (isCodexPackage(packageName)) {
    const activity = createRuntimeActivityReporter(operation, options?.onActivity);
    try {
      const result = await invokeCodexModelChatStream(
        resolveModelChatPrompt(operation),
        operation,
        packageName,
        onChunk,
        activity,
        onEvent,
      );
      await activity.succeed(result?.provider_session_id);
      return result;
    } catch (error) {
      await activity.fail();
      throw error;
    }
  }
  const result = buildModelChatStreamResult(
    await invokeModelChatLive(packageName, operation, { onActivity: options?.onActivity }),
  );
  if (onChunk) {
    for (const chunk of result.chunks) {
      await onChunk(chunk);
    }
    return {
      ...result,
      chunks: [],
    };
  }
  return result;
}

export async function invokeModelChatStreamRuntime(packageName, operation, options = {}) {
  if (!isCodexPackage(packageName)) {
    return invokeModelChatStreamLive(packageName, operation, options);
  }
  const activity = createRuntimeActivityReporter(operation, options?.onActivity);
  const appServerProbe = probeCodexAppServerRuntime();
  if (appServerProbe.app_server_available && codexAppServerPreferred(operation)) {
    try {
      const result = await invokeCodexAppServerModelRuntime(
        operation,
        { ...options, packageName },
        activity,
      );
      await activity.succeed(result.provider_session_id);
      return result;
    } catch (error) {
      if (!isCodexAppServerFallbackSafe(error)) {
        await activity.fail();
        throw error;
      }
    }
  }

  if (probePackage(packageName).resolved) {
    try {
      const result = await invokeCodexModelChatStream(
        resolveModelChatPrompt(operation),
        operation,
        packageName,
        options?.onChunk,
        activity,
        options?.onEvent,
      );
      await activity.succeed(result.provider_session_id);
      return result;
    } catch (error) {
      await activity.fail();
      throw error;
    }
  }

  const cliProbe = probeCodexCli();
  if (cliProbe.available) {
    try {
      const onEvent = createCliActivityHandler(packageName, operation, activity);
      const baseResult = markVerifiedCliProviderSession(
        await invokeCodexCliModelChat(operation, {
          packageName,
          prompt: resolveModelChatPrompt(operation),
          onEvent,
        }),
      );
      const result = buildModelChatStreamResult(baseResult);
      if (options?.onChunk) {
        for (const chunk of result.chunks) {
          await options.onChunk(chunk);
        }
        result.chunks = [];
      }
      await activity.succeed(result.provider_session_id);
      return result;
    } catch (error) {
      await activity.fail();
      throw error;
    }
  }

  await activity.fail();
  throw new Error(`package not resolved and Codex app-server/CLI unavailable: ${packageName}`);
}

export function buildCodexKernelStreamEvent(
  providerEvent,
  operation,
  providerSessionId,
  sequence,
) {
  const providerEventType = String(providerEvent?.type ?? 'unknown').trim() || 'unknown';
  const item = providerEvent?.item && typeof providerEvent.item === 'object'
    ? providerEvent.item
    : null;
  const itemId = typeof item?.id === 'string' && item.id.trim() ? item.id.trim() : null;
  const modelRequestId = optionalOperationString(operation.model_request_id, 'model_request_id');
  if (!modelRequestId) {
    throw new Error('Codex stream event is missing model_request_id');
  }
  const normalizedSequence = Number.isSafeInteger(sequence) && sequence >= 0 ? sequence : 0;
  const resolvedProviderSessionId = readProviderSessionId(providerSessionId)
    ?? readProviderSessionId(providerEvent)
    ?? optionalOperationString(operation.provider_session_id, 'provider_session_id');
  const sessionId = optionalOperationString(operation.session_id, 'session_id');

  return {
    event_id: `event.${modelRequestId}.${normalizedSequence}`,
    event_type: codexKernelEventType(providerEventType, item?.type),
    event_version: '1.0.0',
    occurred_at: new Date().toISOString(),
    source: codexKernelEventSource(providerEventType, item?.type),
    severity: codexKernelEventSeverity(providerEventType, item?.type),
    session_id: sessionId,
    run_id: modelRequestId,
    step_id: itemId,
    correlation_id: modelRequestId,
    redaction_classification: 'tenant_sensitive',
    payload_schema: 'sdkwork.agent.provider_stream_event.v1',
    payload: {
      schemaVersion: 1,
      providerId: 'codex',
      providerEventType,
      providerSessionId: resolvedProviderSessionId,
      sequence: normalizedSequence,
      item,
      usage: providerEvent?.usage ?? null,
      error: providerEvent?.error ?? (
        typeof providerEvent?.message === 'string'
          ? { message: providerEvent.message }
          : null
      ),
      rawProviderPayload: providerEvent,
    },
    replay: false,
  };
}

function codexKernelEventType(providerEventType, itemType) {
  if (providerEventType === 'thread.started') return 'agent.session.started';
  if (providerEventType === 'turn.started') return 'agent.turn.started';
  if (providerEventType === 'turn.completed') return 'agent.turn.completed';
  if (providerEventType === 'turn.failed') return 'agent.turn.failed';
  if (providerEventType === 'error') return 'agent.runtime.failed';

  const action = providerEventType.endsWith('.started')
    ? 'started'
    : providerEventType.endsWith('.completed')
      ? 'completed'
      : 'updated';
  if (itemType === 'agent_message' || itemType === 'reasoning') {
    return `agent.message.${action}`;
  }
  if (['command_execution', 'file_change', 'mcp_tool_call', 'web_search'].includes(itemType)) {
    return `agent.tool.${action}`;
  }
  if (itemType === 'todo_list') {
    return `agent.task.${action}`;
  }
  if (itemType === 'error') {
    return 'agent.step.failed';
  }
  return `agent.step.${action}`;
}

function codexKernelEventSource(providerEventType, itemType) {
  if (providerEventType.startsWith('item.') && itemType === 'agent_message') return 'model';
  if (providerEventType.startsWith('item.') && itemType === 'reasoning') return 'model';
  if (['command_execution', 'file_change', 'mcp_tool_call', 'web_search'].includes(itemType)) {
    return 'tool';
  }
  return 'provider';
}

function codexKernelEventSeverity(providerEventType, itemType) {
  return providerEventType === 'error' || providerEventType === 'turn.failed' || itemType === 'error'
    ? 'error'
    : 'info';
}

export async function invokeModelChatRuntime(packageName, operation, options = {}) {
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
  const activity = createRuntimeActivityReporter(operation, options?.onActivity);
  const claudePackage = isClaudeAgentSdkPackage(packageName);
  const cliFallbackAllowed = !claudePackage || claudeCliFallbackEnabled();

  if (codexPackage
    && codexAppServerPreferred(operation)
    && probeCodexAppServerRuntime().app_server_available) {
    try {
      const result = await invokeCodexAppServerModelRuntime(
        operation,
        { ...options, packageName },
        activity,
      );
      await activity.succeed(result.provider_session_id);
      return result;
    } catch (error) {
      if (!isCodexAppServerFallbackSafe(error)) {
        await activity.fail();
        throw error;
      }
    }
  }

  if (packageProbe.resolved) {
    try {
      return await invokeModelChatLive(packageName, operation, { activityReporter: activity });
    } catch (error) {
      sdkError = error;
    }
  }

  // Claude Code is an Agent SDK integration. The installed SDK owns the
  // native Claude Code runtime; use the CLI only when explicitly enabled.
  if (cliProbe?.available && cliFallbackAllowed) {
    try {
      const prompt = resolveModelChatPrompt(operation);
      const onEvent = createCliActivityHandler(packageName, operation, activity);
      const result = codexPackage
        ? await invokeCodexCliModelChat(operation, { packageName, prompt, onEvent })
        : await invokeProviderCliModelChat(packageName, operation, { prompt, onEvent });
      const verified = markVerifiedCliProviderSession(result);
      await activity.succeed(verified?.provider_session_id);
      return verified;
    } catch (error) {
      cliError = error;
    }
  }

  if ((codexPackage || providerCliPackage) && (!cliProbe?.available || !cliFallbackAllowed)) {
    cliError = new Error(`provider_cli_unavailable: no real executable was found for ${packageName}`);
  }

  if (sdkError || cliError) {
    await activity.fail();
    if (sdkError && cliError) {
      throw new Error(
        `Provider CLI invoke failed (${formatError(cliError)}); Provider SDK invoke failed (${formatError(sdkError)})`,
      );
    }
    throw sdkError ?? cliError;
  }
  await activity.fail();
  throw new Error(`package not resolved: ${packageName}`);
}

export async function respondToSdkLiveServerRequest(command) {
  return respondToCodexAppServerRequest(command);
}

export async function interruptSdkLiveTurn(command) {
  return interruptCodexAppServerTurn(command);
}

export async function closeSdkLiveRuntimes() {
  await closeCodexAppServerRuntime();
}

function createCliActivityHandler(packageName, operation, activity) {
  const requestedProviderSessionId = optionalOperationString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const transport = isCodexPackage(packageName)
    ? 'codex_cli'
    : packageName === '@anthropic-ai/claude-agent-sdk'
      ? 'claude_cli'
      : packageName === '@google/gemini-cli-sdk'
        ? 'gemini_cli'
        : 'opencode_cli';

  return async (event) => {
    const candidate = cliEventProviderSessionId(packageName, event);
    const providerSessionId = candidate
      ? verifyProviderSessionId(transport, candidate, requestedProviderSessionId)
      : null;
    const phase = cliEventActivityPhase(packageName, event);
    if (!phase) {
      return;
    }
    if (phase === 'approval_required' || phase === 'user_input_required') {
      await activity.waiting(providerSessionId, phase);
      return;
    }
    await activity.working(providerSessionId);
  };
}

function cliEventProviderSessionId(packageName, event) {
  if (isCodexPackage(packageName)) {
    return event?.thread_id ?? event?.threadId;
  }
  if (packageName === '@anthropic-ai/claude-agent-sdk') {
    return event?.session_id ?? event?.sessionId ?? event?.message?.session_id;
  }
  if (packageName === '@google/gemini-cli-sdk') {
    return event?.session_id ?? event?.sessionId ?? event?.session?.id;
  }
  return (
    event?.sessionID ??
    event?.session_id ??
    event?.sessionId ??
    event?.part?.sessionID ??
    event?.properties?.sessionID
  );
}

function cliEventActivityPhase(packageName, event) {
  const type = String(event?.type ?? '').trim().toLowerCase();
  if (isCodexPackage(packageName)) {
    return ['thread.started', 'turn.started', 'item.started', 'item.updated'].includes(type)
      ? 'working'
      : null;
  }
  if (packageName === '@anthropic-ai/claude-agent-sdk') {
    if (type === 'permission_request') {
      return 'approval_required';
    }
    return ['system', 'assistant', 'tool_use'].includes(type) ? 'working' : null;
  }
  if (packageName === '@google/gemini-cli-sdk') {
    if (type === 'elicitation_request') {
      return 'user_input_required';
    }
    return ['init', 'content', 'message', 'tool_request', 'tool_use'].includes(type)
      ? 'working'
      : null;
  }
  return ['step_start', 'tool_use', 'tool', 'text', 'message'].includes(type)
    ? 'working'
    : null;
}

function markVerifiedCliProviderSession(result) {
  const providerSessionId = result?.provider_session_id;
  if (typeof providerSessionId !== 'string' || !providerSessionId.trim()) {
    return result;
  }
  return {
    ...result,
    provider_session_id: providerSessionId.trim(),
    [VERIFIED_PROVIDER_SESSION_ID]: true,
  };
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
