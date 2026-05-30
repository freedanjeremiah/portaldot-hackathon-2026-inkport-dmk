import { readSSE } from '@/lib/sse';

// Minimal mock of the Fetch API for Node test environment
function makeMockResponse(chunks: string[]) {
  let idx = 0;
  const stream = new ReadableStream({
    pull(controller) {
      if (idx < chunks.length) {
        controller.enqueue(new TextEncoder().encode(chunks[idx++]));
      } else {
        controller.close();
      }
    },
  });
  return { body: stream, ok: true } as unknown as Response;
}

global.fetch = jest.fn();

describe('readSSE', () => {
  it('calls onLine for log events and onDone for terminal wasm event', async () => {
    const chunks = [
      'data: {"type":"log","cls":"lg-ok","text":"hello"}\n\n',
      'data: {"type":"wasm","data":"abc123","metadata":{}}\n\n',
    ];
    (global.fetch as jest.Mock).mockResolvedValue(makeMockResponse(chunks));

    const lines: any[] = [];
    let done: any = null;

    await readSSE('/api/compile', { solidity: '', sessionId: 'x' }, {
      onLine: (l) => lines.push(l),
      onDone: (p) => { done = p; },
    });

    expect(lines).toHaveLength(1);
    expect(lines[0].text).toBe('hello');
    expect(done?.type).toBe('wasm');
    expect(done?.data).toBe('abc123');
  });

  it('calls onError for error events', async () => {
    const chunks = ['data: {"type":"error","log":"build failed"}\n\n'];
    (global.fetch as jest.Mock).mockResolvedValue(makeMockResponse(chunks));

    let errPayload: any = null;
    await readSSE('/api/compile', {}, { onError: (p) => { errPayload = p; } });

    expect(errPayload?.log).toBe('build failed');
  });
});
