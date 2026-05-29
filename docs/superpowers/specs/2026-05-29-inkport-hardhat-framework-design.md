# InkPort — Design Spec

**A Hardhat-style development framework for Solidity → ink! on Portaldot**

| | |
|---|---|
| Codename | InkPort |
| Status | Design v1.0 (approved) |
| Derived from | `prd.md`, `portaldot-reference.md` |
| Date | 2026-05-29 |

---

## 1. Concept

InkPort is a project-based smart-contract framework modeled on Hardhat. A
developer writes Solidity in `contracts/`, runs `inkport compile` to translate
it to ink! 5.x and build WASM + metadata, runs `inkport deploy` to instantiate
it on Portaldot (gas paid in POT), and writes Python scripts/tests to interact
with the deployed contract.

The Hardhat workflow (`init → compile → deploy → test`, a project config file,
deploy scripts in the CLI's own language) is the user-facing shape. The
Solidity → ink! translation engine is the substance underneath.

This spec describes the **ideal framework**, not a deadline-constrained MVP.
The ERC-20 path remains the first end-to-end target and primary test fixture.

---

## 2. Decisions (resolved during brainstorming)

| Decision | Choice | Rationale |
|---|---|---|
| Deploy/call path | **Python SDK** (`substrateinterface` / portaldot) | Only path documented in `portaldot-reference.md`; proven. |
| Compile path | **Rust** (`solang-parser` → IR → ink! codegen, then `cargo contract build`) | Keeps translation aligned with ink!/Rust tooling. |
| CLI / orchestrator | **Python** (`typer`) | Deploy scripts share the CLI language (Hardhat model); easiest glue to the SDK. |
| Config format | **Python** (`inkport.config.py`) | Matches Hardhat's executable config; env interpolation natural. |
| Framework surfaces | init, config, compile, deploy scripts, test | Full Hardhat-equivalent surface. |
| uint strategy | `u128` default, `U256` opt-in via config | Covers ERC-20; resolves PRD open-Q3. |
| Multi-file Solidity | Single-file first; `import` resolution is a stretch goal | Scope control. |
| Network for deploy | Portaldot mainnet `wss://mainnet.portaldot.io`, decimals 14, ss58 42 | From `portaldot-reference.md`. |

The Rust translator is **pure and offline** — it never touches the network, so
it is unit-testable in isolation. All chain interaction is Python.

---

## 3. Project layout (`inkport init` output)

```
myproject/
  inkport.config.py        # networks, accounts, compiler  (= hardhat.config.js)
  contracts/
    Token.sol              # Solidity sources
  artifacts/               # generated (gitignored)
    Token/
      Cargo.toml
      lib.rs               # generated ink! 5.x source
      Token.contract       # wasm + metadata bundle
      Token.wasm
      translation-report.md
  scripts/
    deploy.py              # Python deploy script   (= scripts/deploy.js)
  tests/
    token_test.py          # behavioral / golden tests
  deployments/
    portaldot.json         # { contract: address, codeHash, ... } per network
  .gitignore               # ignores artifacts/, secrets
```

---

## 4. Components

Each component has one purpose, a defined interface, and is independently
testable.

### 4.1 `inkport` CLI (Python, `typer`)
Orchestrates everything. Commands in §6. Owns no domain logic itself — it wires
together the config loader, translator binary, builder, deployer, and test
runner. Resolves the active network and account, then delegates.

### 4.2 Translator (Rust binary `inkport-translate`)
- **Input:** one `.sol` file path + options (uint strategy, ink! version).
- **Pipeline:** `solang-parser` AST → InkPort IR (storage fields, functions,
  events, errors, modifiers) → analyzer (type mapping, mutability inference,
  builtin substitution, modifier desugaring) → ink! source emission.
- **Output:** ink! `lib.rs` + `Cargo.toml` written to `artifacts/<Name>/`, plus
  a `translation-report.json` (machine) and `translation-report.md` (human).
- **Interface to CLI:** invoked as a subprocess; communicates via files + exit
  code + JSON on stdout. No network, no chain knowledge.

### 4.3 Builder (shell-out)
Runs `cargo contract build` in the generated crate directory, producing
`<Name>.contract` and `<Name>.wasm`. Surfaces cargo stderr on failure. Owned by
the CLI's `compile` command.

### 4.4 Deployer (Python library over the portaldot SDK)
- `ContractCode.create_from_contract_files(metadata, wasm, substrate)` →
  `code.deploy(keypair, endowment, gas_limit, constructor, args, upload_code)`.
- `ContractInstance.create_from_address(...)` for attaching.
- `read()` (dry-run, free) and `exec()` (state-changing; gas predicted via
  `read().gas_required`).
- Wraps connection lifecycle (`SubstrateInterface(url=...)`), keypair creation
  (suri / mnemonic / encrypted JSON), POT/planck conversion (`* 10**14`).
- Records results to `deployments/<network>.json`.

### 4.5 Config loader (Python)
Imports `inkport.config.py`, validates the `config` dict, interpolates
`$ENV_VAR` references in account secrets, resolves `--network` to a concrete
endpoint + chain params, and resolves the named account to a keypair.

### 4.6 Translation report (shared artifact)
First-class output of the translator. For every Solidity construct:
- ✅ **Translated** — direct, semantically equivalent.
- ⚠️ **Translated with warning** — mapped, semantics differ subtly (integer
  overflow, gas model, reentrancy, ABI/event-topic encoding).
- ⛔ **Unsupported** — emitted as `// TODO: manual review` stub + report entry.

Rendered as `translation-report.md`; surfaced by `inkport report`.

### 4.7 Test runner (Python)
Runs `tests/*.py` behavioral/golden tests against a deployed contract (local or
remote network). Golden tests call the deployed ink! contract and assert
results + emitted `ContractEmitted` events match expected values. Can target a
local `substrate-contracts-node` for fast, free iteration.

---

## 5. Data flow

```
contracts/Token.sol
   │  inkport compile
   ▼
[Translator: Rust]  solang AST → IR → ink! lib.rs  ──▶ translation-report.md
   ▼
[Builder]  cargo contract build → Token.contract + Token.wasm  (artifacts/)
   │  inkport deploy --network portaldot
   ▼
[Deployer: Python SDK]  ContractCode.create_from_contract_files
                        → code.deploy(keypair, gas, value=POT)
   ▼
deployments/portaldot.json  { address, codeHash }
   │  inkport test  /  inkport run scripts/*.py
   ▼
[Test runner]  ContractInstance.read / exec
               → assert behavior + watch ContractEmitted events
```

---

## 6. CLI surface

```
inkport init [dir]                          scaffold a new project
inkport compile [--contract Name]           sol → ink! → cargo build + report
inkport deploy --network N [--contract Name] [--value POT] [--account A]
inkport run <script.py> --network N         run a Python script with injected ctx
inkport test [--network local]              behavioral / golden tests
inkport report [--contract Name]            print translation report
inkport console --network N                 (stretch) REPL with portaldot + contracts bound
```

`inkport run` injects a context object (connected `portaldot` client, resolved
keypair, deployed-contract handles) into the script's namespace — analogous to
Hardhat Runtime Environment.

---

## 7. Config shape (`inkport.config.py`)

```python
config = {
    "networks": {
        "portaldot": {"url": "wss://mainnet.portaldot.io", "decimals": 14, "ss58": 42},
        "local":     {"url": "ws://127.0.0.1:9944", "decimals": 14, "ss58": 42},
    },
    "accounts": {
        "deployer": {"suri": "$INKPORT_SURI"},        # env-resolved at load
    },
    "compiler": {"ink": "5.x", "uint_strategy": "u128"},   # u128 default; "u256" opt-in
    "default_network": "portaldot",
}
```

---

## 8. Solidity → ink! mapping (engine reference)

Carried from PRD §8. The translator implements one rule per row; each rule has a
unit test.

| Solidity | ink! / Rust |
|---|---|
| `contract Foo { ... }` | `#[ink::contract] mod foo { #[ink(storage)] pub struct Foo { ... } }` |
| state variable | field on the `#[ink(storage)]` struct |
| `uint256` | `u128` (default) or `U256` (opt-in) |
| `address` | `AccountId` |
| `bool` / `string` / `bytes` | `bool` / `String` / `Vec<u8>` |
| `mapping(K => V)` | `ink::storage::Mapping<K, V>` |
| `constructor(...)` | `#[ink(constructor)] pub fn new(...) -> Self` |
| `function f() public view` | `#[ink(message)] pub fn f(&self)` |
| state-mutating function | `#[ink(message)] pub fn f(&mut self)` |
| `payable` function | `#[ink(message, payable)]` |
| `msg.sender` | `self.env().caller()` |
| `msg.value` | `self.env().transferred_value()` |
| `block.timestamp` | `self.env().block_timestamp()` |
| `m[key]` (read) | `self.m.get(key).unwrap_or_default()` |
| `m[key] = v` (write) | `self.m.insert(key, &v)` |
| `require(cond, "msg")` | `if !cond { return Err(Error::...) }` |
| `revert(...)` | `Err(Error::...)` |
| custom errors | `#[derive(...)] pub enum Error { ... }` |
| `event E(...)` | `#[ink(event)] pub struct E { #[ink(topic)] ... }` |
| `emit E(...)` | `self.env().emit_event(E { ... })` |
| `modifier m()` | guard fn returning `Result<(), Error>`, called at entry |
| checked arithmetic (≥0.8) | `checked_add` / `checked_sub` → `Error::Overflow` |

**Warning-class gaps:** integer width/overflow, gas accounting (EVM gas vs WASM
weights), reentrancy patterns, ABI/event-topic encoding differences.

---

## 9. Error handling

- **Translation:** unsupported construct → `// TODO: manual review` stub +
  ⛔ report entry; compile continues. The report is the contract with the user
  about what is safe.
- **Build:** `cargo contract build` non-zero exit → surface stderr verbatim,
  fail loud, no artifacts marked valid.
- **Deploy:** inspect `receipt.is_success` / `ExtrinsicFailed`; on success print
  block hash + contract address; on failure print the decoded error.

---

## 10. Testing strategy

- **Translator (Rust):** unit test per mapping rule in §8; snapshot-test
  generated ink! source for the ERC-20 fixture.
- **Pipeline (integration):** ERC-20 → compile → deploy to a local
  `substrate-contracts-node` → call `transfer` → assert balances update and the
  `Transfer` event fires.
- **Golden:** behavioral-parity harness comparing expected vs deployed results
  for a defined call set.
- **CLI (Python):** unit test config loading, env interpolation, network
  resolution, and `deployments/*.json` writing.

---

## 11. Success criteria

- `inkport init` scaffolds a working project.
- `inkport compile` translates the ERC-20 fixture to a **compiling** ink! crate
  and produces `.contract` + `.wasm` + a translation report.
- `inkport deploy --network portaldot` instantiates the contract on Portaldot,
  paying gas in POT, and records the address.
- `inkport test` passes behavioral parity for the ERC-20 demo.
- The translation report correctly classifies supported vs unsupported
  constructs.

---

## 12. Non-goals

- Full Solidity coverage (inline assembly/Yul, `delegatecall`, complex
  inheritance, `selfdestruct`).
- EVM-on-WASM emulation — this is source-to-source.
- Production security guarantees on translated output.
- A polished GUI (a web UI is a far-future stretch).

---

## 13. Open questions

1. Exact Portaldot testnet endpoint + POT faucet (reference doc shows mainnet
   only). Local `substrate-contracts-node` covers dev/test in the meantime.
2. ink! version supported by Portaldot's `pallet-contracts` — pin once confirmed.
3. Whether `inkport console` (REPL) earns its place or stays cut.
