/* POST /api/deploy — instantiate_with_code on Portaldot, returns contract address.
   Body: { wasmB64: string, metadata: object, args: string[], sessionId: string }
   Streams SSE log lines, then emits { type:"address", address: string } on success.

   Wire this by calling portaldot.py's deploy() with the decoded wasm and encoded ctor args. */
export async function POST(_request: Request) {
  const encoder = new TextEncoder();
  const body = new ReadableStream({
    start(controller) {
      const msg = JSON.stringify({ type: 'log', cls: 'lg-err', text: 'Backend not connected. Run the playground on the same machine as inkport.' });
      controller.enqueue(encoder.encode('data: ' + msg + '\n\n'));
      controller.enqueue(encoder.encode('data: ' + JSON.stringify({ type: 'error', log: 'Backend not connected' }) + '\n\n'));
      controller.close();
    }
  });
  return new Response(body, {
    headers: { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', 'Connection': 'keep-alive' }
  });
}
