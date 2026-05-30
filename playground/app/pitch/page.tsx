import "../(site)/site.css";
import "./pitch.css";
import PitchClient from "./PitchClient";

export const metadata = {
  title: "InkPort — Write Solidity. Ship on Portaldot.",
  description:
    "The Solidity → Rust compiler + CLI that deploys EVM-style contracts to the live Portaldot chain, paying real POT gas. 30 contracts live on mainnet.",
};

export default function PitchPage() {
  return (
    <main className="pitch">
      <div className="p-bar" id="p-bar" />
      <nav className="p-nav" id="p-nav" />
      <div className="p-hint">↓ scroll · → arrow keys</div>
      <PitchClient />

      {/* 1 — TITLE */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="brand reveal">
            <div className="mark"><img src="/inkport.png" alt="InkPort" style={{ width: '100%', height: '100%', objectFit: 'contain', background: '#f4ecdb', borderRadius: 'inherit' }} /></div>
            <span className="bname">InkPort</span>
          </div>
          <h1 className="reveal d1" style={{ marginTop: 26 }}>
            Write <span className="accent">Solidity</span>.<br />
            Ship on <span className="grad">Portaldot</span>.
          </h1>
          <p className="lead reveal d2" style={{ marginTop: 22, fontSize: "clamp(18px,2vw,26px)" }}>
            The Solidity → Rust compiler + CLI that deploys EVM-style contracts straight to the
            Portaldot chain — paying real <strong>POT</strong> gas.
          </p>
          <div className="row reveal d3">
            <span className="badge"><span className="bdot" />LIVE ON PORTALDOT MAINNET</span>
            <span className="pill">30 contracts deployed</span>
            <span className="pill">real POT gas · not a testnet</span>
          </div>
        </div>
        <div className="foot"><span>INKPORT</span><span>WRITE SOLIDITY · SHIP ON PORTALDOT</span></div>
      </section>

      {/* 2 — PROBLEM */}
      <section className="slide band">
        <div className="slide-inner">
          <div className="eyebrow reveal">The Problem</div>
          <h2 className="reveal d1">Millions of Solidity devs<br />are locked out of Portaldot.</h2>
          <p className="lead reveal d2">
            Portaldot runs Substrate <code>pallet-contracts</code> — Rust compiled to WebAssembly.
            The largest smart-contract developer population on earth writes <strong>Solidity for the
            EVM</strong>. The two worlds don&apos;t meet.
          </p>
          <ul className="clean reveal d2">
            <li><span>Different execution target (EVM bytecode vs WASM), storage model, builtins, ABI, and revert semantics.</span></li>
            <li><span>To ship on Portaldot today, an EVM team must <strong>rewrite every contract from scratch</strong> in Rust/ink! — the single biggest onboarding cost.</span></li>
            <li><span>Result: a live chain with real liquidity and a vast developer base that can&apos;t reach it.</span></li>
          </ul>
        </div>
      </section>

      {/* 3 — SOLUTION */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="eyebrow reveal">The Solution</div>
          <h2 className="reveal d1">Paste a <span className="accent">.sol</span> file. Get a contract<br />live on <span className="grad">Portaldot mainnet</span>.</h2>
          <p className="lead reveal d2">
            InkPort translates Solidity to Rust, compiles it to WASM, and deploys + tests it on the
            live chain — a Hardhat-style workflow whose compile target is Portaldot.
          </p>
          <div className="flow reveal d2">
            <span className="step">Solidity .sol</span><span className="arr">→</span>
            <span className="step">seal0 Rust</span><span className="arr">→</span>
            <span className="step">WASM</span><span className="arr">→</span>
            <span className="step end">Portaldot mainnet · POT gas</span>
          </div>
          <p className="reveal d3" style={{ marginTop: 24 }}>One toolchain. No manual rewrite. The mechanical 100% is automated; you keep your Solidity.</p>
        </div>
      </section>

      {/* 4 — WHY NOW */}
      <section className="slide band">
        <div className="slide-inner">
          <div className="eyebrow reveal">Why Now</div>
          <h2 className="reveal d1">The window just opened.</h2>
          <div className="grid g3">
            <div className="card reveal d1"><h3>Mainnet is live</h3><p>Portaldot&apos;s <code>pallet-contracts</code> runtime is running in production and accepting contract deployments today — real chain, real POT.</p></div>
            <div className="card reveal d2"><h3>Solidity is the default</h3><p>The dominant contract language by an order of magnitude. Every new chain fights to import it. Portaldot couldn&apos;t — until now.</p></div>
            <div className="card reveal d3"><h3>Tooling is tractable</h3><p>A focused source-to-source translator with on-chain verification is finally buildable end-to-end by a small team.</p></div>
          </div>
        </div>
      </section>

      {/* 5 — MARKET */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="eyebrow reveal">Market</div>
          <h2 className="reveal d1">Every Solidity dev × every Substrate chain.</h2>
          <div className="grid g3">
            <div className="card reveal d1"><div className="n">Solidity</div><div className="l">#1 smart-contract language — the developer pool every chain wants to import.</div></div>
            <div className="card reveal d2"><div className="n">Portaldot</div><div className="l">Live <code>pallet-contracts</code> chain that gains an instant EVM-developer on-ramp.</div></div>
            <div className="card reveal d3"><div className="n">Substrate</div><div className="l">The same seal0 backend generalizes to any <code>pallet-contracts</code> chain — a category, not one chain.</div></div>
          </div>
          <p className="reveal d3" style={{ marginTop: 26 }}>InkPort is the on-ramp from the largest contract-dev community to the Substrate contract world.</p>
        </div>
      </section>

      {/* 6 — PRODUCT */}
      <section className="slide band">
        <div className="slide-inner">
          <div className="eyebrow reveal">Product</div>
          <h2 className="reveal d1">One CLI. Full lifecycle.</h2>
          <pre className="reveal d2"><code>{`# write contracts/ERC20.sol, then:
`}<span className="k-cmd">inkport translate</span>{` contracts/ERC20.sol   `}<span className="k-cmt"># Solidity → seal0 Rust + metadata.json</span>{`
`}<span className="k-cmd">inkport build</span>{` ERC20                     `}<span className="k-cmt"># cargo build → wasm → strip</span>{`
`}<span className="k-cmd">inkport deploy</span>{` ERC20 --arg 1000000       `}<span className="k-cmt"># instantiate on Portaldot mainnet (POT gas)</span>{`
`}<span className="k-cmd">inkport call</span>{` ERC20 transfer --arg //Bob --arg 250
`}<span className="k-cmd">inkport test</span>{` ERC20                      `}<span className="k-cmt"># on-chain behavioral assertions</span>{`

`}<span className="k-ok">deployed ERC20 → 5HcQTX3kYCANZVLesSkaEX2Wnk6Fp5xYzMxTtc4PqcBCgvoZ</span>{`
`}<span className="k-ok">✓ ALL STEPS PASSED</span></code></pre>
          <p className="reveal d3" style={{ marginTop: 18 }}>Everything is metadata-driven — keccak4 selectors, SCALE encoding, events — so it&apos;s ABI-compatible with standard Ethereum tooling.</p>
        </div>
      </section>

      {/* 7 — TRACTION */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="eyebrow reveal">Traction · Proof on mainnet</div>
          <h2 className="reveal d1">30 contracts. Live. Verified. Zero mock.</h2>
          <div className="grid g4">
            <div className="card reveal d1"><div className="n ac">30</div><div className="l">Solidity contracts deployed + tested on Portaldot <strong>mainnet</strong></div></div>
            <div className="card reveal d1"><div className="n">89</div><div className="l">translator tests green</div></div>
            <div className="card reveal d2"><div className="n">0</div><div className="l">silent miscompiles (adversarially reviewed)</div></div>
            <div className="card reveal d3"><div className="n green">POT</div><div className="l">real gas paid on every deploy &amp; call</div></div>
          </div>
          <div className="row reveal d3">
            <span className="pill">ERC20</span><span className="pill">ERC721</span><span className="pill">Auction</span><span className="pill">Escrow</span>
            <span className="pill">Bank (payable)</span><span className="pill">Voting</span><span className="pill">cross-contract</span><span className="pill">inheritance</span>
          </div>
          <p className="reveal d3" style={{ marginTop: 20 }}>Every &quot;test&quot; is a real extrinsic on <code>wss://portaldot.philotheephilix.in</code> with real receipts, events, and reverts. Live ERC20: <span className="addr">5HcQTX3kYCANZVLesSkaEX2Wnk6Fp5xYzMxTtc4PqcBCgvoZ</span></p>
        </div>
      </section>

      {/* 8 — UNDER THE HOOD */}
      <section className="slide band">
        <div className="slide-inner">
          <div className="eyebrow reveal">Under the Hood</div>
          <h2 className="reveal d1">Why it actually deploys.</h2>
          <ul className="clean reveal d2">
            <li><span><strong>Targets the real runtime.</strong> The node runs rent-era <code>pallet-contracts</code> (seal0 host ABI). InkPort emits raw <strong>seal0 Rust</strong> that compiles on <em>stable</em> Rust and the chain accepts as-is — no ink! toolchain lock-in.</span></li>
            <li><span><strong>ABI-faithful.</strong> keccak-256 4-byte selectors + canonical event topics, SCALE encoding — interoperable with Ethereum tooling.</span></li>
            <li><span><strong>Correct or loud.</strong> Every construct compiles to semantically-correct Rust or fails the build. <strong>No silent miscompiles</strong> — width-aware integers, checked overflow, narrowing-cast truncation, all verified on-chain.</span></li>
          </ul>
        </div>
      </section>

      {/* 9 — COMPETITION */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="eyebrow reveal">Why We Win</div>
          <h2 className="reveal d1">The only native deploy loop.</h2>
          <table className="reveal d2">
            <tbody>
              <tr><th>Approach</th><th>Solidity in</th><th>Deploys to Portaldot</th><th>Integrity</th></tr>
              <tr><td>Manual rewrite (ink!/Rust)</td><td>—</td><td>yes, by hand</td><td>human error</td></tr>
              <tr><td>Sol2Ink (legacy)</td><td>yes</td><td className="warn">codegen only, legacy ink!</td><td>no on-chain check</td></tr>
              <tr><td className="win">InkPort</td><td className="win">yes</td><td className="win">live mainnet, POT gas</td><td className="win">fail-loud, on-chain tested</td></tr>
            </tbody>
          </table>
          <p className="reveal d3" style={{ marginTop: 24 }}>Not just a transpiler — a <strong>translate → build → deploy → verify</strong> loop with an integrity guarantee, proven against the live chain.</p>
        </div>
      </section>

      {/* 10 — TEAM */}
      <section className="slide band">
        <div className="slide-inner">
          <div className="eyebrow reveal">Team</div>
          <h2 className="reveal d1">Builders who shipped it to mainnet.</h2>
          <p className="lead reveal d2">Built end-to-end: a Rust source-to-source compiler, a Python CLI + chain client, and a 30-contract on-chain regression suite — all live on Portaldot.</p>
          <ul className="clean reveal d2">
            <li><span>Compiler &amp; runtime: solang-parser → IR → seal0 codegen on stable Rust.</span></li>
            <li><span>Chain integration: substrate-interface deploy/call/dry-run, keccak/SCALE ABI, MVP-wasm validator constraints solved.</span></li>
            <li><span>Proof discipline: adversarial review loop until zero silent miscompiles, every contract green on mainnet.</span></li>
          </ul>
        </div>
      </section>

      {/* 11 — ASK */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="eyebrow reveal">The Ask</div>
          <h2 className="reveal d1">Make InkPort the EVM on-ramp<br />for the Substrate world.</h2>
          <div className="grid g3">
            <div className="card reveal d1"><h3>Coverage</h3><p>Extend the supported Solidity surface (true 256-bit ints, libraries, structs across the ABI) — each gated by an on-chain test.</p></div>
            <div className="card reveal d2"><h3>Playground</h3><p>Browser IDE: paste Solidity, compile to Rust, one-click deploy to Portaldot — zero local setup.</p></div>
            <div className="card reveal d3"><h3>Ecosystem</h3><p>Partner with Portaldot + Substrate chains to make InkPort the default EVM-developer on-ramp.</p></div>
          </div>
          <div className="row reveal d3"><span className="badge"><span className="bdot" />LIVE ON PORTALDOT MAINNET TODAY</span></div>
        </div>
      </section>

      {/* 12 — CLOSE */}
      <section className="slide dots">
        <div className="slide-inner">
          <div className="brand reveal"><div className="mark"><img src="/inkport.png" alt="InkPort" style={{ width: '100%', height: '100%', objectFit: 'contain', background: '#f4ecdb', borderRadius: 'inherit' }} /></div><span className="bname">InkPort</span></div>
          <div className="big reveal d1" style={{ marginTop: 28 }}>Write Solidity.<br /><span className="grad">Ship on Portaldot.</span></div>
          <p className="sub reveal d2">30 contracts live on mainnet. Real POT gas. Zero silent miscompiles.</p>
          <div className="row reveal d3">
            <span className="pill">wss://portaldot.philotheephilix.in</span>
            <span className="pill">github.com/freedanjeremiah/inkide</span>
          </div>
        </div>
        <div className="foot"><span>INKPORT</span><span>THE SOLIDITY → PORTALDOT COMPILER</span></div>
      </section>
    </main>
  );
}
