# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

InkPort translates **Solidity → raw seal0 Rust → wasm** and deploys/tests it on a live
**Portaldot** `pallet-contracts` chain. It is NOT ink!. The target node runs a **rent-era
`pallet-contracts` (seal0 host ABI, ~Substrate 2021)** — confirmed by live metadata
(`Compact<Weight>` gas, no `storage_deposit_limit`/`upload_code`, `contracts_rentProjection`
present). ink! 3/4/5 toolchains don't build here and the node rejects ink!5 wasm, so the
codegen emits a `no_std` contract that imports the node's `seal0` functions and compiles on
**stable Rust**. Background contract: `docs/seal-backend-spec.md`; full guide: `docs/GUIDE.md`.

## Environment (must source before any command)

```bash
source "$HOME/.cargo/env"                 # Rust (stable 1.96 + wasm32-unknown-unknown target)
source /home/ubuntu/projects/inkide/.venv/bin/activate   # Python 3.11 + substrate-interface + inkport CLI
```

## Commands

```bash
# Build the Rust translator (inkport CLI prefers target/release, falls back to target/debug)
(cd translator && cargo build --release)

# Translator tests (89)
(cd translator && cargo test)
(cd translator && cargo test <name>)             # single test by name substring

# End-to-end CLI (drives the live node wss://portaldot.philotheephilix.in)
inkport translate contracts/ERC20.sol            # → build/ERC20/src/lib.rs + metadata.json
inkport build ERC20                              # cargo build wasm + strip
inkport deploy ERC20 --arg 1000000
inkport call ERC20 balanceOf --arg //Alice       # view = dry-run read; mutating = real extrinsic
inkport test ERC20                               # run ONE contract's tests/ERC20.json on-chain
inkport all                                      # translate+build+deploy+test ALL contracts/*.sol
```

`inkport all` is the full regression suite (30 contracts) — it deploys fresh and asserts on
the live chain. There is no mock chain; "tests" for the pipeline ARE on-chain runs.

## Architecture (three layers, one pivot)

1. **Translator (Rust, `translator/`)** — `parse.rs` (solang-parser → AST, flattens
   inheritance, rejects >1 concrete contract) → `lower.rs`/`ir.rs` (IR) →
   **`codegen_seal.rs`** (the real backend: emits seal0 Rust + `metadata.json`). An older
   ink! backend (`codegen.rs`) still exists but is unused — work happens in `codegen_seal.rs`.
   Binary: `inkport-translate <file.sol> --target seal --out <dir>`.
2. **CLI (Python, `inkport/inkport/`)** — `cli.py` (typer: translate/build/deploy/call/test/all),
   `pipeline.py` (paths, binary discovery, arg coercion), `config.py` (loads `inkport.config.py`).
3. **Chain client (Python, `inkport_chain/`)** — `portaldot.py` (deploy/call/read over
   substrate-interface, with reconnect+retry), `strip_wasm.py` (MVP-wasm stripper),
   `test_contract.py` (metadata-driven encode/decode + assertion harness).

**Everything is metadata-driven.** `metadata.json` (name, constructor args, messages with
keccak4 `selector`/`args`/`ret`/`mutates`/`payable`, events) is the single source of truth the
harness uses to encode calls + decode returns. Do NOT add per-contract logic to the CLI/harness.

**Data flow:** `.sol → translate → build/<Name>/{src/lib.rs, metadata.json} → cargo build wasm
→ strip_wasm → instantiate_with_code / Contracts.call / contracts_call(dry-run)`.

## Hard constraints (violating these breaks deploys — they are not optional)

- **MVP-wasm only.** The node rejects `memory.fill`/`memory.copy`. Codegen must use
  `MaybeUninit` buffers + explicit byte loops, never large zero-inited arrays or >32-byte slice
  copies. `-C target-feature=-bulk-memory` alone does NOT suppress these — the source must.
- **Input buffers sized to the SCALE payload.** A large fixed `deploy` buffer (e.g. 512 bytes)
  makes `instantiate_with_code` fail with `System.Other`.
- **Strip to `call`/`deploy` exports + imported memory** (max declared). The node errors
  "unknown export" on `__data_end`/`__heap_base` and "Maximum number of pages should be always
  declared" on memory without a max. See `onchain-contracts/counter/.cargo/config.toml` for the
  required linker flags (`--import-memory`, `--initial-memory`, `--max-memory`).
- **Connect with BOTH `ss58_format=42` and `type_registry_preset='substrate-node-template'`**
  or `System.Account` won't decode. The client does this.
- **On-chain steps are SEQUENTIAL.** A single signer (`//Alice`) means concurrent deploys cause
  nonce conflicts. Never run two `inkport all`/deploy processes against the node at once.

## Integrity invariant (the project's core guarantee)

Every Solidity construct either compiles to **semantically-correct** seal0 Rust or makes
`inkport translate` **exit non-zero (fail-loud)**. There must be **no silent miscompiles**
(wrong-but-compiling code, exit 0). When adding features: if you can't translate something
correctly, make it a hard error — never emit a placeholder/widened/dropped result. The codegen
collects unsupported constructs and returns `Err`; preserve that. Integer arithmetic is
width-aware (`uintN`/`intN` overflow reverts at the declared width; `unchecked` wraps; narrowing
casts truncate) — keep that consistency when touching `codegen_seal.rs`.

## Git

Commits in this repo are authored `Freedan Jeremiah <freedanjeremiah4@gmail.com>` (set via
`GIT_AUTHOR_*`/`GIT_COMMITTER_*` env or local config); do not introduce other identities or a
Claude co-author trailer. The remote (`origin`) embeds a PAT in the URL — never echo it. Plain
`git push origin <branch>` has hit no-op behavior here; push with an explicit refspec
(`git push origin HEAD:refs/heads/<branch>`).

## Adding a contract

Drop `contracts/<Name>.sol` (single concrete contract; `is Base` allowed) + a
`tests/<Name>.json` step spec (actions: deploy / read / call / event / revert / deploy_dep with
`@label` address refs; `value` for payable; `signer` per step). Then `inkport test <Name>` or
`inkport all`. Test-spec and metadata formats are documented in `docs/GUIDE.md` §5–6.
