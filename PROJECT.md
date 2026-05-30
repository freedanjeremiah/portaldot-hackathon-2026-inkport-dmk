# InkPort

**Write Solidity. Ship on Portaldot.**
A Solidity → Rust → WebAssembly compiler, CLI, and web IDE that deploys EVM-style
contracts to the live Portaldot `pallet-contracts` chain, with gas paid in POT.

---

## BUIDL submission

| Field | Answer |
|---|---|
| **Project name** | InkPort |
| **Logo** | `playground/public/inkport.png` (PNG, 1024×1024, < 2 MB — downscale to 480×480 if required) |
| **Category** | Developer tooling / smart-contract infrastructure (Solidity → `pallet-contracts` compiler + CLI + web IDE) |
| **Is this an AI Agent?** | No |
| **GitHub** | https://github.com/freedanjeremiah/portaldot-hackathon-2026-inkport-dmk |
| **Website** | https://inkport.philotheephilix.in |
| **Demo video** | _<add YouTube link>_ |

**Vision.** Millions of developers write Solidity; none can ship to Portaldot without a full
rewrite, because Portaldot runs Substrate `pallet-contracts` (Rust → WebAssembly) with a
different execution model, ABI, and toolchain from the EVM. InkPort removes the rewrite: you
write Solidity, and it translates to Portaldot-native Rust, compiles to WebAssembly, and deploys
to the chain. It is the on-ramp connecting the largest smart-contract developer community to the
Substrate contract ecosystem — proven by **30 Solidity contracts deployed and behaviorally
verified on chain**, a compiler that guarantees **no silent miscompiles**, and **keccak-256
selectors** that keep output interoperable with Ethereum tooling.

---

## 1. The problem

Portaldot contracts are Rust compiled to WebAssembly via `pallet-contracts` — a different
execution target, storage model, ABI, and revert semantics from the EVM. The largest
smart-contract developer population writes Solidity. Shipping on Portaldot today means rewriting
every contract from scratch in Rust/ink! — the biggest onboarding cost, and the reason a live
chain stays out of reach for most contract developers.

## 2. Why raw `seal0` Rust

Live metadata introspection confirmed the node runs a **rent-era `pallet-contracts`** with the
**`seal0` host ABI**: `Compact<Weight>` gas, no `storage_deposit_limit` / `upload_code`,
`contracts_rentProjection` present. Two consequences we verified directly:

- ink! 3/4/5 toolchains do not build on current toolchains, and the node's MVP-WebAssembly
  validator rejects ink! 5.x output.
- A `no_std` contract that imports the node's `seal0` host functions **compiles on stable Rust**
  and the node accepts it as-is.

So InkPort emits raw `seal0` Rust, not ink! — which is what makes "deploy to Portaldot" hold.

## 3. Architecture (three layers)

```
Solidity (.sol)
   ▼  Translator (Rust)     solang-parser → IR → seal0 codegen
src/lib.rs + metadata.json
   ▼  Builder               cargo +stable build --target wasm32 → strip
<Name>.wasm
   ▼  Chain client (Python) instantiate_with_code / Contracts.call / contracts_call (dry-run)
contract live on Portaldot (gas in POT)
```

1. **Translator (`translator/`, ~5.5k LOC Rust)** — `parse.rs` (solang-parser → AST, flattens
   inheritance) → `lower.rs`/`ir.rs` (IR) → `codegen_seal.rs` (emits seal0 Rust + `metadata.json`).
   Binary: `inkport-translate <file.sol> --target seal --out <dir>`.
2. **CLI (`inkport/`, Python)** — translate / build / deploy / call / test / all.
3. **Chain client (`inkport_chain/`, ~1.5k LOC Python)** — `portaldot.py` (deploy/call/read with
   reconnect+retry), `strip_wasm.py` (MVP-wasm stripper), `test_contract.py` (metadata-driven
   assertion harness).

Everything is **metadata-driven** — the CLI and harness encode calls and decode returns from
`metadata.json`, with no per-contract logic.

## 4. ABI & runtime

- Call input = 4-byte selector ++ SCALE(args); constructor = SCALE(ctor args); return = SCALE(value).
- **Selectors** = first 4 bytes of `keccak256("name(canonicalTypes)")` — Ethereum-tooling
  compatible; distinct per overload; a collision is a hard error. **Event topics** = keccak of
  the canonical signature.
- **Storage** — scalars use a 32-byte slot key; mappings/array elements/struct fields use
  `blake2_256(slot ++ key…)`; collision-safe by construction.
- **MVP-WebAssembly only** — codegen avoids `memory.fill`/`memory.copy` (`MaybeUninit` + byte
  loops), sizes input buffers to the payload, imports memory with a declared max, strips to
  `call`/`deploy` exports.

## 5. Supported Solidity surface

| Area | Supported |
|---|---|
| Scalars | `bool`, `uintN`→u128 (width-checked), `intN`→i128, `address`, `string`/`bytes`, `bytes32` |
| Collections | `mapping` (address & scalar keys), nested mappings, dynamic arrays (`push`/`length`/index), `mapping(K=>Struct)` |
| Functions | constructor, view / mutating / `payable`, public-var auto-getters, multiple returns, **overloading** |
| Statements | assignment, compound assign, `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`revert`, `unchecked {}` |
| Expressions | arithmetic (checked → revert at the declared width; `unchecked` → wrap), comparisons, logical, bitwise/shift, narrowing casts (truncate) |
| Context | `msg.sender`, `msg.value`, `block.timestamp`, `block.number`, `address(this).balance` |
| Events / OOP | `emit` → `seal_deposit_event`; inheritance/interface flattening (`is`), modifiers, enums, struct locals; cross-contract `IFoo(addr).bar()` |

## 6. Integrity guarantee

**Every Solidity construct either compiles to semantically-correct seal0 Rust, or makes
`inkport translate` exit non-zero (fail-loud). There are no silent miscompiles.** This was
hardened through a six-round adversarial review that hunted wrong-but-compiling output and
closed each case on chain: function overloading, `constant`/`receive`/multi-contract files,
integer bit-width semantics (`uint8 255+1` reverts; `unchecked` wraps), narrowing-cast
truncation (`uint8(256) == 0`), keccak4 selectors, inheritance flattening, cross-contract calls.
Unsupported constructs (libraries, `delegatecall`, inline assembly, `tx.origin`, struct-in-array,
non-trailing `string` params, ternary) fail loudly — honest limitations, never traps.

## 7. Proof — 30 contracts on chain

All 30 translate → build → deploy → test on `wss://portaldot.philotheephilix.in`, asserted
through submitted extrinsics, receipts, events, and reverts. Each has a `tests/<Name>.json` spec.

`Flipper · Counter · SimpleStorage · Pub · Inc · Sum · Bits · MinMax · Signed · IdStore · Timed ·
NarrowMath · Narrow16 · Unchecked · Cast · ERC20 · ERC721 · Ownable · Bank · Escrow · Auction ·
Voting · Greeter · IntList · Structs · Enum · Inherit · Caller · Target · Overload`

Example deployed ERC20: `5HcQTX3kYCANZVLesSkaEX2Wnk6Fp5xYzMxTtc4PqcBCgvoZ`
(`balanceOf` / `transfer` / `approve` / `transferFrom`, `Transfer`/`Approval` events,
over-balance revert — all asserted on chain). The suite spans ERC-20/721, payable deposits +
POT withdrawals, time/owner-gated logic, storage structs + arrays, cross-contract calls,
inheritance, overloading, and the integer-width edge cases.

## 8. CLI

```bash
inkport translate contracts/ERC20.sol     # Solidity → seal0 Rust + metadata.json
inkport build ERC20                        # cargo build wasm + strip
inkport deploy ERC20 --arg 1000000         # instantiate on Portaldot (POT gas)
inkport call ERC20 balanceOf --arg //Alice # view → dry-run read; mutating → extrinsic
inkport test ERC20                         # run tests/ERC20.json on-chain assertions
inkport all                                # translate+build+deploy+test every contract
```

`inkport all` deploys fresh and asserts on chain — the pipeline's tests are on-chain runs.

## 9. Web playground

A Next.js app (`playground/`, https://inkport.philotheephilix.in): a **template gallery**
(30 contracts + custom), an **editor** with live Solidity → seal0 Rust translation and streamed
compile logs, and **one-click compile + deploy** whose API routes run the translator + `cargo` +
chain client on the backend — a browser deploy is an on-chain instantiation.

## 10. Reproduce

```bash
source "$HOME/.cargo/env"; rustup target add wasm32-unknown-unknown
(cd translator && cargo build --release && cargo test)        # 89 tests
python3.11 -m venv .venv && source .venv/bin/activate && pip install -e inkport
inkport all                                                    # deploy + assert on Portaldot
```

Connection uses `ss58_format=42` + `type_registry_preset='substrate-node-template'` (set by the
client). Token: POT, 14 decimals (1 POT = 10¹⁴ planck).

## 11. Limitations

- `uint256` is represented as `u128`; arithmetic is fail-safe (checked → reverts above 2^128,
  never wraps), but values larger than 2^128 cannot be represented — true 256-bit math is next.
- Advanced Solidity (libraries, `delegatecall`, assembly, struct-across-ABI) is rejected
  fail-loud, not supported.
- The broader Portaldot pallet matrix (Balances/Staking/Assets…) is chain/SDK orchestration,
  outside the translator's scope.

## 12. Why it matters

InkPort is the native, verified on-ramp from the world's largest smart-contract language to
Portaldot — a full *translate → build → deploy → verify* loop with a correctness guarantee,
demonstrated by 30 contracts running on the live chain.

---

**Declaration.** All code was independently developed during this hackathon or legally modified
from official Substrate templates; all delivery requirements are met; the organizing committee
may publicly review and technically reproduce the code.
