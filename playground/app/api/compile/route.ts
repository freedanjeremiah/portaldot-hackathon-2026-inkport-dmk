import { NextRequest } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { sessionDir } from '@/lib/session';
import { spawnCollect, spawnLive } from '@/lib/shell';

function parseName(solidity: string): string {
  const stripped = solidity
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/[^\n]*/g, '');
  const m = /contract\s+([A-Za-z_]\w*)/.exec(stripped);
  return m ? m[1] : 'Contract';
}

function readCrateName(cargoTomlPath: string, fallback: string): string {
  try {
    const toml = fs.readFileSync(cargoTomlPath, 'utf8');
    const line = toml.split('\n').find(l => l.trim().startsWith('name') && l.includes('='));
    if (line) return line.split('=')[1].trim().replace(/"/g, '');
  } catch { /* ignore */ }
  return fallback;
}

export async function POST(request: NextRequest) {
  let body: { solidity?: string; sessionId?: string };
  try { body = await request.json(); } catch {
    const enc = new TextEncoder();
    const msg = JSON.stringify({ type: 'error', log: 'Invalid JSON body' });
    return new Response(`data: ${msg}\n\n`, { headers: { 'Content-Type': 'text/event-stream' } });
  }
  const { solidity, sessionId } = body;

  if (!solidity || !sessionId) {
    const encoder = new TextEncoder();
    const msg = JSON.stringify({ type: 'error', log: 'Missing solidity or sessionId' });
    return new Response(`data: ${msg}\n\n`, {
      headers: { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' },
    });
  }

  const encoder = new TextEncoder();

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      function emit(obj: object) {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify(obj)}\n\n`));
      }

      try {
        const env = buildEnv();
        const inkportRoot = env.INKPORT_ROOT as string;
        const name = parseName(solidity);
        const tmpdir = sessionDir(sessionId);
        const solFile = path.join(tmpdir, `${name}.sol`);
        const buildDir = path.join(tmpdir, 'build', name);


        fs.mkdirSync(path.join(buildDir, 'src'), { recursive: true });
        fs.writeFileSync(solFile, solidity, 'utf8');

        // ── Step 1: translate ──────────────────────────────────────
        emit({ type: 'log', cls: 'lg-cmd', text: `$ inkport-translate ${name}.sol --target seal` });

        const translatorBin = path.join(inkportRoot, 'translator', 'target', 'release', 'inkport-translate');
        const tr = await spawnCollect(translatorBin, [solFile, '--target', 'seal', '--out', buildDir], { env });

        if (tr.code !== 0) {
          emit({ type: 'log', cls: 'lg-err', text: (tr.stderr || tr.stdout).trim() });
          emit({ type: 'error', log: (tr.stderr || tr.stdout).trim() });
          controller.close();
          return;
        }

        emit({ type: 'log', cls: 'lg-dim', text: `  wrote ${name}/src/lib.rs + Cargo.toml + metadata.json` });
        emit({ type: 'log', cls: '', text: '' });

        const metaPath = path.join(buildDir, 'metadata.json');
        const metadata = JSON.parse(fs.readFileSync(metaPath, 'utf8'));
        // Pad argNames so the UI can show parameter labels (translator binary omits them).
        if (!metadata.constructor.argNames) {
          metadata.constructor.argNames = metadata.constructor.args.map((_: string, i: number) => `arg${i}`);
        }
        if (metadata.messages) {
          for (const m of metadata.messages) {
            if (!m.argNames) m.argNames = m.args.map((_: string, i: number) => `arg${i}`);
          }
        }
        const crateName = readCrateName(path.join(buildDir, 'Cargo.toml'), name.toLowerCase());

        // ── Step 2: cargo build (streaming) ───────────────────────
        emit({ type: 'log', cls: 'lg-cmd', text: '$ cargo +stable build --release --target wasm32-unknown-unknown' });

        const cargo = await spawnLive(
          'cargo',
          ['+stable', 'build', '--release', '--target', 'wasm32-unknown-unknown'],
          {
            cwd: buildDir,
            env,
            onLine: (line: string) => {
              let cls = 'lg-dim';
              if (/Finished/.test(line)) cls = 'lg-ok';
              if (/Compiling/.test(line)) cls = 'lg-warn';
              if (/^error/.test(line.trim())) cls = 'lg-err';
              emit({ type: 'log', cls, text: line });
            },
          }
        );

        if (cargo.code !== 0) {
          emit({ type: 'error', log: cargo.stderr });
          controller.close();
          return;
        }

        // ── Step 3: strip_wasm ─────────────────────────────────────
        emit({ type: 'log', cls: '', text: '' });
        emit({ type: 'log', cls: 'lg-cmd', text: `$ strip_wasm ${name}.wasm` });

        // Raw wasm sits in the default cargo target dir inside buildDir.
        const rawWasm = path.join(buildDir, 'target', 'wasm32-unknown-unknown', 'release', `${crateName}.wasm`);
        const strippedWasm = path.join(buildDir, `${name}.wasm`);

        const stripScript = [
          'import sys',
          'sys.path.insert(0, sys.argv[1])',
          'import strip_wasm',
          'n = strip_wasm.strip(sys.argv[2], sys.argv[3])',
          'print(n)',
        ].join('; ');

        const strip = await spawnCollect(
          'python3',
          ['-c', stripScript, path.join(inkportRoot, 'inkport_chain'), rawWasm, strippedWasm],
          { env }
        );

        if (strip.code !== 0) {
          emit({ type: 'log', cls: 'lg-err', text: strip.stderr.trim() });
          emit({ type: 'error', log: strip.stderr.trim() });
          controller.close();
          return;
        }

        const size = parseInt(strip.stdout.trim(), 10);
        const displaySize = isNaN(size) ? '?' : size.toLocaleString();
        emit({ type: 'log', cls: 'lg-ok', text: `✓ ${name}.wasm — ${displaySize} bytes (Portaldot-ready)` });

        // ── Done: base64-encode wasm, emit terminal event ──────────
        const wasmBytes = fs.readFileSync(strippedWasm);
        const wasmB64 = wasmBytes.toString('base64');

        emit({ type: 'wasm', data: wasmB64, metadata, size: isNaN(size) ? wasmBytes.length : size });
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
