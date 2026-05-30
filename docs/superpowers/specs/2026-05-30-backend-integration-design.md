# InkPort Playground — Real Backend Integration Design Spec

**Date:** 2026-05-30  
**Status:** Approved  
**Approach:** Option A — Next.js API routes shell out directly to inkport CLI  

---

## 1. Overview

Wire the real `inkport` CLI into the existing Next.js playground API routes, completely
replacing the simulated backend (`lib/simbackend.ts`). The playground runs on the same Linux
machine as the inkport CLI. Node's `child_process.spawn` invokes the CLI tools directly from
the API routes. The live Rust preview (`lib/translator.ts`, client-side) is unchanged.

---

## 2. Architecture

**Single Next.js process** — no extra server. API routes shell out to:
- `inkport-translate` (Rust binary) — codegen only, fast
- `cargo +stable build --release --target wasm32-unknown-unknown` — full compile
- `python3 -c "import strip_wasm; strip_wasm.strip(...)"` — MVP-wasm stripper
- `inkport deploy / call` (Python CLI) — chain operations

**Session temp directories** — each browser session has a UUID sent with every request.
Routes create and use `/tmp/inkport-playground/<uuid>/` for all session files (Solidity
source, generated Rust crate, built wasm). No cleanup — OS handles temp dirs on reboot.

**Environment** — routes need `$HOME/.cargo/env` sourced and the Python venv active.
`lib/env.ts` builds the correct `env` object for `spawn` from three env vars set in
`playground/.env.local` (gitignored) on the dev machine.

---

## 3. New Files

### `lib/env.ts`

Exports `buildEnv(): NodeJS.ProcessEnv`. Merges `process.env` with:

```
PATH = $INKPORT_VENV/bin:$CARGO_HOME/bin:$PATH
INKPORT_ROOT = /path/to/inkport/repo
```

Reads from `playground/.env.local`:

| Variable | Example | Purpose |
|---|---|---|
| `INKPORT_ROOT` | `/home/ubuntu/projects/inkide` | Repo root — finds translator binary + inkport_chain |
| `CARGO_HOME` | `/home/ubuntu/.cargo` | Finds `cargo` binary |
| `INKPORT_VENV` | `/home/ubuntu/projects/inkide/.venv` | Python venv with inkport + substrate-interface |

### `lib/session.ts`

Exports `sessionDir(sessionId: string): string` → `/tmp/inkport-playground/<uuid>/`.
Callers run `fs.mkdirSync(dir, { recursive: true })` before use.

### `lib/shell.ts`

Exports two functions:

**`spawnStream(cmd, args, opts): ReadableStream<Uint8Array>`**
- Spawns a process, returns an SSE `ReadableStream`
- Each stdout/stderr line classified by prefix into a log class:
  - Lines starting with `✓` → `lg-ok`
  - Lines starting with `✗` or `error` → `lg-err`
  - Lines starting with `warning` → `lg-warn`
  - Everything else → `lg-dim`
- Emits each line as: `data: {"type":"log","cls":"<cls>","text":"<line>"}\n\n`
- On process exit: emits `data: {"type":"done"}\n\n` (exit 0) or `data: {"type":"error","log":"<stderr>"}\n\n` (exit ≠ 0)
- Used by `/api/compile` and `/api/deploy`

**`spawnCollect(cmd, args, opts): Promise<{stdout, stderr, code}>`**
- Runs a process to completion, returns all output + exit code
- Used by `/api/translate` and `/api/call`

### `lib/sse.ts` (client-side)

Exports `readSSE(url, body, { onLine, onDone, onError })`:
- Opens a `fetch` POST to an SSE route
- Reads the response body as a stream, parses `data: ...` lines
- Calls `onLine(parsed)` for `type:"log"` events
- Calls `onDone(parsed)` for `type:"wasm"` or `type:"address"` events
- Calls `onError(parsed)` for `type:"error"` events

Replaces the old `stream()` function from `lib/simbackend.ts`.

### `playground/.env.local.example` (committed)

```
# Copy to .env.local and fill in paths for your dev machine
INKPORT_ROOT=/home/ubuntu/projects/inkide
CARGO_HOME=/home/ubuntu/.cargo
INKPORT_VENV=/home/ubuntu/projects/inkide/.venv
```

---

## 4. API Routes

### `POST /api/translate`

**Body:** `{ solidity: string, sessionId: string }`

**Steps:**
1. `sessionDir(sessionId)` → tmpdir
2. Parse contract name from `contract Foo {` regex
3. Write solidity to `<tmpdir>/<name>.sol`
4. `spawnCollect("inkport-translate", [solFile, "--target", "seal", "--out", buildDir], env)`
5. On success: read `buildDir/src/lib.rs` + `buildDir/metadata.json`
6. Return `{ rust: string, metadata: object }`

**On error:** Return `{ error: string }` with status 400.

---

### `POST /api/compile` (SSE)

**Body:** `{ solidity: string, sessionId: string }`

**Steps:**
1. Parse name, write `.sol` to tmpdir
2. Run `inkport-translate` via `spawnCollect` (fast, not streamed)
3. On translate error: emit error SSE event and close
4. Run `cargo +stable build --release --target wasm32-unknown-unknown` via `spawnStream` (cwd = build dir) — stream all output as SSE log lines
5. On cargo success: run `spawnCollect("python3", ["-c", "import sys; sys.path.insert(0, INKPORT_ROOT+'/inkport_chain'); import strip_wasm; strip_wasm.strip(raw, out)"])` 
6. Read stripped wasm, base64-encode it
7. Emit terminal event: `{ type:"wasm", data: base64, metadata: object }`

**Response headers:** `Content-Type: text/event-stream`

---

### `POST /api/deploy` (SSE)

**Body:** `{ wasmB64: string, metadata: object, args: string[], sessionId: string }`

**Steps:**
1. Decode `wasmB64` and write to `INKPORT_ROOT/build/<name>/<name>.wasm` — `inkport deploy`
   reads the wasm from `build/<name>/` relative to the repo root, so the file must be there.
   Also write `metadata.json` to `INKPORT_ROOT/build/<name>/metadata.json`.
2. Build `inkport deploy <name> --arg <a1> --arg <a2>...` arg list
3. `spawnStream("inkport", ["deploy", name, ...argFlags], { cwd: INKPORT_ROOT, env })`
4. Stream log lines as SSE
5. `inkport deploy` saves the address to `INKPORT_ROOT/deployments/portaldot.json` automatically.
   Parse the deployed address from stdout (line matching `deployed <name> -> <addr>`).
6. Emit terminal event: `{ type:"address", address: string }`

---

### `POST /api/call`

**Body:** `{ address: string, metadata: object, message: string, args: string[], sessionId: string }`

**Steps:**
1. `inkport call` reads the contract address from `INKPORT_ROOT/deployments/portaldot.json`
   (written by the prior deploy step). No extra setup needed — just run the command.
2. Build `inkport call <name> <message> --arg ...` arg list
3. `spawnCollect("inkport", ["call", name, message, ...argFlags], { cwd: INKPORT_ROOT, env })`
4. Parse stdout for result value (line matching `-> <value>` or JSON output)
5. Return `{ result: string, events: object[] }` or `{ error: string }`

---

## 5. Frontend Changes (`app/page.tsx`)

**Deleted:** All imports from `lib/simbackend.ts`. File itself deleted.

**Added:** `sessionId` — generated once with `crypto.randomUUID()` in a `useRef`, persisted in `sessionStorage`.

**`onCompile` rewrite:**
```
readSSE('/api/compile', { solidity, sessionId },
  { onLine: append to compile.lines,
    onDone: store wasmB64 + metadata in state, advance to deploy,
    onError: mark compile failed })
```

**`onDeploy` rewrite:**
```
readSSE('/api/deploy', { wasmB64, metadata, args, sessionId },
  { onLine: append to deploy.lines,
    onDone: store address in state, advance to call,
    onError: mark deploy failed })
```

**`onCall` rewrite:**
```
fetch('/api/call', { method:'POST', body: JSON.stringify({address, metadata, message, args, sessionId}) })
  .then(r => r.json())
  .then(({ result, events, error }) => update call state)
```

**Unchanged:** `lib/translator.ts` (live Rust preview), `lib/highlight.ts`, all CSS, all pipeline UI components.

---

## 6. Environment File

`playground/.env.local` (gitignored, created manually on dev machine):
```
INKPORT_ROOT=/home/ubuntu/projects/inkide
CARGO_HOME=/home/ubuntu/.cargo
INKPORT_VENV=/home/ubuntu/projects/inkide/.venv
```

`playground/.env.local.example` (committed as documentation).

---

## 7. Out of Scope

- Cleanup of old session temp dirs
- Multi-user / concurrent session isolation beyond UUID temp dirs
- Auth / rate limiting
- `inkport test` or `inkport all` commands in the UI
- Windows support (shell-out assumes Linux paths and bash)
