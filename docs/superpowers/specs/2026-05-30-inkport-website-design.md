# InkPort Website — Design Spec

| | |
|---|---|
| Status | Approved |
| Date | 2026-05-30 |
| Audience | Solidity developers + Portaldot ecosystem builders |

---

## 1. Goal

Build a complete end-to-end website for InkPort on top of the existing Next.js playground
app. The playground moves to `/playground`; all other pages are new. The site targets two
audiences simultaneously: Solidity developers who want to deploy to Portaldot, and
Portaldot ecosystem builders who want a high-level language for pallet-contracts.

---

## 2. Site map

```
/                   Landing page
/playground         Full-screen interactive tool (existing, route-moved)
/docs               Documentation hub with sidebar nav
  /docs/getting-started/install
  /docs/getting-started/first-contract
  /docs/getting-started/project-layout
  /docs/cli/translate
  /docs/cli/build
  /docs/cli/deploy
  /docs/cli/call
  /docs/cli/test
  /docs/cli/all
  /docs/solidity/supported
  /docs/solidity/rejected
  /docs/guides/erc20
  /docs/guides/payable
  /docs/guides/cross-contract
  /docs/guides/integers
  /docs/reference/metadata
  /docs/reference/test-spec
  /docs/reference/portaldot-node
  /docs/troubleshooting
/contracts          Validated contract showcase (30 contracts)
/why-inkport        seal0 vs ink!, integrity guarantee, design decisions
```

---

## 3. Architecture

**Base:** extend the existing `playground/` Next.js app (App Router).

**Layout split:**
- `/playground` — full-screen, no shared nav (existing topbar, unchanged)
- All other pages — shared site layout: top nav + footer

**Route change:** current `app/page.tsx` → `app/playground/page.tsx`. New
`app/page.tsx` is the landing page.

**Docs rendering:** MDX files in `app/docs/content/` rendered via a shared
`app/docs/layout.tsx` with a sidebar nav component. No external CMS.

**Styling:** extends existing `globals.css` (dark theme, JetBrains Mono +
IBM Plex Sans, CSS variables). New site-layout styles added without touching
playground styles.

---

## 4. Navigation

**Site nav** (appears on all pages except `/playground`):
```
[⚡ InkPort]   Docs   Contracts   Why InkPort   [Try Playground →]
```

**Footer:**
```
MIT License  ·  GitHub  ·  Portaldot: wss://portaldot.philotheephilix.in
```

---

## 5. Page designs

### 5.1 Landing page (`/`)

Seven sections in order:

**Hero**
- Headline: `Write Solidity. Deploy to Portaldot.`
- Subtext: one sentence explaining the pipeline (Solidity → seal0 Rust → wasm → Portaldot)
- Two CTAs: `[Try the Playground →]` and `[View on GitHub]`
- Monospace subtext badge: `sol → seal0 rust → wasm → portaldot`

**Pipeline visual**
- Horizontal 4-step strip: `.sol` → Translate → Compile → Deploy → Call
- Each step card shows: step name, internal description (e.g. "solang-parser → seal0 Rust"),
  and the real CLI command underneath

**Stats bar**
- `30 contracts validated · 89 translator tests · 0 silent miscompiles · live on Portaldot`
- All numbers are real, sourced from the codebase

**Two-column value prop**
- Left column: "For Solidity developers" — deploy without learning ink! or Rust
- Right column: "For Portaldot builders" — only toolchain targeting seal0 from a high-level language

**Quick install**
- 4-step code block: `cargo build`, `pip install`, `inkport translate`, `inkport deploy`
- Link to full install guide at `/docs/getting-started/install`

**Playground teaser**
- Static screenshot of the split editor (Solidity left, seal0 Rust right)
- Short caption: "Try it in the browser — no install needed"
- CTA: `[Open Playground →]`

**Supported Solidity (abbreviated)**
- Two-column table: supported constructs (left) vs rejected/fail-loud (right)
- Link to full coverage table at `/docs/solidity/supported`

---

### 5.2 Playground (`/playground`)

Existing `app/page.tsx` moved to `app/playground/page.tsx`. No content or
layout changes. The playground has its own topbar and is self-contained.

---

### 5.3 Docs (`/docs`)

**Layout:** left sidebar (collapsible sections) + right content area.

**Sidebar nav tree:**
```
Getting Started
  Install
  First contract
  Project layout

CLI Reference
  translate
  build
  deploy
  call
  test
  all

Solidity Coverage
  Supported surface
  Rejected constructs

Guides
  ERC20 walkthrough
  Payable contracts
  Cross-contract calls
  Integer width semantics

Reference
  metadata.json format
  Test spec format
  Portaldot node

Troubleshooting
```

**Per-page content:**

- **Install** — numbered steps: rustup + wasm32 target, `cargo build --release`,
  `python3.11 -m venv`, `pip install -e inkport`. Exact commands from GUIDE.md §3.

- **First contract** — end-to-end Counter example: write → translate → build →
  deploy → call → read. Shows exact CLI output at each step.

- **Project layout** — visual file tree with role annotations for every directory
  and file (from GUIDE.md §8).

- **CLI reference pages (one per command)** — synopsis, description, all flags,
  exit codes (0 = success, 1 = unsupported construct), example invocation. Content
  sourced from GUIDE.md §4.

- **Supported surface** — full table: scalar types, collections, functions,
  statements, expressions, integer semantics, context vars, events, OOP, cross-contract.
  Sourced from README.md and GUIDE.md §2.

- **Rejected constructs** — table of unsupported constructs with the exact error
  message each one produces (inline assembly, delegatecall, tx.origin, ternary ?:,
  new factory, libraries, abi.encodePacked).

- **ERC20 walkthrough** — full `contracts/ERC20.sol` shown, metadata output explained,
  deploy with `--arg 1000000`, call `transfer`, read `Transfer` event via `tests/ERC20.json`.

- **Payable contracts guide** — Bank contract: `msg.value` → seal0 mapping, `--value 5` POT,
  deposit/withdraw/balanceOf pattern.

- **Cross-contract calls guide** — Caller + Target: `IFoo(addr).bar(args)` → `seal_call`,
  `@label` address refs in test specs, `deploy_dep` action.

- **Integer width semantics guide** — `uintN`→u128 with width checking: `uint8 255+1` reverts,
  `unchecked {}` wraps to 0, `uint8(256)==0` narrowing cast. Each with Solidity snippet +
  on-chain behavior.

- **metadata.json format** — full schema with all fields annotated: name, constructor.args,
  messages[].selector, messages[].mutates, messages[].payable, events[].fields. Sourced from
  GUIDE.md §6.

- **Test spec format** — all action types: deploy, read, call, event, revert, deploy_dep.
  Real examples from `tests/ERC20.json` and `tests/Caller.json`. Sourced from GUIDE.md §5.

- **Portaldot node** — connection table: WSS endpoint, POT (14 decimals), SS58 prefix 42,
  faucet //Alice, `type_registry_preset='substrate-node-template'` requirement. From GUIDE.md §9.

- **Troubleshooting** — 5 known issues from GUIDE.md §10: System.Other on deploy, Decoder class
  not found, dropped websocket, memory.fill/memory.copy rejection, view prints `reverted`.

---

### 5.4 Contracts (`/contracts`)

**Header:** "30 Solidity contracts — each translated, built, deployed, and asserted on the
live Portaldot node. Real extrinsics, real receipts, no mocks."

**Filter bar:** All · Tokens · Payable · Cross-contract · Math · OOP · Basics

**Contract grid (card per contract):**
Each card shows: name, tag(s), one-line description of what pattern it demonstrates,
constructor signature, key message names, test step count, deployed checkmark, links to
`.sol` source and `tests/<Name>.json` on GitHub.

**The 30 contracts:**

| Contract | Tag | Demonstrates |
|---|---|---|
| Counter | basics | Stateful increment, constructor arg, view getter |
| Flipper | basics | Boolean toggle |
| SimpleStorage | basics | Single uint store/get |
| Pub | basics | Public variable auto-getter |
| Inc | basics | Increment-only counter |
| Sum | math | Accumulator with running total |
| MinMax | math | Comparison and conditional assignment |
| Bits | math | Bitwise operators |
| Signed | math | i128 signed arithmetic |
| NarrowMath | math | uint16 overflow reverts at declared width |
| Narrow16 | math | Width-checked arithmetic on narrow integers |
| Unchecked | math | unchecked {} wraps instead of reverts |
| Cast | math | Narrowing cast: uint8(256)==0 |
| ERC20 | tokens | Fungible token: transfer, approve, allowance, events |
| ERC721 | tokens | Non-fungible token: mint, transfer, ownership |
| Ownable | access | onlyOwner modifier, ownership transfer |
| Bank | payable | msg.value, deposit/withdraw, balance tracking |
| Escrow | payable | Conditional release of held funds |
| Auction | payable | Timed bidding, highest-bidder tracking |
| Voting | access | Proposal creation, vote recording, result query |
| Greeter | strings | string storage and retrieval |
| IntList | arrays | Dynamic uint[] array, .push, .length, index |
| Structs | OOP | Struct locals, field access |
| Enum | OOP | Enum state machine |
| Inherit | OOP | Inheritance flattening (is Base) |
| Caller | cross-contract | Calls into Target via IFoo(addr).bar(args) |
| Target | cross-contract | Deployed as dependency, referenced via @label |
| Overload | OOP | Function overloading, keccak4 selector per signature |
| IdStore | strings | bytes32 storage keyed by address |
| Timed | access | block.timestamp gating |

---

### 5.5 Why InkPort (`/why-inkport`)

Six sections in order:

**The problem** — Portaldot runs a rent-era pallet-contracts (seal0 ABI, ~Substrate 2021).
ink! 3/4/5 toolchains don't build or the node rejects their wasm. Raw seal0 Rust by hand
is the only option today — and that's what InkPort automates.

**What "seal0 Rust" means** — three-column explainer: what the node expects (MVP wasm,
call/deploy exports, seal0 host functions), what InkPort emits (#![no_std] on stable cargo,
seal_input/seal_return/seal_get_storage/seal_deposit_event), what you write (Solidity).

**The integrity guarantee** — "Every construct either compiles to semantically-correct seal0
Rust or inkport translate exits non-zero. No silent miscompiles." Explains what was hardened
in the adversarial review loop: integer widths, overloading, inheritance, cross-contract
calls, events.

**Why Solidity, not a new language** — Solidity is known by tens of thousands of developers.
The translation is source-to-source (AST → seal0 Rust), not EVM emulation. The output is
readable, auditable Rust.

**What InkPort is not** — honest limits: not a security auditor, not full Solidity coverage,
not an EVM emulator, not an ink! replacement (if your node supports ink! 5.x, use ink!).

**CTA** — links to `/docs/getting-started/install` and `/playground`.

---

## 6. Content sources

All content is derived from existing project files — no new facts are invented:

| Content | Source |
|---|---|
| CLI commands, flags, exit codes | `docs/GUIDE.md` §3–4 |
| Supported/rejected Solidity surface | `README.md`, `GUIDE.md` §2 |
| Metadata format | `GUIDE.md` §6 |
| Test spec format | `GUIDE.md` §5 |
| Portaldot node details | `GUIDE.md` §9 |
| Troubleshooting | `GUIDE.md` §10 |
| Stats (30 contracts, 89 tests) | `README.md` |
| Contract descriptions | `contracts/*.sol`, `tests/*.json` |
| seal0 / integrity story | `CLAUDE.md`, `docs/seal-backend-spec.md` |

---

## 7. Implementation notes

- MDX is the preferred format for docs pages (code blocks, tables, prose in one file).
- The contracts page card data can be a static `lib/contracts.ts` array — no DB needed.
- The playground screenshot used on the landing page can be a static PNG committed to
  `public/` (taken manually, or captured programmatically at build time with a headless browser).
- No new fonts or external dependencies beyond what the playground already uses.
- `/docs` root should redirect to `/docs/getting-started/install`.

---

## 8. Out of scope

- Search within docs (can be added later with Pagefind or similar).
- Dark/light mode toggle (site inherits the playground's dark theme).
- Blog or changelog page.
- i18n.
