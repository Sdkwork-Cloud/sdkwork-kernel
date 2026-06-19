#!/usr/bin/env node
import { createRequire } from 'node:module';
import readline from 'node:readline';

const packageIndex = process.argv.indexOf('--package');
const packageName =
  packageIndex >= 0 && packageIndex + 1 < process.argv.length
    ? process.argv[packageIndex + 1]
    : 'unknown';

function writeResponse(response) {
  process.stdout.write(`${JSON.stringify(response)}\n`);
}

function probePackage(name) {
  const require = createRequire(import.meta.url);
  try {
    require.resolve(name);
    return { resolved: true };
  } catch {
    return { resolved: false };
  }
}

function handleCapabilityInvoke(params) {
  const operation = params.operation ?? {};
  const op = operation.operation ?? operation;
  const packageProbe = probePackage(packageName);

  if (op === 'ping') {
    return {
      ok: true,
      backend: 'typescript_node',
      package: packageName,
      package_resolved: packageProbe.resolved,
    };
  }

  if (op === 'session_create') {
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
      agent_id: operation.agent_id ?? null,
      user_ref: operation.user_ref ?? null,
      package: packageName,
    };
  }

  if (op === 'model_chat') {
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

  if (op === 'tool_invoke') {
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
      output: JSON.stringify({
        tool_id: operation.tool_id ?? null,
        arguments: operation.arguments ?? null,
        package: packageName,
      }),
      package: packageName,
      tool_call_id: operation.tool_call_id ?? null,
    };
  }

  return {
    ok: true,
    mode: 'unknown_operation',
    operation: op,
    package: packageName,
  };
}

function handleRequest(request) {
  if (request.method === 'sdkwork/ping') {
    const probe = probePackage(packageName);
    writeResponse({
      jsonrpc: '2.0',
      id: request.id,
      result: {
        ok: true,
        backend: 'typescript_node',
        package: packageName,
        package_resolved: probe.resolved,
      },
    });
    return;
  }

  if (request.method === 'sdkwork/capability.invoke') {
    const params = request.params ?? {};
    writeResponse({
      jsonrpc: '2.0',
      id: request.id,
      result: handleCapabilityInvoke(params),
    });
    return;
  }

  writeResponse({
    jsonrpc: '2.0',
    id: request.id,
    error: {
      code: -32601,
      message: `Method not found: ${request.method}`,
    },
  });
}

const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) {
    return;
  }

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch (error) {
    writeResponse({
      jsonrpc: '2.0',
      id: null,
      error: {
        code: -32700,
        message: `Parse error: ${error.message}`,
      },
    });
    return;
  }

  handleRequest(request);
});
