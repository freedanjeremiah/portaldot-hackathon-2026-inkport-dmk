import { apiUrl } from './api';

export interface SSEPayload {
  type: string;
  cls?: string;
  text?: string;
  data?: string;
  metadata?: unknown;
  address?: string;
  log?: string;
  [key: string]: unknown;
}

export async function readSSE(
  url: string,
  body: object,
  handlers: {
    onLine?: (payload: SSEPayload) => void;
    onDone?: (payload: SSEPayload) => void;
    onError?: (payload: SSEPayload) => void;
  }
): Promise<void> {
  const response = await fetch(apiUrl(url), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!response.body) throw new Error('No response body from ' + url);

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const chunks = buffer.split('\n\n');
    buffer = chunks.pop() ?? '';

    for (const chunk of chunks) {
      const dataLine = chunk.split('\n').find(l => l.startsWith('data: '));
      if (!dataLine) continue;
      try {
        const parsed: SSEPayload = JSON.parse(dataLine.slice(6));
        if (parsed.type === 'log') {
          handlers.onLine?.(parsed);
        } else if (parsed.type === 'error') {
          handlers.onError?.(parsed);
        } else {
          // type: 'wasm' | 'address' | 'done'
          handlers.onDone?.(parsed);
        }
      } catch {
        // malformed SSE line — skip
      }
    }
  }
}
