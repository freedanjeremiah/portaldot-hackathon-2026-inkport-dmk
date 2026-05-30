import Link from 'next/link';

export const metadata = {
  title: 'Why InkPort — seal0 vs ink!, design decisions',
  description: 'Why InkPort targets raw seal0 Rust instead of ink!, the integrity guarantee, and why Solidity.',
};

export default function WhyInkPortPage() {
  return (
    <div className="site-container" style={{ paddingTop: 48, paddingBottom: 80 }}>

      {/* Header */}
      <div style={{ maxWidth: 680, marginBottom: 56 }}>
        <p className="site-hero-eyebrow" style={{ justifyContent: 'flex-start', marginBottom: 16 }}>Design decisions</p>
        <h1 className="site-h1" style={{ textAlign: 'left', fontSize: 'var(--h2)', marginBottom: 16 }}>Why InkPort</h1>
        <p style={{ fontSize: '1rem', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          The reasoning behind a Solidity toolchain that targets raw seal0 — not ink!.
        </p>
      </div>

      {/* Section 1: The problem */}
      <div className="why-section">
        <h2 className="section-title">The problem</h2>
        <div style={{ maxWidth: 680 }}>
          <p style={{ fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
            Portaldot runs a rent-era <code>pallet-contracts</code> node — a version of Substrate&apos;s smart contract
            pallet from ~2021 that speaks the seal0 host ABI. ink! 3, 4, and 5 each require an era-matched Rust
            nightly and dependency set that doesn&apos;t build against this node. ink! 5.x wasm uses host functions this
            node rejects outright.
          </p>
          <p style={{ fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
            If you want to deploy a smart contract to Portaldot today, you need to write raw seal0 Rust by hand —
            a <code>no_std</code> crate that imports the node&apos;s host functions directly.
            That&apos;s what InkPort generates for you.
          </p>
        </div>
      </div>

      {/* Section 2: What seal0 means */}
      <div className="why-section">
        <h2 className="section-title">What &quot;seal0 Rust&quot; means</h2>
        <div className="three-col">
          <div className="three-col-card">
            <h4>The node expects</h4>
            <p>MVP WebAssembly (no memory.fill, no memory.copy), call / deploy exports + imported memory, seal0 host functions: seal_input, seal_return, seal_get_storage, seal_set_storage, seal_deposit_event.</p>
          </div>
          <div className="three-col-card">
            <h4>InkPort emits</h4>
            <p><code>#![no_std]</code> Rust compiled on stable cargo. No nightly, no ink! dependency. Calls seal0 host functions directly. Buffers sized to SCALE payload. Stripped to MVP wasm with imported memory + max declared.</p>
          </div>
          <div className="three-col-card">
            <h4>You write</h4>
            <p>Plain Solidity (.sol file). A single concrete contract. The 30 validated contracts range from a simple counter to ERC20, ERC721, payable escrow, cross-contract calls, and overloading.</p>
          </div>
        </div>
      </div>

      {/* Section 3: Integrity guarantee */}
      <div className="why-section">
        <h2 className="section-title">The integrity guarantee</h2>
        <div className="integrity-callout" style={{ marginBottom: 28 }}>
          <h3>Every construct either compiles to semantically-correct seal0 Rust,<br />or <code style={{ color: 'inherit', background: 'transparent', border: 'none', padding: 0, fontSize: 'inherit', fontFamily: 'var(--mono)' }}>inkport translate</code> exits non-zero.</h3>
          <p>There are no silent miscompiles.</p>
        </div>
        <div style={{ maxWidth: 680, fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          <p>
            Unsupported constructs (<code>delegatecall</code>, <code>assembly</code>, <code>tx.origin</code>,
            ternary <code>?:</code>) produce a clear error and nothing is emitted. The integrity guarantee was hardened
            across an adversarial review loop covering: integer widths (<code>uint8 255+1</code> reverts,{' '}
            <code>unchecked{'{}'}</code> wraps to 0, <code>uint8(256)==0</code> narrowing cast), function overloading
            (distinct keccak4 selectors per signature), inheritance flattening, cross-contract <code>seal_call</code>,
            and events (keccak topic + SCALE data).
          </p>
          <p>
            All 30 validated contracts passed a reviewer-verified adversarial loop of 6 rounds — 89 translator
            unit tests green, 0 silent miscompiles detected.
          </p>
        </div>
      </div>

      {/* Section 4: Why Solidity */}
      <div className="why-section">
        <h2 className="section-title">Why Solidity, not a new language</h2>
        <div style={{ maxWidth: 680, fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          <p>
            Solidity is the language tens of thousands of smart contract developers already know. A new DSL would
            require learning new syntax, new idioms, and new tooling. InkPort gives Portaldot developers
            Hardhat&apos;s workflow: write <code>contracts/</code>, run <code>inkport deploy</code>, get an address.
          </p>
          <p>
            The translation is source-to-source — Solidity AST → seal0 Rust — not EVM-on-WASM emulation. The output
            is idiomatic, readable Rust that you can inspect, audit, and extend.
          </p>
        </div>
      </div>

      {/* Section 5: What InkPort is NOT */}
      <div className="why-section">
        <h2 className="section-title">What InkPort is not</h2>
        <ul className="limits-list" style={{ maxWidth: 680 }}>
          {[
            ['A production security auditor', 'Translated output should be reviewed before deployment with real value.'],
            ['A full Solidity compiler', 'Inline assembly, delegatecall, libraries, and new ContractFactory() are intentionally rejected.'],
            ['An EVM emulator', 'Gas accounting, storage layout, and ABI encoding differ from Ethereum. The integrity guarantee covers semantic correctness on pallet-contracts, not EVM parity.'],
            ['An ink! replacement', "If your node supports ink! 5.x, use ink!. InkPort exists specifically for rent-era pallet-contracts nodes that reject ink! wasm."],
          ].map(([title, body]) => (
            <li key={title} style={{ flexDirection: 'column', gap: 2 }}>
              <strong style={{ color: 'var(--text)', fontSize: 'var(--body)' }}>{title}</strong>
              <span>{body}</span>
            </li>
          ))}
        </ul>
      </div>

      {/* CTA */}
      <div className="why-section" style={{ paddingBottom: 0, borderTop: 'none' }}>
        <h2 className="section-title">Ready to deploy?</h2>
        <div className="cta-row">
          <Link href="/docs/getting-started/install" className="btn btn-primary">Read the getting started guide →</Link>
          <Link href="/" className="btn">Try the playground</Link>
        </div>
      </div>

    </div>
  );
}
