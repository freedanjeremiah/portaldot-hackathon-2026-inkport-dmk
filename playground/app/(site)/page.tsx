import Link from 'next/link';
import RevealInit from '@/components/RevealInit';

export const metadata = {
  title: 'InkPort — Write Solidity. Deploy to Portaldot.',
  description: 'InkPort translates Solidity into raw seal0 Rust, compiles it to Portaldot-compatible WebAssembly, and deploys to a live pallet-contracts chain.',
};

const IcCheck = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
    <path d="M5 12l5 5L19 7" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const IcCheckSm = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
    <path d="M5 12l5 5L19 7" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const IcX = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4">
    <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round"/>
  </svg>
);
const IcArrow = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path d="M5 12h14M13 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const IcGH = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.5v-2c-3.2.7-3.9-1.4-3.9-1.4-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.8 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.3 1.2a11.5 11.5 0 016 0C17.3 4.6 18.3 5 18.3 5c.6 1.6.2 2.8.1 3.1.8.8 1.2 1.8 1.2 3.1 0 4.5-2.7 5.5-5.3 5.8.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.5A11.5 11.5 0 0023.5 12C23.5 5.7 18.3.5 12 .5z"/>
  </svg>
);

const PipeConn = () => (
  <div className="pipe-conn">
    <svg width="34" height="16" viewBox="0 0 34 16" fill="none" stroke="currentColor" strokeWidth="1.6">
      <path className="pipe-flow" d="M2 8h26" strokeLinecap="round"/>
      <path d="M24 3l5 5-5 5" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  </div>
);

export default function HomePage() {
  return (
    <>
      <RevealInit />

      {/* ── HERO ── */}
      <section className="hero grid-bg">
        <div className="hero-inner">
          <div className="hero-badge"><span className="hb-dot" /> Live on Portaldot · seal0 pallet-contracts</div>
          <h1>Write Solidity.<br /><span className="grad">Deploy to Portaldot.</span></h1>
          <p className="hero-sub">
            InkPort translates Solidity into raw seal0 Rust, compiles it to Portaldot-compatible WebAssembly,
            and deploys it to a live <code className="inline">pallet-contracts</code> chain — no ink!, no Rust, no EVM emulation.
          </p>
          <div className="hero-ctas">
            <Link href="/playground" className="btn btn-primary btn-lg">
              Try the Playground <IcArrow />
            </Link>
            <a className="btn btn-lg" href="https://github.com/freedanjeremiah/inkide" target="_blank" rel="noopener noreferrer">
              <IcGH /> View on GitHub
            </a>
          </div>
          <div className="hero-mono">
            <span>sol</span>
            <span className="hm-arrow">→</span>
            <b>seal0 rust</b>
            <span className="hm-arrow">→</span>
            <b>wasm</b>
            <span className="hm-arrow">→</span>
            <span className="hm-end">portaldot</span>
          </div>
        </div>
      </section>

      {/* ── PIPELINE STRIP ── */}
      <section className="section-sm">
        <div className="wrap">
          <div className="pipe-strip">
            <div className="pipe-card is-file reveal">
              <div className="pipe-num">input</div>
              <div className="pipe-ic">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
                  <path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8l-5-5z" strokeLinejoin="round"/>
                  <path d="M14 3v5h5" strokeLinejoin="round"/>
                </svg>
              </div>
              <div className="pipe-name">Token.sol</div>
              <div className="pipe-desc">your Solidity contract — the only thing you write</div>
              <div className="pipe-cmd">contracts/Token.sol</div>
            </div>
            <PipeConn />
            <div className="pipe-card reveal">
              <div className="pipe-num">01</div>
              <div className="pipe-ic">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
                  <path d="M4 7h11M9 4l-5 3 5 3M20 17H9m6-3l5 3-5 3" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
              </div>
              <div className="pipe-name">Translate</div>
              <div className="pipe-desc">solang-parser → IR → seal0 Rust codegen</div>
              <div className="pipe-cmd">inkport translate</div>
            </div>
            <PipeConn />
            <div className="pipe-card reveal">
              <div className="pipe-num">02</div>
              <div className="pipe-ic">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
                  <path d="M12 2l8 4.5v9L12 20l-8-4.5v-9L12 2z" strokeLinejoin="round"/>
                  <path d="M12 11l8-4.5M12 11v9" strokeLinejoin="round"/>
                </svg>
              </div>
              <div className="pipe-name">Compile</div>
              <div className="pipe-desc">cargo +stable build → wasm → MVP strip</div>
              <div className="pipe-cmd">inkport build</div>
            </div>
            <PipeConn />
            <div className="pipe-card reveal">
              <div className="pipe-num">03</div>
              <div className="pipe-ic">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
                  <path d="M12 3c3 1.5 5 4.5 5 8 0 2-1 4-2 5H9c-1-1-2-3-2-5 0-3.5 2-6.5 5-8z" strokeLinejoin="round"/>
                  <circle cx="12" cy="10" r="1.5"/>
                  <path d="M9 16l-2 4 3-1.5M15 16l2 4-3-1.5" strokeLinejoin="round"/>
                </svg>
              </div>
              <div className="pipe-name">Deploy</div>
              <div className="pipe-desc">instantiate_with_code on the live node</div>
              <div className="pipe-cmd">inkport deploy</div>
            </div>
            <PipeConn />
            <div className="pipe-card reveal">
              <div className="pipe-num">04</div>
              <div className="pipe-ic">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
                  <rect x="3" y="4" width="18" height="16" rx="2"/>
                  <path d="M7 9l3 3-3 3M13 15h4" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
              </div>
              <div className="pipe-name">Call</div>
              <div className="pipe-desc">Contracts.call / dry-run, decoded results</div>
              <div className="pipe-cmd">inkport call</div>
            </div>
          </div>
        </div>
      </section>

      {/* ── STATS BAR ── */}
      <section className="stats-band">
        <div className="stats-row">
          <div className="stat"><div className="stat-num g">30</div><div className="stat-label">contracts validated</div></div>
          <div className="stat"><div className="stat-num b">89</div><div className="stat-label">translator tests</div></div>
          <div className="stat"><div className="stat-num">0</div><div className="stat-label">silent miscompiles</div></div>
          <div className="stat"><div className="stat-num g">live</div><div className="stat-label">on Portaldot</div></div>
        </div>
      </section>

      {/* ── VALUE PROP ── */}
      <section className="section">
        <div className="wrap">
          <div className="section-head reveal">
            <div className="eyebrow">Two audiences, one toolchain</div>
            <h2 className="h-sec">Built for both sides of the chain</h2>
          </div>
          <div className="vp-grid">
            <div className="vp-card blue reveal">
              <div className="vp-kicker">For Solidity developers</div>
              <h3>Ship to Portaldot without learning Rust</h3>
              <p>Keep writing the Solidity you already know. InkPort handles the seal0 ABI, the storage layout, the SCALE encoding, and the wasm strip — you never touch ink! or <code className="inline">no_std</code> Rust.</p>
              <ul className="vp-list">
                <li><IcCheck />Familiar syntax — mappings, modifiers, events, inheritance</li>
                <li><IcCheck />True integer-width semantics — <code className="inline">uint8(255)+1</code> reverts</li>
                <li><IcCheck />Readable, auditable Rust output you can inspect</li>
              </ul>
            </div>
            <div className="vp-card green reveal">
              <div className="vp-kicker">For Portaldot builders</div>
              <h3>The only high-level path to seal0</h3>
              <p>Portaldot runs a rent-era <code className="inline">pallet-contracts</code> that ink! 3/4/5 wasm can&apos;t target. InkPort is the only toolchain that emits compatible bytecode from a high-level language.</p>
              <ul className="vp-list">
                <li><IcCheck />Targets the seal0 host functions your node actually exposes</li>
                <li><IcCheck />MVP-wasm only — no <code className="inline">memory.fill</code> / <code className="inline">memory.copy</code></li>
                <li><IcCheck />Builds on stable Rust — no nightly, no ink! cargo-contract</li>
              </ul>
            </div>
          </div>
        </div>
      </section>

      {/* ── QUICK INSTALL ── */}
      <section className="section-sm">
        <div className="wrap">
          <div className="section-head reveal">
            <div className="eyebrow">Quick start</div>
            <h2 className="h-sec">Four commands to a live contract</h2>
            <p className="sub-sec">From a clean checkout to a deployed contract on Portaldot.</p>
          </div>
          <div className="install-grid">
            <div className="install-step reveal">
              <div className="is-head"><span className="is-num">1</span><span className="is-title">Build the translator</span></div>
              <pre>{`cargo build --release\n# adds target/release/inkport-translate`}</pre>
            </div>
            <div className="install-step reveal">
              <div className="is-head"><span className="is-num">2</span><span className="is-title">Install the CLI harness</span></div>
              <pre>{`pip install -e inkport\n# python deploy/call/test harness`}</pre>
            </div>
            <div className="install-step reveal">
              <div className="is-head"><span className="is-num">3</span><span className="is-title">Translate + build</span></div>
              <pre>{`inkport translate contracts/Counter.sol\ninkport build Counter`}</pre>
            </div>
            <div className="install-step reveal">
              <div className="is-head"><span className="is-num">4</span><span className="is-title">Deploy to the node</span></div>
              <pre>{`inkport deploy Counter --arg 0\n# → contract address on Portaldot`}</pre>
            </div>
          </div>
          <div style={{ marginTop: 22 }} className="reveal">
            <Link href="/docs/getting-started/install" className="btn">
              Full install guide <IcArrow />
            </Link>
          </div>
        </div>
      </section>

      {/* ── PLAYGROUND TEASER ── */}
      <section className="section">
        <div className="wrap">
          <div className="teaser">
            <div className="teaser-frame reveal">
              <div className="tf-bar">
                <i/><i/><i/>
                <span className="tf-url">inkport.dev/playground</span>
              </div>
              <div className="teaser-split">
                <div className="ts-pane">
                  <div className="ts-label">
                    <span className="dot" style={{ background: 'var(--accent)' }}/>
                    Counter.sol
                  </div>
                  <pre>{`contract Counter {
    uint256 private count;

    constructor(uint256 initial) {
        count = initial;
    }

    function inc() public {
        count += 1;
    }

    function get() public view
        returns (uint256) {
        return count;
    }
}`}</pre>
                </div>
                <div className="ts-pane">
                  <div className="ts-label">
                    <span className="dot" style={{ background: 'var(--green)' }}/>
                    lib.rs · seal0
                  </div>
                  <pre>{`#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn call() {
    let mut input = [0u8; 1024];
    let sel = u32::from_be_bytes(
        [input[0], input[1],
         input[2], input[3]]);
    match sel {
        0x371303c0 => {
            let c = read_u128(&SLOT);
            write_u128(&SLOT, c + 1);
        }
        _ => revert(b"selector"),
    }
}`}</pre>
                </div>
              </div>
            </div>
            <div className="reveal">
              <div className="eyebrow">Live in the browser</div>
              <h2 className="h-sec" style={{ fontSize: 32 }}>See the Rust as you type</h2>
              <p className="sub-sec" style={{ marginBottom: 24 }}>
                The playground runs the full translate → compile → deploy → call pipeline. Edit Solidity on the left,
                watch the generated seal0 Rust regenerate on every keystroke — no install needed.
              </p>
              <Link href="/playground" className="btn btn-primary">
                Open Playground <IcArrow />
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* ── SUPPORTED SOLIDITY ── */}
      <section className="section" style={{ borderTop: '1px solid var(--border)' }}>
        <div className="wrap">
          <div className="section-head reveal">
            <div className="eyebrow">Fail-loud by design</div>
            <h2 className="h-sec">What translates — and what won&apos;t</h2>
            <p className="sub-sec">Every construct either compiles to semantically-correct seal0 Rust, or <code className="inline">inkport translate</code> exits non-zero. Never a silent miscompile.</p>
          </div>
          <div className="cov-grid">
            <div className="cov-col ok reveal">
              <div className="cov-head"><IcCheckSm /> Supported</div>
              <ul className="cov-list">
                <li><IcCheckSm /><div><div className="cl-name">mapping, T[], structs, enums</div><div className="cl-desc">collections incl. nested mappings + dynamic arrays</div></div></li>
                <li><IcCheckSm /><div><div className="cl-name">modifiers, inheritance, overloading</div><div className="cl-desc">flattened via <code className="inline">is</code>, inlined as guards</div></div></li>
                <li><IcCheckSm /><div><div className="cl-name">checked integer arithmetic</div><div className="cl-desc">true bit-width semantics, <code className="inline">unchecked {'{}'}</code> wraps</div></div></li>
                <li><IcCheckSm /><div><div className="cl-name">events, payable, cross-contract</div><div className="cl-desc"><code className="inline">emit</code>, <code className="inline">msg.value</code>, <code className="inline">IFoo(addr).bar()</code></div></div></li>
              </ul>
            </div>
            <div className="cov-col no reveal">
              <div className="cov-head"><IcX /> Rejected — exits non-zero</div>
              <ul className="cov-list">
                <li><IcX /><div><div className="cl-name">inline assembly</div><div className="cl-desc">no host function on the seal0 target</div></div></li>
                <li><IcX /><div><div className="cl-name">delegatecall, tx.origin</div><div className="cl-desc">unavailable under rent-era pallet-contracts</div></div></li>
                <li><IcX /><div><div className="cl-name">ternary ?:, new factory</div><div className="cl-desc">use if/else; factory deployment unsupported</div></div></li>
                <li><IcX /><div><div className="cl-name">libraries, abi.encodePacked</div><div className="cl-desc">flatten into the contract; use typed args</div></div></li>
              </ul>
            </div>
          </div>
          <div style={{ marginTop: 24 }} className="reveal">
            <Link href="/docs/solidity/supported" className="btn">
              Full coverage table <IcArrow />
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}
