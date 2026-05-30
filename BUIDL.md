# InkPort — BUIDL Submission

### BUIDL (project) name
**InkPort**

### BUIDL logo
`playground/public/inkport.png` (PNG, 1024×1024, < 2 MB — downscale to 480×480 if required).

### Vision — the problem this solves
Millions of developers write Solidity. None of them can ship to Portaldot without a full rewrite — Portaldot runs Substrate `pallet-contracts` (Rust → WebAssembly), a different execution model, ABI, and toolchain from the EVM.

**InkPort removes the rewrite.** You write Solidity; InkPort translates it to Portaldot-native Rust, compiles it to WebAssembly, and deploys it to the live chain — paying gas in POT. It is the on-ramp that connects the largest smart-contract developer community to the Substrate contract ecosystem.

What makes it real: **30 Solidity contracts** — ERC-20, ERC-721, Auction, Escrow, payable Bank, Voting, inheritance, cross-contract calls — are deployed and behaviorally verified on chain. The compiler guarantees **no silent miscompiles**: every construct either compiles correctly or fails loudly. ABI output uses keccak-256 selectors, so contracts stay interoperable with Ethereum tooling.

### Category
Developer tooling / Smart-contract infrastructure (Solidity → Substrate `pallet-contracts` compiler + CLI + web IDE).

### Is this BUIDL an AI Agent?
**No.**

### Links
- **GitHub:** https://github.com/freedanjeremiah/portaldot-hackathon-2026-inkport-dmk
- **Project website:** https://inkport.philotheephilix.in
- **Demo video:** _<add YouTube link>_
