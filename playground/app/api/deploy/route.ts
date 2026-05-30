import { NextRequest } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { spawnCollect } from '@/lib/shell';

export async function POST(request: NextRequest) {
  const body = await request.json() as { wasmB64?: string; metadata?: { name: string; [k: string]: unknown }; args?: string[] };
  const { wasmB64, metadata, args } = body;

  const encoder = new TextEncoder();

  if (!wasmB64 || !metadata?.name) {
    const msg = JSON.stringify({ type: 'error', log: 'Missing wasmB64 or metadata' });
    return new Response(`data: ${msg}\n\n`, {
      headers: { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' },
    });
  }

  const name = metadata.name;

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      function emit(obj: object) {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify(obj)}\n\n`));
      }

      try {
        const env = buildEnv();
        const inkportRoot = env.INKPORT_ROOT as string;

        // Single-user constraint: playground writes to INKPORT_ROOT/build/<name>/ so inkport deploy
        // can find the wasm and metadata. Concurrent playground sessions deploying the same contract
        // name will collide. Do not run inkport all / inkport test concurrently with the playground.
        const buildDir = path.join(inkportRoot, 'build', name);
        fs.mkdirSync(path.join(buildDir, 'src'), { recursive: true });
        fs.writeFileSync(path.join(buildDir, `${name}.wasm`), Buffer.from(wasmB64, 'base64'));
        fs.writeFileSync(path.join(buildDir, 'metadata.json'), JSON.stringify(metadata, null, 2));

        // Replace empty-string args with '0' so a user who leaves a numeric
        // field blank still gets a valid deploy (coerce_args rejects empty strings).
        const cleanArgs = (args ?? []).map(a => (String(a).trim() === '' ? '0' : String(a)));
        const argStr = cleanArgs.map(a => `--arg ${a}`).join(' ');
        emit({ type: 'log', cls: 'lg-cmd', text: `$ inkport deploy ${name} ${argStr}`.trim() });

        const argFlags = cleanArgs.flatMap(a => ['--arg', a]);
        const result = await spawnCollect(
          'inkport',
          ['deploy', name, ...argFlags],
          { cwd: inkportRoot, env }
        );

        // Emit output lines
        const allOutput = result.stdout + result.stderr;
        for (const line of allOutput.split('\n').filter(l => l.trim())) {
          let cls = 'lg-dim';
          if (/deployed|✓/.test(line)) cls = 'lg-ok';
          if (/error|Error/.test(line)) cls = 'lg-err';
          if (/⛏|block #/.test(line)) cls = 'lg-warn';
          emit({ type: 'log', cls, text: line });
        }

        if (result.code !== 0) {
          emit({ type: 'error', log: (result.stderr || result.stdout).trim() });
          controller.close();
          return;
        }

        // Parse address: "deployed <Name> -> <SS58addr>"
        const addrMatch = /deployed\s+\S+\s+->\s+(\S+)/.exec(result.stdout);
        if (!addrMatch) {
          emit({ type: 'error', log: 'Could not parse deployed address from inkport output' });
          controller.close();
          return;
        }

        emit({ type: 'address', address: addrMatch[1] });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        emit({ type: 'error', log: msg });
      }

      controller.close();
    },
  });

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    },
  });
}
