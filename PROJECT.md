# InkPort — Project Details (for judges)

**Write Solidity. Ship on Portaldot.**
InkPort is a Solidity → Rust → WebAssembly compiler, CLI, and web IDE that deploys
EVM-style contracts to the live Portaldot `pallet-contracts` chain, paying gas in POT.

- **Repository:** https://github.com/freedanjeremiah/portaldot-hackathon-2026-inkport-dmk
- **Website / playground:** https://inkport.philotheephilix.in
- **Live node:** `wss://portaldot.philotheephilix.in`
- **Proof:** 30 Solidity contracts deployed + behaviorally verified on chain · 89 compiler tests · zero silent miscompiles

---

## 1. The problem

Portaldot runs Substrate `pallet-contracts`: contracts are Rust compiled to WebAssembly,
with a different execution target, storage model, ABI, builtins, and revert semantics from
the EVM. The largest smart-contract developer population on earth writes **Solidity**. To ship
on Portaldot today they must **rewrite every contract from scratch** in Rust/ink! — the single
biggest onboarding cost, and the reason a live chain with real liquidity stays out of reach for
most contract developers.

**InkPort removes the rewrite.** Paste Solidity, get a contract live on Portaldot.

---

## 2. Why raw `seal0` Rust (the core design decision)

We confirmed by live metadata introspection that the target node runs a **rent-era
`pallet-contracts`** with the **`seal0` host ABI** (~Substrate 2021): gas is `Compact<Weight>`,
there is no `storage_deposit_limit` / `upload_code`, and `contracts_rentProjection` is present.

Consequences we measured directly:
- ink! 3/4/5 toolchains require an era-matched Rust nightly + dependency set that does **not**
  build on current toolchains; and the node's MVP-WebAssembly validator **rejects ink! 5.x
  output** (newer host functions / post-MVP opcodes).
- A **raw `seal0` Rust** contract — `no_std`, importing the node's `seal0` host functions —
  **compiles on stable Rust** and the node accepts it as-is.

So InkPort's backend emits raw `seal0` Rust, not ink!. This is what makes "deploy to Portaldot"
actually true rather than aspirational.

---

## 3. Architecture (three layers, one pivot)

```
Solidity (.sol)
   │
   ▼  [ Translator — Rust ]   solang-parser → IR → seal0 codegen
src/lib.rs  +  metadata.json
   │
   ▼  [ Builder ]   cargo +stable build --release --target wasm32-unknown-unknown → strip
<Name>.wasm
   │
   ▼  [ Chain client — Python ]   instantiate_with_code / Contracts.call / contracts_call (dry-run)
live contract on Portaldot  (gas paid in POT)
```

1. **Translator (Rust, `translator/`)** — `parse.rs` (solang-parser → AST, flattens
   inheritance, rejects multiple concrete contracts) → `lower.rs` / `ir.rs` (intermediate
   representation) → **`codegen_seal.rs`** (the backend: emits seal0 Rust + `metadata.json`).
   Binary: `inkport-translate <file.sol> --target seal --out <dir>`. ~5.5k LOC of Rust.
2. **CLI (Python, `inkport/`)** — `cli.py` (translate / build / deploy / call / test / all),
   `pipeline.py`, `config.py`.
3. **Chain client (Python, `inkport_chain/`)** — `portaldot.py` (deploy/call/read over
   `substrate-interface`, with reconnect + retry), `strip_wasm.py` (MVP-wasm stripper),
   `test_contract.py` (metadata-driven encode/decode + assertion harness). ~1.5k LOC of Python.

**Everything is metadata-driven.** `metadata.json` is the single source of truth the CLI and
harness use to encode calls and decode returns — there is **no per-contract logic** in the
tooling.

---

## 4. The ABI and runtime contract

- **Call input** = 4-byte selector ++ SCALE(args). **Constructor input** = SCALE(ctor args).
  **Return** = SCALE(value).
- **Selectors** = first 4 bytes of `keccak256("name(canonicalTypes)")` — **ABI-compatible with
  Ethereum tooling**; distinct per overload; a 4-byte collision is a hard translate error.
- **Event topics** = `keccak256` of the canonical event signature.
- **Storage** — each state variable gets a slot. Scalars use a 32-byte key; mappings and array
  elements use `blake2_256(slot ++ key…)`; nested mappings and struct fields extend the
  preimage. Collision-safe by construction.
- **MVP-WebAssembly only** — the validator rejects `memory.fill`/`memory.copy`, so codegen uses
  `MaybeUninit` buffers + explicit byte loops, payload-sized input buffers, imported memory with
  a declared max, and strips to `call`/`deploy` exports.

---

## 5. Supported Solidity surface

| Area | Supported |
|---|---|
| Scalars | `bool`, `uintN`→u128 (width-checked), `intN`→i128 (width-checked), `address`→AccountId, `string`/`bytes` (compact-length), `bytes32` |
| Collections | `mapping(K=>V)` (address & scalar keys), nested mappings, dynamic arrays `T[]` (`push`/`length`/index), `mapping(K=>Struct)` field access |
| Functions | constructor, view / mutating / `payable`, public-variable auto-getters, multiple returns, **function overloading** |
| Statements | assignment, compound assign (`+= -= *= /= %= |= &= ^= <<= >>=`), `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`assert`/`revert`, locals, `unchecked {}` |
| Expressions | arithmetic (checked → revert on overflow at the declared width; `unchecked` → wrap), comparisons, logical, bitwise/shift, narrowing casts `uintN(x)` (truncate), literals |
| Context | `msg.sender`, `msg.value`, `block.timestamp`, `block.number`, `address(this).balance` |
| Events | `emit E(...)` → `seal_deposit_event` (keccak topic + SCALE data) |
| OOP | inheritance / interface flattening (`is`), modifiers (inlined guards), enums, struct locals |
| Cross-contract | `IFoo(addr).bar(args)` via `seal_call` |
| Constants | `constant` inlined at compile time; `immutable` as constructor-written storage |

---

## 6. Integrity guarantee (the project's spine)

**Every Solidity construct either compiles to semantically-correct seal0 Rust, or makes
`inkport translate` exit non-zero (fail-loud). There are no silent miscompiles** —
wrong-but-compiling code never ships. This was hardened through an **adversarial review loop**
(six rounds) that specifically hunted silent miscompiles and closed each on chain:

- function overloading (was dropping overloads) → per-signature dispatch
- `constant` / `receive` / multiple-contract files (were silently dropped) → inlined / handled / hard error
- integer **bit-width** semantics → `uint8 255+1` reverts; `unchecked` wraps to `0`
- explicit narrowing casts → `uint8(256) == 0`
- real keccak4 selectors + inheritance flattening + cross-contract calls

Constructs we do not support (libraries/`using for`, `delegatecall`, inline `assembly`,
`tx.origin`, struct-in-array, non-trailing `string` params, ternary) **fail loudly** — they are
honest limitations, never traps.

---

## 7. Proof — 30 contracts, live on chain

All translate → build → deploy → test **pass on `wss://portaldot.philotheephilix.in`** with real
extrinsics, receipts, events, and reverts (no mocks). Each has a `tests/<Name>.json` spec.

`Flipper · Counter · SimpleStorage · Pub · Inc · Sum · Bits · MinMax · Signed · IdStore · Timed ·
NarrowMath · Narrow16 · Unchecked · Cast · ERC20 · ERC721 · Ownable · Bank · Escrow · Auction ·
Voting · Greeter · IntList · Structs · Enum · Inherit · Caller · Target · Overload`

Representative on-chain deployment (live address):
- **ERC20** → `5HcQTX3kYCANZVLesSkaEX2Wnk6Fp5xYzMxTtc4PqcBCgvoZ`
  (`balanceOf`, `transfer`, `approve`, `transferFrom`, `Transfer`/`Approval` events, over-balance revert — all asserted on chain.)

What the suite exercises: ERC-20/721 token logic, payable deposits + POT withdrawals (Bank,
Escrow), time/owner-gated logic (Auction, Ownable), storage structs + arrays (Voting, IntList,
Structs), cross-contract calls (Caller→Target), inheritance (Inherit), overloading (Overload),
and the full integer-width edge cases (NarrowMath, Unchecked, Cast).

---

## 8. CLI

```bash
inkport translate contracts/ERC20.sol     # Solidity → seal0 Rust + metadata.json
inkport build ERC20                        # cargo build wasm + strip
inkport deploy ERC20 --arg 1000000         # instantiate on Portaldot (POT gas)
inkport call ERC20 balanceOf --arg //Alice # view → dry-run read; mutating → real extrinsic
inkport test ERC20                         # run tests/ERC20.json on-chain assertions
inkport all                                # translate+build+deploy+test every contract
```

`inkport all` is the full regression suite — it deploys fresh and asserts on the live chain.
There is no mock chain; the pipeline's tests **are** on-chain runs.

---

## 9. Web playground

A Next.js app (`playground/`, served at https://inkport.philotheephilix.in):
- **Template gallery** — pick any of the 30 repo contracts, or start a custom one.
- **Editor / IDE** — live Solidity → seal0 Rust translation, with streamed compile logs.
- **One-click compile + deploy** — the API routes run the *real* translator + `cargo` + chain
  client on the backend (this is not a simulation), so a deploy in the browser is a real
  on-chain instantiation.

---

## 10. Reproduce it (judges)

```bash
# Rust translator
source "$HOME/.cargo/env"; rustup target add wasm32-unknown-unknown
(cd translator && cargo build --release && cargo test)      # 89 tests

# Python CLI + chain client
python3.11 -m venv .venv && source .venv/bin/activate && pip install -e inkport

# Full on-chain regression suite (deploys + asserts on Portaldot)
inkport all
```

Node connection requires both `ss58_format=42` and
`type_registry_preset='substrate-node-template'` (the client sets this). Token: POT, 14
decimals (1 POT = 10¹⁴ planck).

---

## 11. Quality & engineering

- **89** Rust translator unit/integration tests; the full 30-contract suite green on chain.
- **Zero silent miscompiles**, established by an adversarial multi-round review.
- **Resilient chain client** — reconnect + retry on dropped sockets, so transient network drops
  never cause false failures.
- **~5.5k LOC Rust** (compiler) + **~1.5k LOC Python** (CLI + chain client) + a Next.js IDE.

---

## 12. Honest limitations

- `uint256` is represented as `u128` (16-byte). Arithmetic is **fail-safe** (checked → reverts
  above 2^128), never silently wrapping; values larger than 2^128 cannot be represented. True
  256-bit math is the next coverage step.
- Advanced Solidity (libraries, `delegatecall`, inline assembly, struct-across-ABI, etc.) is
  rejected fail-loud, not supported.
- The Portaldot reference's broader pallet matrix (Balances/Staking/Assets…) is chain/SDK
  orchestration, not contract-language surface — correctly out of the translator's scope and
  driven by the Python chain client where needed.

---

## 13. Why it matters

InkPort is the **native, verified on-ramp** from the world's largest smart-contract language to
Portaldot — not a code generator, but a full *translate → build → deploy → verify* loop with a
correctness guarantee, proven by 30 contracts running on the live chain.
