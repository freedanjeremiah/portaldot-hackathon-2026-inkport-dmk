# InkPort — Complete Guide

Everything about the project: what it does, what works, how to run and test it, how to
add your own contracts, and the exact limits.

---

## 1. What InkPort is

InkPort lets you **write a smart contract in Solidity and deploy it to the Portaldot chain**.
It translates Solidity to Rust, compiles that Rust to WebAssembly, and deploys + tests the
result on a live Portaldot node — a Hardhat-style workflow whose compile target is
Portaldot's `pallet-contracts`.

```
contracts/Token.sol
   │  inkport translate          (Rust: solang-parser → IR → seal0 codegen)
   ▼
build/Token/src/lib.rs  +  metadata.json
   │  inkport build              (cargo +stable build → wasm → strip)
   ▼
build/Token/Token.wasm
   │  inkport deploy / call / test   (Python: instantiate_with_code / Contracts.call / dry-run)
   ▼
live contract on wss://portaldot.philotheephilix.in
```

### Why "seal0 Rust", not ink!

The target node runs a **rent-era `pallet-contracts`** (verified by live metadata:
`Compact<Weight>` gas, no `storage_deposit_limit`, no `upload_code`, `contracts_rentProjection`
present — i.e. the seal0 host ABI, ~Substrate 2021). The ink! 3/4/5 toolchains require an
era-matched Rust nightly + dependency set that does not build on this machine, and ink! 5.x
wasm uses host functions this node rejects. So InkPort emits **raw `seal0` Rust**: a `no_std`
contract that imports the node's `seal0` host functions, compiles on **stable Rust**, and
deploys as-is. Full backend contract: `docs/seal-backend-spec.md`.

---

## 2. What works (verified)

- **30 Solidity contracts** translate → build → deploy → test, asserted on the live node.
- **CLI**: `inkport translate | build | deploy | call | test | all`.
- **`cargo test`: 89** translator unit/integration tests.
- **Integrity guarantee**: every construct either compiles to semantically-correct seal0 Rust
  or makes `inkport translate` **exit non-zero (fail-loud)** — verified across an adversarial
  review loop. No silent miscompiles.

### Supported Solidity surface

| Area | Supported |
|---|---|
| Scalar types | `bool`, `uintN`→u128 (width-checked), `intN`→i128 (width-checked), `address`→AccountId, `bytes`/`string` (compact-length, trailing param), `bytes32` (as dynamic bytes) |
| Collections | `mapping(K=>V)` (address & scalar keys), nested mappings, dynamic arrays `T[]` (`.push`/`.length`/index), `mapping(K=>Struct)` field access |
| Functions | constructor, view / mutating / `payable`, public-var auto-getters, **multiple returns**, **function overloading** |
| Statements | assignment, compound assign (`+= -= *= /= %= |= &= ^= <<= >>=`), `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`assert`/`revert`, local vars, `unchecked { }` |
| Expressions | arithmetic (checked → revert on overflow at the declared width; `unchecked` → wrap), comparisons, logical, bitwise/shift, narrowing casts `uintN(x)` (truncate), literals |
| Integers | true bit-width semantics: `uint8 255+1` reverts; `unchecked` wraps to 0; `uint8(256)==0` |
| Context | `msg.sender`, `msg.value`, `block.timestamp`, `block.number`, `address(this).balance` |
| Events | `emit E(...)` → `seal_deposit_event` (keccak topic + SCALE data), decoded in the harness |
| OOP | inheritance / interface flattening (`is`), modifiers (inlined as guards), enums, struct locals |
| Cross-contract | `IFoo(addr).bar(args)` via `seal_call` with keccak4 selector |
| ABI | keccak256 4-byte selectors + keccak event-signature topics |

### Deliberately rejected (fail-loud — `translate` exits non-zero, never miscompiles)

Libraries / `using for`, `delegatecall`, inline `assembly`, `tx.origin`, `block.coinbase`,
struct-in-array, nested structs, `string`/`bytes` as struct fields or non-trailing params,
ternary `?:`, struct return across the ABI, `new` factory, `abi.encodePacked` concat.
True `uint256` values above 2^128 cannot be *represented* (arithmetic is fail-safe: reverts,
never wraps silently).

### The 30 validated contracts

`Flipper` `Counter` `SimpleStorage` `Pub` `Inc` `Sum` `Bits` `MinMax` `Signed` `IdStore`
`Timed` `NarrowMath` `Narrow16` `Unchecked` `Cast` `ERC20` `ERC721` `Ownable` `Bank`
`Escrow` `Auction` `Voting` `Greeter` `IntList` `Structs` `Enum` `Inherit` `Caller` `Target`
`Overload`. Each has a `tests/<Name>.json` spec.

---

## 3. Install

Prereqs: `rustup` (stable + the `wasm32-unknown-unknown` target), Python 3.11.

```bash
# Rust translator
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
(cd translator && cargo build --release)          # builds the inkport-translate binary

# Python CLI + chain client (uses uv or venv + pip)
python3.11 -m venv .venv
source .venv/bin/activate
pip install -e inkport                              # provides the `inkport` command
```

The chain client needs `substrate-interface` (installed by the editable install). The node,
accounts, and token decimals are preconfigured in `inkport.config.py`.

---

## 4. Use the CLI

```bash
source .venv/bin/activate && source "$HOME/.cargo/env"

inkport translate contracts/ERC20.sol       # → build/ERC20/src/lib.rs + metadata.json
inkport build ERC20                          # → build/ERC20/ERC20.wasm (stripped)
inkport deploy ERC20 --arg 1000000           # instantiate on Portaldot (signer //Alice)
inkport call ERC20 balanceOf --arg //Alice   # view → dry-run read, prints decoded value
inkport call ERC20 transfer --arg //Bob --arg 250   # mutating → real extrinsic + event
inkport test ERC20                           # run tests/ERC20.json on-chain, PASS/FAIL per step
inkport all                                  # translate+build+deploy+test EVERY contract
```

Flags: `--network <name>` (from `inkport.config.py`; default `portaldot`), `--signer //Bob`,
`--value <POT>` (payable calls), `--out <dir>` (translate).

Dev accounts `//Alice`, `//Bob`, `//Charlie` are accepted anywhere an `address` arg is
expected; they resolve to the 32-byte AccountId. `//Alice` is the funded faucet/sudo account.

### Single-command example (full lifecycle)

```bash
inkport translate contracts/Bank.sol
inkport build Bank
inkport deploy Bank
inkport call Bank deposit --value 5            # deposit 5 POT
inkport call Bank balanceOf --arg //Alice      # → 500000000000000 (5 POT in plancks)
inkport call Bank withdraw --arg 200000000000000
```

---

## 5. How to test

### Run the whole suite

```bash
source .venv/bin/activate && source "$HOME/.cargo/env"
inkport all
# → per-contract steps, then:
# ========== inkport all: SUMMARY ==========
#   ... 30 contracts ... PASS
#   ALL PASS
```

`inkport all` deploys a **fresh** instance of each contract and runs its `tests/<Name>.json`
against the live node — real `instantiate_with_code` / `Contracts.call` extrinsics with
`wait_for_inclusion`, real receipts, real revert/event checks. No mocks. The chain client
auto-reconnects + retries on dropped sockets, so transient drops don't cause false failures.

### Translator unit tests

```bash
source "$HOME/.cargo/env" && (cd translator && cargo test)     # 89 tests
```

### Test-spec format (`tests/<Name>.json`)

```jsonc
{
  "deployer": "//Alice",          // signer for the deploy
  "steps": [
    { "action": "deploy", "args": [1000000] },                  // constructor args
    { "action": "read",   "message": "balanceOf", "args": ["//Alice"], "expected": 1000000 },
    { "action": "call",   "message": "transfer",  "args": ["//Bob", 1000], "signer": "//Alice" },
    { "action": "event",  "name": "Transfer", "expected": { "from": "//Alice", "to": "//Bob", "value": 1000 } },
    { "action": "revert", "message": "transfer", "args": ["//Bob", 1e9], "signer": "//Charlie" }
  ]
}
```

Action types:
- `deploy` — instantiate; `args` = constructor args.
- `read` — dry-run a view message; assert decoded return == `expected`.
- `call` — submit a mutating extrinsic (`signer` optional; `value` = POT for payable). Fails
  if the contract reverts unexpectedly (dry-run revert-bit checked).
- `revert` — assert the message reverts (negative test).
- `event` — assert an event with `name` and `expected` fields was emitted by the prior call.
- `deploy_dep` — deploy a helper contract (cross-contract tests); `name`, `args`, `as` label.
  Reference its address later with `"@<label>"` in another step's args (see `tests/Caller.json`).

Arg encoding is driven entirely by the contract's `metadata.json` — no per-contract logic in
the harness. `address` args accept `//Name` SURIs or `0x..` 32-byte hex; ints are decimal;
bools are `true`/`false`; strings are JSON strings.

---

## 6. Metadata format (`build/<Name>/metadata.json`)

Emitted by `inkport translate`; the CLI/harness use it to encode calls + decode returns.

```json
{
  "name": "Counter",
  "constructor": { "args": ["u128"] },
  "messages": [
    { "name": "inc",   "selector": "0x371303c0", "args": [],      "ret": null,  "mutates": true,  "payable": false },
    { "name": "incBy", "selector": "0x70119d06", "args": ["u128"], "ret": null,  "mutates": true,  "payable": false },
    { "name": "get",   "selector": "0x6d4ce63c", "args": [],      "ret": "u128", "mutates": false, "payable": false }
  ],
  "events": []
}
```

- `selector` = first 4 bytes of `keccak256("name(canonicalTypes)")` — ABI-compatible; distinct
  per overload.
- Call input = selector ++ SCALE(args). Constructor input = SCALE(ctor args), no selector.
- Return = SCALE(ret). Scalars: u128 = 16-byte LE, bool = 1 byte, address = 32 bytes,
  string/bytes = compact length ++ bytes.

---

## 7. Add your own contract

```bash
# 1. write contracts/MyToken.sol  (a single concrete contract; `is Base` allowed)
# 2. write tests/MyToken.json  (steps, as in §5)
# 3. run it
inkport translate contracts/MyToken.sol
inkport build MyToken
inkport test MyToken          # deploys fresh + asserts on-chain
# or just:  inkport all       # picks up any contracts/*.sol with a tests/*.json
```

If you use an unsupported construct, `inkport translate` exits non-zero with the exact reason
(e.g. `unsupported: inline assembly`, `string param must be last`) — it will not emit a
wrong-but-compiling contract.

---

## 8. Repository layout

| Path | Role |
|---|---|
| `translator/` | Rust crate. `src/parse.rs` (solang-parser), `src/lower.rs`/`ir.rs` (IR), `src/codegen_seal.rs` (seal0 codegen), binary `inkport-translate`. |
| `inkport/` | Python package: the `inkport` CLI (`inkport/cli.py`), config loader, pipeline glue. |
| `inkport_chain/` | `portaldot.py` (deploy/call/read client w/ retry), `strip_wasm.py` (MVP-wasm stripper), `test_contract.py` (metadata-driven harness). |
| `contracts/` | 30 Solidity fixtures. |
| `tests/` | One `<Name>.json` on-chain test spec per contract. |
| `onchain-contracts/counter/` | The original hand-written raw seal0 reference contract. |
| `inkport.config.py` | Networks (Portaldot wss, decimals 14, ss58 42), default signer. |
| `docs/seal-backend-spec.md` | The codegen ABI/storage/runtime contract. |
| `docs/GUIDE.md` | This file. |
| `build/`, `deployments/` | Generated (gitignored). |

---

## 9. The Portaldot node

| Field | Value |
|---|---|
| Public WSS | `wss://portaldot.philotheephilix.in` |
| Local WS | `ws://127.0.0.1:9944` |
| Token | `POT`, **14 decimals** (1 POT = 10¹⁴ plancks) |
| SS58 prefix | 42 |
| Pallet | rent-era `pallet-contracts` (seal0 ABI) |
| Connect | `SubstrateInterface(url, ss58_format=42, type_registry_preset='substrate-node-template')` — both params required or `System.Account` won't decode |
| Faucet/Sudo | `//Alice` (`5GrwvaEF…HGKutQY`), prefunded |
| Existential deposit | 1 POT; contract instantiation endows 10 POT by default |

> Dev chain — no real value, state resets on node restart, `//Alice` is reproducible by
> anyone. Never reuse these keys anywhere with real value.

---

## 10. Troubleshooting

- **`cargo contract`/ink! errors** — not used; InkPort emits raw seal0 Rust built with plain
  `cargo build --target wasm32-unknown-unknown`.
- **`System.Other` on deploy** — the node rejects non-MVP wasm and oversized deploy buffers.
  The codegen avoids `memory.fill`/`memory.copy` and sizes input buffers to the payload; the
  stripper keeps only `call`/`deploy` exports + imported memory. Custom contracts hand-written
  outside the codegen must follow `onchain-contracts/counter` exactly.
- **`Decoder class for "AccountInfo…" not found`** — you connected without
  `type_registry_preset='substrate-node-template'`. The client sets it for you.
- **Dropped websocket mid-run** — the client reconnects + retries; if a run still flakes,
  re-run `inkport test <Name>`.
- **A view prints `reverted`** — the contract reverted (e.g. overflow at a narrow width); that
  is correct fail-safe behavior, not a client bug.

---

## 11. Status

Reviewer-verified (adversarial loop, 6 rounds): **30/30 contracts pass on the live node,
89 translator tests green, zero silent miscompiles.** Branch `feat/inkport-framework` is
merged to `main` on `github.com/freedanjeremiah/inkide`.
