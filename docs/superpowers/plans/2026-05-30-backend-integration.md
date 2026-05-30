# Backend Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the real `inkport` CLI into the Next.js playground API routes, replacing the simulated backend completely.

**Architecture:** Next.js API routes use Node's `child_process.spawn` to invoke `inkport-translate`, `cargo build`, `strip_wasm`, `inkport deploy`, and `inkport call` on the same Linux machine. The compile route streams stdout/stderr as SSE log lines; deploy likewise; call returns plain JSON. The client-side live Rust preview (`lib/translator.ts`) is unchanged — it remains a client-side JS simulation used only for the split-pane live preview, not for the actual compile.

**Tech Stack:** Next.js 15 App Router, TypeScript, Node.js `child_process`, ReadableStream SSE, Jest + ts-jest for unit tests.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `playground/lib/env.ts` | Build process env for child_process (PATH, INKPORT_ROOT, etc.) |
| Create | `playground/lib/session.ts` | Session temp dir: `/tmp/inkport-playground/<uuid>/` |
| Create | `playground/lib/shell.ts` | `spawnCollect` + `spawnStream` wrappers |
| Create | `playground/lib/sse.ts` | Client-side SSE fetch reader |
| Create | `playground/.env.local.example` | Documents the 3 required env vars |
| Modify | `playground/app/api/translate/route.ts` | Real inkport-translate invocation |
| Modify | `playground/app/api/compile/route.ts` | Real translate+cargo+strip SSE stream |
| Modify | `playground/app/api/deploy/route.ts` | Real inkport deploy SSE stream |
| Modify | `playground/app/api/call/route.ts` | Real inkport call JSON response |
| Modify | `playground/app/page.tsx` | Remove simbackend, add sessionId+wasmB64, rewrite 3 handlers |
| Delete | `playground/lib/simbackend.ts` | No longer needed |
| Create | `playground/__tests__/env.test.ts` | Unit test buildEnv() |
| Create | `playground/__tests__/session.test.ts` | Unit test sessionDir() |
| Create | `playground/__tests__/shell.test.ts` | Unit test spawnCollect() |
| Create | `playground/__tests__/sse.test.ts` | Unit test readSSE() |
| Modify | `playground/package.json` | Add jest + ts-jest devDependencies |
| Create | `playground/jest.config.ts` | Jest config for Next.js TypeScript |

---

## Task 1: Add Jest to playground

**Files:**
- Modify: `playground/package.json`
- Create: `playground/jest.config.ts`

- [ ] **Step 1: Add jest dependencies to package.json**

Replace the `devDependencies` block in `playground/package.json`:
```json
{
  "name": "inkport-playground",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "test": "jest"
  },
  "dependencies": {
    "next": "^15.1.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@types/jest": "^29.5.0",
    "@types/node": "^22.0.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "jest": "^29.7.0",
    "ts-jest": "^29.2.0",
    "typescript": "^5.0.0"
  }
}
```

- [ ] **Step 2: Create jest.config.ts**

Create `playground/jest.config.ts`:
```typescript
import type { Config } from 'jest';

const config: Config = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/$1',
  },
  testMatch: ['**/__tests__/**/*.test.ts'],
};

export default config;
```

- [ ] **Step 3: Install dependencies**

```bash
cd playground && npm install
```

Expected: Installs jest, ts-jest, @types/jest. No errors.

- [ ] **Step 4: Verify jest runs**

```bash
cd playground && npx jest --listTests
```

Expected: No tests found yet (that's fine — zero output is correct).

- [ ] **Step 5: Commit**

```bash
git add playground/package.json playground/jest.config.ts playground/package-lock.json
git commit -m "Add Jest + ts-jest to playground"
```

---

## Task 2: `lib/env.ts` and `lib/session.ts`

**Files:**
- Create: `playground/lib/env.ts`
- Create: `playground/lib/session.ts`
- Create: `playground/__tests__/env.test.ts`
- Create: `playground/__tests__/session.test.ts`

- [ ] **Step 1: Write failing tests**

Create `playground/__tests__/env.test.ts`:
```typescript
import path from 'path';

// Must set before importing to avoid module-level evaluation issues
process.env.INKPORT_ROOT = '/fake/inkport';
process.env.CARGO_HOME = '/fake/.cargo';
process.env.INKPORT_VENV = '/fake/.venv';

import { buildEnv } from '@/lib/env';

describe('buildEnv', () => {
  it('includes venv/bin and cargo/bin in PATH', () => {
    const env = buildEnv();
    expect(env.PATH).toContain('/fake/.venv/bin');
    expect(env.PATH).toContain('/fake/.cargo/bin');
  });

  it('exposes INKPORT_ROOT', () => {
    const env = buildEnv();
    expect(env.INKPORT_ROOT).toBe('/fake/inkport');
  });
});
```

Create `playground/__tests__/session.test.ts`:
```typescript
import os from 'os';
import path from 'path';
import fs from 'fs';
import { sessionDir } from '@/lib/session';

describe('sessionDir', () => {
  it('returns path under /tmp/inkport-playground/<uuid>', () => {
    const dir = sessionDir('test-uuid-1234');
    expect(dir).toBe('/tmp/inkport-playground/test-uuid-1234');
  });

  it('creates the directory', () => {
    const dir = sessionDir('test-uuid-mkdir');
    expect(fs.existsSync(dir)).toBe(true);
    fs.rmSync(dir, { recursive: true });
  });
});
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cd playground && npx jest --testPathPattern="env|session" 2>&1 | head -20
```

Expected: `Cannot find module '@/lib/env'`

- [ ] **Step 3: Create `lib/env.ts`**

Create `playground/lib/env.ts`:
```typescript
import path from 'path';

export function buildEnv(): NodeJS.ProcessEnv {
  const inkportRoot = process.env.INKPORT_ROOT ?? '';
  const cargoHome = process.env.CARGO_HOME ?? path.join(process.env.HOME ?? '', '.cargo');
  const inkportVenv = process.env.INKPORT_VENV ?? '';

  const extraPath = [
    inkportVenv ? path.join(inkportVenv, 'bin') : '',
    cargoHome ? path.join(cargoHome, 'bin') : '',
  ].filter(Boolean).join(':');

  return {
    ...process.env,
    PATH: `${extraPath}:${process.env.PATH ?? ''}`,
    INKPORT_ROOT: inkportRoot,
    CARGO_HOME: cargoHome,
    INKPORT_VENV: inkportVenv,
  };
}
```

- [ ] **Step 4: Create `lib/session.ts`**

Create `playground/lib/session.ts`:
```typescript
import path from 'path';
import fs from 'fs';

export function sessionDir(sessionId: string): string {
  const dir = path.join('/tmp', 'inkport-playground', sessionId);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}
```

- [ ] **Step 5: Run tests — expect PASS**

```bash
cd playground && npx jest --testPathPattern="env|session"
```

Expected:
```
PASS __tests__/env.test.ts
PASS __tests__/session.test.ts
```

- [ ] **Step 6: Create `.env.local.example`**

Create `playground/.env.local.example`:
```
# Copy to .env.local and fill in the correct paths for your dev machine.
# .env.local is gitignored and must be created manually.

# Absolute path to the inkport repo root (contains translator/, inkport_chain/, etc.)
INKPORT_ROOT=/home/ubuntu/projects/inkide

# Cargo home directory (parent of bin/cargo)
CARGO_HOME=/home/ubuntu/.cargo

# Python venv with inkport + substrate-interface installed
INKPORT_VENV=/home/ubuntu/projects/inkide/.venv
```

- [ ] **Step 7: Commit**

```bash
git add playground/lib/env.ts playground/lib/session.ts playground/__tests__/env.test.ts playground/__tests__/session.test.ts playground/.env.local.example
git commit -m "Add lib/env.ts, lib/session.ts with tests + .env.local.example"
```

---

## Task 3: `lib/shell.ts`

**Files:**
- Create: `playground/lib/shell.ts`
- Create: `playground/__tests__/shell.test.ts`

- [ ] **Step 1: Write failing tests**

Create `playground/__tests__/shell.test.ts`:
```typescript
import { spawnCollect } from '@/lib/shell';

describe('spawnCollect', () => {
  it('captures stdout and returns exit 0', async () => {
    const result = await spawnCollect('echo', ['hello world'], {});
    expect(result.stdout.trim()).toBe('hello world');
    expect(result.code).toBe(0);
  });

  it('captures stderr and returns non-zero exit on failure', async () => {
    const result = await spawnCollect('bash', ['-c', 'echo err >&2; exit 1'], {});
    expect(result.stderr.trim()).toBe('err');
    expect(result.code).toBe(1);
  });
});
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cd playground && npx jest --testPathPattern="shell"
```

Expected: `Cannot find module '@/lib/shell'`

- [ ] **Step 3: Create `lib/shell.ts`**

Create `playground/lib/shell.ts`:
```typescript
import { spawn } from 'child_process';

export interface SpawnResult {
  stdout: string;
  stderr: string;
  code: number;
}

export function spawnCollect(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv }
): Promise<SpawnResult> {
  return new Promise((resolve) => {
    const proc = spawn(cmd, args, {
      cwd: opts.cwd,
      env: opts.env ?? process.env,
      shell: false,
    });
    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', (d: Buffer) => { stdout += d.toString(); });
    proc.stderr.on('data', (d: Buffer) => { stderr += d.toString(); });
    proc.on('close', (code: number | null) => resolve({ stdout, stderr, code: code ?? 1 }));
  });
}

function classifyLine(line: string): string {
  const t = line.trim();
  if (/^✓|Finished/.test(t)) return 'lg-ok';
  if (/^✗|^error[^:]*:/i.test(t)) return 'lg-err';
  if (/^warning/i.test(t)) return 'lg-warn';
  if (/^\$/.test(t)) return 'lg-cmd';
  return 'lg-dim';
}

export function spawnStream(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv }
): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();

  return new ReadableStream<Uint8Array>({
    start(controller) {
      const proc = spawn(cmd, args, {
        cwd: opts.cwd,
        env: opts.env ?? process.env,
        shell: false,
      });

      function emitLine(line: string) {
        const cls = classifyLine(line);
        const event = JSON.stringify({ type: 'log', cls, text: line });
        controller.enqueue(encoder.encode(`data: ${event}\n\n`));
      }

      let stderrBuf = '';

      proc.stdout.on('data', (chunk: Buffer) => {
        chunk.toString().split('\n').forEach(l => { if (l) emitLine(l); });
      });
      proc.stderr.on('data', (chunk: Buffer) => {
        stderrBuf += chunk.toString();
        chunk.toString().split('\n').forEach(l => { if (l) emitLine(l); });
      });
      proc.on('close', (code: number | null) => {
        if ((code ?? 1) === 0) {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'done' })}\n\n`));
        } else {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'error', log: stderrBuf })}\n\n`));
        }
        controller.close();
      });
    },
  });
}
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cd playground && npx jest --testPathPattern="shell"
```

Expected:
```
PASS __tests__/shell.test.ts
  spawnCollect
    ✓ captures stdout and returns exit 0
    ✓ captures stderr and returns non-zero exit on failure
```

- [ ] **Step 5: Commit**

```bash
git add playground/lib/shell.ts playground/__tests__/shell.test.ts
git commit -m "Add lib/shell.ts (spawnCollect + spawnStream) with tests"
```

---

## Task 4: `lib/sse.ts` (client-side SSE reader)

**Files:**
- Create: `playground/lib/sse.ts`
- Create: `playground/__tests__/sse.test.ts`

- [ ] **Step 1: Write failing test**

Create `playground/__tests__/sse.test.ts`:
```typescript
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
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
cd playground && npx jest --testPathPattern="sse"
```

Expected: `Cannot find module '@/lib/sse'`

- [ ] **Step 3: Create `lib/sse.ts`**

Create `playground/lib/sse.ts`:
```typescript
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
  const response = await fetch(url, {
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
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cd playground && npx jest --testPathPattern="sse"
```

Expected:
```
PASS __tests__/sse.test.ts
  readSSE
    ✓ calls onLine for log events and onDone for terminal wasm event
    ✓ calls onError for error events
```

- [ ] **Step 5: Commit**

```bash
git add playground/lib/sse.ts playground/__tests__/sse.test.ts
git commit -m "Add lib/sse.ts (client-side SSE reader) with tests"
```

---

## Task 5: `POST /api/translate` — real implementation

**Files:**
- Modify: `playground/app/api/translate/route.ts`

- [ ] **Step 1: Replace the stub with the real implementation**

Overwrite `playground/app/api/translate/route.ts`:
```typescript
import { NextRequest, NextResponse } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { sessionDir } from '@/lib/session';
import { spawnCollect } from '@/lib/shell';

function parseName(solidity: string): string {
  const m = /contract\s+([A-Za-z_]\w*)/.exec(solidity);
  return m ? m[1] : 'Contract';
}

export async function POST(request: NextRequest) {
  const body = await request.json();
  const { solidity, sessionId } = body as { solidity: string; sessionId: string };

  if (!solidity || !sessionId) {
    return NextResponse.json({ error: 'Missing solidity or sessionId' }, { status: 400 });
  }

  const env = buildEnv();
  const inkportRoot = env.INKPORT_ROOT as string;
  const name = parseName(solidity);
  const tmpdir = sessionDir(sessionId);
  const solFile = path.join(tmpdir, `${name}.sol`);
  const buildDir = path.join(tmpdir, 'build', name);

  fs.mkdirSync(path.join(buildDir, 'src'), { recursive: true });
  fs.writeFileSync(solFile, solidity, 'utf8');

  const translatorBin = path.join(
    inkportRoot, 'translator', 'target', 'release', 'inkport-translate'
  );

  const result = await spawnCollect(
    translatorBin,
    [solFile, '--target', 'seal', '--out', buildDir],
    { env }
  );

  if (result.code !== 0) {
    return NextResponse.json(
      { error: (result.stderr || result.stdout).trim() },
      { status: 400 }
    );
  }

  const rustPath = path.join(buildDir, 'src', 'lib.rs');
  const metaPath = path.join(buildDir, 'metadata.json');
  const rust = fs.readFileSync(rustPath, 'utf8');
  const metadata = JSON.parse(fs.readFileSync(metaPath, 'utf8'));

  return NextResponse.json({ rust, metadata });
}
```

- [ ] **Step 2: Manually verify on dev machine**

On the Linux dev machine with the venv active and `.env.local` set:
```bash
cd playground
curl -s -X POST http://localhost:3000/api/translate \
  -H 'Content-Type: application/json' \
  -d '{"sessionId":"test-123","solidity":"pragma solidity ^0.8.0; contract Counter { uint256 public count; function inc() public { count++; } }"}' \
  | python3 -m json.tool | head -20
```

Expected: JSON with `rust` (seal0 Rust source) and `metadata` (name, constructor, messages).

- [ ] **Step 3: Commit**

```bash
git add playground/app/api/translate/route.ts
git commit -m "Wire /api/translate to real inkport-translate binary"
```

---

## Task 6: `POST /api/compile` — SSE streaming implementation

**Files:**
- Modify: `playground/app/api/compile/route.ts`

- [ ] **Step 1: Replace the stub with the real SSE implementation**

Overwrite `playground/app/api/compile/route.ts`:
```typescript
import { NextRequest } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { sessionDir } from '@/lib/session';
import { spawnCollect } from '@/lib/shell';

function parseName(solidity: string): string {
  const m = /contract\s+([A-Za-z_]\w*)/.exec(solidity);
  return m ? m[1] : 'Contract';
}

export async function POST(request: NextRequest) {
  const { solidity, sessionId } = await request.json() as { solidity: string; sessionId: string };
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
        emit({ type: 'log', cls: 'lg-cmd', text: `$ inkport-translate ${name}.sol --target seal --out build/${name}/` });

        const translatorBin = path.join(inkportRoot, 'translator', 'target', 'release', 'inkport-translate');
        const tr = await spawnCollect(translatorBin, [solFile, '--target', 'seal', '--out', buildDir], { env });

        if (tr.code !== 0) {
          emit({ type: 'log', cls: 'lg-err', text: (tr.stderr || tr.stdout).trim() });
          emit({ type: 'error', log: (tr.stderr || tr.stdout).trim() });
          controller.close();
          return;
        }

        emit({ type: 'log', cls: 'lg-dim', text: `  wrote build/${name}/src/lib.rs` });
        emit({ type: 'log', cls: 'lg-dim', text: `  wrote build/${name}/metadata.json` });
        emit({ type: 'log', cls: '', text: '' });

        const metaPath = path.join(buildDir, 'metadata.json');
        const metadata = JSON.parse(fs.readFileSync(metaPath, 'utf8'));
        const crateName = name.toLowerCase().replace(/[^a-z0-9_]/g, '_');

        // ── Step 2: cargo build ────────────────────────────────────
        emit({ type: 'log', cls: 'lg-cmd', text: '$ cargo +stable build --release --target wasm32-unknown-unknown' });

        const cargo = await spawnCollect(
          'cargo',
          ['+stable', 'build', '--release', '--target', 'wasm32-unknown-unknown'],
          { cwd: buildDir, env }
        );

        // Emit every non-empty line from cargo (stdout + stderr interleaved)
        const cargoLines = (cargo.stdout + cargo.stderr).split('\n').filter(l => l.trim());
        for (const line of cargoLines) {
          let cls = 'lg-dim';
          if (/Finished/.test(line)) cls = 'lg-warn';
          if (/^error/.test(line.trim())) cls = 'lg-err';
          emit({ type: 'log', cls, text: line });
        }

        if (cargo.code !== 0) {
          emit({ type: 'error', log: cargo.stderr });
          controller.close();
          return;
        }

        // ── Step 3: strip_wasm ─────────────────────────────────────
        emit({ type: 'log', cls: '', text: '' });
        emit({ type: 'log', cls: 'lg-cmd', text: `$ strip_wasm ${name}.wasm` });

        const rawWasm = path.join(
          buildDir, 'target', 'wasm32-unknown-unknown', 'release', `${crateName}.wasm`
        );
        const strippedWasm = path.join(buildDir, `${name}.wasm`);

        const stripScript = [
          'import sys',
          `sys.path.insert(0, sys.argv[1])`,
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
        emit({ type: 'log', cls: 'lg-ok', text: `✓ ${name}.wasm — ${displaySize} bytes stripped (Portaldot-compatible)` });

        // ── Done: read wasm, base64-encode, emit terminal event ────
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
```

- [ ] **Step 2: Manually verify on dev machine**

```bash
curl -s -N -X POST http://localhost:3000/api/compile \
  -H 'Content-Type: application/json' \
  -d '{"sessionId":"test-456","solidity":"pragma solidity ^0.8.0; contract Counter { uint256 public count; constructor(uint256 i) { count = i; } function inc() public { count++; } function get() public view returns (uint256) { return count; } }"}'
```

Expected: SSE lines streaming — translate logs, cargo compile lines, strip log, then final `data: {"type":"wasm","data":"...","metadata":{...},"size":...}`.

- [ ] **Step 3: Commit**

```bash
git add playground/app/api/compile/route.ts
git commit -m "Wire /api/compile to real translate+cargo+strip pipeline (SSE)"
```

---

## Task 7: `POST /api/deploy` — SSE streaming implementation

**Files:**
- Modify: `playground/app/api/deploy/route.ts`

- [ ] **Step 1: Replace the stub with the real SSE implementation**

Overwrite `playground/app/api/deploy/route.ts`:
```typescript
import { NextRequest } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { spawnCollect } from '@/lib/shell';

export async function POST(request: NextRequest) {
  const { wasmB64, metadata, args } = await request.json() as {
    wasmB64: string;
    metadata: { name: string; [k: string]: unknown };
    args: string[];
  };

  const encoder = new TextEncoder();
  const name = metadata.name;

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      function emit(obj: object) {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify(obj)}\n\n`));
      }

      try {
        const env = buildEnv();
        const inkportRoot = env.INKPORT_ROOT as string;

        // Write wasm + metadata to INKPORT_ROOT/build/<name>/ where inkport deploy expects them
        const buildDir = path.join(inkportRoot, 'build', name);
        fs.mkdirSync(path.join(buildDir, 'src'), { recursive: true });
        fs.writeFileSync(path.join(buildDir, `${name}.wasm`), Buffer.from(wasmB64, 'base64'));
        fs.writeFileSync(path.join(buildDir, 'metadata.json'), JSON.stringify(metadata, null, 2));

        const argStr = args.map(a => `--arg ${a}`).join(' ');
        emit({ type: 'log', cls: 'lg-cmd', text: `$ inkport deploy ${name} ${argStr}` });

        const argFlags = args.flatMap(a => ['--arg', String(a)]);
        const result = await spawnCollect(
          'inkport',
          ['deploy', name, ...argFlags],
          { cwd: inkportRoot, env }
        );

        // Stream output lines
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
```

- [ ] **Step 2: Manually verify on dev machine**

After running compile to get a wasmB64, test deploy (replace `<WASM_B64>` with actual output):
```bash
curl -s -N -X POST http://localhost:3000/api/deploy \
  -H 'Content-Type: application/json' \
  -d '{"wasmB64":"<WASM_B64>","metadata":{"name":"Counter","constructor":{"args":["u128"],"argNames":["initial"]},"messages":[],"events":[]},"args":["0"],"sessionId":"test-456"}'
```

Expected: SSE log lines then `data: {"type":"address","address":"5Xxxx..."}`.

- [ ] **Step 3: Commit**

```bash
git add playground/app/api/deploy/route.ts
git commit -m "Wire /api/deploy to real inkport deploy command (SSE)"
```

---

## Task 8: `POST /api/call` — real implementation

**Files:**
- Modify: `playground/app/api/call/route.ts`

- [ ] **Step 1: Replace the stub with the real implementation**

Overwrite `playground/app/api/call/route.ts`:
```typescript
import { NextRequest, NextResponse } from 'next/server';
import { buildEnv } from '@/lib/env';
import { spawnCollect } from '@/lib/shell';

export async function POST(request: NextRequest) {
  const { metadata, message, args } = await request.json() as {
    metadata: { name: string };
    message: string;
    args: string[];
  };

  const name = metadata.name;
  const env = buildEnv();
  const inkportRoot = env.INKPORT_ROOT as string;

  const argFlags = (args ?? []).flatMap(a => ['--arg', String(a)]);

  const result = await spawnCollect(
    'inkport',
    ['call', name, message, ...argFlags],
    { cwd: inkportRoot, env }
  );

  if (result.code !== 0) {
    return NextResponse.json(
      { error: (result.stderr || result.stdout).trim() },
      { status: 400 }
    );
  }

  // inkport call prints: "call Name.msg(...) -> <value>"
  // followed by a JSON line: {"result": <value>}
  let parsed: unknown = null;
  try {
    const jsonLine = result.stdout.split('\n').find(l => l.trim().startsWith('{'));
    if (jsonLine) parsed = (JSON.parse(jsonLine) as { result: unknown }).result;
  } catch { /* ignore */ }

  if (parsed === null) {
    const match = /call\s+\S+\s+->\s+(.+)/.exec(result.stdout);
    parsed = match ? match[1].trim() : 'ok';
  }

  return NextResponse.json({ result: parsed, events: [] });
}
```

- [ ] **Step 2: Manually verify on dev machine**

After deploy, call the `get` message:
```bash
curl -s -X POST http://localhost:3000/api/call \
  -H 'Content-Type: application/json' \
  -d '{"metadata":{"name":"Counter"},"message":"get","args":[]}'
```

Expected: `{"result":"0","events":[]}` (or the deployed initial value).

- [ ] **Step 3: Commit**

```bash
git add playground/app/api/call/route.ts
git commit -m "Wire /api/call to real inkport call command"
```

---

## Task 9: Frontend — replace simbackend with real API calls

**Files:**
- Modify: `playground/components/Pipeline.tsx`
- Modify: `playground/app/page.tsx`
- Delete: `playground/lib/simbackend.ts`

- [ ] **Step 1: Fix `components/Pipeline.tsx` imports**

`Pipeline.tsx` imports `LogLine` and `CallState` from `simbackend.ts` — delete those before simbackend is removed.

In `playground/components/Pipeline.tsx`, find and remove:
`	ypescript
import type { LogLine, CallState } from '@/lib/simbackend';
`

Replace with (just after the other imports):
`	ypescript
export type LogSegment = [string, string];
export type LogLine = LogSegment[];
`

Remove every remaining reference to `CallState` in `Pipeline.tsx` (search for it — it should not appear anywhere after this change).

- [ ] **Step 2: Update `app/page.tsx` imports**

In `playground/app/page.tsx`, remove:
`	ypescript
import { buildCompile, buildDeploy, buildCall, stream } from '@/lib/simbackend';
import type { LogLine, CallState } from '@/lib/simbackend';
`

Add:
`	ypescript
import { readSSE } from '@/lib/sse';
import type { SSEPayload } from '@/lib/sse';
import type { LogLine } from '@/components/Pipeline';
`

- [ ] **Step 3: Add `wasmB64` state and `sessionId` ref**

In the `PlaygroundPage` function body, add after the existing `useState` declarations:
`	ypescript
const [wasmB64, setWasmB64] = useState<string | null>(null);
const sessionIdRef = useRef<string>('');

useEffect(() => {
  if (!sessionIdRef.current) {
    const stored = sessionStorage.getItem('inkport-session-id');
    const id = stored ?? crypto.randomUUID();
    if (!stored) sessionStorage.setItem('inkport-session-id', id);
    sessionIdRef.current = id;
  }
}, []);
`
- [ ] **Step 4: Rewrite `onCompile`**

Replace the entire `onCompile` function:
```typescript
const onCompile = async () => {
  if (compile.running) return;
  setActive('compile');
  setCompile({ lines: [], running: true, wasmSize: 0, error: null });
  setWasmB64(null);

  try {
    await readSSE(
      '/api/compile',
      { solidity, sessionId: sessionIdRef.current },
      {
        onLine: (payload: SSEPayload) => {
          const seg: LogLine = [[payload.cls ?? 'lg-dim', payload.text ?? '']];
          setCompile(c => ({ ...c, lines: [...c.lines, seg] }));
        },
        onDone: (payload: SSEPayload) => {
          if (payload.type === 'wasm') {
            const wasm = payload.data as string;
            const meta = payload.metadata as Metadata;
            const sz = (payload.size as number) ?? 0;
            setWasmB64(wasm);
            setMetadata(meta);
            setCompile(c => ({ ...c, running: false, wasmSize: sz }));
            setActive('deploy');
          }
        },
        onError: (payload: SSEPayload) => {
          setCompile(c => ({ ...c, running: false, error: payload.log as string ?? 'compile failed' }));
        },
      }
    );
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    setCompile(c => ({ ...c, running: false, error: msg }));
  }
};
```

- [ ] **Step 5: Rewrite `onDeploy`**

Replace the entire `onDeploy` function:
```typescript
const onDeploy = async () => {
  if (deploy.running || !wasmB64) return;
  setActive('deploy');
  setDeploy(d => ({ ...d, running: true, lines: [], address: null, error: null }));

  try {
    await readSSE(
      '/api/deploy',
      { wasmB64, metadata: meta, args: deploy.args, sessionId: sessionIdRef.current },
      {
        onLine: (payload: SSEPayload) => {
          const seg: LogLine = [[payload.cls ?? 'lg-dim', payload.text ?? '']];
          setDeploy(d => ({ ...d, lines: [...d.lines, seg] }));
        },
        onDone: (payload: SSEPayload) => {
          if (payload.type === 'address') {
            setDeploy(d => ({ ...d, running: false, address: payload.address as string }));
            setActive('call');
          }
        },
        onError: (payload: SSEPayload) => {
          setDeploy(d => ({ ...d, running: false, error: payload.log as string ?? 'deploy failed' }));
        },
      }
    );
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    setDeploy(d => ({ ...d, running: false, error: msg }));
  }
};
```

- [ ] **Step 6: Rewrite `onCall`**

Replace the entire `onCall` function:
```typescript
const onCall = async () => {
  if (call.running || !deploy.address) return;
  const msg = meta.messages[call.selected];
  if (!msg) return;

  setCall(c => ({ ...c, running: true, lines: [], result: null, events: [], error: null }));

  try {
    const response = await fetch('/api/call', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        metadata: meta,
        message: msg.name,
        args: call.args,
        sessionId: sessionIdRef.current,
      }),
    });

    const data = await response.json() as { result?: unknown; events?: unknown[]; error?: string };

    if (!response.ok || data.error) {
      setCall(c => ({ ...c, running: false, error: data.error ?? 'call failed' }));
    } else {
      setCall(c => ({
        ...c,
        running: false,
        result: String(data.result ?? 'ok'),
        events: data.events ?? [],
        error: null,
      }));
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    setCall(c => ({ ...c, running: false, error: msg }));
  }
};
```

- [ ] **Step 7: Remove `cancelRef` and `counter` state (no longer used)**

Remove these lines from `PlaygroundPage`:
```typescript
const [counter, setCounter] = useState<CallState>({ count: 0 });
const cancelRef = useRef<(() => void) | null>(null);
```

Remove any remaining references to `cancelRef` and `counter` in the JSX/handlers.

- [ ] **Step 8: Remove the `bus` keys that referenced simbackend**

The `bus` object had a `copy` function — keep it. Remove these unused bus keys if they exist:
- Nothing from bus needs removal — the pipeline components still use `bus.compile`, `bus.deploy`, `bus.call`, the same structure.

Update the `bus` object's `onCompile`, `onDeploy`, `onCall` — they now reference the async versions. Since `bus.onCompile` etc. are just references, the existing `bus` construction still works:
```typescript
const bus = {
  metadata: meta, statuses, compile, deploy, call,
  onCompile, onDeploy, onCall,
  setDeployArg: (i: number, v: string) => setDeploy(d => { const a = [...d.args]; a[i] = v; return { ...d, args: a }; }),
  setCallMsg: (i: number) => setCall(c => ({ ...c, selected: i, args: [], value: '', result: null, error: null, lines: [] })),
  setCallArg: (i: number, v: string) => setCall(c => { const a = [...c.args]; a[i] = v; return { ...c, args: a }; }),
  setCallValue: (v: string) => setCall(c => ({ ...c, value: v })),
  copy: (t: string) => { navigator.clipboard?.writeText(t); },
};
```

- [ ] **Step 9: Delete `lib/simbackend.ts`**

```bash
rm playground/lib/simbackend.ts
```

- [ ] **Step 10: Fix any TypeScript errors**

```bash
cd playground && npx tsc --noEmit 2>&1
```

Fix any remaining type errors. Common ones:
- `LogLine` type: make sure it's defined in `page.tsx` as `type LogLine = [string, string][]`
- `CallState` type: was imported from simbackend, now unused — remove it
- `counter` / `cancelRef`: remove any remaining references

- [ ] **Step 11: Run all unit tests to confirm nothing broke**

```bash
cd playground && npx jest
```

Expected: All 4 test files pass (env, session, shell, sse).

- [ ] **Step 12: Smoke test the full pipeline on dev machine**

1. Start the dev server: `cd playground && npm run dev`
2. Open `http://localhost:3000`
3. The default Counter.sol should show seal0 Rust live (still driven by `lib/translator.ts`)
4. Click **Compile** — real cargo build streams in the log panel
5. Click **Deploy** — real instantiate_with_code on Portaldot
6. Select `get`, click **Read** — returns the initial value
7. Select `inc`, click **Send** — increments on-chain
8. Select `get` again, click **Read** — value should be incremented

- [ ] **Step 13: Commit**

```bash
git add playground/app/page.tsx playground/lib/sse.ts
git rm playground/lib/simbackend.ts
git commit -m "Replace simulated backend with real inkport CLI calls in playground"
```

---

## All tests

```bash
cd playground && npx jest
```

Expected:
```
PASS __tests__/env.test.ts
PASS __tests__/session.test.ts
PASS __tests__/shell.test.ts
PASS __tests__/sse.test.ts

Test Suites: 4 passed, 4 total
Tests:       7 passed, 7 total
```
