import assert from 'node:assert/strict';
import test from 'node:test';

import {
  SdkworkSseProtocolError,
  decodeSseJsonEvents,
} from '../src/sse-parser.ts';

async function* lines(values) {
  yield* values;
}

test('pairs named SSE metadata with normalized JSON data lines', async () => {
  const decoded = [];
  for await (const event of decodeSseJsonEvents(lines([
    'event: model.chunk',
    '{"modelRequestId":"model.1","sequence":0,"content":"hello"}',
    'event: model.done',
    '{}',
  ]))) {
    decoded.push(event);
  }

  assert.deepEqual(decoded, [
    {
      event: 'model.chunk',
      id: undefined,
      data: { modelRequestId: 'model.1', sequence: 0, content: 'hello' },
    },
    { event: 'model.done', id: undefined, data: {} },
  ]);
});

test('accepts raw SSE data prefixes and event ids', async () => {
  const decoded = [];
  for await (const event of decodeSseJsonEvents(lines([
    'id: evt.1',
    'event: runtime.event',
    'data: {"eventId":"evt.1"}',
    '',
  ]))) {
    decoded.push(event);
  }

  assert.deepEqual(decoded, [
    {
      event: 'runtime.event',
      id: 'evt.1',
      data: { eventId: 'evt.1' },
    },
  ]);
});

test('rejects an event above the configured byte budget', async () => {
  await assert.rejects(
    async () => {
      for await (const _event of decodeSseJsonEvents(lines(['{"value":"too large"}']), {
        maxEventBytes: 8,
      })) {
        // Consume the iterator to force decoding.
      }
    },
    SdkworkSseProtocolError,
  );
});
