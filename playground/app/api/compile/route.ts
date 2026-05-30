/* POST /api/compile — translate + cargo build + strip_wasm → returns Portaldot-compatible wasm.
   Streams log lines via SSE, then emits { type:"wasm", data: base64, metadata } on success.

   Wire this by:
     1. Write body.solidity to /tmp/inkport-playground/<sessionId>/<name>.sol
     2. Run inkport-translate → read lib.rs + metadata.json
     3. Run cargo +stable build --release --target wasm32-unknown-unknown
     4. Run strip_wasm (Python) → stripped .wasm
     5. base64-encode the wasm and SSE-emit { type:"wasm", data, metadata } */
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
