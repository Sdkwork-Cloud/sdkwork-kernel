import assert from 'node:assert/strict';
import test from 'node:test';

import { createAgentInternalStreamingApi } from '../src/streaming.ts';

async function* lines(values) {
  yield* values;
}

async function collect(iterable) {
  const values = [];
  for await (const value of iterable) {
    values.push(value);
  }
  return values;
}

test('model streaming uses the generated transport contract and decodes named events', async () => {
  const calls = [];
  const transport = {
    stream(path, options) {
      calls.push({ path, options });
      return lines([
        'event: model.chunk',
        'id: model-event-1',
        'data: {"modelRequestId":"request-1","sequence":0,"content":"hello"}',
        '',
        'event: model.done',
        'data: {}',
        '',
      ]);
    },
  };
  const signal = new AbortController().signal;
  const api = createAgentInternalStreamingApi(transport);

  const events = await collect(api.model({
    sessionId: 'session/1',
    modelId: 'model-1',
    messages: ['hello'],
    xSdkworkTenantId: 'tenant-1',
    xSdkworkUserId: 'user-1',
    xSdkworkIdentityMac: 'mac-1',
    signal,
  }));

  assert.deepEqual(events, [
    {
      event: 'model.chunk',
      id: 'model-event-1',
      data: {
        modelRequestId: 'request-1',
        sequence: 0,
        content: 'hello',
      },
    },
    { event: 'model.done', id: undefined, data: {} },
  ]);
  assert.equal(calls.length, 1);
  assert.equal(
    calls[0].path,
    '/internal/v3/api/intelligence/runtime/sessions/session%2F1/model/stream',
  );
  assert.equal(calls[0].options.method, 'POST');
  assert.deepEqual(calls[0].options.body, {
    modelId: 'model-1',
    messages: ['hello'],
  });
  assert.deepEqual(calls[0].options.headers, {
    'x-sdkwork-tenant-id': 'tenant-1',
    'x-sdkwork-user-id': 'user-1',
    'x-sdkwork-identity-mac': 'mac-1',
  });
  assert.equal(calls[0].options.signal, signal);
});

test('session event streaming encodes continuation parameters and falls back to eventType', async () => {
  const calls = [];
  const transport = {
    stream(path, options) {
      calls.push({ path, options });
      return lines([
        'id: event-1',
        'data: {"eventId":"event-1","eventType":"runtime.message","sequence":1,"payload":"{}"}',
        '',
      ]);
    },
  };
  const api = createAgentInternalStreamingApi(transport);

  const events = await collect(api.sessionEvents({
    sessionId: 'session-1',
    lastEventId: 'event/0',
    live: false,
    xSdkworkTenantId: 'tenant-1',
  }));

  assert.deepEqual(events, [
    {
      event: 'runtime.message',
      id: 'event-1',
      data: {
        eventId: 'event-1',
        eventType: 'runtime.message',
        sequence: 1,
        payload: '{}',
      },
    },
  ]);
  assert.equal(
    calls[0].path,
    '/internal/v3/api/intelligence/runtime/sessions/session-1/events/stream?lastEventId=event%2F0&live=false',
  );
  assert.equal(calls[0].options.method, 'GET');
  assert.deepEqual(calls[0].options.headers, {
    'x-sdkwork-tenant-id': 'tenant-1',
  });
});
