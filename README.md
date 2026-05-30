# InkPort

**Write Solidity, deploy to Portaldot.** InkPort translates Solidity contracts to
Rust, compiles them to WebAssembly, and deploys + tests them on the live Portaldot
chain — like Hardhat, but the compile target is Portaldot's `pallet-contracts`.

```
contracts/Token.sol  ──inkport translate──▶  Rust (seal0)  ──build──▶  wasm
        └──────────── inkport deploy / call / test ──────────▶  live Portaldot node
```

## Why raw seal0 (not ink!)

The target Portaldot node runs a **rent-era `pallet-contracts` (seal0 ABI)** — verified by
live metadata introspection (`Compact<Weight>` gas, no `storage_deposit_limit`, no
`upload_code`, `contracts_rentProjection` present). ink! 3/4/5 toolchains require an
era-matched Rust nightly + dependency set that does not build on this machine, and ink! 5.x
wasm uses host functions this node rejects. InkPort therefore emits **raw `seal0` Rust**: a
`no_std` contract that imports the node's `seal0` host functions, compiles on **stable Rust**,
and deploys as-is. See `docs/seal-backend-spec.md`.

## Pipeline

`Solidity (.sol)` → `solang-parser` → IR → **seal0 Rust** → `cargo +stable build --target
wasm32-unknown-unknown` → strip to MVP wasm (`call`/`deploy` exports + imported memory) →
`instantiate_with_code` / `Contracts.call` on `wss://portaldot.philotheephilix.in`.

## Install

```bash
# Rust translator
source "$HOME/.cargo/env" && (cd translator && cargo build)        # inkport-translate binary
# Python CLI + chain client
python3.11 -m venv .venv && source .venv/bin/activate && pip install -e inkport
```

## Use

```bash
inkport translate contracts/ERC20.sol     # Solidity -> seal0 Rust + metadata.json
inkport build ERC20                        # cargo build wasm + strip
inkport deploy ERC20 --arg 1000000         # instantiate on Portaldot (signer //Alice)
inkport call ERC20 balanceOf --arg //Alice # dry-run read -> decoded result
inkport call ERC20 transfer --arg //Bob --arg 250   # mutating extrinsic + event
inkport test ERC20                         # run tests/ERC20.json on-chain assertions
inkport all                                # translate+build+deploy+test every contract
```

Every `deploy`, `test`, and `all` run saves the deployed contract address to
`deployments/<network>.json` (e.g. `deployments/portaldot.json`). `inkport call`
reads from that file. The 30 live addresses from the latest `inkport all` run are
committed at `deployments/portaldot.json`.

```
```

Network/signers live in `inkport.config.py` (Portaldot wss, decimals 14, ss58 42, default
`//Alice`). Dev accounts `//Alice`/`//Bob`/`//Charlie` are accepted wherever an `address`
arg is expected (resolved to the 32-byte AccountId).

## Supported Solidity surface

- **Types:** `bool`, `uintN`→`u128`, `intN`→`i128`, `address`→`AccountId`,
  `mapping(K=>V)` and nested mappings (address- or scalar-keyed, blake2-256 keys).
- **Functions:** constructor, view / mutating / `payable` messages, public-variable
  auto-getters, multiple return values.
- **Statements:** assignment, compound assignment (`+= -= *= /= %= |= &= ^= <<= >>=`),
  `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`assert`/`revert`,
  local variables.
- **Expressions:** arithmetic (`+ - * / %`, checked → revert on overflow), comparisons,
  logical (`&& || !`), bitwise (`& | ^ ~ << >>`), literals, mapping/index access.
- **Context:** `msg.sender`, `msg.value`, `block.timestamp`, `block.number`,
  `address(this).balance`.
- **Events:** `emit E(...)` → `seal_deposit_event` (indexed topics + SCALE data), decoded
  from `ContractEmitted` in the harness.
- **Modifiers:** inlined as entry guards (e.g. `onlyOwner`).

Every supported construct is enforced **fail-loud**: an unsupported construct makes
`inkport translate` exit non-zero rather than silently mis-compile.

## Validated contracts (deployed + asserted on the live node)

30 contracts, each with a `tests/<Name>.json` spec exercised by `inkport test` against
`wss://portaldot.philotheephilix.in` — real extrinsics, real receipts, real revert/event
checks, no mocks:

`Flipper` `Counter` `SimpleStorage` `ERC20` `ERC721` `Ownable` `Bank`(payable) `Escrow`
`Auction` `Voting` `Greeter`(string) `IntList`(array) `Structs` `Enum` `Inherit`(inheritance)
`Caller`+`Target`(cross-contract) `Overload`(overloading) `Signed`(i128) `NarrowMath`/`Narrow16`
(narrow-width overflow→revert) `Unchecked`(wrap) `Cast`(narrowing truncation) `Bits` `Inc`
`Sum` `MinMax` `Pub` `IdStore` `Timed`.

## Integrity guarantee

Every construct either **compiles to semantically-correct seal0 Rust** or makes
`inkport translate` **exit non-zero (fail-loud)** — never a silent miscompile. This was
hardened across an adversarial review loop that probed overloading, inheritance, `constant`,
`receive`/`fallback`, keccak4 selectors, true checked arithmetic, narrow integer widths
(`uintN`/`intN` overflow→revert, `unchecked`→wrap), and narrowing casts (`uint8(256)==0`),
each verified on-chain. Unsupported surface (libraries, `delegatecall`, inline assembly,
`tx.origin`, struct-in-array, non-trailing `string` params, …) is rejected with a clear
error, not mis-translated.

## Scope note — Portaldot reference features

`portaldot-reference.md` is ~95% chain/SDK pallet operations (Balances, Assets, Staking,
Treasury, …). Those are off-chain orchestration driven by the Python SDK, **not** things a
contract emits; InkPort's `inkport_chain/portaldot.py` already drives the relevant Contracts
pallet path (instantiate / call / dry-run read / event decode). The translator's job is the
contract-language subset above.

## Layout

| Path | Role |
|---|---|
| `translator/` | Rust: solang-parser → IR → seal0 codegen (`--target seal`) |
| `inkport/` | Python `inkport` CLI (translate/build/deploy/call/test/all) |
| `inkport_chain/` | Portaldot client, wasm stripper, metadata-driven test harness |
| `contracts/` | Solidity fixtures | `tests/` | on-chain test specs |
| `deployments/` | Live contract addresses per network (`portaldot.json`) |
| `docs/seal-backend-spec.md` | the codegen ABI/storage/runtime contract |

## License

MIT.
