import Link from 'next/link';

export const metadata = {
  title: 'Why InkPort — seal0 vs ink!, design decisions',
  description: 'Why InkPort targets raw seal0 Rust instead of ink!, the integrity guarantee, and why Solidity.',
};

export default function WhyInkPortPage() {
  return (
    <div className="wrap" style={{ paddingTop: 64, paddingBottom: 80 }}>

      {/* Header */}
      <div style={{ maxWidth: 680, marginBottom: 56 }}>
        <p className="eyebrow" style={{ justifyContent: 'flex-start' }}>Design decisions</p>
        <h1 style={{ fontSize: 46, lineHeight: 1.08, letterSpacing: '-0.03em', fontWeight: 600, margin: '0 0 16px' }}>
          Why InkPort
        </h1>
        <p style={{ fontSize: 18, color: 'var(--text-dim)', lineHeight: 1.65, margin: 0 }}>
          The reasoning behind a Solidity toolchain that targets raw seal0 — not ink!.
        </p>
      </div>

      {/* Section 1: The problem */}
      <div className="why-section">
        <h2 className="h-sec" style={{ fontSize: 28, marginBottom: 16 }}>The problem</h2>
        <div style={{ maxWidth: 680 }}>
          <p style={{ fontSize: 16, color: 'var(--text-dim)', lineHeight: 1.75, marginBottom: 14 }}>
            Portaldot runs a rent-era <code className="inline">pallet-contracts</code> node — a version of Substrate&apos;s smart contract
            pallet from ~2021 that speaks the seal0 host ABI. ink! 3, 4, and 5 each require an era-matched Rust
            nightly and dependency set that doesn&apos;t build against this node. ink! 5.x wasm uses host functions this
            node rejects outright.
          </p>
          <p style={{ fontSize: 16, color: 'var(--text-dim)', lineHeight: 1.75, marginBottom: 0 }}>
            If you want to deploy a smart contract to Portaldot today, you need to write raw seal0 Rust by hand —
            a <code className="inline">no_std</code> crate that imports the node&apos;s host functions directly.
            That&apos;s what InkPort generates for you.
          </p>
        </div>
      </div>

      {/* Section 2: What seal0 means */}
      <div className="why-section">
        <h2 className="h-sec" style={{ fontSize: 28, marginBottom: 16 }}>What &quot;seal0 Rust&quot; means</h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 18, marginTop: 8 }}>
          <div className="tc-card expects">
            <div className="tc-label">The node expects</div>
            <ul>
              <li>MVP WebAssembly (no memory.fill, no memory.copy)</li>
              <li>call / deploy exports + imported memory</li>
              <li>seal0 host functions: seal_input, seal_return, seal_get_storage, seal_set_storage, seal_deposit_event</li>
            </ul>
          </div>
          <div className="tc-card emits">
            <div className="tc-label">InkPort emits</div>
            <ul>
              <li><code className="inline">#![no_std]</code> Rust on stable cargo</li>
              <li>No nightly, no ink! dependency</li>
              <li>Buffers sized to SCALE payload</li>
              <li>Stripped to MVP wasm with imported memory + max declared</li>
            </ul>
          </div>
          <div className="tc-card write">
            <div className="tc-label">You write</div>
            <ul>
              <li>Plain Solidity (.sol file)</li>
              <li>A single concrete contract</li>
              <li>30 validated contracts: Counter → ERC20 → ERC721 → payable → cross-contract</li>
            </ul>
          </div>
        </div>
      </div>

      {/* Section 3: Integrity guarantee */}
      <div className="why-section">
        <h2 className="h-sec" style={{ fontSize: 28, marginBottom: 16 }}>The integrity guarantee</h2>
        <div className="guarantee" style={{ marginBottom: 28 }}>
          <div className="g-quote">
            Every construct either compiles to <span className="hl">semantically-correct seal0 Rust</span>,<br />
            or <code style={{ fontFamily: 'var(--mono)', fontWeight: 500 }}>inkport translate</code> exits non-zero.
          </div>
          <p style={{ color: 'var(--text-dim)', marginTop: 10, marginBottom: 0, fontSize: 15 }}>
            There are no silent miscompiles.
          </p>
        </div>
        <div style={{ maxWidth: 680, fontSize: 16, color: 'var(--text-dim)', lineHeight: 1.75 }}>
          <p style={{ marginBottom: 14 }}>
            Unsupported constructs (<code className="inline">delegatecall</code>, <code className="inline">assembly</code>, <code className="inline">tx.origin</code>,
            ternary <code className="inline">?:</code>) produce a clear error and nothing is emitted. The integrity guarantee was hardened
            across an adversarial review loop covering: integer widths (<code className="inline">uint8 255+1</code> reverts,{' '}
            <code className="inline">{'unchecked{}'}</code> wraps to 0, <code className="inline">uint8(256)==0</code> narrowing cast), function overloading
            (distinct keccak4 selectors per signature), inheritance flattening, cross-contract <code className="inline">seal_call</code>,
            and events (keccak topic + SCALE data).
          </p>
          <p style={{ marginBottom: 0 }}>
            All 30 validated contracts passed a reviewer-verified adversarial loop of 6 rounds — 89 translator
            unit tests green, 0 silent miscompiles detected.
          </p>
        </div>
      </div>

      {/* Section 4: Why Solidity */}
      <div className="why-section">
        <h2 className="h-sec" style={{ fontSize: 28, marginBottom: 16 }}>Why Solidity, not a new language</h2>
        <div style={{ maxWidth: 680, fontSize: 16, color: 'var(--text-dim)', lineHeight: 1.75 }}>
          <p style={{ marginBottom: 14 }}>
            Solidity is the language tens of thousands of smart contract developers already know. A new DSL would
            require learning new syntax, new idioms, and new tooling. InkPort gives Portaldot developers
            Hardhat&apos;s workflow: write <code className="inline">contracts/</code>, run <code className="inline">inkport deploy</code>, get an address.
          </p>
          <p style={{ marginBottom: 0 }}>
            The translation is source-to-source — Solidity AST → seal0 Rust — not EVM-on-WASM emulation. The output
            is idiomatic, readable Rust that you can inspect, audit, and extend.
          </p>
        </div>
      </div>

      {/* Section 5: What InkPort is NOT */}
      <div className="why-section">
        <h2 className="h-sec" style={{ fontSize: 28, marginBottom: 16 }}>What InkPort is not</h2>
        <div className="notlist">
          {[
            ['A production security auditor', 'Translated output should be reviewed before deployment with real value.'],
            ['A full Solidity compiler', 'Inline assembly, delegatecall, libraries, and new ContractFactory() are intentionally rejected.'],
            ['An EVM emulator', 'Gas accounting, storage layout, and ABI encoding differ from Ethereum. The guarantee covers pallet-contracts correctness, not EVM parity.'],
            ['An ink! replacement', "If your node supports ink! 5.x, use ink!. InkPort exists specifically for rent-era pallet-contracts nodes that reject ink! wasm."],
          ].map(([title, body]) => (
            <div key={title} className="nl-item">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
                <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round"/>
              </svg>
              <div>
                <b>{title}</b>
                <span>{body}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* CTA */}
      <div className="why-section" style={{ paddingBottom: 0, borderTop: 'none' }}>
        <div className="cta-band">
          <h2>Ready to deploy?</h2>
          <p>From Solidity to a live contract on Portaldot in four commands.</p>
          <div className="hero-ctas">
            <Link href="/docs/getting-started/install" className="btn btn-primary btn-lg">
              Read the getting started guide
            </Link>
            <Link href="/playground" className="btn btn-lg">
              Try the playground
            </Link>
          </div>
        </div>
      </div>

    </div>
  );
}
