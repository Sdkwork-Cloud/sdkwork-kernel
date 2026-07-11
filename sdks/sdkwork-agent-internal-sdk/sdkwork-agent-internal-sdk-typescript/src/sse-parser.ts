export const DEFAULT_MAX_SSE_EVENT_BYTES = 1024 * 1024;

export interface SdkworkSseEvent<TData = unknown> {
  event?: string;
  id?: string;
  data: TData;
}

export interface DecodeSseJsonEventsOptions {
  defaultEvent?: string;
  maxEventBytes?: number;
}

export class SdkworkSseProtocolError extends Error {
  public constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = 'SdkworkSseProtocolError';
  }
}

/**
 * Decode protocol lines emitted by the generated SDK transport.
 *
 * `BaseHttpClient.stream` preserves `event:` and `id:` lines but strips the
 * `data:` prefix, so this decoder accepts both raw SSE lines and that normalized
 * transport form. It never buffers more than one bounded event.
 */
export async function* decodeSseJsonEvents<TData = unknown>(
  lines: AsyncIterable<string>,
  options: DecodeSseJsonEventsOptions = {},
): AsyncIterable<SdkworkSseEvent<TData>> {
  const maxEventBytes = options.maxEventBytes ?? DEFAULT_MAX_SSE_EVENT_BYTES;
  if (!Number.isSafeInteger(maxEventBytes) || maxEventBytes <= 0) {
    throw new SdkworkSseProtocolError('maxEventBytes must be a positive safe integer');
  }

  let event = options.defaultEvent;
  let id: string | undefined;
  let dataLines: string[] = [];
  let dataBytes = 0;

  const flush = (): SdkworkSseEvent<TData> | undefined => {
    if (dataLines.length === 0) {
      return undefined;
    }

    const dataText = dataLines.join('\n');
    dataLines = [];
    dataBytes = 0;

    let data: TData;
    try {
      data = JSON.parse(dataText) as TData;
    } catch (error) {
      throw new SdkworkSseProtocolError('SSE data is not valid JSON', {
        cause: error,
      });
    }

    const decoded = { event, id, data };
    event = options.defaultEvent;
    id = undefined;
    return decoded;
  };

  for await (const rawLine of lines) {
    const line = rawLine.replace(/\r$/, '');

    if (line.length === 0) {
      const decoded = flush();
      if (decoded) {
        yield decoded;
      }
      continue;
    }

    if (line.startsWith(':')) {
      continue;
    }

    if (line.startsWith('event:')) {
      const decoded = flush();
      if (decoded) {
        yield decoded;
      }
      event = line.slice('event:'.length).trimStart() || options.defaultEvent;
      continue;
    }

    if (line.startsWith('id:')) {
      const decoded = flush();
      if (decoded) {
        yield decoded;
      }
      id = line.slice('id:'.length).trimStart() || undefined;
      continue;
    }

    if (line.startsWith('retry:')) {
      continue;
    }

    const dataLine = line.startsWith('data:')
      ? line.slice('data:'.length).replace(/^ /, '')
      : line;
    if (dataLine === '[DONE]') {
      const decoded = flush();
      if (decoded) {
        yield decoded;
      }
      return;
    }

    dataBytes += new TextEncoder().encode(dataLine).byteLength;
    if (dataLines.length > 0) {
      dataBytes += 1;
    }
    if (dataBytes > maxEventBytes) {
      throw new SdkworkSseProtocolError(
        `SSE event exceeds the ${maxEventBytes} byte limit`,
      );
    }
    dataLines.push(dataLine);
  }

  const decoded = flush();
  if (decoded) {
    yield decoded;
  }
}
