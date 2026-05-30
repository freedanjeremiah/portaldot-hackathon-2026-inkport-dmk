# 🖋️ InkPort

**Write Solidity. Ship on Portaldot.**

> Paste an EVM contract, get it live on Portaldot. InkPort translates Solidity into
> Portaldot-native Rust, compiles it to WebAssembly, and deploys it to the chain — gas paid in
> POT. Hardhat for a Substrate chain.

 **Live repo:** https://github.com/freedanjeremiah/portaldot-hackathon-2026-inkport-dmk
 **Playground:** https://inkport.philotheephilix.in
 **Demo:** _<add YouTube link>_

---

## The problem

Portaldot contracts are Rust compiled to WebAssembly through `pallet-contracts` — a different
execution target, storage model, ABI, and revert semantics from the EVM. The largest
smart-contract developer population on earth writes **Solidity**.

To ship on Portaldot today, they must **rewrite every contract from scratch** in Rust/ink!:
learn a new toolchain, a new memory model, new builtins, new encoding. So a live chain with
liquidity stays out of reach for the people who could fill it with apps.

There is no simple way to say **"take my Solidity and put it on Portaldot."**

## The solution

InkPort turns porting into a **single command** — or a single click.

You write Solidity. InkPort parses it, lowers it to an intermediate model, and emits raw
`seal0` Rust that the node accepts as-is; it compiles to WebAssembly on stable Rust, strips to a
deployable module, and instantiates on Portaldot with your key — gas in **POT**. Translate,
build, deploy, and verify the contract on chain, all from one CLI or the browser playground.

No manual rewrite. No ink! toolchain. No EVM emulation.

## Why it wins

- **It actually deploys.** Not a code generator that stops at Rust — a full *translate → build
  → deploy → verify* loop that instantiates on Portaldot and returns the contract address.
  **30 Solidity contracts** are live on chain, asserted through submitted extrinsics, events,
  and reverts.
- **No silent miscompiles.** Every Solidity construct either compiles to semantically-correct
  Rust or fails the build loudly — never wrong-but-compiling output. Hardened by a six-round
  adversarial review that closed each edge case on chain (integer widths, narrowing casts,
  overloading, inheritance, constants, `receive`).
- **ABI-faithful.** keccak-256 4-byte selectors and canonical event topics — output stays
  interoperable with standard Ethereum tooling, distinct per overload, collisions rejected.
- **Runtime-native, no lock-in.** We confirmed the node runs a rent-era `pallet-contracts`
  (`seal0` host ABI) that rejects ink! 5.x; InkPort emits raw `seal0` Rust that compiles on
  *stable* Rust and instantiates directly. That's why "deploy to Portaldot" holds.
- **The playground can't fake it.** Compile and deploy in the browser run the same translator +
  `cargo` + chain client on the backend — a browser deploy is an on-chain instantiation.
- **Width-correct by construction.** `uint8 255 + 1` reverts; `unchecked` wraps; `uint8(256)`
  truncates to `0` — Solidity 0.8 semantics preserved at the declared bit-width, verified on chain.

## What you can build

Port the EVM contracts you already have:

- **Tokens** — ERC-20 (`transfer` / `approve` / `transferFrom`, events) and ERC-721
  (`ownerOf` / approvals / operators).
- **DeFi primitives** — payable Bank (deposits + POT withdrawals), Escrow, Auction with
  time/owner gating.
- **Governance** — Voting with storage structs, Ownable with modifier guards.
- **Composed systems** — cross-contract calls (`IFoo(addr).bar()`), inheritance (`is Base`),
  function overloading, enums, dynamic arrays.
- **Your own** — open the playground, pick a template or paste a contract, and ship it.

## How it's built

**Translator (`translator/`, ~5.5k LOC Rust)** — `solang-parser` → AST (inheritance flattened,
multiple-contract files rejected) → IR (`ir.rs`/`lower.rs`) → `codegen_seal.rs`, which emits a
`no_std` `seal0` contract + `metadata.json`. Binary: `inkport-translate <file.sol> --target seal`.

- **ABI** — call input = keccak4 selector ++ SCALE(args); return = SCALE(value); events via
  `seal_deposit_event` with keccak topics.
- **Storage** — scalar slots use a 32-byte key; mappings, array elements, and struct fields use
  `blake2_256(slot ++ key…)`, collision-safe by construction.
- **MVP-WebAssembly discipline** — the node's validator rejects post-MVP opcodes, so codegen
  avoids `memory.fill`/`memory.copy` (`MaybeUninit` + byte loops), sizes input buffers to the
  payload, imports memory with a declared max, and strips to `call`/`deploy` exports.

**CLI (`inkport/`, Python + typer)** — `translate / build / deploy / call / test / all`, fully
metadata-driven (no per-contract logic).

**Chain client (`inkport_chain/`, ~1.5k LOC Python)** — one `substrate-interface` websocket with
reconnect + retry; connects with `ss58_format=42` and
`type_registry_preset="substrate-node-template"` so `System.Account` decodes;
`instantiate_with_code` / `Contracts.call` / `contracts_call` dry-run; a metadata-driven
assertion harness drives the 30-contract on-chain suite.

**Playground (`playground/`, Next.js 15)** — template gallery (30 contracts + custom), an editor
with live Solidity → seal0 Rust and streamed compile logs, and one-click compile + deploy backed
by the actual toolchain.

**Quality** — 89 translator tests; the full 30-contract suite green on chain; zero silent
miscompiles.

## Portaldot-native, by design

- **POT is gas** on every deploy and call.
- Emits raw **`seal0` Rust** for the node's `pallet-contracts` — the compiler output *is* the
  on-chain integration.
- Token POT · SS58 prefix 42 · 14 decimals · `instantiate_with_code` / `Contracts.call`.

## What's next

- True 256-bit `uint256` arithmetic (currently fail-safe u128).
- Wider Solidity coverage — libraries, structs across the ABI, more builtins.
- A richer browser IDE: in-page deploy history, call console, and shareable contract links.
- A growing on-chain contract gallery beyond the current 30.

## Team

**dmk** — Freedan Jeremiah (compiler, CLI, chain integration, playground).

## License

MIT.
