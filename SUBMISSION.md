# 🌀 PortalDot Hackathon 2026 — Submission

## 2.2 Submission Form

**Basic Info**
- **Project Name:** InkPort
- **Repository URL:** https://github.com/freedanjeremiah/inkide
- **Demo Video URL:** _<add link>_

---

### Demo Scene Description

The video shows the full InkPort lifecycle, end to end against the live Portaldot chain.

1. **Choose a contract.** In the playground, the reviewer opens the template gallery and selects a Solidity contract (e.g. `ERC20.sol`), or writes a custom one.
2. **Translate.** `inkport translate` parses the Solidity and emits raw `seal0` Rust plus a `metadata.json` (keccak-256 4-byte selectors, SCALE ABI). The generated Rust is shown.
3. **Build.** `inkport build` compiles the Rust to WebAssembly on stable Rust and strips it to a Portaldot-ready module.
4. **Deploy.** `inkport deploy` instantiates the contract on Portaldot via `instantiate_with_code`, paying gas in POT, and returns the on-chain contract address.
5. **Call & verify.** `inkport call` and `inkport test` exercise the contract with real `Contracts.call` extrinsics and dry-run reads — balances update, events fire, and invalid operations revert — each assertion confirmed on chain.

Reviewers can reproduce every step from the repository and observe the resulting transactions and contract state on the Portaldot node.

---

### Technical Highlights

**Solidity → Rust → WebAssembly → Portaldot.** InkPort is a source-to-source compiler plus CLI and web IDE that targets Portaldot's `pallet-contracts` directly. It emits a `no_std` contract importing the node's `seal0` host ABI, which compiles on stable Rust and deploys without an ink! toolchain.

- **Substrate-native deploy loop.** Deployment and calls use `instantiate_with_code` / `Contracts.call` over `substrate-interface`, with metadata-driven encoding (SCALE) and `ContractEmitted` event decoding. The metadata carries **keccak-256 4-byte selectors** and canonical event-signature topics, making contracts ABI-compatible with standard Ethereum tooling.
- **Verified breadth.** 30 Solidity contracts — ERC-20, ERC-721, Auction, Escrow, payable Bank, Voting, Ownable, inheritance, cross-contract calls, function overloading — are deployed and behaviorally asserted on chain. The Rust translator carries 89 passing tests.
- **Correctness guarantee.** Every Solidity construct either compiles to semantically-correct Rust or makes translation fail loudly — there are **no silent miscompiles**. Integer arithmetic is width-aware (`uintN`/`intN` overflow reverts at the declared width, `unchecked` wraps, narrowing casts truncate), validated on chain.
- **Runtime-faithful codegen.** The backend respects the validator's MVP-WebAssembly constraints (no `memory.fill`/`memory.copy`, payload-sized buffers, imported memory, `call`/`deploy` exports) so generated modules instantiate reliably.
- **Developer surface.** A single CLI (`translate / build / deploy / call / test / all`) and a Next.js playground with a template chooser, live translation, one-click deploy, and streamed logs.

---

### Declaration

I/We confirm that:
1. All code was independently developed during this hackathon or legally modified from official Substrate templates;
2. All delivery requirements of this specification have been met;
3. I/We agree that the organizing committee may publicly review and technically reproduce the code.
