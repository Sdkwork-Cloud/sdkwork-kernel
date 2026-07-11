#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { statSync } from 'node:fs';
import path from 'node:path';

import {
  isExecutableFile,
  resolveExecutableOnPath,
  resolveLaunchCommand,
  terminateProcessTree,
} from './codex-cli-live.mjs';

const DEFAULT_TIMEOUT_MS = 300_000;
const MAX_TIMEOUT_MS = 3_600_000;
const DEFAULT_MAX_OUTPUT_BYTES = 8 * 1024 * 1024;
const MAX_CAPTURE_BYTES = 64 * 1024 * 1024;

const CLI_DEFINITIONS = new Map([
  [
    '@anthropic-ai/claude-agent-sdk',
    { command: 'claude', environmentKey: 'SDKWORK_CLAUDE_CLI_BIN', provider: 'claude' },
  ],
  [
    '@google/gemini-cli-sdk',
    { command: 'gemini', environmentKey: 'SDKWORK_GEMINI_CLI_BIN', provider: 'gemini' },
  ],
  [
    '@opencode-ai/sdk',
    { command: 'opencode', environmentKey: 'SDKWORK_OPENCODE_CLI_BIN', provider: 'opencode' },
  ],
]);

export function isProviderCliPackage(packageName) {
  return CLI_DEFINITIONS.has(packageName);
}

export function probeProviderCli(packageName, environment = process.env) {
  const definition = CLI_DEFINITIONS.get(packageName);
  if (!definition) {
    return { available: false, executable: null, mode: null, provider: null };
  }
  const configured = environment[definition.environmentKey]?.trim();
  const executable = configured
    ? resolveConfiguredExecutable(configured)
    : resolveExecutableOnPath(definition.command, environment);
  return {
    available: Boolean(executable),
    executable,
    mode: executable ? 'sdk_cli' : null,
    provider: definition.provider,
  };
}

export async function invokeProviderCliModelChat(packageName, operation, options = {}) {
  const definition = CLI_DEFINITIONS.get(packageName);
  if (!definition) {
    throw new Error(`provider_cli_unsupported: ${packageName}`);
  }
  const environment = options.env ?? process.env;
  const probe = options.probe ?? probeProviderCli(packageName, environment);
  if (!probe.available || !probe.executable) {
    throw new Error(`${definition.provider}_cli_unavailable: executable not found`);
  }

  const prompt = String(options.prompt ?? resolvePrompt(operation));
  const workingDirectory = resolveWorkingDirectory(operation.working_directory);
  const timeoutMs = resolveTimeoutMs(operation.timeout_ms);
  const maxOutputBytes = resolveMaxOutputBytes(operation.execution_options?.max_output_bytes);
  const captureLimit = Math.min(
    MAX_CAPTURE_BYTES,
    Math.max(DEFAULT_MAX_OUTPUT_BYTES, maxOutputBytes * 8),
  );
  const invocation = buildProviderInvocation(definition.provider, operation, prompt);
  const processResult = await runProviderProcess({
    executable: probe.executable,
    args: invocation.args,
    cwd: workingDirectory,
    environment,
    input: invocation.input,
    timeoutMs,
    captureLimit,
    provider: definition.provider,
    parse: invocation.parse,
  });
  const parsed = invocation.parse(processResult.stdout);
  if (parsed.error) {
    throw new Error(`${definition.provider}_cli_turn_failed: ${parsed.error}`);
  }
  const assistantContent = parsed.messages.join('');
  if (!assistantContent.trim()) {
    throw new Error(`${definition.provider}_cli_empty_response: no assistant content was emitted`);
  }
  const assistantBytes = Buffer.byteLength(assistantContent, 'utf8');
  if (assistantBytes > maxOutputBytes) {
    throw new Error(
      `${definition.provider}_cli_output_limit_exceeded: assistant output ${assistantBytes} bytes exceeds ${maxOutputBytes}`,
    );
  }
  return {
    ok: true,
    mode: 'sdk_cli',
    messages: [assistantContent],
    finish_reason: parsed.finish_reason ?? 'stop',
    model_request_id: operation.model_request_id ?? null,
    native_session_id:
      parsed.native_session_id ?? optionalNonBlankString(operation.session_id, 'session_id'),
    package: packageName,
  };
}

export function buildClaudeCliArgs(operation) {
  const executionOptions = readExecutionOptions(operation);
  rejectUnsupportedFullAuto(executionOptions, 'claude');
  const args = ['-p', '--output-format', 'stream-json', '--verbose'];
  const modelId = optionalNonBlankString(operation.model_id, 'model_id');
  if (modelId) args.push('--model', modelId);
  const sessionId = optionalNonBlankString(operation.session_id, 'session_id');
  if (sessionId) args.push('--resume', sessionId);
  if (optionalBoolean(executionOptions.ephemeral, 'ephemeral')) {
    if (sessionId) throw new Error('claude_cli_ephemeral_resume_conflict');
    args.push('--no-session-persistence');
  }
  args.push('--permission-mode', resolveClaudePermissionMode(executionOptions));
  const workingDirectory = resolveWorkingDirectory(operation.working_directory);
  if (workingDirectory) args.push('--add-dir', workingDirectory);
  return args;
}

export function buildGeminiCliArgs(operation) {
  const executionOptions = readExecutionOptions(operation);
  rejectUnsupportedFullAuto(executionOptions, 'gemini');
  rejectUnsupportedEphemeral(executionOptions, 'gemini');
  const sandboxMode = resolveSandboxMode(executionOptions, 'gemini');
  const args = [
    '-p',
    'Follow the complete user request provided on standard input.',
    '-o',
    'stream-json',
    '--approval-mode',
    resolveGeminiApprovalMode(executionOptions),
  ];
  if (sandboxMode === 'workspace-write' || sandboxMode === 'read-only') {
    args.push('--sandbox');
  }
  const modelId = optionalNonBlankString(operation.model_id, 'model_id');
  if (modelId) args.push('-m', modelId);
  const sessionId = optionalNonBlankString(operation.session_id, 'session_id');
  if (sessionId) args.push('--resume', sessionId);
  return args;
}

export function buildOpenCodeCliArgs(operation) {
  const executionOptions = readExecutionOptions(operation);
  rejectUnsupportedFullAuto(executionOptions, 'opencode');
  rejectUnsupportedEphemeral(executionOptions, 'opencode');
  const sandboxMode = resolveSandboxMode(executionOptions, 'opencode');
  const approvalPolicy = resolveApprovalPolicy(executionOptions, 'opencode');
  if (sandboxMode === 'read-only' || approvalPolicy === 'plan') {
    throw new Error(
      'opencode_cli_unsupported_policy: read-only execution cannot be enforced by the OpenCode CLI lane',
    );
  }
  const args = ['run', '--format', 'json', '--pure'];
  const workingDirectory = resolveWorkingDirectory(operation.working_directory);
  if (workingDirectory) args.push('--dir', workingDirectory);
  const modelId = optionalNonBlankString(operation.model_id, 'model_id');
  if (modelId) args.push('--model', modelId);
  const sessionId = optionalNonBlankString(operation.session_id, 'session_id');
  if (sessionId) args.push('--session', sessionId);
  return args;
}

export function parseClaudeStreamJson(stdout) {
  return parseJsonLines(stdout, 'claude', (event, state) => {
    state.native_session_id = firstNonBlank(
      event?.session_id,
      event?.sessionId,
      event?.message?.session_id,
      state.native_session_id,
    );
    if (event?.type === 'assistant') {
      appendText(state.messages, extractText(event?.message?.content ?? event?.content));
    }
    if (event?.type === 'result') {
      if (event.is_error || !['success', undefined, null].includes(event.subtype)) {
        state.error = firstNonBlank(event.error, event.result, event.subtype, 'Claude turn failed');
      } else if (state.messages.length === 0) {
        appendText(state.messages, typeof event.result === 'string' ? event.result : '');
      }
    }
    if (event?.type === 'error') {
      state.error = extractError(event) ?? 'Claude CLI emitted an error event';
    }
  });
}

export function parseGeminiStreamJson(stdout) {
  return parseJsonLines(stdout, 'gemini', (event, state) => {
    state.native_session_id = firstNonBlank(
      event?.session_id,
      event?.sessionId,
      event?.session?.id,
      state.native_session_id,
    );
    if (event?.type === 'content') {
      appendText(state.messages, typeof event.value === 'string' ? event.value : '');
    } else if (event?.type === 'message' && event?.role === 'assistant') {
      appendText(state.messages, extractText(event.content));
    }
    if (event?.type === 'error' && event?.severity !== 'warning') {
      state.error = extractError(event) ?? 'Gemini CLI emitted an error event';
    } else if (
      event?.type === 'result' &&
      (event?.status === 'error' || event?.is_error === true)
    ) {
      state.error = extractError(event) ?? 'Gemini CLI result reported an error';
    }
  });
}

export function parseOpenCodeJson(stdout) {
  return parseJsonLines(stdout, 'opencode', (event, state) => {
    state.native_session_id = firstNonBlank(
      event?.sessionID,
      event?.session_id,
      event?.sessionId,
      event?.part?.sessionID,
      event?.properties?.sessionID,
      state.native_session_id,
    );
    if (event?.type === 'text') {
      appendText(state.messages, firstNonBlank(event?.part?.text, event?.text, event?.content));
    } else if (event?.type === 'message' && event?.role === 'assistant') {
      appendText(state.messages, extractText(event.content));
    }
    if (event?.type === 'error' || event?.error) {
      state.error = extractError(event) ?? 'OpenCode CLI emitted an error event';
    }
  });
}

function buildProviderInvocation(provider, operation, prompt) {
  if (provider === 'claude') {
    return { args: buildClaudeCliArgs(operation), input: prompt, parse: parseClaudeStreamJson };
  }
  if (provider === 'gemini') {
    return { args: buildGeminiCliArgs(operation), input: prompt, parse: parseGeminiStreamJson };
  }
  return {
    args: buildOpenCodeCliArgs(operation),
    input: prompt,
    parse: parseOpenCodeJson,
  };
}

function resolveClaudePermissionMode(options) {
  const sandbox = resolveSandboxMode(options, 'claude');
  const approval = resolveApprovalPolicy(options, 'claude');
  if (sandbox === 'read-only' || approval === 'plan') return 'plan';
  if (approval === 'on-request' || approval === 'untrusted') {
    return 'default';
  }
  return 'acceptEdits';
}

function resolveGeminiApprovalMode(options) {
  const sandbox = resolveSandboxMode(options, 'gemini');
  const approval = resolveApprovalPolicy(options, 'gemini');
  if (sandbox === 'read-only' || approval === 'plan') return 'plan';
  if (approval === 'on-request' || approval === 'untrusted') {
    return 'default';
  }
  return 'auto_edit';
}

function resolveSandboxMode(options, provider) {
  const sandbox = normalizePolicyValue(options.sandbox_mode);
  if (!sandbox || sandbox === 'workspace-write' || sandbox === 'workspacewrite') {
    return 'workspace-write';
  }
  if (sandbox === 'read-only' || sandbox === 'readonly') {
    return 'read-only';
  }
  if (['danger-full-access', 'dangerfullaccess', 'none'].includes(sandbox)) {
    throw new Error('dangerous permission bypass is prohibited for kernel-owned CLI execution');
  }
  throw new Error(`${provider}_cli_unsupported_sandbox_mode: ${String(options.sandbox_mode)}`);
}

function resolveApprovalPolicy(options, provider) {
  const approval = normalizePolicyValue(options.approval_policy);
  if (!approval || approval === 'on-failure' || approval === 'onfailure') {
    return 'on-failure';
  }
  if (approval === 'on-request' || approval === 'onrequest') {
    return 'on-request';
  }
  if (approval === 'untrusted' || approval === 'restricted' || approval === 'unless-trusted') {
    return 'untrusted';
  }
  if (approval === 'plan') {
    return 'plan';
  }
  if (['never', 'bypasspermissions', 'bypass-permissions', 'yolo'].includes(approval)) {
    throw new Error('dangerous permission bypass is prohibited for kernel-owned CLI execution');
  }
  throw new Error(`${provider}_cli_unsupported_approval_policy: ${String(options.approval_policy)}`);
}

function rejectUnsupportedFullAuto(options, provider) {
  if (optionalBoolean(options.full_auto, 'full_auto')) {
    throw new Error(`${provider}_cli_full_auto_unsupported: use explicit sandbox and approval policies`);
  }
}

function rejectUnsupportedEphemeral(options, provider) {
  if (optionalBoolean(options.ephemeral, 'ephemeral')) {
    throw new Error(`${provider}_cli_ephemeral_unsupported: session persistence cannot be disabled safely`);
  }
}

function normalizePolicyValue(value) {
  return typeof value === 'string' ? value.trim().toLowerCase().replace(/[_\s]/gu, '-') : '';
}

function readExecutionOptions(operation) {
  const options = operation?.execution_options;
  if (options == null) return {};
  if (typeof options !== 'object' || Array.isArray(options)) {
    throw new Error('execution_options must be an object');
  }
  return options;
}

function resolvePrompt(operation) {
  return Array.isArray(operation?.messages) ? operation.messages.join('\n') : '';
}

function resolveConfiguredExecutable(configured) {
  const resolved = path.resolve(configured);
  return isExecutableFile(resolved) ? resolved : null;
}

function resolveWorkingDirectory(value) {
  const configured = optionalNonBlankString(value, 'working_directory');
  const resolved = path.resolve(configured ?? process.cwd());
  let metadata;
  try {
    metadata = statSync(resolved);
  } catch (error) {
    throw new Error(
      `provider_cli_invalid_working_directory: ${resolved}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (!metadata.isDirectory()) {
    throw new Error(`provider_cli_invalid_working_directory: not a directory: ${resolved}`);
  }
  return resolved;
}

function resolveTimeoutMs(value) {
  if (value == null) return DEFAULT_TIMEOUT_MS;
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error('timeout_ms must be a positive safe integer');
  }
  return Math.min(value, MAX_TIMEOUT_MS);
}

function resolveMaxOutputBytes(value) {
  if (value == null) return DEFAULT_MAX_OUTPUT_BYTES;
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error('execution_options.max_output_bytes must be a positive safe integer');
  }
  return Math.min(value, MAX_CAPTURE_BYTES);
}

function optionalNonBlankString(value, fieldName) {
  if (value == null) return null;
  if (typeof value !== 'string') throw new Error(`${fieldName} must be a string`);
  return value.trim() || null;
}

function optionalBoolean(value, fieldName) {
  if (value == null) return false;
  if (typeof value !== 'boolean') {
    throw new Error(`execution_options.${fieldName} must be a boolean`);
  }
  return value;
}

function parseJsonLines(stdout, provider, project) {
  const state = {
    messages: [],
    native_session_id: null,
    finish_reason: 'stop',
    error: null,
  };
  for (const [index, rawLine] of String(stdout ?? '').split(/\r?\n/u).entries()) {
    const line = stripAnsi(rawLine).trim();
    if (!line) continue;
    let event;
    try {
      event = JSON.parse(line);
    } catch (error) {
      throw new Error(
        `${provider}_cli_invalid_jsonl: line ${index + 1}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
    project(event, state);
  }
  if (state.error) state.finish_reason = 'error';
  return state;
}

function extractText(value) {
  if (typeof value === 'string') return value;
  if (!Array.isArray(value)) return '';
  return value
    .map((part) =>
      typeof part === 'string'
        ? part
        : typeof part?.text === 'string'
          ? part.text
          : typeof part?.content === 'string'
            ? part.content
            : '',
    )
    .join('');
}

function appendText(messages, value) {
  if (typeof value === 'string' && value) messages.push(value);
}

function extractError(value) {
  return firstNonBlank(value?.error?.message, value?.message, value?.error, value?.reason);
}

function firstNonBlank(...values) {
  return values.find((value) => typeof value === 'string' && value.trim())?.trim() ?? null;
}

function stripAnsi(value) {
  return value.replace(/\u001B\[[0-?]*[ -/]*[@-~]/gu, '');
}

function runProviderProcess({
  executable,
  args,
  cwd,
  environment,
  input,
  timeoutMs,
  captureLimit,
  provider,
  parse,
}) {
  return new Promise((resolve, reject) => {
    let launch;
    try {
      launch = resolveLaunchCommand(executable, args, environment);
    } catch (error) {
      reject(error);
      return;
    }
    const child = spawn(launch.command, launch.args, {
      cwd,
      env: environment,
      windowsHide: true,
      windowsVerbatimArguments: launch.windowsVerbatimArguments ?? false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const stdout = [];
    const stderr = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settledReason = null;
    const timer = setTimeout(() => {
      settledReason = new Error(`${provider}_cli_timeout: exceeded ${timeoutMs} ms`);
      terminateProcessTree(child);
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > captureLimit) {
        settledReason = new Error(
          `${provider}_cli_output_limit_exceeded: stdout exceeded ${captureLimit} bytes`,
        );
        terminateProcessTree(child);
        return;
      }
      stdout.push(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderrBytes += chunk.length;
      if (stderrBytes <= DEFAULT_MAX_OUTPUT_BYTES) stderr.push(chunk);
    });
    child.once('error', (error) => {
      clearTimeout(timer);
      reject(new Error(`${provider}_cli_spawn_failed: ${error.message}`));
    });
    child.once('close', (code, signal) => {
      clearTimeout(timer);
      if (settledReason) {
        reject(settledReason);
        return;
      }
      const stdoutText = Buffer.concat(stdout).toString('utf8');
      const stderrText = Buffer.concat(stderr).toString('utf8').trim();
      if (code !== 0) {
        const status = code == null ? `signal ${signal ?? 'unknown'}` : `status ${code}`;
        let stdoutError = null;
        try {
          stdoutError = parse(stdoutText).error;
        } catch {
          stdoutError = null;
        }
        reject(
          new Error(
            `${provider}_cli_exit_failed: ${status}${stdoutError || stderrText ? `: ${stdoutError || stderrText}` : ''}`,
          ),
        );
        return;
      }
      resolve({ stdout: stdoutText, stderr: stderrText });
    });
    child.stdin.on('error', (error) => {
      if (!settledReason) {
        settledReason = new Error(`${provider}_cli_stdin_failed: ${error.message}`);
        terminateProcessTree(child);
      }
    });
    child.stdin.end(input, 'utf8');
  });
}
