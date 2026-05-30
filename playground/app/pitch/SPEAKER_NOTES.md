# InkPort — Speaker Notes

---

## 30-Second Elevator Pitch

"There are millions of Solidity developers who can't deploy to Portaldot today — not because
Portaldot isn't ready, but because the tooling gap is too wide. InkPort closes it. You write
Solidity, run one CLI, and your contract is live on Portaldot mainnet paying real POT gas.
Not a transpiler demo — 30 contracts deployed and behaviorally verified on the live chain,
right now."

---

## Per-Slide Notes

---

### Slide 1 — Title: "Write Solidity. Ship on Portaldot."

**Script (≈20 seconds):**
"InkPort is the Solidity-to-Portaldot compiler and deploy toolchain. You write a `.sol` file,
run `inkport all`, and your contract is live on the Portaldot mainnet paying real POT gas.
Not a proof of concept — 30 contracts are live on the chain right now. No testnet, no mock
environment, no manual rewrite."

**Key takeaway:** InkPort is production-ready and already deployed on mainnet.

**Transition:** "To understand why this matters, let's start with the problem it solves."

---

### Slide 2 — Problem: "Millions of Solidity devs are locked out of Portaldot."

**Script (≈30 seconds):**
"Portaldot runs Substrate's `pallet-contracts` — Rust compiled to WebAssembly. The world's
largest smart-contract developer community writes Solidity for the EVM. Those two worlds
don't share a runtime, a storage model, a calling convention, or a revert semantics.
Today, if an EVM team wants to ship on Portaldot, they rewrite every contract from scratch
in Rust. That's not a minor friction — it's a full re-implementation. The result is a live
chain with real liquidity that the biggest developer pool on earth simply can't reach."

**Key takeaway:** The onboarding cost is a full rewrite. That's why Solidity devs don't show up.

**Transition:** "InkPort eliminates that rewrite entirely."

---

### Slide 3 — Solution: "Paste a .sol file. Get a contract live on Portaldot mainnet."

**Script (≈25 seconds):**
"InkPort translates Solidity to raw seal0 Rust, compiles it to WebAssembly with a stock
stable-Rust toolchain, and deploys it to Portaldot in one CLI step. The workflow is
Hardhat-familiar — translate, build, deploy, call, test — but the compile target is
Portaldot, not the EVM. You keep your Solidity. The mechanical 100% is automated."

**Key takeaway:** One command replaces a full rewrite. The Solidity doesn't change.

**Transition:** "Why does this opportunity exist right now?"

---

### Slide 4 — Why Now: "The window just opened."

**Script (≈25 seconds):**
"Three things converged. First, Portaldot's `pallet-contracts` runtime is live in production
and accepting real contract deployments today — real POT gas on every transaction. Second,
Solidity is the dominant contract language by an order of magnitude; every new chain fights
to import that developer base. Third, a focused source-to-source translator with on-chain
verification is now buildable by a small team — we proved it. The chain is ready. The
developer pool exists. The tooling was missing."

**Key takeaway:** The chain is live, the developer pool is massive, and the gap is now closeable.

**Transition:** "Let's size the market that unlocks."

---

### Slide 5 — Market: "Every Solidity dev × every Substrate chain."

**Script (≈20 seconds):**
"Solidity is the number-one smart-contract language — the developer pool every chain wants to
import. Portaldot gains an instant EVM developer on-ramp. And the same seal0 backend
generalizes: any chain running `pallet-contracts` can plug into InkPort. This isn't one
chain — it's a category. InkPort is the on-ramp from the largest contract-dev community to
the entire Substrate contract world."

**Key takeaway:** The addressable market is every `pallet-contracts` chain, not just Portaldot.

**Transition:** "Here's exactly what the product does."

---

### Slide 6 — Product: "One CLI. Full lifecycle."

**Script (≈30 seconds):**
"Five commands cover the full contract lifecycle. `inkport translate` turns your `.sol` into
seal0 Rust and emits a `metadata.json` with keccak4 selectors — ABI-compatible with standard
Ethereum tooling. `inkport build` compiles to WASM and strips it to the MVP subset the node
requires. `inkport deploy` instantiates it on Portaldot mainnet. `inkport call` handles both
dry-run reads and live mutating extrinsics. `inkport test` runs on-chain behavioral assertions.
And `inkport all` runs the full suite in one shot. Everything is metadata-driven — no
per-contract logic in the harness."

**Key takeaway:** Hardhat-familiar workflow, Portaldot as the target chain.

**Transition:** "Now let's talk about what's actually been proven on-chain."

---

### Slide 7 — Traction: "30 contracts. Live. Verified. Zero mock."

**Script (≈30 seconds):**
"Thirty Solidity contracts — ERC20, ERC721, Auction, Escrow, Bank with payable, Voting,
cross-contract calls, inheritance, overloading, narrow integer widths, unchecked arithmetic —
all deployed and behaviorally tested on Portaldot mainnet. 89 translator unit tests green.
Zero silent miscompiles after an adversarial review loop. Every 'test' is a real extrinsic on
`wss://portaldot.philotheephilix.in` with real receipts and real revert checks.
The live ERC20 address is on screen — you can query it right now."

**Key takeaway:** The proof is on-chain, not in a demo environment.

**Transition:** "Let me show you why it actually deploys when others don't."

---

### Slide 8 — Under the Hood: "Why it actually deploys."

**Script (≈25 seconds):**
"Three technical decisions make this work. First, InkPort targets the real runtime: the
Portaldot node runs a rent-era `pallet-contracts` with a seal0 host ABI. InkPort emits raw
seal0 Rust — no ink! toolchain, no nightly dependency, no host function mismatch. It
compiles on stable Rust and the node accepts it as-is. Second, ABI fidelity: keccak-256
4-byte selectors plus SCALE encoding means Ethereum tooling speaks to InkPort contracts out
of the box. Third, the integrity guarantee: every construct either compiles to semantically
correct Rust or the translator exits non-zero. There are no silent miscompiles — width-aware
integers, checked overflow, narrowing-cast truncation, all verified on-chain."

**Key takeaway:** Raw seal0, stable Rust, fail-loud integrity — that's why it works on the real node.

**Transition:** "How does this compare to what else exists?"

---

### Slide 9 — Competition: "The only native deploy loop."

**Script (≈20 seconds):**
"Manual rewrite in ink! works — if you want to rewrite everything. Sol2Ink was a legacy
transpiler; it produced ink! code but had no on-chain verification and doesn't target this
node. InkPort is the only path that takes Solidity in, deploys to Portaldot mainnet, and
guarantees correctness through a fail-loud integrity check backed by 30 on-chain test suites.
It's not just a transpiler — it's a translate, build, deploy, verify loop with a live proof."

**Key takeaway:** InkPort is the only end-to-end, on-chain-verified deploy loop for this stack.

**Transition:** "Who built this?"

---

### Slide 10 — Team: "Builders who shipped it to mainnet."

**Script (≈20 seconds):**
"We built the full stack: a Rust source-to-source compiler using solang-parser through IR to
seal0 codegen, a Python CLI and chain client with substrate-interface for deploy, call, and
dry-run, and a 30-contract on-chain regression suite. We solved the hard parts — the MVP-wasm
constraints the node enforces, the keccak/SCALE ABI, the seal0 host function binding — not on
paper, but by getting every contract green on mainnet. The proof discipline: adversarial review
loop until zero silent miscompiles."

**Key takeaway:** This team debugged against the live chain, not a simulator. The receipts exist.

**Transition:** "Here's what we want to build next, and what we need to get there."

---

### Slide 11 — Ask: "Make InkPort the EVM on-ramp for the Substrate world."

**Script (≈30 seconds):**
"Three priorities. Coverage: extend the supported Solidity surface — true 256-bit integers,
libraries, structs across the ABI — each gated by an on-chain test before it ships, keeping
the integrity guarantee intact. Playground: a browser IDE where you paste Solidity, see the
generated seal0 Rust, and one-click deploy to Portaldot with zero local setup — the lowest
possible friction for a new Solidity developer. Ecosystem: partner with Portaldot and other
`pallet-contracts` chains to make InkPort the default EVM developer on-ramp. The foundation
is proven. This is about scale."

**Key takeaway:** The core is done and working. The ask is to scale it to the full developer market.

**Transition:** "Let me leave you with one clear summary."

---

### Slide 12 — Close: "Write Solidity. Ship on Portaldot."

**Script (≈15 seconds):**
"Thirty contracts live on mainnet. Real POT gas on every deploy and call. Zero silent
miscompiles. One CLI that any Solidity developer can run today.
InkPort is the bridge the Substrate contract world has been waiting for.
Questions?"

**Key takeaway:** It works today. The address is public. The proof is on-chain.

---

## Anticipated Q&A

**Q: Isn't this just Sol2Ink?**
Sol2Ink was a code-generation demo targeting legacy ink! with no on-chain verification loop.
InkPort emits raw seal0 Rust that actually deploys to the Portaldot node — the same node
that rejects ink! 5.x wasm — and every contract is proven by on-chain extrinsics with real
receipts, not just "it compiles."

**Q: Why not ink!?**
The Portaldot node runs a rent-era `pallet-contracts` runtime (seal0 host ABI, ~Substrate
2021). The ink! 3/4/5 toolchains require a matched Rust nightly and dependency tree that
doesn't build here, and ink! 5.x wasm uses host functions this specific node rejects — we
verified this against live node metadata. Raw seal0 Rust on stable Rust is the only path
that works on this chain.

**Q: Is it really mainnet? How do I verify?**
The live ERC20 contract is at address `5HcQTX3kYCANZVLesSkaEX2Wnk6Fp5xYzMxTtc4PqcBCgvoZ`
on `wss://portaldot.philotheephilix.in`. Run `inkport call ERC20 totalSupply` and the node
returns a value backed by a real extrinsic receipt. Every test in the suite pays real POT gas.

**Q: What Solidity isn't supported?**
Libraries and `using for`, `delegatecall`, inline assembly, `tx.origin`, struct-in-array,
nested structs, non-trailing `string`/`bytes` parameters, ternary `?:`, `new` factory
pattern, and `abi.encodePacked` concatenation are all rejected with a clear error — the
translator exits non-zero rather than emitting incorrect code. True `uint256` values above
2^128 are unsupported (arithmetic is fail-safe: it reverts rather than wrapping silently).

**Q: How do you make money?**
The immediate priority is ecosystem adoption — making InkPort the default Solidity on-ramp
for Portaldot and the broader `pallet-contracts` ecosystem. Revenue paths include a hosted
playground (SaaS), chain partnership integrations, and enterprise support for teams migrating
EVM codebases to Substrate.

**Q: What about 256-bit integers?**
Today `uintN` and `intN` map to Rust `u128`/`i128` with true bit-width semantics — overflow
reverts at the declared width, `unchecked` wraps, narrowing casts truncate, all verified
on-chain. True `uint256` above 2^128 is on the roadmap; it's in the Ask slide as a concrete
next coverage milestone, gated by an on-chain test before it ships.

---

## Delivery Tips

- **Pace the traction numbers.** On slide 7, slow down when you say "30 contracts, 89 tests,
  zero silent miscompiles." Let each number land separately. These are the credibility anchor
  for everything else.

- **Demo live on the CLI during slide 6 or 7 if you can.** Run `inkport call ERC20 totalSupply`
  against the live node in a terminal — one decoded value returned in real time is worth more
  than any slide. Have the command pre-typed so the demo takes under 10 seconds.

- **Pause before "zero silent miscompiles."** It's the phrase that separates InkPort from a
  demo-ware transpiler. Let the audience sit with it for a beat.

- **Keep the Under the Hood slide (8) fast if the audience isn't technical.** The one line
  that always lands with any audience: "It compiles on stable Rust and the chain accepts it
  as-is." The deep explanation of seal0 vs ink! is for follow-up questions, not the main pitch.
