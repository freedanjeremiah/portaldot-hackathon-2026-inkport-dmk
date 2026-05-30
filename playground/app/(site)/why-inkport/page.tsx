import Link from 'next/link';
import RevealInit from '@/components/RevealInit';

export const metadata = {
  title: 'Why InkPort — seal0, integrity, and design decisions',
  description: 'Why InkPort targets raw seal0 Rust instead of ink!, the integrity guarantee, and why Solidity.',
};

const IcCheck = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--green)" strokeWidth="2.2" style={{ flexShrink: 0, marginTop: 3 }}>
    <path d="M5 12l5 5L19 7" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const IcX = () => (
  <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
    <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round"/>
  </svg>
);
const IcArrow = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path d="M5 12h14M13 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

export default function WhyInkPortPage() {
  return (
    <>
      <RevealInit />
      {/* Hero */}
      <section className="why-hero grid-bg">
        <div className="wrap-narrow">
          <div className="eyebrow reveal">Why InkPort exists</div>
          <h1 className="reveal" style={{ fontSize: 52, letterSpacing: '-0.03em', fontWeight: 700, margin: '12px 0 18px', lineHeight: 1.08 }}>
            A high-level language for a node that ink! can&apos;t reach.
          </h1>
          <p className="reveal" style={{ fontSize: 20, color: 'var(--text-dim)', lineHeight: 1.6, maxWidth: 720, margin: 0 }}>
            Portaldot runs an older pallet-contracts that modern toolchains abandoned. InkPort is the bridge:
            Solidity in, Portaldot-compatible wasm out, with a hard guarantee against silent miscompiles.
          </p>
        </div>
      </section>

      <div className="wrap-narrow">

        {/* 1 — The problem */}
        <section className="why-block reveal">
          <h2>The problem</h2>
          <p className="why-lead">
            Portaldot runs a <strong>rent-era <code className="inline">pallet-contracts</code></strong> — the seal0 ABI,
            roughly Substrate 2021. The chain&apos;s live metadata confirms it: <code className="inline">Compact&lt;Weight&gt;</code> gas,
            no <code className="inline">storage_deposit_limit</code>, no <code className="inline">upload_code</code>,
            and <code className="inline">contracts_rentProjection</code> present.
          </p>
          <p>
            ink! 3/4/5 wasm imports host functions this node simply doesn&apos;t expose, so it either won&apos;t build
            or gets rejected on instantiate. That leaves exactly one option today: <strong>write raw seal0 Rust by hand</strong> —
            importing only the node&apos;s host functions, on stable Rust, with <code className="inline">no_std</code>.
            That&apos;s painful, error-prone work. InkPort automates all of it.
          </p>
        </section>

        {/* 2 — What seal0 Rust means */}
        <section className="why-block reveal">
          <h2>What &quot;seal0 Rust&quot; actually means</h2>
          <p className="why-lead">Three layers, one source file. You write Solidity; InkPort emits the raw Rust; the node runs the stripped wasm.</p>
          <div className="three-col">
            <div className="tc-card expects">
              <div className="tc-label">The node expects</div>
              <ul>
                <li>MVP WebAssembly only</li>
                <li>call / deploy exports</li>
                <li>imported memory</li>
                <li>seal0 host functions</li>
                <li>no memory.fill / memory.copy</li>
              </ul>
            </div>
            <div className="tc-card emits">
              <div className="tc-label">InkPort emits</div>
              <ul>
                <li><code className="inline">#![no_std]</code> on stable cargo</li>
                <li>seal_input / seal_return</li>
                <li>seal_get_storage / set_storage</li>
                <li>seal_deposit_event</li>
                <li>SCALE-encoded args + storage</li>
              </ul>
            </div>
            <div className="tc-card write">
              <div className="tc-label">You write</div>
              <ul>
                <li>contract Token {'{ … }'}</li>
                <li>mapping, modifiers, events</li>
                <li>require / revert / emit</li>
                <li>msg.sender, msg.value</li>
                <li>plain, familiar Solidity</li>
              </ul>
            </div>
          </div>
        </section>

        {/* 3 — Integrity guarantee */}
        <section className="why-block reveal">
          <h2>The integrity guarantee</h2>
          <div className="guarantee">
            <div className="g-quote">
              Every construct either compiles to <span className="hl">semantically-correct seal0 Rust</span>,
              or <code className="inline">inkport translate</code> exits non-zero.{' '}
              <span className="hl">No silent miscompiles.</span>
            </div>
          </div>
          <p>
            This is the line InkPort will not cross: it never produces wasm that looks right but behaves wrong.
            An adversarial review loop hardened the areas most likely to miscompile quietly:
          </p>
          <ul style={{ listStyle: 'none', padding: 0, margin: '18px 0 0', display: 'flex', flexDirection: 'column', gap: 12, maxWidth: 760 }}>
            <li style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
              <IcCheck />
              <span style={{ color: 'var(--text-dim)', lineHeight: 1.6 }}>
                <strong>Integer widths</strong> — <code className="inline">uintN</code> maps to u128 but keeps true bit-width semantics:
                <code className="inline">uint8(255)+1</code> reverts, <code className="inline">unchecked {'{}'}</code> wraps,
                <code className="inline">uint8(256)==0</code> narrows.
              </span>
            </li>
            <li style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
              <IcCheck />
              <span style={{ color: 'var(--text-dim)', lineHeight: 1.6 }}>
                <strong>Overloading</strong> — each signature gets a distinct keccak4 selector, so{' '}
                <code className="inline">add(uint256)</code> and <code className="inline">add(uint256,uint256)</code> never collide.
              </span>
            </li>
            <li style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
              <IcCheck />
              <span style={{ color: 'var(--text-dim)', lineHeight: 1.6 }}>
                <strong>Inheritance</strong> — <code className="inline">is Base</code> is flattened deterministically;
                modifiers inline as guards in the right order.
              </span>
            </li>
            <li style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
              <IcCheck />
              <span style={{ color: 'var(--text-dim)', lineHeight: 1.6 }}>
                <strong>Cross-contract calls + events</strong> — <code className="inline">IFoo(addr).bar()</code> lowers
                to <code className="inline">seal_call</code> with a keccak4 selector; <code className="inline">emit</code>{' '}
                becomes <code className="inline">seal_deposit_event</code> with keccak topics and SCALE data.
              </span>
            </li>
          </ul>
        </section>

        {/* 4 — Why Solidity */}
        <section className="why-block reveal">
          <h2>Why Solidity, not a new language</h2>
          <p className="why-lead">Tens of thousands of developers already write Solidity. A new DSL would mean a new audience of zero.</p>
          <p>
            InkPort is a <strong>source-to-source translator</strong> — AST → seal0 Rust — not an EVM emulator.
            There&apos;s no EVM interpreter shipped in your contract, no 256-bit-everywhere overhead, no opcode dispatch loop.
            The output is ordinary Rust you can read, diff, and audit. If you don&apos;t trust the translation,
            you can inspect exactly what runs on-chain.
          </p>
        </section>

        {/* 5 — What InkPort is not */}
        <section className="why-block reveal">
          <h2>What InkPort is not</h2>
          <p className="why-lead">Honest limits matter more than a long feature list.</p>
          <div className="notlist">
            {[
              ['Not a security auditor', 'It translates faithfully — it does not find reentrancy, logic bugs, or economic flaws in your contract.'],
              ['Not full Solidity coverage', 'A deliberate subset. Unsupported constructs fail loudly at translate time, never silently.'],
              ['Not an EVM emulator', 'No bytecode interpreter. Real native Rust compiled to wasm, not EVM-in-wasm.'],
              ['Not an ink! replacement', "If your node supports ink! 5.x, use ink!. InkPort is for nodes that don't."],
            ].map(([title, body]) => (
              <div key={title} className="nl-item">
                <IcX />
                <div>
                  <b>{title}</b>
                  <span>{body}</span>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* 6 — CTA */}
        <section className="why-block reveal" style={{ borderTop: 0 }}>
          <div className="cta-band">
            <h2>Ready to deploy your first contract?</h2>
            <p>Install the toolchain, or try the full pipeline in your browser right now.</p>
            <div className="hero-ctas">
              <Link href="/docs/getting-started/install" className="btn btn-primary btn-lg">
                Install InkPort
              </Link>
              <Link href="/playground" className="btn btn-lg">
                Open the Playground <IcArrow />
              </Link>
            </div>
          </div>
        </section>

      </div>
    </>
  );
}
