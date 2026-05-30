import Link from 'next/link';
import { Ic } from '@/components/Pipeline';

export const metadata = {
  title: 'InkPort — Write Solidity, Deploy to Portaldot',
  description: 'InkPort translates Solidity to raw seal0 Rust, compiles to WebAssembly, and deploys on the live Portaldot chain.',
};

const PIPELINE_STEPS = [
  { label: 'Translate', sub: 'solang-parser\n→ seal0 Rust', cmd: 'inkport translate' },
  { label: 'Compile',   sub: 'cargo +stable\nwasm32', cmd: 'inkport build' },
  { label: 'Deploy',    sub: 'instantiate\n_with_code', cmd: 'inkport deploy' },
  { label: 'Call',      sub: 'Contracts.call\ndry-run / extrinsic', cmd: 'inkport call' },
];

const SUPPORTED = [
  'bool, uintN→u128, intN→i128, address',
  'mapping(K=>V), nested mappings, T[]',
  'events, modifiers, inheritance',
  'msg.sender, msg.value, block.timestamp',
  'payable functions, cross-contract calls',
  'Function overloading, enums, structs',
];

const REJECTED = [
  'inline assembly',
  'delegatecall',
  'tx.origin',
  'ternary ?:',
  'new ContractFactory()',
  'libraries / using for',
];

const INSTALL_LINES: [string, string][] = [
  ['comment', '# 1. Build the Rust translator'],
  ['dollar', 'source "$HOME/.cargo/env"'],
  ['dollar', '(cd translator && cargo build --release)'],
  ['blank', ''],
  ['comment', '# 2. Install the Python CLI'],
  ['dollar', 'python3.11 -m venv .venv && source .venv/bin/activate'],
  ['dollar', 'pip install -e inkport'],
  ['blank', ''],
  ['comment', '# 3. Translate → build → deploy'],
  ['dollar', 'inkport translate contracts/ERC20.sol'],
  ['dollar', 'inkport build ERC20'],
  ['dollar', 'inkport deploy ERC20 --arg 1000000'],
  ['dollar', 'inkport call ERC20 balanceOf --arg //Alice'],
];

export default function HomePage() {
  return (
    <div>
      {/* ── Hero ── */}
      <section className="site-hero">
        <div className="site-container">
          <p className="site-hero-eyebrow">Portaldot · pallet-contracts · seal0</p>
          <h1 className="site-h1">
            Write <span className="gradient-text">Solidity.</span><br />
            Deploy to Portaldot.
          </h1>
          <p className="site-hero-sub">
            InkPort translates Solidity contracts to raw seal0 Rust, compiles them to
            WebAssembly, and deploys + tests them on the live Portaldot chain —
            a Hardhat-style workflow whose compile target is pallet-contracts.
          </p>
          <div className="site-hero-actions">
            <Link href="/" className="btn btn-primary">Try the Playground →</Link>
            <a href="https://github.com/freedanjeremiah/inkide" className="btn" target="_blank" rel="noopener noreferrer">View on GitHub</a>
          </div>
          <p className="inline-chip" style={{ marginTop: 20, display: 'inline-block' }}>sol → seal0 rust → wasm → portaldot</p>
        </div>
      </section>

      {/* ── Pipeline strip ── */}
      <section className="section" style={{ paddingTop: 0, paddingBottom: 48 }}>
        <div className="site-container">
          <div className="pipeline-strip" style={{ justifyContent: 'center' }}>
            <div className="pipeline-step" style={{ background: 'var(--panel-2)', borderColor: 'var(--border-2)', minWidth: 80, padding: '12px 16px', borderRadius: 'var(--radius-sm)' }}>
              <span style={{ fontFamily: 'var(--mono)', fontSize: 13, color: 'var(--s-str)' }}>.sol</span>
            </div>
            {PIPELINE_STEPS.map((s) => (
              <span key={s.label} style={{ display: 'contents' }}>
                <span className="pipeline-arrow">→</span>
                <div className="pipeline-step">
                  <span className="pipeline-step-label">{s.label}</span>
                  <span className="pipeline-step-sub">{s.sub}</span>
                  <span className="pipeline-step-cmd">{s.cmd}</span>
                </div>
              </span>
            ))}
          </div>
        </div>
      </section>

      {/* ── Stats ── */}
      <section className="section" style={{ paddingTop: 0 }}>
        <div className="site-container">
          <div className="stats-bar">
            {[
              { num: '30', label: 'contracts validated' },
              { num: '89', label: 'translator tests' },
              { num: '0', label: 'silent miscompiles' },
              { num: '✓', label: 'live on Portaldot' },
            ].map(s => (
              <div key={s.label} className="stat-item">
                <div className="stat-num">{s.num}</div>
                <div className="stat-label">{s.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Two-column value prop ── */}
      <section className="section">
        <div className="site-container">
          <div className="value-cols">
            <div className="value-col">
              <div className="value-col-title"><span className="vc-dot" style={{ background: 'var(--accent)' }} />For Solidity developers</div>
              <p>You already know Solidity. InkPort lets you deploy it directly to Portaldot without learning ink!, Rust nightly builds, or a new framework from scratch. The CLI mirrors Hardhat: <code>translate → build → deploy → call</code>.</p>
            </div>
            <div className="value-col">
              <div className="value-col-title"><span className="vc-dot" style={{ background: 'var(--green)' }} />For Portaldot builders</div>
              <p>Portaldot&apos;s pallet-contracts speaks seal0 — a raw host ABI from ~Substrate 2021. InkPort is the only toolchain that targets it from a high-level language today, compiling on stable Rust with no ink! dependency.</p>
            </div>
          </div>
        </div>
      </section>

      {/* ── Quick install ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Quick install</h2>
          <p className="section-sub">Prereqs: rustup (stable + wasm32-unknown-unknown target), Python 3.11.</p>
          <div className="install-block">
            <div className="install-block-header">
              <span className="dot" style={{ background: '#ef4444' }} />
              <span className="dot" style={{ background: '#f59e0b' }} />
              <span className="dot" style={{ background: '#10b981' }} />
              <span style={{ marginLeft: 8 }}>bash</span>
            </div>
            <pre>
              {INSTALL_LINES.map((line, i) => {
                if (line[0] === 'blank') return <span key={i}>{'\n'}</span>;
                if (line[0] === 'comment') return <span key={i} className="cmd-comment">{line[1]}{'\n'}</span>;
                return <span key={i}><span className="cmd-dollar">$ </span>{line[1]}{'\n'}</span>;
              })}
            </pre>
          </div>
          <p className="mt-4" style={{ fontSize: 'var(--small)', color: 'var(--text-faint)' }}>
            <Link href="/docs/getting-started/install" style={{ color: 'var(--accent-2)', textDecoration: 'none' }}>Full install guide →</Link>
          </p>
        </div>
      </section>

      {/* ── Playground teaser ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Try it in the browser</h2>
          <p className="section-sub">No install needed. Edit Solidity on the left, watch seal0 Rust generate live on the right.</p>
          <div className="playground-teaser">
            <div className="playground-teaser-bar">
              {['#ef4444','#f59e0b','#10b981'].map(c => <span key={c} className="playground-teaser-dot" style={{ background: c }} />)}
              <span style={{ fontFamily: 'var(--mono)', fontSize: 11, color: 'var(--text-faint)', marginLeft: 8 }}>InkPort Playground — sol → seal0 → wasm</span>
            </div>
            <div className="playground-preview">
              <div className="playground-preview-pane">
                <span style={{ color: 'var(--s-com)' }}>{'// contracts/Counter.sol\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'contract '}</span>
                <span style={{ color: 'var(--s-type)' }}>{'Counter '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--s-id)' }}>{'  uint256 '}</span>
                <span>{'private count;\n\n'}</span>
                <span style={{ color: 'var(--s-fn)' }}>{'  function '}</span>
                <span>{'inc() '}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'public '}</span>
                <span>{'{\n'}</span>
                <span>{'    count += 1;\n  }\n'}</span>
                <span style={{ color: 'var(--s-fn)' }}>{'  function '}</span>
                <span>{'get() '}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'public view\n  returns (uint256) '}</span>
                <span>{'{ return count; }\n}'}</span>
              </div>
              <div className="playground-preview-pane">
                <span style={{ color: 'var(--s-com)' }}>{'// build/Counter/src/lib.rs\n'}</span>
                <span style={{ color: 'var(--s-attr)' }}>{'#![no_std]\n#![no_main]\n\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'mod '}</span>
                <span style={{ color: 'var(--s-type)' }}>{'seal0 '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--s-attr)' }}>{'  #[link(wasm_import_module="seal0")]\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'  extern "C" '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--text-faint)' }}>{'    pub fn seal_input(...);\n    pub fn seal_return(...);\n    ...\n  }\n}'}</span>
              </div>
            </div>
            <div className="playground-teaser-overlay">
              <p>Compile → deploy → call, all simulated in-session.</p>
              <Link href="/" className="btn btn-primary">Open Playground →</Link>
            </div>
          </div>
        </div>
      </section>

      {/* ── Supported Solidity ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Supported Solidity</h2>
          <p className="section-sub">
            Every construct either compiles to semantically-correct seal0 Rust or <code>inkport translate</code> exits non-zero — never a silent miscompile.
          </p>
          <div className="coverage-grid">
            <div className="coverage-col">
              <h4>Supported</h4>
              <ul className="coverage-list coverage-ok">
                {SUPPORTED.map(s => <li key={s}>{s}</li>)}
              </ul>
            </div>
            <div className="coverage-col">
              <h4>Rejected (fail-loud)</h4>
              <ul className="coverage-list coverage-no">
                {REJECTED.map(s => <li key={s}>{s}</li>)}
              </ul>
            </div>
          </div>
          <p className="mt-6" style={{ fontSize: 'var(--small)', color: 'var(--text-faint)' }}>
            <Link href="/docs/solidity/supported" style={{ color: 'var(--accent-2)', textDecoration: 'none' }}>Full coverage table →</Link>
          </p>
        </div>
      </section>
    </div>
  );
}
