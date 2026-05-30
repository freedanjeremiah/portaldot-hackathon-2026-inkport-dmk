# InkPort Playground UI — Design Spec

**Date:** 2026-05-30  
**Status:** Approved  
**Stack:** Next.js + Monaco Editor  

---

## 1. Overview

A Babel-style web playground for the InkPort pipeline. Users write Solidity in a split-pane
editor, see the generated seal0 Rust output live, then manually progress through build →
deploy → call stages via a pipeline drawer below the editors.

Reference: https://babeljs.io/repl — left = input, right = transformed output, live on keystroke.

---

## 2. Architecture

**Single Next.js project** — React frontend + Next.js API routes as a thin shell wrapper.
Runs on the same Linux machine as the `inkport` CLI and translator binary. No database, no
auth. Single-user local playground.

### API Routes

| Route | Method | Action |
|---|---|---|
| `/api/translate` | POST | Writes Solidity to temp file, runs `inkport-translate --target seal --out <tmpdir>`, returns `lib.rs` + `metadata.json` |
| `/api/build` | POST | Runs `inkport build <name>`, streams stdout/stderr as SSE |
| `/api/deploy` | POST | Runs `inkport deploy <name> --arg ...`, streams output, returns address |
| `/api/call` | POST | Runs `inkport call <name> <message> --arg ...`, returns decoded result |

### Session Strategy

Each browser session gets a UUID stored in `sessionStorage`. All API routes receive
`{ sessionId, solidity, name, ... }`. Temp files live under `/tmp/inkport-playground/<uuid>/`.
Contract name is parsed client-side from `contract Foo {` in the editor text.

### State Machine (per session)

```
idle → translating → translated → building → built → deploying → deployed → calling
```

Build/deploy/call buttons are only enabled once the prior stage completes successfully.

---

## 3. Layout

```
┌─────────────────────────────────────────────────────────────┐
│  InkPort Playground                           [status badge] │
├──────────────────────────┬──────────────────────────────────┤
│                          │                                   │
│   Monaco Editor          │   Monaco Editor (read-only)      │
│   (Solidity)             │   (Generated Rust / seal0)       │
│                          │                                   │
│   ~50% width             │   ~50% width                     │
│                          │   live-updates on translate       │
│                          │   spinner while translating       │
│                          │                                   │
├──────────────────────────┴──────────────────────────────────┤
│  Pipeline  [Build ▶]  [Deploy ▶]  [Call ▶]                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ (active stage panel expands here)                   │    │
│  │                                                     │    │
│  │  Build:   scrollable log output                     │    │
│  │  Deploy:  arg inputs + deploy button + address out  │    │
│  │  Call:    message picker + arg inputs + result out  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

Minimum width: 1024px (developer tool, mobile not a priority).  
Split pane has a draggable divider between the two editors.

---

## 4. Components

| Component | Responsibility |
|---|---|
| `SolidityEditor` | Monaco with Solidity syntax; fires `onChange` with 600ms debounce |
| `RustOutput` | Read-only Monaco with Rust syntax; shows spinner during translate; red error banner on failure |
| `PipelineBar` | Three stage buttons (Build / Deploy / Call); disabled until prior stage done; active stage highlighted |
| `BuildPanel` | Scrollable log fed by SSE stream; shows exit status on stream close |
| `DeployPanel` | Constructor arg inputs from `metadata.constructor.args`; deploy button; displays resulting address |
| `CallPanel` | Message dropdown from `metadata.messages`; arg inputs per message; result/event display |

---

## 5. Data Flow

### Translate (live)
1. User types → 600ms debounce fires
2. `POST /api/translate` — writes Solidity to temp file, runs translator, reads back `lib.rs` + `metadata.json`
3. Returns `{ rust: string, metadata: object, error?: string }`
4. `RustOutput` updates; `metadata` stored in React state — drives arg inputs in `DeployPanel` and `CallPanel`

### Build (manual)
1. User clicks Build → `POST /api/build` opens SSE stream
2. API route runs `inkport build <name>`, pipes stdout/stderr line-by-line as SSE events
3. `BuildPanel` appends lines; on stream close receives `{ exit: 0 }` or `{ exit: 1 }`
4. On success: Build button turns green, Deploy button enables

### Deploy (manual)
1. User fills constructor args (auto-generated from `metadata.constructor.args`) → clicks Deploy
2. `POST /api/deploy` runs `inkport deploy <name> --arg <a1> --arg <a2> ...`, streams output
3. On success: returns deployed address, stored in React state, Call button enables

### Call (manual)
1. User picks message from dropdown (from `metadata.messages`), fills args → clicks Call
2. `POST /api/call` runs `inkport call <name> <message> --arg ...`
3. Returns decoded result (view) or tx confirmation + events (mutating); displayed in `CallPanel`

---

## 6. Styling & UX

**Theme:** Dark throughout — `vs-dark` Monaco theme. Background `#0f0f0f`, panels `#1a1a1a`, borders `#2a2a2a`.

**Status badge (header, top-right):**

| State | Color |
|---|---|
| idle | grey |
| translating... | yellow |
| building... | yellow |
| deployed ✓ | green |
| error | red |

**PipelineBar stage button states:**
- Disabled → greyed out, not clickable
- Ready → outlined, clickable
- Running → pulsing highlight
- Done → solid green with checkmark ✓
- Failed → solid red with ✗

**Translate feedback:** Subtle spinner in Rust pane header while debounce is pending or request is in flight. Error state replaces code with red error message. Editor is never blocked.

**Arg inputs:** Plain text inputs labeled `<name>: <type>` from metadata (e.g. `initialSupply: uint256`). No client-side type validation — CLI errors surface in the panel output.

**Default content:** Editor pre-loaded with `Counter.sol` so the playground is never blank on first load.

---

## 7. File Structure

```
playground/                  ← new Next.js app at repo root
├── app/
│   ├── page.tsx             ← main playground page
│   ├── layout.tsx
│   └── api/
│       ├── translate/route.ts
│       ├── build/route.ts
│       ├── deploy/route.ts
│       └── call/route.ts
├── components/
│   ├── SolidityEditor.tsx
│   ├── RustOutput.tsx
│   ├── PipelineBar.tsx
│   ├── BuildPanel.tsx
│   ├── DeployPanel.tsx
│   └── CallPanel.tsx
├── lib/
│   ├── session.ts           ← UUID session helpers
│   └── shell.ts             ← exec/stream helpers for API routes
├── package.json
└── tsconfig.json
```

---

## 8. Out of Scope

- Authentication / multi-user
- Saving/loading contracts (no persistence beyond session temp files)
- Mobile layout
- `inkport all` / `inkport test` commands (CLI-only for now)
- Syntax error highlighting inside Monaco (no Solidity language server)
