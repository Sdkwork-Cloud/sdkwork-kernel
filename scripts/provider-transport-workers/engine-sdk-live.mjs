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
  controlCodexAppServerSession,
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

const pendingSdkInteractions = new Map();

function providerInteractionKey(modelRequestId, providerRequestId) {
  const requestType = typeof providerRequestId;
  if (
    (requestType !== 'string' || !providerRequestId.trim())
    && (requestType !== 'number' || !Number.isSafeInteger(providerRequestId))
  ) {
    throw new Error('provider_request_id must be a non-empty string or safe integer');
  }
  return `${modelRequestId}\u0000${requestType}\u0000${String(providerRequestId)}`;
}

function registerPendingSdkInteraction(entry) {
  const key = providerInteractionKey(entry.modelRequestId, entry.providerRequestId);
  const existing = pendingSdkInteractions.get(key);
  if (existing) {
    if (
      existing.providerId !== entry.providerId
      || existing.providerSessionId !== entry.providerSessionId
      || existing.sessionId !== entry.sessionId
    ) {
      throw new Error('provider interaction request id was reused across Session affinity');
    }
    return { entry: existing, created: false };
  }

  let resolveCallback;
  let rejectCallback;
  const callbackResult = entry.compileResolution
    ? new Promise((resolve, reject) => {
        resolveCallback = resolve;
        rejectCallback = reject;
      })
    : null;
  const registered = {
    ...entry,
    key,
    callbackResult,
    resolveCallback,
    rejectCallback,
    abortListener: null,
  };
  if (entry.signal) {
    registered.abortListener = () => {
      if (pendingSdkInteractions.get(key) !== registered) {
        return;
      }
      pendingSdkInteractions.delete(key);
      rejectCallback?.(new Error(`${entry.providerId} interaction was aborted`));
    };
    entry.signal.addEventListener('abort', registered.abortListener, { once: true });
  }
  pendingSdkInteractions.set(key, registered);
  return { entry: registered, created: true };
}

function removePendingSdkInteraction(entry) {
  if (pendingSdkInteractions.get(entry.key) === entry) {
    pendingSdkInteractions.delete(entry.key);
  }
  if (entry.signal && entry.abortListener) {
    entry.signal.removeEventListener('abort', entry.abortListener);
  }
}

function clearPendingSdkInteractions(modelRequestId, reason) {
  for (const entry of pendingSdkInteractions.values()) {
    if (entry.modelRequestId !== modelRequestId) {
      continue;
    }
    removePendingSdkInteraction(entry);
    entry.rejectCallback?.(new Error(reason));
  }
}

async function respondToPendingSdkInteraction(command) {
  const modelRequestId = requiredOperationString(
    command.model_request_id ?? command.modelRequestId,
    'model_request_id',
  );
  const providerRequestId = command.provider_request_id
    ?? command.request_id
    ?? command.providerRequestId
    ?? command.requestId;
  const key = providerInteractionKey(modelRequestId, providerRequestId);
  const entry = pendingSdkInteractions.get(key);
  if (!entry) {
    return null;
  }
  const exactAffinity = [
    ['session_id', command.session_id ?? command.sessionId, entry.sessionId],
    [
      'provider_session_id',
      command.provider_session_id ?? command.providerSessionId,
      entry.providerSessionId,
    ],
  ];
  for (const [field, candidate, expected] of exactAffinity) {
    if (candidate !== expected) {
      throw new Error(`${field} does not match the pending provider interaction`);
    }
  }
  const providerTurnId = command.provider_turn_id ?? command.providerTurnId ?? null;
  if (providerTurnId != null && providerTurnId !== entry.providerTurnId) {
    throw new Error('provider_turn_id does not match the pending provider interaction');
  }
  const canonicalTurnId = command.turn_id ?? command.turnId;
  if (canonicalTurnId !== entry.turnId) {
    throw new Error('turn_id does not match the pending provider interaction');
  }
  const resolution = command.resolution
    ?? command.interaction_resolution
    ?? command.interactionResolution;
  if (!resolution || typeof resolution !== 'object' || Array.isArray(resolution)) {
    throw new Error('resolution must be an object');
  }

  const providerResult = entry.compileResolution
    ? entry.compileResolution(resolution)
    : await entry.sendResolution(resolution);
  removePendingSdkInteraction(entry);
  entry.resolveCallback?.(providerResult);
  return {
    ok: true,
    model_request_id: entry.modelRequestId,
    provider_session_id: entry.providerSessionId,
    provider_turn_id: entry.providerTurnId,
    provider_request_id: entry.providerRequestId,
    interaction_kind: entry.interaction.kind,
    status: 'responded',
  };
}

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

  const requestedSpecifier =
    exportKey === '.' ? packageName : `${packageName}/${exportKey.slice(2)}`;
  try {
    return import.meta.resolve(requestedSpecifier);
  } catch {
    // Some CommonJS-capable SDKs only resolve through require conditions.
  }

  const require = createRequire(import.meta.url);
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

function optionalOpaqueOperationString(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value !== 'string') {
    throw new Error(`${fieldName} must be a string`);
  }
  return value.trim() ? value : null;
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

function canonicalProviderInteraction({
  providerId,
  providerSessionId,
  providerRequestId,
  providerInteractionId = null,
  providerItemId = null,
  providerToolCallId = null,
  providerToolName = null,
  protocolMethod,
  operation,
  category,
  kind,
  prompt,
  allowedActions,
  request,
}) {
  const sessionId = requiredOperationString(operation.session_id, 'session_id');
  const modelRequestId = requiredOperationString(
    operation.model_request_id,
    'model_request_id',
  );
  providerInteractionKey(modelRequestId, providerRequestId);
  return {
    schemaVersion: 1,
    interactionId: String(providerInteractionId ?? providerRequestId),
    sessionId,
    category,
    kind,
    prompt,
    allowedActions,
    request,
    correlation: {
      modelRequestId,
      providerId,
      providerInteractionId: providerInteractionId == null
        ? null
        : String(providerInteractionId),
      providerItemId,
      providerRequestId,
      providerRequestIdType: typeof providerRequestId,
      providerSessionId,
      providerToolCallId,
      providerToolName,
      providerToolNamespace: null,
      // Claude and OpenCode do not expose a provider-native Turn id on these
      // callbacks. Keep absence explicit instead of aliasing the canonical Turn.
      providerTurnId: null,
      protocolMethod,
    },
    receivedAt: new Date().toISOString(),
  };
}

function normalizedQuestionSet(questions, idPrefix, provider) {
  if (!Array.isArray(questions) || questions.length === 0) {
    throw new Error(`${provider} question request must contain questions`);
  }
  return questions.map((question, index) => {
    if (!question || typeof question !== 'object' || Array.isArray(question)) {
      throw new Error(`${provider} question ${index} must be an object`);
    }
    const prompt = requiredProviderString(
      provider,
      question.question ?? question.prompt,
      `questions[${index}].question`,
    );
    const header = optionalProviderString(question.header) ?? prompt;
    const options = Array.isArray(question.options)
      ? question.options.map((option, optionIndex) => ({
          label: requiredProviderString(
            provider,
            option?.label,
            `questions[${index}].options[${optionIndex}].label`,
          ),
          description: optionalProviderString(option?.description) ?? '',
        }))
      : null;
    return {
      id: `${idPrefix}:${index}`,
      header,
      prompt,
      allowOther: question.custom !== false,
      secret: false,
      options,
      multiple: question.multiple === true || question.multiSelect === true,
    };
  });
}

function claudeCanUseToolInteraction(toolName, input, callbackOptions, operation, providerSessionId) {
  const providerRequestId = requiredProviderString(
    'claude agent sdk',
    callbackOptions?.requestId,
    'requestId',
  );
  const toolUseId = requiredProviderString(
    'claude agent sdk',
    callbackOptions?.toolUseID,
    'toolUseID',
  );
  if (toolName === 'AskUserQuestion') {
    const questions = normalizedQuestionSet(
      input?.questions,
      toolUseId,
      'claude agent sdk',
    );
    return canonicalProviderInteraction({
      providerId: 'claude-code',
      providerSessionId,
      providerRequestId,
      providerInteractionId: providerRequestId,
      providerItemId: toolUseId,
      providerToolCallId: toolUseId,
      providerToolName: toolName,
      protocolMethod: 'can_use_tool',
      operation,
      category: 'user_input',
      kind: 'question_set',
      prompt: questions[0].prompt,
      allowedActions: ['submit', 'cancel'],
      request: { questions },
    });
  }

  const title = optionalProviderString(callbackOptions?.title);
  const description = optionalProviderString(callbackOptions?.description);
  const decisionReason = optionalProviderString(callbackOptions?.decisionReason);
  return canonicalProviderInteraction({
    providerId: 'claude-code',
    providerSessionId,
    providerRequestId,
    providerInteractionId: providerRequestId,
    providerItemId: toolUseId,
    providerToolCallId: toolUseId,
    providerToolName: toolName,
    protocolMethod: 'can_use_tool',
    operation,
    category: 'approval',
    kind: 'permission_profile',
    prompt: title ?? description ?? decisionReason ?? `Allow ${toolName}`,
    allowedActions: ['grant', 'decline', 'cancel'],
    request: {
      message: title ?? description,
      reason: decisionReason,
      requestedPermissions: {
        toolName,
        input,
        blockedPath: callbackOptions?.blockedPath ?? null,
        suggestions: callbackOptions?.suggestions ?? [],
        matchedAskRule: callbackOptions?.matchedAskRule ?? null,
      },
    },
  });
}

function compileClaudeCanUseToolResolution(interaction, input, callbackOptions, resolution) {
  const action = requiredOperationString(resolution.action, 'resolution.action');
  if (!interaction.allowedActions.includes(action)) {
    throw new Error(`unsupported Claude interaction action: ${action}`);
  }
  const toolUseID = callbackOptions.toolUseID;
  if (interaction.kind === 'question_set') {
    if (action === 'cancel') {
      return {
        behavior: 'deny',
        message: optionalProviderString(resolution.reason) ?? 'User cancelled the question',
        interrupt: false,
        toolUseID,
        decisionClassification: 'user_reject',
      };
    }
    const answers = resolution.answers;
    if (!answers || typeof answers !== 'object' || Array.isArray(answers)) {
      throw new Error('resolution.answers must be an object');
    }
    const providerAnswers = {};
    for (const question of interaction.request.questions) {
      const values = answers[question.id];
      if (!Array.isArray(values) || values.length === 0) {
        throw new Error(`resolution.answers.${question.id} must be a non-empty array`);
      }
      providerAnswers[question.prompt] = values.map(String).join(', ');
    }
    return {
      behavior: 'allow',
      updatedInput: { ...input, answers: providerAnswers },
      toolUseID,
      decisionClassification: 'user_temporary',
    };
  }

  if (action === 'grant') {
    const scope = requiredOperationString(resolution.scope, 'resolution.scope');
    if (scope !== 'turn' && scope !== 'session') {
      throw new Error('resolution.scope must be turn or session');
    }
    return {
      behavior: 'allow',
      ...(scope === 'session' && Array.isArray(callbackOptions.suggestions)
        ? { updatedPermissions: callbackOptions.suggestions }
        : {}),
      toolUseID,
      decisionClassification: scope === 'session' ? 'user_permanent' : 'user_temporary',
    };
  }
  return {
    behavior: 'deny',
    message: optionalProviderString(resolution.reason) ?? (
      action === 'cancel' ? 'User cancelled the tool request' : 'User denied the tool request'
    ),
    interrupt: action === 'cancel',
    toolUseID,
    decisionClassification: 'user_reject',
  };
}

function claudeElicitationInteraction(request, operation, providerSessionId) {
  const providerRequestId = optionalProviderString(request?.elicitationId);
  if (!providerRequestId) {
    return null;
  }
  const mode = request.mode ?? 'form';
  return canonicalProviderInteraction({
    providerId: 'claude-code',
    providerSessionId,
    providerRequestId,
    providerInteractionId: providerRequestId,
    protocolMethod: 'mcp_elicitation',
    operation,
    category: 'elicitation',
    kind: 'mcp_elicitation',
    prompt: optionalProviderString(request.title)
      ?? optionalProviderString(request.message)
      ?? 'MCP server requests input',
    allowedActions: ['accept', 'decline', 'cancel'],
    request: {
      serverName: request.serverName,
      mode,
      message: request.message,
      elicitationId: providerRequestId,
      ...(mode === 'url'
        ? { url: request.url }
        : { requestedSchema: request.requestedSchema ?? {} }),
    },
  });
}

function compileClaudeElicitationResolution(interaction, resolution) {
  const action = requiredOperationString(resolution.action, 'resolution.action');
  if (!interaction.allowedActions.includes(action)) {
    throw new Error(`unsupported Claude elicitation action: ${action}`);
  }
  return {
    action,
    ...(action === 'accept' ? { content: resolution.content ?? {} } : {}),
  };
}

function createClaudeSdkInteractionCallbacks(operation, sessionState, stream, activity) {
  if (!stream) {
    return {};
  }
  const registerCallback = async ({
    providerRequestId,
    interaction,
    rawProviderPayload,
    signal,
    compileResolution,
    interactionHint,
  }) => {
    const registration = registerPendingSdkInteraction({
      providerId: 'claude-code',
      modelRequestId: stream.modelRequestId,
      sessionId: requiredOperationString(operation.session_id, 'session_id'),
      turnId: requiredOperationString(operation.turn_id, 'turn_id'),
      providerSessionId: sessionState.providerSessionId,
      providerTurnId: null,
      providerRequestId,
      interaction,
      signal,
      compileResolution,
    });
    if (registration.created) {
      try {
        await activity.waiting(sessionState.providerSessionId, interactionHint);
        await stream.event(
          'interaction.requested',
          null,
          { ...rawProviderPayload, interaction },
          sessionState.providerSessionId,
          { interaction },
        );
      } catch (error) {
        removePendingSdkInteraction(registration.entry);
        registration.entry.rejectCallback?.(error);
        throw error;
      }
    }
    return registration.entry.callbackResult;
  };

  return {
    canUseTool: async (toolName, input, callbackOptions) => {
      const providerSessionId = requireProviderSessionId(
        'claude_agent_sdk',
        sessionState.providerSessionId,
      );
      const interaction = claudeCanUseToolInteraction(
        toolName,
        input,
        callbackOptions,
        operation,
        providerSessionId,
      );
      return registerCallback({
        providerRequestId: callbackOptions.requestId,
        interaction,
        rawProviderPayload: {
          type: 'can_use_tool',
          request_id: callbackOptions.requestId,
          tool_name: toolName,
          tool_use_id: callbackOptions.toolUseID,
          input,
        },
        signal: callbackOptions.signal,
        compileResolution: (resolution) => compileClaudeCanUseToolResolution(
          interaction,
          input,
          callbackOptions,
          resolution,
        ),
        interactionHint: interaction.category === 'approval'
          ? 'approval_required'
          : 'user_input_required',
      });
    },
    onElicitation: async (request, callbackOptions) => {
      const providerSessionId = requireProviderSessionId(
        'claude_agent_sdk',
        sessionState.providerSessionId,
      );
      const interaction = claudeElicitationInteraction(
        request,
        operation,
        providerSessionId,
      );
      if (!interaction) {
        return { action: 'cancel' };
      }
      return registerCallback({
        providerRequestId: request.elicitationId,
        interaction,
        rawProviderPayload: {
          type: 'mcp_elicitation',
          elicitation_id: request.elicitationId,
          request,
        },
        signal: callbackOptions.signal,
        compileResolution: (resolution) => compileClaudeElicitationResolution(
          interaction,
          resolution,
        ),
        interactionHint: 'user_input_required',
      });
    },
  };
}

async function invokeClaudeModelChat(
  prompt,
  operation,
  packageName,
  activity,
  streamOptions = null,
) {
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
    const stream = streamOptions
      ? createProviderStreamEmitter('claude-code', operation, streamOptions)
      : null;
    const sessionState = { providerSessionId: requestedProviderSessionId };
    const options = {
      cwd: resolveProviderWorkingDirectory(operation),
      abortController,
      ...(modelId ? { model: modelId } : {}),
      ...(requestedProviderSessionId ? { resume: requestedProviderSessionId } : {}),
      ...(streamOptions ? { includePartialMessages: true } : {}),
      ...permissionSettings,
      ...createClaudeSdkInteractionCallbacks(operation, sessionState, stream, activity),
    };
    const projection = stream ? createClaudeStreamProjection(stream) : null;
    let text = '';
    let completed = false;
    let idleObserved = false;
    let resultText = null;
    let providerSessionId = null;

    try {
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
          sessionState.providerSessionId = providerSessionId;
          await activity.establish(providerSessionId);
        }
        if (event?.type === 'assistant' || event?.type === 'tool_use') {
          await activity.working(providerSessionId);
        }
        if (projection) {
          const projected = await projection.push(event, providerSessionId);
          idleObserved ||= projected.idle;
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
      if (projection) {
        await projection.complete(providerSessionId, {
          type: 'result',
          subtype: 'success',
          result: output,
          session_id: providerSessionId,
          idle_observed: idleObserved,
        });
      }
      const result = liveSuccess(output, operation, {
        package: packageName,
        provider_session_id: verifiedSessionId,
        [VERIFIED_PROVIDER_SESSION_ID]: true,
      });
      return stream ? { ...result, chunks: stream.chunks } : result;
    } finally {
      clearPendingSdkInteractions(
        operation.model_request_id,
        'Claude turn ended before the interaction was resolved',
      );
    }
  });
}

function createClaudeStreamProjection(stream) {
  const blocks = new Map();
  const tools = new Map();
  let currentMessageId = null;
  let turnStarted = false;
  let terminal = false;

  const emitTurnStarted = async (event, providerSessionId) => {
    if (turnStarted) {
      return;
    }
    turnStarted = true;
    await stream.event('turn.started', null, event, providerSessionId);
  };

  const blockKey = (event, index) => {
    const messageId = optionalProviderIdentifier(event?.uuid)
      ?? currentMessageId
      ?? `message.${stream.modelRequestId}`;
    return `${messageId}:block:${Number.isSafeInteger(index) ? index : 0}`;
  };

  const emitBlock = async (lifecycle, block, rawEvent, providerSessionId) => {
    if (!block || block.completed) {
      return;
    }
    if (lifecycle === 'item.completed') {
      block.completed = true;
    }
    await stream.event(lifecycle, block.item, rawEvent, providerSessionId);
  };

  const ensureContentBlock = async (
    event,
    index,
    contentBlock,
    providerSessionId,
  ) => {
    const id = optionalProviderIdentifier(contentBlock?.id) ?? blockKey(event, index);
    let block = blocks.get(id);
    if (block) {
      return block;
    }
    const item = claudeContentBlockItem(id, contentBlock);
    if (!item) {
      return null;
    }
    block = { id, item, completed: false, inputJson: '' };
    blocks.set(id, block);
    if (item.type === 'mcp_tool_call' || item.type === 'command_execution'
      || item.type === 'file_change' || item.type === 'web_search') {
      tools.set(id, block);
    }
    await emitBlock('item.started', block, event, providerSessionId);
    return block;
  };

  const pushPartial = async (event, providerSessionId) => {
    const nativeEvent = event?.event;
    const nativeType = nativeEvent?.type;
    if (nativeType === 'message_start') {
      currentMessageId = optionalProviderIdentifier(nativeEvent?.message?.id)
        ?? optionalProviderIdentifier(event?.uuid)
        ?? currentMessageId;
      await emitTurnStarted(event, providerSessionId);
      return;
    }
    const index = nativeEvent?.index;
    if (nativeType === 'content_block_start') {
      await ensureContentBlock(
        event,
        index,
        nativeEvent?.content_block,
        providerSessionId,
      );
      return;
    }
    if (nativeType === 'content_block_delta') {
      const key = blockKey(event, index);
      const block = blocks.get(key);
      if (!block) {
        return;
      }
      const delta = nativeEvent?.delta;
      if (delta?.type === 'text_delta' && typeof delta.text === 'string') {
        block.item.text += delta.text;
        await stream.chunk(delta.text);
      } else if (delta?.type === 'thinking_delta' && typeof delta.thinking === 'string') {
        block.item.text += delta.thinking;
      } else if (
        delta?.type === 'input_json_delta'
        && typeof delta.partial_json === 'string'
      ) {
        block.inputJson += delta.partial_json;
        block.item.arguments = parsePartialJson(block.inputJson);
      }
      await emitBlock('item.updated', block, event, providerSessionId);
      return;
    }
    if (nativeType === 'content_block_stop') {
      const block = blocks.get(blockKey(event, index));
      if (block && !isToolItem(block.item)) {
        await emitBlock('item.completed', block, event, providerSessionId);
      }
    }
  };

  const pushAssistant = async (event, providerSessionId) => {
    const content = Array.isArray(event?.message?.content) ? event.message.content : [];
    for (const [index, part] of content.entries()) {
      const block = await ensureContentBlock(event, index, part, providerSessionId);
      if (!block) {
        continue;
      }
      if (part?.type === 'text' && typeof part.text === 'string') {
        const delta = monotonicTextDelta(block.item.text, part.text, 'claude_text');
        if (delta) {
          block.item.text = part.text;
          await stream.chunk(delta);
          await emitBlock('item.updated', block, event, providerSessionId);
        }
        await emitBlock('item.completed', block, event, providerSessionId);
      } else if (part?.type === 'thinking' && typeof part.thinking === 'string') {
        monotonicTextDelta(block.item.text, part.thinking, 'claude_reasoning');
        block.item.text = part.thinking;
        await emitBlock('item.completed', block, event, providerSessionId);
      } else if (part?.type === 'tool_use') {
        block.item = normalizeProviderToolItem(
          'claude_code',
          block.id,
          part.name,
          part.input,
          'running',
        );
        tools.set(block.id, block);
        await emitBlock('item.updated', block, event, providerSessionId);
      }
    }
  };

  const pushToolResults = async (event, providerSessionId) => {
    const content = Array.isArray(event?.message?.content) ? event.message.content : [];
    for (const part of content) {
      if (part?.type !== 'tool_result') {
        continue;
      }
      const toolId = optionalProviderIdentifier(part.tool_use_id);
      if (!toolId) {
        continue;
      }
      const block = tools.get(toolId) ?? {
        id: toolId,
        item: normalizeProviderToolItem('claude_code', toolId, 'tool', {}, 'running'),
        completed: false,
        inputJson: '',
      };
      if (!tools.has(toolId)) {
        tools.set(toolId, block);
        blocks.set(toolId, block);
        await emitBlock('item.started', block, event, providerSessionId);
      }
      block.item = {
        ...block.item,
        status: part.is_error === true ? 'failed' : 'completed',
        result: part.content ?? event?.tool_use_result ?? null,
        ...(part.is_error === true ? { error: providerErrorPayload(part.content) } : {}),
      };
      await emitBlock('item.completed', block, event, providerSessionId);
    }
  };

  return {
    async push(event, providerSessionId) {
      if (terminal) {
        return { idle: false };
      }
      if (event?.type === 'stream_event') {
        await pushPartial(event, providerSessionId);
      } else if (event?.type === 'assistant') {
        await pushAssistant(event, providerSessionId);
      } else if (event?.type === 'user') {
        await pushToolResults(event, providerSessionId);
      } else if (event?.type === 'tool_progress') {
        const toolId = optionalProviderIdentifier(event.tool_use_id);
        const block = toolId ? tools.get(toolId) : null;
        if (block) {
          block.item = {
            ...block.item,
            status: 'running',
            elapsedTimeSeconds: event.elapsed_time_seconds ?? null,
          };
          await emitBlock('item.updated', block, event, providerSessionId);
        }
      } else if (event?.type === 'system' && event?.subtype === 'permission_denied') {
        const toolId = optionalProviderIdentifier(event.tool_use_id);
        const block = toolId ? tools.get(toolId) : null;
        if (block) {
          block.item = {
            ...block.item,
            status: 'failed',
            error: providerErrorPayload(event.message),
          };
          await emitBlock('item.completed', block, event, providerSessionId);
        }
      } else if (
        event?.type === 'system'
        && event?.subtype === 'session_state_changed'
      ) {
        if (event.state === 'running') {
          await emitTurnStarted(event, providerSessionId);
        }
        return { idle: event.state === 'idle' };
      }
      return { idle: false };
    },
    async complete(providerSessionId, rawEvent) {
      if (terminal) {
        return;
      }
      for (const block of blocks.values()) {
        if (block.completed) {
          continue;
        }
        if (isToolItem(block.item)) {
          block.item = { ...block.item, status: 'completed' };
        }
        if (block.item.type === 'agent_message' && !block.item.text) {
          continue;
        }
        await emitBlock('item.completed', block, rawEvent, providerSessionId);
      }
      await stream.terminal('turn.completed', rawEvent, providerSessionId);
      terminal = true;
    },
  };
}

function claudeContentBlockItem(id, contentBlock) {
  if (contentBlock?.type === 'text') {
    return { id, type: 'agent_message', text: String(contentBlock.text ?? '') };
  }
  if (contentBlock?.type === 'thinking' || contentBlock?.type === 'redacted_thinking') {
    return {
      id,
      type: 'reasoning',
      text: String(contentBlock.thinking ?? ''),
      redacted: contentBlock.type === 'redacted_thinking',
    };
  }
  if (contentBlock?.type === 'tool_use') {
    return normalizeProviderToolItem(
      'claude_code',
      optionalProviderIdentifier(contentBlock.id) ?? id,
      contentBlock.name,
      contentBlock.input,
      'running',
    );
  }
  return null;
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

async function invokeOpencodeModelChat(
  prompt,
  operation,
  packageName,
  activity,
  streamOptions = null,
) {
  // Prefer the official v2 surface (durable `/api/...` routes); fall back to
  // the package root for SDK versions that do not ship the `./v2` export.
  let moduleNamespace;
  try {
    moduleNamespace = await loadPackageExport(packageName, './v2');
  } catch {
    moduleNamespace = await loadPackage(packageName);
  }

  return runProviderOperation(operation, 'opencode_sdk', async (abortController) => {
    const workingDirectory = resolveProviderWorkingDirectory(operation);
    return withOpencodeClient(
      moduleNamespace,
      workingDirectory,
      abortController.signal,
      (client) => invokeOpencodeClient(
        client,
        prompt,
        operation,
        packageName,
        abortController.signal,
        activity,
        streamOptions,
      ),
    );
  });
}

async function withOpencodeClient(moduleNamespace, workingDirectory, signal, invoke) {
  const createOpencode = moduleNamespace.createOpencode;
  const createOpencodeClient = moduleNamespace.createOpencodeClient;
  const createOpencodeServer = moduleNamespace.createOpencodeServer;
  const baseUrl = process.env.OPENCODE_SERVER_URL?.trim();
  if (baseUrl) {
    if (typeof createOpencodeClient !== 'function') {
      throw new Error('opencode sdk missing createOpencodeClient() for OPENCODE_SERVER_URL');
    }
    return invoke(createOpencodeClient({ baseUrl, directory: workingDirectory }));
  }

  if (typeof createOpencodeServer === 'function' && typeof createOpencodeClient === 'function') {
    const server = await createOpencodeServer({ signal });
    try {
      return await invoke(createOpencodeClient({
        baseUrl: server?.url,
        directory: workingDirectory,
      }));
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
    const { client, server } = await createOpencode({ signal });
    try {
      return await invoke(client);
    } finally {
      await server?.close?.();
    }
  }

  throw new Error('opencode sdk missing createOpencodeServer/createOpencodeClient session entrypoints');
}

const SESSION_DISCOVERY_OPERATIONS = new Set(['session_list', 'session_history']);
const MAX_SESSION_DISCOVERY_LIMIT = 200;

export async function invokeSessionDiscoveryRuntime(packageName, operation) {
  const operationName = requiredOperationString(operation?.operation, 'operation');
  if (!SESSION_DISCOVERY_OPERATIONS.has(operationName)) {
    throw new Error(`unsupported provider Session discovery operation: ${operationName}`);
  }
  const limit = providerSessionDiscoveryLimit(operation?.limit);
  const workingDirectory = optionalOperationString(
    operation?.working_directory,
    'working_directory',
  );
  const providerSessionId = operationName === 'session_history'
    ? requiredOperationString(operation?.provider_session_id, 'provider_session_id')
    : null;
  const cursor = optionalOpaqueOperationString(operation?.cursor, 'cursor');

  return runProviderOperation(operation, 'provider_session_discovery', async (abortController) => {
    if (packageName === '@anthropic-ai/claude-agent-sdk') {
      const moduleNamespace = await loadPackage(packageName);
      const offset = sessionDiscoveryOffset(
        cursor,
        packageName,
        operationName,
        providerSessionId,
        workingDirectory,
        limit,
      );
      return invokeClaudeSessionDiscovery(
        moduleNamespace,
        operationName,
        providerSessionId,
        workingDirectory,
        limit,
        offset,
        packageName,
      );
    }
    if (packageName === '@opencode-ai/sdk') {
      const moduleNamespace = await loadPackageExport(packageName, './v2');
      return withOpencodeClient(
        moduleNamespace,
        workingDirectory,
        abortController.signal,
        (client) => invokeOpencodeSessionDiscovery(
          client,
          operationName,
          providerSessionId,
          workingDirectory,
          limit,
          cursor,
          packageName,
          abortController.signal,
        ),
      );
    }
    throw new Error(`no live provider Session discovery handler for package ${packageName}`);
  });
}

async function invokeClaudeSessionDiscovery(
  moduleNamespace,
  operationName,
  providerSessionId,
  workingDirectory,
  limit,
  offset,
  packageName,
) {
  const options = {
    ...(workingDirectory ? { dir: workingDirectory } : {}),
    limit,
    ...(offset > 0 ? { offset } : {}),
  };
  if (operationName === 'session_list') {
    if (typeof moduleNamespace.listSessions !== 'function') {
      throw new Error('claude agent sdk is missing listSessions()');
    }
    const sessions = await moduleNamespace.listSessions(options);
    if (!Array.isArray(sessions)) {
      throw new Error('claude agent sdk listSessions() returned a non-array result');
    }
    return sessionDiscoveryPage(
      packageName,
      operationName,
      providerSessionId,
      sessions.map((session) => normalizeClaudeProviderSession(session, workingDirectory)),
      offset,
      limit,
      sessions.length === limit,
      workingDirectory,
    );
  }

  if (typeof moduleNamespace.getSessionMessages !== 'function') {
    throw new Error('claude agent sdk is missing getSessionMessages()');
  }
  const messages = await moduleNamespace.getSessionMessages(providerSessionId, {
    ...options,
    includeSystemMessages: true,
  });
  if (!Array.isArray(messages)) {
    throw new Error('claude agent sdk getSessionMessages() returned a non-array result');
  }
  return sessionDiscoveryPage(
    packageName,
    operationName,
    providerSessionId,
    messages.map((message) => normalizeClaudeProviderMessage(
      message,
      providerSessionId,
    )),
    offset,
    limit,
    messages.length === limit,
    workingDirectory,
  );
}

async function invokeOpencodeSessionDiscovery(
  client,
  operationName,
  providerSessionId,
  workingDirectory,
  limit,
  cursor,
  packageName,
  signal,
) {
  const sessionClient = client?.v2?.session ?? client?.session;
  if (operationName === 'session_list') {
    if (typeof sessionClient?.list !== 'function') {
      throw new Error('opencode v2 sdk client is missing session.list');
    }
    const response = await sessionClient.list({
      ...(workingDirectory ? { directory: workingDirectory } : {}),
      limit,
      ...(cursor ? { cursor } : { order: 'desc' }),
    }, { signal });
    const page = readOpencodeResponsePage(response, 'session.list');
    return {
      ok: true,
      mode: 'sdk_live',
      package: packageName,
      operation: operationName,
      items: page.items.map(normalizeOpencodeProviderSession),
      ...(page.previous_cursor ? { previous_cursor: page.previous_cursor } : {}),
      ...(page.next_cursor ? { next_cursor: page.next_cursor } : {}),
    };
  }

  if (typeof sessionClient?.messages !== 'function') {
    throw new Error('opencode v2 sdk client is missing session.messages');
  }
  const response = await sessionClient.messages({
    sessionID: providerSessionId,
    limit,
    ...(cursor ? { cursor } : { order: 'asc' }),
  }, { signal });
  const page = readOpencodeResponsePage(response, 'session.messages');
  return {
    ok: true,
    mode: 'sdk_live',
    package: packageName,
    operation: operationName,
    provider_session_id: providerSessionId,
    items: page.items.map((message) => normalizeOpencodeProviderMessage(
      message,
      providerSessionId,
    )),
    ...(page.previous_cursor ? { previous_cursor: page.previous_cursor } : {}),
    ...(page.next_cursor ? { next_cursor: page.next_cursor } : {}),
  };
}

function sessionDiscoveryPage(
  packageName,
  operationName,
  providerSessionId,
  items,
  offset,
  limit,
  hasMore,
  workingDirectory,
) {
  return {
    ok: true,
    mode: 'sdk_live',
    package: packageName,
    operation: operationName,
    ...(providerSessionId ? { provider_session_id: providerSessionId } : {}),
    items,
    ...(hasMore
      ? {
          next_cursor: encodeSessionDiscoveryCursor(
            packageName,
            operationName,
            providerSessionId,
            offset + items.length,
            workingDirectory,
            limit,
          ),
        }
      : {}),
    ...(offset > 0
      ? {
          previous_cursor: encodeSessionDiscoveryCursor(
            packageName,
            operationName,
            providerSessionId,
            Math.max(0, offset - limit),
            workingDirectory,
            limit,
          ),
        }
      : {}),
  };
}

function providerSessionDiscoveryLimit(value) {
  if (!Number.isSafeInteger(value) || value <= 0 || value > MAX_SESSION_DISCOVERY_LIMIT) {
    throw new Error(`limit must be an integer between 1 and ${MAX_SESSION_DISCOVERY_LIMIT}`);
  }
  return value;
}

function encodeSessionDiscoveryCursor(
  packageName,
  operationName,
  providerSessionId,
  offset,
  workingDirectory,
  limit,
) {
  const payload = JSON.stringify({
    version: 1,
    package: packageName,
    operation: operationName,
    provider_session_id: providerSessionId,
    working_directory: workingDirectory,
    limit,
    offset,
  });
  return `sdkwork-session-v1.${Buffer.from(payload, 'utf8').toString('base64url')}`;
}

function sessionDiscoveryOffset(
  cursor,
  packageName,
  operationName,
  providerSessionId,
  workingDirectory,
  limit,
) {
  if (!cursor) {
    return 0;
  }
  const prefix = 'sdkwork-session-v1.';
  if (!cursor.startsWith(prefix)) {
    throw new Error('cursor is not an SDKWork provider Session cursor');
  }
  let payload;
  try {
    payload = JSON.parse(Buffer.from(cursor.slice(prefix.length), 'base64url').toString('utf8'));
  } catch {
    throw new Error('cursor payload is not valid base64url JSON');
  }
  if (
    payload?.version !== 1 ||
    payload?.package !== packageName ||
    payload?.operation !== operationName ||
    payload?.provider_session_id !== providerSessionId ||
    payload?.working_directory !== workingDirectory ||
    payload?.limit !== limit ||
    !Number.isSafeInteger(payload?.offset) ||
    payload.offset < 0
  ) {
    throw new Error(
      `cursor does not match the requested ${operationName} provider Session operation`,
    );
  }
  return payload.offset;
}

function normalizeClaudeProviderSession(session, requestedWorkingDirectory) {
  const providerSessionId = requiredProviderString(
    'claude_agent_sdk',
    session?.sessionId,
    'sessionId',
  );
  const summary = optionalProviderString(session?.summary);
  const firstPrompt = optionalProviderString(session?.firstPrompt);
  const title = optionalProviderString(session?.customTitle) ?? summary ?? firstPrompt;
  const metadata = providerMetadata({
    git_branch: optionalProviderString(session?.gitBranch),
    tag: optionalProviderString(session?.tag),
  });
  const createdAt = providerTimestamp(session?.createdAt, 'claude session createdAt');
  const cwd = optionalProviderString(session?.cwd)
    ?? optionalProviderString(requestedWorkingDirectory);
  return {
    provider_session_id: providerSessionId,
    ...(title ? { title } : {}),
    ...(firstPrompt ? { preview: firstPrompt } : {}),
    ...(summary ? { summary } : {}),
    ...(createdAt ? { created_at: createdAt } : {}),
    updated_at: requiredProviderTimestamp(
      session?.lastModified,
      'claude session lastModified',
    ),
    ...(cwd ? { cwd } : {}),
    ...(metadata ? { metadata } : {}),
  };
}

function normalizeClaudeProviderMessage(message, requestedProviderSessionId) {
  const providerSessionId = verifyProviderSessionId(
    'claude_agent_sdk',
    message?.session_id,
    requestedProviderSessionId,
  );
  const providerMessageId = requiredProviderString(
    'claude_agent_sdk',
    message?.uuid,
    'message uuid',
  );
  const role = normalizeProviderMessageRole(
    message?.message?.role ?? message?.type,
    'claude_agent_sdk',
  );
  const content = message?.message?.content ?? message?.message ?? null;
  const metadata = providerMetadata({
    parent_tool_use_id: optionalProviderString(message?.parent_tool_use_id),
    parent_agent_id: optionalProviderString(message?.parent_agent_id),
  });
  const createdAt = providerTimestamp(message?.timestamp, 'claude message timestamp');
  return {
    provider_message_id: providerMessageId,
    provider_session_id: providerSessionId,
    role,
    parts: normalizeClaudeMessageParts(providerMessageId, content),
    ...(createdAt ? { created_at: createdAt } : {}),
    ...(metadata ? { metadata } : {}),
  };
}

function normalizeOpencodeProviderSession(session) {
  const inputTokens = providerTokenCount(
    session?.tokens?.input,
    'opencode session tokens.input',
  );
  const outputTokens = providerTokenCount(
    session?.tokens?.output,
    'opencode session tokens.output',
  );
  const reasoningTokens = providerTokenCount(
    session?.tokens?.reasoning,
    'opencode session tokens.reasoning',
  );
  const cachedReadTokens = providerTokenCount(
    session?.tokens?.cache?.read,
    'opencode session tokens.cache.read',
  );
  const cachedWriteTokens = providerTokenCount(
    session?.tokens?.cache?.write,
    'opencode session tokens.cache.write',
  );
  const metadata = providerMetadata({
    project_id: optionalProviderString(session?.projectID),
    workspace_id: optionalProviderString(session?.location?.workspaceID),
    agent: optionalProviderString(session?.agent),
    cache_write_tokens: cachedWriteTokens,
  });
  const parentProviderSessionId = optionalProviderString(session?.parentID);
  const title = optionalProviderString(session?.title);
  const cwd = optionalProviderString(session?.location?.directory);
  const model = optionalProviderString(session?.model?.id);
  const modelProvider = optionalProviderString(session?.model?.providerID);
  const archivedAt = providerTimestamp(
    session?.time?.archived,
    'opencode session time.archived',
  );
  const costCents = providerCostCents(session?.cost, 'opencode session cost');
  return {
    provider_session_id: requiredProviderString('opencode_sdk', session?.id, 'session id'),
    ...(parentProviderSessionId
      ? { parent_provider_session_id: parentProviderSessionId }
      : {}),
    ...(title ? { title } : {}),
    created_at: requiredProviderTimestamp(
      session?.time?.created,
      'opencode session time.created',
    ),
    updated_at: requiredProviderTimestamp(
      session?.time?.updated,
      'opencode session time.updated',
    ),
    ...(archivedAt ? { archived_at: archivedAt } : {}),
    ...(cwd ? { cwd } : {}),
    ...(model ? { model } : {}),
    ...(modelProvider ? { model_provider: modelProvider } : {}),
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    cached_tokens: cachedReadTokens,
    reasoning_tokens: reasoningTokens,
    ...(costCents == null ? {} : { cost_cents: costCents }),
    additions: providerTokenCount(
      session?.summary?.additions,
      'opencode session summary.additions',
    ),
    deletions: providerTokenCount(
      session?.summary?.deletions,
      'opencode session summary.deletions',
    ),
    files_changed: providerTokenCount(
      session?.summary?.files,
      'opencode session summary.files',
    ),
    ...(metadata ? { metadata } : {}),
  };
}

function normalizeOpencodeProviderMessage(message, requestedProviderSessionId) {
  const providerSessionId = optionalProviderString(message?.sessionID)
    ? verifyProviderSessionId(
        'opencode_sdk',
        message.sessionID,
        requestedProviderSessionId,
      )
    : requestedProviderSessionId;
  const providerMessageId = requiredProviderString(
    'opencode_sdk',
    message?.id,
    'message id',
  );
  return {
    provider_message_id: providerMessageId,
    provider_session_id: providerSessionId,
    role: normalizeOpencodeV2MessageRole(message?.type),
    parts: normalizeOpencodeV2MessageParts(providerMessageId, providerSessionId, message),
    created_at: requiredProviderTimestamp(
      message?.time?.created,
      'opencode message time.created',
    ),
  };
}

function normalizeOpencodeV2MessageRole(type) {
  if (type === 'assistant') {
    return 'agent';
  }
  if (type === 'user') {
    return 'user';
  }
  if (type === 'system' || type === 'compaction') {
    return 'system';
  }
  if (type === 'shell') {
    return 'tool';
  }
  if (['synthetic', 'agent-switched', 'model-switched'].includes(type)) {
    return 'adapter';
  }
  throw new Error(`opencode_sdk returned an unsupported message type: ${type ?? '<missing>'}`);
}

function normalizeOpencodeV2MessageParts(providerMessageId, providerSessionId, message) {
  if (message?.type === 'assistant') {
    return normalizeOpencodeMessageParts(providerMessageId, providerSessionId, message.content);
  }
  if (message?.type === 'user') {
    const parts = optionalProviderString(message.text)
      ? [{ id: `${providerMessageId}:text`, type: 'text', text: message.text }]
      : [];
    for (const [index, file] of (message.files ?? []).entries()) {
      parts.push({
        id: `${providerMessageId}:file:${index}`,
        type: 'file',
        url: file.uri,
        mime: file.mime,
        filename: file.name,
      });
    }
    return normalizeOpencodeMessageParts(providerMessageId, providerSessionId, parts);
  }
  if (message?.type === 'system' || message?.type === 'synthetic') {
    return normalizeOpencodeMessageParts(providerMessageId, providerSessionId, [{
      id: `${providerMessageId}:text`,
      type: 'text',
      text: message.text,
    }]);
  }
  if (message?.type === 'compaction') {
    return normalizeOpencodeMessageParts(providerMessageId, providerSessionId, [{
      id: `${providerMessageId}:summary`,
      type: 'text',
      text: message.summary,
      metadata: providerPartMetadata({ contentType: 'compaction' }),
    }]);
  }
  if (message?.type === 'shell') {
    return [{
      part_id: `${providerMessageId}:shell`,
      kind: 'tool_call_ref',
      tool_call_id: optionalProviderString(message.callID) ?? `${providerMessageId}:shell`,
      name: 'shell',
      json: message,
      metadata: providerPartMetadata({
        contentType: 'tool_result',
        status: message?.time?.completed ? 'completed' : 'running',
        hasResult: Boolean(message?.time?.completed),
      }),
    }];
  }
  if (message?.type === 'agent-switched' || message?.type === 'model-switched') {
    return [{
      part_id: `${providerMessageId}:event`,
      kind: 'text',
      text: message.type === 'agent-switched'
        ? `Agent switched to ${message.agent}`
        : `Model switched to ${message.model?.providerID}/${message.model?.id}`,
      metadata: providerPartMetadata({ contentType: message.type }),
    }];
  }
  return [{
    part_id: `${providerMessageId}:event`,
    kind: 'json',
    json: message,
  }];
}

function normalizeClaudeMessageParts(providerMessageId, content) {
  const parts = Array.isArray(content) ? content : content == null ? [] : [content];
  return parts.map((part, index) => {
    const partId = optionalProviderString(part?.id) ?? `${providerMessageId}:${index}`;
    if (typeof part === 'string') {
      return { part_id: partId, kind: 'text', text: part };
    }
    if (part?.type === 'text' && typeof part.text === 'string') {
      return { part_id: partId, kind: 'text', text: part.text };
    }
    if (part?.type === 'thinking' && typeof part.thinking === 'string') {
      return {
        part_id: partId,
        kind: 'text',
        text: part.thinking,
        metadata: providerPartMetadata({ contentType: 'thinking' }),
      };
    }
    if (part?.type === 'tool_use') {
      return {
        part_id: partId,
        kind: 'tool_call_ref',
        tool_call_id: optionalProviderString(part.id) ?? partId,
        ...(optionalProviderString(part.name) ? { name: part.name.trim() } : {}),
        ...(part.input == null ? {} : { json: part.input }),
        metadata: providerPartMetadata({ contentType: 'tool' }),
      };
    }
    if (part?.type === 'tool_result') {
      return {
        part_id: partId,
        kind: 'json',
        json: part,
        ...(optionalProviderString(part.tool_use_id)
          ? { tool_call_id: part.tool_use_id.trim() }
          : {}),
        metadata: providerPartMetadata({
          contentType: 'tool_result',
          status: part.is_error === true ? 'failed' : 'completed',
          hasResult: true,
        }),
      };
    }
    return {
      part_id: partId,
      kind: 'json',
      json: part,
      ...(optionalProviderString(part?.type)
        ? { metadata: providerPartMetadata({ contentType: part.type }) }
        : {}),
    };
  });
}

function normalizeOpencodeMessageParts(providerMessageId, providerSessionId, parts) {
  if (!Array.isArray(parts)) {
    return [];
  }
  return parts.map((part, index) => {
    if (optionalProviderString(part?.sessionID)) {
      verifyProviderSessionId('opencode_sdk', part.sessionID, providerSessionId);
    }
    const partMessageId = optionalProviderString(part?.messageID);
    if (partMessageId && partMessageId !== providerMessageId) {
      throw new Error(
        `opencode_sdk returned part for message ${partMessageId}, expected ${providerMessageId}`,
      );
    }
    const partId = optionalProviderString(part?.id) ?? `${providerMessageId}:${index}`;
    if (
      (part?.type === 'text' || part?.type === 'reasoning') &&
      typeof part.text === 'string'
    ) {
      const inheritedMetadata = part?.metadata
        && typeof part.metadata === 'object'
        && !Array.isArray(part.metadata)
        ? part.metadata
        : null;
      const metadata = part.type === 'reasoning'
        ? {
            ...(inheritedMetadata ?? {}),
            ...providerPartMetadata({ contentType: 'reasoning' }),
          }
        : inheritedMetadata;
      return {
        part_id: partId,
        kind: 'text',
        text: part.text,
        ...(metadata ? { metadata } : {}),
      };
    }
    if (part?.type === 'tool') {
      const status = optionalProviderString(part?.state?.status);
      const hasResult = status === 'completed' || status === 'error';
      return {
        part_id: partId,
        kind: 'tool_call_ref',
        tool_call_id: optionalProviderString(part.callID)
          ?? optionalProviderString(part.id)
          ?? partId,
        ...(optionalProviderString(part.tool ?? part.name)
          ? { name: optionalProviderString(part.tool ?? part.name) }
          : {}),
        json: part,
        metadata: providerPartMetadata({
          contentType: 'tool',
          status,
          hasResult,
        }),
      };
    }
    if (part?.type === 'file' && optionalProviderString(part.url)) {
      return {
        part_id: partId,
        kind: 'file_ref',
        content_ref: part.url.trim(),
        mime_type: optionalProviderString(part.mime) ?? 'application/octet-stream',
        ...(optionalProviderString(part.filename) ? { name: part.filename.trim() } : {}),
        metadata: providerPartMetadata({ contentType: 'file' }),
      };
    }
    return { part_id: partId, kind: 'json', json: part };
  });
}

function providerPartMetadata({ contentType, status, hasResult } = {}) {
  return providerMetadata({
    'sdkwork.provider.content_type': optionalProviderString(contentType),
    'sdkwork.provider.status': optionalProviderString(status),
    'sdkwork.provider.has_result': hasResult == null ? null : Boolean(hasResult),
  });
}

function providerTokenCount(value, fieldName) {
  if (value == null) {
    return 0;
  }
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${fieldName} must be a non-negative integer`);
  }
  return value;
}

function providerCostCents(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) {
    throw new Error(`${fieldName} must be a non-negative finite number`);
  }
  const cents = Math.round(value * 100);
  if (!Number.isSafeInteger(cents)) {
    throw new Error(`${fieldName} exceeds the supported cent range`);
  }
  return cents;
}

function readOpencodeResponsePage(response, action) {
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode ${action} failed: ${error}`);
  }
  const value = response?.data ?? response;
  if (!value || !Array.isArray(value.data) || !value.cursor || typeof value.cursor !== 'object') {
    throw new Error(`opencode ${action} returned an invalid paged result`);
  }
  return {
    items: value.data,
    previous_cursor: optionalOpaqueProviderString(value.cursor.previous),
    next_cursor: optionalOpaqueProviderString(value.cursor.next),
  };
}

function normalizeProviderMessageRole(value, provider) {
  const role = optionalProviderString(value);
  if (role === 'assistant' || role === 'model') {
    return 'agent';
  }
  if (['user', 'agent', 'system', 'tool', 'adapter'].includes(role)) {
    return role;
  }
  throw new Error(`${provider} returned an unsupported message role: ${role ?? '<missing>'}`);
}

function providerTimestamp(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value === 'string') {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      throw new Error(`${fieldName} must be an RFC 3339 timestamp`);
    }
    return date.toISOString();
  }
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${fieldName} must be an RFC 3339 timestamp or epoch millisecond integer`);
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`${fieldName} is outside the supported timestamp range`);
  }
  return date.toISOString();
}

function requiredProviderTimestamp(value, fieldName) {
  const timestamp = providerTimestamp(value, fieldName);
  if (!timestamp) {
    throw new Error(`${fieldName} is required`);
  }
  return timestamp;
}

function requiredProviderString(provider, value, fieldName) {
  const normalized = optionalProviderString(value);
  if (!normalized) {
    throw new Error(`${provider} returned an empty ${fieldName}`);
  }
  return normalized;
}

function optionalProviderString(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function optionalOpaqueProviderString(value) {
  return typeof value === 'string' && value.trim() ? value : null;
}

function providerMetadata(values) {
  const entries = Object.entries(values).filter(([, value]) => value != null);
  return entries.length > 0 ? Object.fromEntries(entries) : null;
}

const OPENCODE_SESSION_CONTROL_OPERATIONS = new Set([
  'session_interrupt',
  'session_compact',
  'session_fork',
]);

export async function invokeSessionControlRuntime(packageName, operation) {
  if (isCodexPackage(packageName)) {
    return controlCodexAppServerSession(operation);
  }
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
      const moduleNamespace = await loadPackageExport(packageName, './v2');
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
      const response = await client.session.fork(
        {
          sessionID: providerSessionId,
          ...(beforeMessageId ? { messageID: beforeMessageId } : {}),
        },
        { signal },
      );
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

function opencodeInteractionFromEvent(event, operation, sessionId) {
  const properties = opencodeEventData(event);
  const providerRequestId = requiredProviderString(
    'opencode sdk',
    properties.id,
    'interaction id',
  );
  const providerItemId = optionalProviderString(
    properties.tool?.messageID
      ?? properties.tool?.messageId
      ?? properties.messageID,
  );
  const providerToolCallId = optionalProviderString(
    properties.tool?.callID
      ?? properties.tool?.callId
      ?? properties.callID,
  );
  const providerToolName = optionalProviderString(
    properties.permission ?? properties.action,
  );
  if (
    event.type === 'question.asked'
    || event.type === 'question.v2.asked'
  ) {
    const questions = normalizedQuestionSet(
      properties.questions,
      providerRequestId,
      'opencode sdk',
    );
    return canonicalProviderInteraction({
      providerId: 'opencode',
      providerSessionId: sessionId,
      providerRequestId,
      providerInteractionId: providerRequestId,
      providerItemId,
      providerToolCallId,
      providerToolName,
      protocolMethod: event.type,
      operation,
      category: 'user_input',
      kind: 'question_set',
      prompt: questions[0].prompt,
      allowedActions: ['submit', 'cancel'],
      request: { questions },
    });
  }
  const permission = optionalProviderString(properties.permission)
    ?? optionalProviderString(properties.action)
    ?? 'tool';
  const patterns = Array.isArray(properties.patterns)
    ? properties.patterns
    : properties.resources ?? [];
  return canonicalProviderInteraction({
    providerId: 'opencode',
    providerSessionId: sessionId,
    providerRequestId,
    providerInteractionId: providerRequestId,
    providerItemId,
    providerToolCallId,
    providerToolName,
    protocolMethod: event.type,
    operation,
    category: 'approval',
    kind: 'permission_profile',
    prompt: `Allow ${permission}`,
    allowedActions: ['grant', 'decline', 'cancel'],
    request: {
      requestedPermissions: {
        permission,
        patterns,
        always: properties.always ?? properties.save ?? [],
        metadata: properties.metadata ?? {},
        tool: properties.tool ?? null,
      },
    },
  });
}

function compileOpencodeInteractionResolution(interaction, event, resolution) {
  const action = requiredOperationString(resolution.action, 'resolution.action');
  if (!interaction.allowedActions.includes(action)) {
    throw new Error(`unsupported OpenCode interaction action: ${action}`);
  }
  const properties = opencodeEventData(event);
  if (interaction.kind === 'question_set') {
    if (action === 'cancel') {
      return { reject: true };
    }
    const answers = resolution.answers;
    if (!answers || typeof answers !== 'object' || Array.isArray(answers)) {
      throw new Error('resolution.answers must be an object');
    }
    return {
      answers: interaction.request.questions.map((question) => {
        const value = answers[question.id];
        if (!Array.isArray(value) || value.length === 0) {
          throw new Error(`resolution.answers.${question.id} must be a non-empty array`);
        }
        return value.map(String);
      }),
    };
  }
  if (action === 'grant') {
    const scope = requiredOperationString(resolution.scope, 'resolution.scope');
    if (scope !== 'turn' && scope !== 'session') {
      throw new Error('resolution.scope must be turn or session');
    }
    return { reply: scope === 'session' ? 'always' : 'once' };
  }
  return { reply: 'reject', message: optionalProviderString(resolution.reason) ?? null };
}

async function sendOpencodeInteractionResolution(
  client,
  signal,
  event,
  interaction,
  response,
) {
  const properties = opencodeEventData(event);
  const providerRequestId = interaction.correlation.providerRequestId;
  let result;
  if (interaction.kind === 'question_set') {
    if (response.reject) {
      if (event.type === 'question.v2.asked') {
        result = await client.v2.session.question.reject(
          { sessionID: interaction.sessionId, requestID: String(providerRequestId) },
          { signal },
        );
      } else {
        result = await client.question.reject(
          { requestID: String(providerRequestId) },
          { signal },
        );
      }
    } else if (event.type === 'question.v2.asked') {
      result = await client.v2.session.question.reply(
        {
          sessionID: interaction.sessionId,
          requestID: String(providerRequestId),
          questionV2Reply: { answers: response.answers },
        },
        { signal },
      );
    } else {
      result = await client.question.reply(
        { requestID: String(providerRequestId), answers: response.answers },
        { signal },
      );
    }
  } else if (event.type === 'permission.v2.asked') {
    result = await client.v2.session.permission.reply(
      {
        sessionID: interaction.sessionId,
        requestID: String(providerRequestId),
        reply: response.reply,
        ...(response.message ? { message: response.message } : {}),
      },
      { signal },
    );
  } else if (event.type === 'permission.updated') {
    result = await client.permission.respond(
      {
        sessionID: interaction.sessionId,
        permissionID: String(properties.id),
        response: response.reply,
      },
      { signal },
    );
  } else {
    result = await client.permission.reply(
      {
        requestID: String(providerRequestId),
        reply: response.reply,
        ...(response.message ? { message: response.message } : {}),
      },
      { signal },
    );
  }
  const error = readProviderError(result?.error);
  if (error) {
    throw new Error(`opencode ${event.type} interaction response failed: ${error}`);
  }
  return response;
}

async function registerOpencodeInteraction(
  client,
  signal,
  operation,
  sessionId,
  stream,
  activity,
  event,
) {
  if (!stream) {
    throw new Error('opencode interactive requests require model_chat_stream');
  }
  const interaction = opencodeInteractionFromEvent(event, operation, sessionId);
  const providerRequestId = interaction.correlation.providerRequestId;
  const registration = registerPendingSdkInteraction({
    providerId: 'opencode',
    modelRequestId: stream.modelRequestId,
    sessionId: requiredOperationString(operation.session_id, 'session_id'),
    turnId: requiredOperationString(operation.turn_id, 'turn_id'),
    providerSessionId: sessionId,
    providerTurnId: null,
    providerRequestId,
    interaction,
    signal,
    sendResolution: (resolution) => sendOpencodeInteractionResolution(
      client,
      signal,
      event,
      interaction,
      compileOpencodeInteractionResolution(interaction, event, resolution),
    ),
  });
  if (!registration.created) {
    return;
  }
  try {
    await activity.waiting(
      sessionId,
      interaction.category === 'approval' ? 'approval_required' : 'user_input_required',
    );
    await stream.event(
      'interaction.requested',
      null,
      { ...event, interaction },
      sessionId,
      { interaction },
    );
  } catch (error) {
    removePendingSdkInteraction(registration.entry);
    throw error;
  }
}

async function invokeOpencodeClient(
  client,
  prompt,
  operation,
  packageName,
  signal,
  activity,
  streamOptions = null,
) {
  // Prefer the durable v2 surface (`/api/session/{id}/prompt` + `/api/event`)
  // when the SDK ships it; fall back to the legacy v1 routes.
  const v2Durable = Boolean(
    client?.v2?.session?.prompt && client?.v2?.event?.subscribe,
  );
  if (
    !client?.session?.create
    || (!v2Durable && (!client?.session?.prompt || !client?.event?.subscribe))
  ) {
    throw new Error(
      'opencode sdk client is missing session.create/session.prompt/event.subscribe',
    );
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
  const stream = streamOptions
    ? createProviderStreamEmitter('opencode', operation, streamOptions)
    : null;
  const subscription = v2Durable
    ? await client.v2.event.subscribe({}, { signal })
    : await client.event.subscribe({}, { signal });
  if (!subscription?.stream || typeof subscription.stream[Symbol.asyncIterator] !== 'function') {
    throw new Error('opencode event.subscribe() returned an invalid event stream');
  }
  const eventProjection = createOpencodeEventProjection(
    sessionId,
    stream,
    activity,
    client,
    signal,
    operation,
  );
  const eventCompletion = consumeOpencodeSessionEvents(
    subscription.stream,
    eventProjection,
  );
  const promptRequest = v2Durable
    ? client.v2.session.prompt(
        {
          sessionID: sessionId,
          id: buildOpencodeV2PromptId(operation),
          prompt: { text: buildOpencodeV2PromptText(operation) },
          delivery: 'steer',
          resume: Boolean(requestedProviderSessionId),
        },
        { signal },
      )
    : client.session.prompt(
        {
          sessionID: sessionId,
          ...buildOpencodePromptBody(operation),
        },
        { signal },
      );
  let response;
  try {
    [response] = await Promise.all([promptRequest, eventCompletion]);
  } finally {
    await subscription.stream.return?.();
    clearPendingSdkInteractions(
      operation.model_request_id,
      'OpenCode turn ended before the interaction was resolved',
    );
  }
  const error = readProviderError(response?.error);
  if (error) {
    throw new Error(`opencode session.prompt failed: ${error}`);
  }
  await eventProjection.includeResponse(response, sessionId);
  await eventProjection.complete(sessionId);
  const text =
    extractTextParts(response?.data?.parts)
    || extractTextParts(response?.parts)
    || String(response?.data?.content ?? response?.content ?? '')
    || eventProjection.assistantText();
  if (!text.trim()) {
    throw new Error('opencode session.prompt completed without assistant content');
  }
  const result = liveSuccess(text, operation, {
    package: packageName,
    provider_session_id: sessionId,
    [VERIFIED_PROVIDER_SESSION_ID]: true,
  });
  return stream ? { ...result, chunks: stream.chunks } : result;
}

function buildOpencodeV2PromptId(operation) {
  const requestId = optionalOperationString(operation.model_request_id, 'model_request_id');
  const suffix = requestId
    ? requestId.replace(/[^A-Za-z0-9_-]/gu, '-')
    : `turn_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
  return suffix.startsWith('msg_') ? suffix : `msg_${suffix}`;
}

function buildOpencodeV2PromptText(operation) {
  return resolveOpencodePromptParts(operation)
    .map((part) => {
      if (part?.type === 'text' && typeof part.text === 'string') {
        return part.text;
      }
      return part?.url ? `[image: ${part.url}]` : '[attachment]';
    })
    .join('\n');
}

async function consumeOpencodeSessionEvents(eventStream, projection) {
  for await (const envelope of eventStream) {
    const event = unwrapOpencodeEvent(envelope);
    if (!event || !projection.belongsToSession(event)) {
      continue;
    }
    if (await projection.push(event)) {
      return;
    }
  }
  throw new Error('opencode event stream ended without a matching session.idle event');
}

function createOpencodeEventProjection(sessionId, stream, activity, client, signal, operation) {
  const messageRoles = new Map();
  const parts = new Map();
  let turnObserved = false;
  let idleEvent = null;
  let latestAssistantMessageId = null;

  const emitPart = async (lifecycle, partState, rawEvent) => {
    if (!stream || !partState || partState.completed) {
      return;
    }
    if (lifecycle === 'item.completed') {
      partState.completed = true;
    }
    await stream.event(lifecycle, partState.item, rawEvent, sessionId);
  };

  const updatePart = async (part, delta, rawEvent) => {
    const partId = optionalProviderIdentifier(part?.id);
    const messageId = optionalProviderIdentifier(part?.messageID);
    if (!partId || !messageId || messageRoles.get(messageId) !== 'assistant') {
      return;
    }
    const normalized = normalizeOpencodeStreamPart(part);
    if (!normalized) {
      return;
    }
    let state = parts.get(partId);
    if (!state) {
      state = { item: normalized, messageId, completed: false };
      parts.set(partId, state);
      await emitPart('item.started', state, rawEvent);
      if (normalized.type === 'agent_message' && normalized.text) {
        const initialDelta = typeof delta === 'string' && delta
          ? delta
          : normalized.text;
        await stream?.chunk(initialDelta);
        state.emittedInitialText = true;
      }
    } else if (normalized.type === 'agent_message' || normalized.type === 'reasoning') {
      const nextText = normalized.text;
      const emittedDelta = monotonicTextDelta(
        state.item.text,
        nextText,
        `opencode_${normalized.type}`,
      );
      state.item = normalized;
      if (normalized.type === 'agent_message' && emittedDelta) {
        await stream?.chunk(emittedDelta);
      }
    } else {
      state.item = normalized;
    }
    const terminal = isTerminalProviderItemStatus(normalized.status);
    await emitPart(terminal ? 'item.completed' : 'item.updated', state, rawEvent);
  };

  const completeMessageParts = async (messageId, rawEvent) => {
    for (const state of parts.values()) {
      if (state.messageId !== messageId || state.completed) {
        continue;
      }
      if (isToolItem(state.item) && !isTerminalProviderItemStatus(state.item.status)) {
        state.item = { ...state.item, status: 'completed' };
      }
      await emitPart('item.completed', state, rawEvent);
    }
  };

  return {
    belongsToSession(event) {
      const candidate = opencodeEventSessionId(event);
      return candidate == null || candidate === sessionId;
    },
    async push(event) {
      const properties = opencodeEventData(event);
      if (event?.type === 'message.updated') {
        const info = properties.info;
        const messageId = optionalProviderIdentifier(info?.id);
        if (messageId && (info?.role === 'assistant' || info?.role === 'user')) {
          messageRoles.set(messageId, info.role);
        }
        if (info?.role === 'assistant') {
          latestAssistantMessageId = messageId;
          turnObserved = true;
          await activity.working(sessionId);
          if (info?.time?.completed != null && messageId) {
            await completeMessageParts(messageId, event);
          }
        }
      } else if (event?.type === 'message.part.updated') {
        turnObserved = true;
        await updatePart(properties.part, properties.delta, event);
      } else if (event?.type === 'session.status') {
        const status = properties.status?.type;
        if (status === 'busy' || status === 'retry') {
          turnObserved = true;
          await activity.working(sessionId);
          await stream?.event(
            status === 'busy' ? 'turn.started' : 'turn.updated',
            null,
            event,
            sessionId,
          );
        }
      } else if (
        event?.type === 'permission.updated'
        || event?.type === 'permission.asked'
        || event?.type === 'permission.v2.asked'
      ) {
        turnObserved = true;
        await registerOpencodeInteraction(
          client,
          signal,
          operation,
          sessionId,
          stream,
          activity,
          event,
        );
      } else if (
        event?.type === 'question.asked'
        || event?.type === 'question.v2.asked'
      ) {
        turnObserved = true;
        await registerOpencodeInteraction(
          client,
          signal,
          operation,
          sessionId,
          stream,
          activity,
          event,
        );
      } else if (event?.type === 'session.error') {
        const message = readOpencodeEventError(event) ?? 'OpenCode session failed';
        await stream?.terminal('turn.failed', event, sessionId, {
          error: { message },
        });
        throw new Error(`opencode session failed: ${message}`);
      } else if (event?.type === 'session.idle') {
        if (!turnObserved) {
          return false;
        }
        idleEvent = event;
        return true;
      }
      return false;
    },
    assistantText() {
      // Durable v2 responses only admit the input (`SessionInputAdmitted`);
      // the assistant content arrives exclusively through the event stream.
      return [...parts.values()]
        .filter((state) => state.item.type === 'agent_message' && state.item.text)
        .map((state) => String(state.item.text))
        .join('\n');
    },
    async includeResponse(response, providerSessionId) {
      const responseInfo = response?.data?.info ?? response?.info;
      const messageId = optionalProviderIdentifier(responseInfo?.id)
        ?? optionalProviderIdentifier(response?.data?.messageID)
        ?? latestAssistantMessageId
        ?? `message.${providerSessionId}.${stream?.modelRequestId ?? 'turn'}`;
      messageRoles.set(messageId, 'assistant');
      const responseParts = response?.data?.parts ?? response?.parts;
      if (!Array.isArray(responseParts)) {
        return;
      }
      for (const [index, responsePart] of responseParts.entries()) {
        if (!optionalProviderIdentifier(responsePart?.id)) {
          const matchingSnapshot = [...parts.values()].find((state) => (
            responsePart?.type === 'text'
            && state.item.type === 'agent_message'
            && state.item.text === String(responsePart.text ?? '')
          ));
          if (matchingSnapshot) {
            continue;
          }
        }
        const part = {
          ...responsePart,
          id: optionalProviderIdentifier(responsePart?.id) ?? `${messageId}:part:${index}`,
          messageID: optionalProviderIdentifier(responsePart?.messageID) ?? messageId,
          sessionID: optionalProviderIdentifier(responsePart?.sessionID) ?? providerSessionId,
        };
        await updatePart(part, null, { type: 'prompt.response', properties: { part } });
      }
    },
    async complete(providerSessionId) {
      for (const state of parts.values()) {
        if (state.completed) {
          continue;
        }
        if (isToolItem(state.item) && !isTerminalProviderItemStatus(state.item.status)) {
          state.item = { ...state.item, status: 'completed' };
        }
        await emitPart('item.completed', state, idleEvent);
      }
      await stream?.terminal('turn.completed', idleEvent, providerSessionId);
    },
  };
}

function unwrapOpencodeEvent(envelope) {
  if (envelope?.type && typeof envelope.type === 'string') {
    if (envelope.type === 'sync' && envelope.syncEvent) {
      // Durable sync bridge events carry their payload under `syncEvent` with
      // a versioned type suffix ("message.updated.1"); normalize to the plain
      // event type while keeping the v2 `data` payload shape.
      return unwrapOpencodeSyncEvent(envelope.syncEvent);
    }
    return envelope;
  }
  if (envelope?.payload?.type && typeof envelope.payload.type === 'string') {
    return envelope.payload;
  }
  return null;
}

function unwrapOpencodeSyncEvent(syncEvent) {
  if (!syncEvent?.type || !syncEvent?.data) {
    return null;
  }
  return {
    ...syncEvent,
    type: String(syncEvent.type).replace(/\.\d+$/u, ''),
  };
}

function opencodeEventData(event) {
  // v2 durable events carry the payload under `data`; v1 legacy events use
  // `properties`. Field names are identical across both shapes.
  return event?.data ?? event?.properties ?? {};
}

function opencodeEventSessionId(event) {
  const payload = opencodeEventData(event);
  return optionalProviderIdentifier(payload?.sessionID)
    ?? optionalProviderIdentifier(payload?.part?.sessionID)
    ?? optionalProviderIdentifier(payload?.info?.sessionID)
    ?? optionalProviderIdentifier(payload?.info?.id);
}

function readOpencodeEventError(event) {
  const payload = opencodeEventData(event);
  const error = payload?.error;
  return readProviderError(error)
    ?? readProviderError(error?.data)
    ?? readProviderError(error?.data?.message)
    ?? readProviderError(payload);
}

function normalizeOpencodeStreamPart(part) {
  const id = optionalProviderIdentifier(part?.id);
  if (!id) {
    return null;
  }
  if (part.type === 'text') {
    return { id, type: 'agent_message', text: String(part.text ?? '') };
  }
  if (part.type === 'reasoning') {
    return { id, type: 'reasoning', text: String(part.text ?? '') };
  }
  if (part.type === 'tool') {
    const status = normalizeProviderToolStatus(part.state?.status);
    return normalizeProviderToolItem(
      'opencode',
      id,
      part.tool,
      part.state?.input,
      status,
      part.state?.output,
      part.state?.error,
      {
        callId: optionalProviderIdentifier(part.callID),
        title: optionalProviderString(part.state?.title),
      },
    );
  }
  if (part.type === 'patch') {
    return {
      id,
      type: 'file_change',
      status: 'completed',
      files: Array.isArray(part.files) ? part.files : [],
      hash: part.hash ?? null,
    };
  }
  if (part.type === 'retry') {
    return {
      id,
      type: 'error',
      status: 'completed',
      message: readProviderError(part.error?.data) ?? 'OpenCode retrying provider request',
      attempt: part.attempt ?? null,
    };
  }
  return {
    id,
    type: 'status_notice',
    status: 'completed',
    providerPartType: String(part.type ?? 'unknown'),
    data: part,
  };
}

async function verifyOpencodeSession(client, requestedProviderSessionId, signal) {
  if (typeof client?.session?.get !== 'function') {
    throw new Error(
      'opencode sdk client is missing session.get required to verify a resumed provider session',
    );
  }
  const response = await client.session.get(
    { sessionID: requestedProviderSessionId },
    { signal },
  );
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
  const created = await client.session.create(
    permission ? { permission } : {},
    { signal },
  );
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
  const response = await client.session.update(
    { sessionID: sessionId, permission },
    { signal },
  );
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

function optionalProviderIdentifier(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function monotonicTextDelta(previousValue, currentValue, fieldName) {
  const previous = String(previousValue ?? '');
  const current = String(currentValue ?? '');
  if (!current.startsWith(previous)) {
    throw new Error(`${fieldName}_stream_non_monotonic`);
  }
  return current.slice(previous.length);
}

function parsePartialJson(value) {
  if (!value) {
    return {};
  }
  try {
    return JSON.parse(value);
  } catch {
    return { partialJson: value };
  }
}

function providerErrorPayload(value) {
  if (value && typeof value === 'object') {
    return value;
  }
  return { message: String(value ?? 'provider tool failed') };
}

function normalizeProviderToolStatus(value) {
  const status = typeof value === 'string' ? value.trim().toLowerCase() : '';
  if (status === 'completed' || status === 'success' || status === 'succeeded') {
    return 'completed';
  }
  if (status === 'error' || status === 'failed') {
    return 'failed';
  }
  if (status === 'cancelled' || status === 'canceled' || status === 'aborted') {
    return 'cancelled';
  }
  if (status === 'pending' || status === 'queued') {
    return 'pending';
  }
  return 'running';
}

function isTerminalProviderItemStatus(value) {
  return value === 'completed' || value === 'failed' || value === 'cancelled';
}

function isToolItem(item) {
  return ['command_execution', 'file_change', 'mcp_tool_call', 'web_search'].includes(
    item?.type,
  );
}

function normalizeProviderToolItem(
  providerId,
  id,
  toolName,
  input,
  status,
  result = null,
  error = null,
  metadata = {},
) {
  const name = optionalProviderIdentifier(toolName) ?? 'tool';
  const normalizedStatus = normalizeProviderToolStatus(status);
  const lowerName = name.toLowerCase();
  const common = {
    id,
    status: normalizedStatus,
    providerToolName: name,
    ...(metadata.callId ? { callId: metadata.callId } : {}),
    ...(metadata.title ? { title: metadata.title } : {}),
    ...(result != null ? { result } : {}),
    ...(error != null ? { error: providerErrorPayload(error) } : {}),
  };
  if (['bash', 'shell', 'shell_command', 'execute'].includes(lowerName)) {
    return {
      ...common,
      type: 'command_execution',
      command: input?.command ?? input?.cmd ?? input ?? null,
      arguments: input ?? {},
    };
  }
  if (['edit', 'write', 'notebookedit', 'apply_patch'].includes(lowerName)) {
    return {
      ...common,
      type: 'file_change',
      changes: input ?? {},
    };
  }
  if (['websearch', 'web_search', 'webfetch'].includes(lowerName)) {
    return {
      ...common,
      type: 'web_search',
      query: input?.query ?? input?.url ?? input ?? null,
      arguments: input ?? {},
    };
  }
  return {
    ...common,
    type: 'mcp_tool_call',
    server: providerId,
    tool: name,
    arguments: input ?? {},
  };
}

function createProviderStreamEmitter(providerId, operation, options = {}) {
  const onChunk = options?.onChunk;
  const onEvent = options?.onEvent;
  if (onChunk != null && typeof onChunk !== 'function') {
    throw new Error('stream onChunk must be a function');
  }
  if (onEvent != null && typeof onEvent !== 'function') {
    throw new Error('stream onEvent must be a function');
  }
  const modelRequestId = requiredOperationString(
    operation.model_request_id,
    'model_request_id',
  );
  const chunks = [];
  let chunkSequence = 0;
  let eventSequence = 0;
  let terminal = false;

  return {
    modelRequestId,
    chunks,
    async chunk(content) {
      if (terminal) {
        throw new Error(`${providerId}_stream_chunk_after_terminal`);
      }
      if (typeof content !== 'string' || !content) {
        return;
      }
      const chunk = { sequence: chunkSequence, content };
      chunkSequence += 1;
      if (onChunk) {
        await onChunk(chunk);
      } else {
        chunks.push(chunk);
      }
    },
    async event(
      providerEventType,
      item,
      rawProviderPayload,
      providerSessionId,
      overrides = {},
    ) {
      if (terminal) {
        return;
      }
      if (onEvent) {
        await onEvent(buildProviderKernelStreamEvent(
          providerId,
          providerEventType,
          rawProviderPayload,
          item,
          operation,
          providerSessionId,
          eventSequence,
          overrides,
        ));
      }
      eventSequence += 1;
    },
    async terminal(
      providerEventType,
      rawProviderPayload,
      providerSessionId,
      overrides = {},
    ) {
      if (terminal) {
        return;
      }
      await this.event(
        providerEventType,
        null,
        rawProviderPayload,
        providerSessionId,
        overrides,
      );
      terminal = true;
    },
  };
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
  if (
    packageName === '@anthropic-ai/claude-agent-sdk'
    || packageName === '@opencode-ai/sdk'
  ) {
    const activity = createRuntimeActivityReporter(operation, options?.onActivity);
    const handler = LIVE_MODEL_CHAT_HANDLERS[packageName];
    try {
      const result = await handler(
        resolveModelChatPrompt(operation),
        operation,
        packageName,
        activity,
        { onChunk, onEvent },
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
  const item = providerEvent?.item && typeof providerEvent.item === 'object'
    ? providerEvent.item
    : null;
  return buildProviderKernelStreamEvent(
    'codex',
    String(providerEvent?.type ?? 'unknown').trim() || 'unknown',
    providerEvent,
    item,
    operation,
    providerSessionId,
    sequence,
  );
}

export function buildProviderKernelStreamEvent(
  providerId,
  providerEventType,
  rawProviderPayload,
  item,
  operation,
  providerSessionId,
  sequence,
  overrides = {},
) {
  const normalizedProviderId = requiredOperationString(providerId, 'provider_id');
  const normalizedProviderEventType = requiredOperationString(
    providerEventType,
    'provider_event_type',
  );
  const itemId = typeof item?.id === 'string' && item.id.trim() ? item.id.trim() : null;
  const modelRequestId = optionalOperationString(operation.model_request_id, 'model_request_id');
  if (!modelRequestId) {
    throw new Error('provider stream event is missing model_request_id');
  }
  const normalizedSequence = Number.isSafeInteger(sequence) && sequence >= 0 ? sequence : 0;
  const resolvedProviderSessionId = readProviderSessionId(providerSessionId)
    ?? readProviderSessionId(rawProviderPayload?.session_id)
    ?? readProviderSessionId(rawProviderPayload?.sessionId)
    ?? readProviderSessionId(rawProviderPayload?.thread_id)
    ?? readProviderSessionId(rawProviderPayload?.threadId)
    ?? optionalOperationString(operation.provider_session_id, 'provider_session_id');
  const sessionId = optionalOperationString(operation.session_id, 'session_id');
  const turnId = optionalOperationString(
    operation.turn_id ?? operation.turnId,
    'turn_id',
  );

  return {
    event_id: `event.${modelRequestId}.${normalizedSequence}`,
    event_type: providerKernelEventType(
      normalizedProviderEventType,
      item?.type,
      overrides.interaction,
    ),
    event_version: '1.0.0',
    occurred_at: new Date().toISOString(),
    source: providerKernelEventSource(normalizedProviderEventType, item?.type),
    severity: providerKernelEventSeverity(normalizedProviderEventType, item?.type),
    session_id: sessionId,
    run_id: modelRequestId,
    step_id: turnId,
    correlation_id: modelRequestId,
    redaction_classification: 'tenant_sensitive',
    payload_schema: 'sdkwork.agent.provider_stream_event.v1',
    payload: {
      schemaVersion: 1,
      providerId: normalizedProviderId,
      providerEventType: normalizedProviderEventType,
      rawProviderEventType: providerRawEventType(rawProviderPayload),
      providerSessionId: resolvedProviderSessionId,
      providerItemId: itemId,
      providerTurnId: optionalProviderIdentifier(rawProviderPayload?.turn_id)
        ?? optionalProviderIdentifier(rawProviderPayload?.turnId),
      sequence: normalizedSequence,
      item,
      usage: overrides.usage ?? rawProviderPayload?.usage ?? null,
      error: overrides.error ?? rawProviderPayload?.error ?? (
        typeof rawProviderPayload?.message === 'string'
          ? { message: rawProviderPayload.message }
          : null
      ),
      ...(overrides.interaction ? { interaction: overrides.interaction } : {}),
      rawProviderPayload,
    },
    replay: false,
  };
}

function providerRawEventType(event) {
  const type = optionalProviderIdentifier(event?.type) ?? 'unknown';
  const subtype = optionalProviderIdentifier(event?.subtype)
    ?? optionalProviderIdentifier(event?.event?.type);
  return subtype ? `${type}.${subtype}` : type;
}

function providerKernelEventType(providerEventType, itemType, interaction = null) {
  if (providerEventType === 'thread.started') return 'agent.session.started';
  if (providerEventType === 'turn.started') return 'agent.turn.started';
  if (providerEventType === 'turn.updated') return 'agent.turn.updated';
  if (providerEventType === 'turn.completed') return 'agent.turn.completed';
  if (providerEventType === 'turn.failed') return 'agent.turn.failed';
  if (providerEventType === 'interaction.requested') {
    return interaction?.category === 'approval'
      ? 'agent.policy.paused'
      : 'agent.message.paused';
  }
  if (providerEventType === 'interaction.resolved') return 'agent.interaction.resolved';
  if (providerEventType === 'error') return 'agent.runtime.failed';

  const action = providerEventType.endsWith('.started')
    ? 'started'
    : providerEventType.endsWith('.completed')
      ? 'completed'
      : 'updated';
  if (itemType === 'agent_message') {
    return `agent.message.${action}`;
  }
  if (itemType === 'reasoning') {
    return `agent.reasoning.${action}`;
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

function providerKernelEventSource(providerEventType, itemType) {
  if (providerEventType.startsWith('item.') && itemType === 'agent_message') return 'model';
  if (providerEventType.startsWith('item.') && itemType === 'reasoning') return 'model';
  if (['command_execution', 'file_change', 'mcp_tool_call', 'web_search'].includes(itemType)) {
    return 'tool';
  }
  return 'provider';
}

function providerKernelEventSeverity(providerEventType, itemType) {
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
  const providerResponse = await respondToPendingSdkInteraction(command);
  if (providerResponse) {
    return providerResponse;
  }
  return respondToCodexAppServerRequest({
    ...command,
    request_id: command.request_id
      ?? command.provider_request_id
      ?? command.providerRequestId,
  });
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
