# InkPort Playground UI — Design Spec

**Date:** 2026-05-30  
**Status:** Approved  
**Stack:** Next.js + Monaco Editor  

---

## 1. Overview

A Babel-style web playground for the full InkPort pipeline. Users write Solidity in a
split-pane editor, see the generated seal0 Rust output live on every keystroke, then
manually progress through compile → deploy → call stages via a pipeline drawer below the
editors.

Reference: https://babeljs.io/repl — left = input, right = transformed output, live on
keystroke. InkPort extends this to a full on-chain lifecycle.

### What InkPort is (background for implementors)

InkPort translates Solidity → raw seal0 Rust → WebAssembly and deploys/tests on a live
Portaldot `pallet-contracts` chain. It is NOT ink!. The target node runs a rent-era
`pallet-contracts` (seal0 ABI, ~Substrate 2021 — verified by live metadata:
`Compact<Weight>` gas, no `storage_deposit_limit`, no `upload_code`,
`contracts_rentProjection` present). ink! 3/4/5 wasm uses host functions this node rejects;
InkPort emits raw `no_std` Rust that imports only the node's seal0 host functions and
compiles on stable Rust.

**Pipeline:**

```
contracts/Token.sol
   │  translate          (Rust: solang-parser → IR → seal0 codegen)
   ▼
build/Token/src/lib.rs  +  metadata.json
   │  build              (cargo +stable build → wasm → MVP-wasm strip)
   ▼
build/Token/Token.wasm  ← Portaldot-compatible file
   │  deploy / call / test   (Python: instantiate_with_code / Contracts.call / dry-run)
   ▼
live contract on wss://portaldot.philotheephilix.in
```

---

## 2. Architecture

**Single Next.js project** — React frontend + Next.js API routes as a thin shell wrapper
around the existing `inkport` CLI and translator binary. Runs on the same Linux machine as
the CLI. No database, no auth. Single-user local playground.

### Core compile route

The central backend route is `POST /api/compile`. It takes raw Solidity text from the
frontend, runs the full translate → build pipeline server-side (using the existing
`inkport-translate` binary + `cargo build`), and returns the stripped
**Portaldot-compatible `.wasm` file** plus `metadata.json` back to the frontend. The
frontend holds the wasm in memory; subsequent deploy and call routes receive it directly.

This is the only route that does heavy work. Translate-only (for live Rust preview) is a
fast separate route that only runs the Rust codegen, not `cargo build`.

### All API Routes

| Route | Method | Body | Response |
|---|---|---|---|
| `POST /api/translate` | POST | `{ solidity, sessionId }` | `{ rust: string, metadata: object, error?: string }` — runs only the codegen step; no cargo build; used for live Rust preview |
| `POST /api/compile` | POST | `{ solidity, sessionId }` | SSE stream: log lines, then on success `{ type:"wasm", data: base64, metadata: object }` — runs translate + cargo build + strip; returns the Portaldot-compatible wasm |
| `POST /api/deploy` | POST | `{ wasmB64, metadata, args, sessionId }` | SSE stream: log lines, then `{ type:"address", address: string }` |
| `POST /api/call` | POST | `{ address, metadata, message, args, sessionId }` | `{ result: any, events: object[] }` |

### Session strategy

Each browser session gets a UUID stored in `sessionStorage`. Temp working directories live
under `/tmp/inkport-playground/<uuid>/`. Contract name is parsed client-side from
`contract Foo {` in the editor text and sent with every request.

### State machine (per session)

```
idle → translating → translated → compiling → compiled → deploying → deployed → calling
```

- `translating` — live Rust preview in flight (debounced)
- `compiled` — wasm + metadata held in React state; Deploy button enabled
- `deployed` — address held in React state; Call button enabled

Compile/deploy/call buttons are only enabled once the prior stage completes successfully.
A translate error shows inline in the Rust pane but does NOT block the Compile button
(the user may still attempt a full compile; the error will surface there too).

### How the compile route works internally

```
POST /api/compile  { solidity, sessionId }
  │
  ├─ write solidity to /tmp/inkport-playground/<uuid>/<Name>.sol
  ├─ run: inkport-translate <file> --target seal --out <uuid-dir>/build/<Name>/
  │        → generates <uuid-dir>/build/<Name>/src/lib.rs + metadata.json
  ├─ run: cargo +stable build --release --target wasm32-unknown-unknown
  │        cwd = <uuid-dir>/build/<Name>/
  │        → produces target/wasm32-unknown-unknown/release/<crate>.wasm
  ├─ run: strip_wasm.strip(raw_wasm, out_path)
  │        → Portaldot-compatible stripped wasm (MVP-wasm only:
  │           call/deploy exports + imported memory; no memory.fill/memory.copy)
  └─ SSE: stream log lines, then emit { type:"wasm", data: base64(wasm), metadata }
```

The stripped wasm is the same artifact that `inkport build` produces — it is exactly what
`instantiate_with_code` expects on the Portaldot node.

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
│  Pipeline  [Compile ▶]  [Deploy ▶]  [Call ▶]                │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ (active stage panel expands here)                   │    │
│  │                                                     │    │
│  │  Compile: scrollable build log (SSE stream)         │    │
│  │  Deploy:  arg inputs + deploy button + address out  │    │
│  │  Call:    message picker + arg inputs + result out  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

Minimum width: 1024px (developer tool, mobile not in scope).  
Split pane has a draggable divider between the two editors.

---

## 4. Components

| Component | Responsibility |
|---|---|
| `SolidityEditor` | Monaco with Solidity syntax; fires `onChange` with 600ms debounce triggering `/api/translate` for live Rust preview |
| `RustOutput` | Read-only Monaco with Rust syntax; shows spinner during translate; red error banner on translate failure |
| `PipelineBar` | Three stage buttons (Compile / Deploy / Call); Compile always enabled when editor has content; Deploy enabled after successful compile; Call enabled after deploy |
| `CompilePanel` | Scrollable SSE log from `/api/compile`; shows wasm size on success; exit status on failure |
| `DeployPanel` | Constructor arg inputs auto-generated from `metadata.constructor.args` (label = `name: type`); deploy button; displays resulting address |
| `CallPanel` | Message dropdown from `metadata.messages`; arg inputs per message (with `mutates` / `payable` badges); result and event display |

---

## 5. Data Flow

### Translate — live Rust preview (automatic, debounced)

1. User types in `SolidityEditor` → 600ms debounce fires
2. `POST /api/translate` — writes Solidity to temp file, runs `inkport-translate --target seal`, reads back `src/lib.rs` + `metadata.json`
3. Returns `{ rust: string, metadata: object, error?: string }`
4. `RustOutput` updates; `metadata` stored in React state (drives arg input scaffolding in later panels)
5. Translate errors show as a red banner in the Rust pane; editor is never blocked

### Compile — full build, returns wasm (manual)

1. User clicks Compile → `POST /api/compile` opens SSE stream
2. API route runs translate + `cargo +stable build --target wasm32-unknown-unknown` + `strip_wasm`, streams log lines
3. `CompilePanel` appends log lines live
4. On success: SSE emits `{ type:"wasm", data: base64, metadata: object }`; frontend stores wasm + metadata in React state; Deploy button enables
5. On failure: SSE emits `{ type:"error", log: string }`; shown in CompilePanel

### Deploy (manual)

1. User fills constructor args (auto-generated from `metadata.constructor.args`) → clicks Deploy
2. `POST /api/deploy` — body includes `{ wasmB64, metadata, args, sessionId }`; API route writes wasm to temp file, runs `inkport deploy` (or direct `portaldot.deploy`), streams output
3. On success: SSE emits `{ type:"address", address: string }`; address stored in React state; Call button enables
4. Address shown in `DeployPanel` as a copyable chip

### Call (manual)

1. User picks a message from dropdown (populated from `metadata.messages`); arg inputs render per message schema
2. `POST /api/call` — `{ address, metadata, message, args, sessionId }`; API route runs `inkport call <name> <message> --arg ...`
3. View messages: returns decoded result (`{ result: any }`)
4. Mutating messages: returns `{ result: "ok", events: [...] }` — events decoded from `ContractEmitted` receipts against the metadata event schema
5. Revert: API returns non-200; `CallPanel` shows error in red

---

## 6. Metadata format reference (from `inkport translate` output)

The `metadata.json` emitted by the translate step is the source of truth the frontend uses
to scaffold arg inputs and decode results. It must never be hardcoded per contract.

```json
{
  "name": "Counter",
  "constructor": { "args": ["u128"] },
  "messages": [
    { "name": "inc",   "selector": "0x371303c0", "args": [],       "ret": null,   "mutates": true,  "payable": false },
    { "name": "incBy", "selector": "0x70119d06", "args": ["u128"], "ret": null,   "mutates": true,  "payable": false },
    { "name": "get",   "selector": "0x6d4ce63c", "args": [],       "ret": "u128", "mutates": false, "payable": false }
  ],
  "events": [
    { "name": "Transfer", "fields": [
      { "name": "from",  "type": "address" },
      { "name": "to",    "type": "address" },
      { "name": "value", "type": "u128" }
    ]}
  ]
}
```

**Selector:** first 4 bytes of `keccak256("name(canonicalTypes)")` — ABI-compatible, distinct per overload.  
**Encoding:** call input = selector ++ SCALE(args); ctor input = SCALE(args), no selector.  
**Return:** SCALE(ret). Scalars: u128 = 16-byte LE, bool = 1 byte, address = 32 bytes, string/bytes = compact-length ++ bytes.

The `DeployPanel` reads `metadata.constructor.args` to render inputs; `CallPanel` reads
`metadata.messages[i].args` and `metadata.messages[i].ret` to render inputs and format output.

---

## 7. Supported Solidity surface (what the translator accepts)

| Area | Supported |
|---|---|
| Scalar types | `bool`, `uintN`→u128 (width-checked), `intN`→i128 (width-checked), `address`→AccountId, `bytes`/`string` (compact-length, trailing param), `bytes32` |
| Collections | `mapping(K=>V)`, nested mappings, dynamic arrays `T[]` (`.push`/`.length`/index), `mapping(K=>Struct)` |
| Functions | constructor, view / mutating / `payable`, public-var auto-getters, multiple returns, overloading |
| Statements | assignment, compound assign, `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`assert`/`revert`, `unchecked {}` |
| Expressions | arithmetic (checked → revert on overflow), comparisons, logical, bitwise/shift, narrowing casts `uintN(x)` (truncate), literals |
| Integers | true bit-width semantics: `uint8(255)+1` reverts; `unchecked` wraps; `uint8(256)==0` |
| Context | `msg.sender`, `msg.value`, `block.timestamp`, `block.number`, `address(this).balance` |
| Events | `emit E(...)` → `seal_deposit_event` (keccak topic + SCALE data) |
| OOP | inheritance / interface flattening (`is`), modifiers (inlined as guards), enums, struct locals |
| Cross-contract | `IFoo(addr).bar(args)` via `seal_call` with keccak4 selector |

**Rejected with a clear error (never silently miscompiled):** libraries, `delegatecall`,
inline `assembly`, `tx.origin`, struct-in-array, nested structs, `string`/`bytes` as
non-trailing params, ternary `?:`, `new` factory, `abi.encodePacked`.

When the user's Solidity uses a rejected construct, `inkport-translate` exits non-zero and
the Compile step surfaces the error in the `CompilePanel` log.

---

## 8. Portaldot node reference

| Field | Value |
|---|---|
| WSS endpoint | `wss://portaldot.philotheephilix.in` |
| Token | POT, **14 decimals** (1 POT = 10¹⁴ plancks) |
| SS58 prefix | 42 |
| Pallet | rent-era `pallet-contracts` (seal0 ABI, ~Substrate 2021) |
| Connect params | `ss58_format=42` + `type_registry_preset='substrate-node-template'` — both required |
| Default signer | `//Alice` (prefunded faucet account) |
| Endowment | 10 POT default on instantiate |

Dev accounts `//Alice`, `//Bob`, `//Charlie` are accepted as `address` args in `CallPanel`
(resolved to 32-byte AccountId by the harness).

---

## 9. Styling & UX

**Theme:** Dark throughout — `vs-dark` Monaco theme. Background `#0f0f0f`, panels `#1a1a1a`, borders `#2a2a2a`.

**Status badge (header, top-right):**

| State | Label | Color |
|---|---|---|
| idle | `idle` | grey |
| translating | `translating...` | yellow |
| compiling | `compiling...` | yellow (pulsing) |
| compiled | `compiled ✓` | blue |
| deploying | `deploying...` | yellow |
| deployed | `deployed ✓` | green |
| error | `error` | red |

**PipelineBar stage button states:**
- Disabled → greyed out, not clickable
- Ready → outlined, clickable
- Running → pulsing highlight
- Done → solid green with ✓
- Failed → solid red with ✗

**Translate feedback:** Subtle spinner in Rust pane header while debounce is pending or request in flight. Error state replaces code output with red error message. Editor is never blocked.

**Compile feedback:** `CompilePanel` streams log lines live via SSE. On success shows wasm size (e.g. `Counter.wasm — 4,218 bytes stripped`). On failure shows stderr.

**Arg inputs:** Plain `<input type="text">` labeled `<name>: <type>` from metadata (e.g. `initialSupply: uint256`). No client-side type validation — errors surface in panel output. `payable` messages show an extra `value (POT)` input. `mutates: true` messages show a distinct "Send" button vs `mutates: false` showing "Read".

**Default content:** Editor pre-loaded with `Counter.sol` so the playground is never blank on first load.

---

## 10. File structure

```
playground/                      ← new Next.js app at repo root
├── app/
│   ├── page.tsx                 ← main playground page
│   ├── layout.tsx
│   └── api/
│       ├── translate/route.ts   ← fast codegen-only preview
│       ├── compile/route.ts     ← translate + cargo build + strip → returns wasm
│       ├── deploy/route.ts      ← instantiate_with_code → returns address
│       └── call/route.ts        ← Contracts.call / dry-run → returns decoded result
├── components/
│   ├── SolidityEditor.tsx
│   ├── RustOutput.tsx
│   ├── PipelineBar.tsx
│   ├── CompilePanel.tsx
│   ├── DeployPanel.tsx
│   └── CallPanel.tsx
├── lib/
│   ├── session.ts               ← UUID session helpers, temp dir management
│   └── shell.ts                 ← exec / SSE stream helpers for API routes
├── package.json
└── tsconfig.json
```

---

## 11. Out of scope

- Authentication / multi-user
- Saving/loading contracts (no persistence beyond session temp files)
- Mobile layout
- `inkport all` / `inkport test` commands (CLI-only)
- Solidity language server / inline diagnostics inside Monaco
- wasm download button (wasm is held in React state for deploy; not exposed as a file download)
