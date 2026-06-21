#!/usr/bin/env node
import readline from 'node:readline';
import {
  buildStubModelChatResult,
  invokeModelChatLive,
  mockProviderInvocationAllowed,
  probePackage,
} from './engine-sdk-live.mjs';

const packageIndex = process.argv.indexOf('--package');
const packageName =
  packageIndex >= 0 && packageIndex + 1 < process.argv.length
    ? process.argv[packageIndex + 1]
    : 'unknown';

function writeResponse(response) {
  process.stdout.write(`${JSON.stringify(response)}\n`);
}

async function handleCapabilityInvoke(params) {
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
      mode: packageProbe.resolved ? 'sdk_live' : 'stub',
      agent_id: operation.agent_id ?? null,
      user_ref: operation.user_ref ?? null,
      package: packageName,
    };
  }

  if (op === 'model_chat') {
    if (packageProbe.resolved) {
      try {
        return await invokeModelChatLive(packageName, operation);
      } catch (error) {
        if (!mockProviderInvocationAllowed()) {
          return {
            ok: false,
            mode: 'sdk_live_failed',
            package: packageName,
            error: error instanceof Error ? error.message : String(error),
            model_request_id: operation.model_request_id ?? null,
          };
        }
      }
    }

    if (!mockProviderInvocationAllowed()) {
      return {
        ok: false,
        mode: 'sdk_live_failed',
        package: packageName,
        error: packageProbe.resolved
          ? 'official sdk live invoke failed and mock fallback is disabled'
          : `official sdk package is not resolved: ${packageName}`,
        model_request_id: operation.model_request_id ?? null,
      };
    }

    return buildStubModelChatResult(packageName, operation, packageProbe);
  }

  if (op === 'tool_invoke') {
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_live' : 'stub',
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
    Promise.resolve(handleCapabilityInvoke(params))
      .then((result) => {
        writeResponse({
          jsonrpc: '2.0',
          id: request.id,
          result,
        });
      })
      .catch((error) => {
        writeResponse({
          jsonrpc: '2.0',
          id: request.id,
          error: {
            code: -32000,
            message: error instanceof Error ? error.message : String(error),
          },
        });
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
