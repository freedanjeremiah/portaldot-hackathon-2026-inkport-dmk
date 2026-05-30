'use client';
import { useState, useEffect, useRef } from 'react';
import dynamic from 'next/dynamic';
import { translate } from '@/lib/translator';
import { buildCompile, buildDeploy, buildCall, stream } from '@/lib/simbackend';
import { Stepper, CompilePanel, DeployPanel, CallPanel, Ic } from '@/components/Pipeline';
import type { Metadata } from '@/lib/translator';
import type { LogLine, CallState } from '@/lib/simbackend';
import type { Statuses, CompileState, DeployState, CallPanelState } from '@/components/Pipeline';

const FauxEditor = dynamic(() => import('@/components/FauxEditor'), { ssr: false });

const DEFAULT_SOL = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// Counter — a minimal stateful contract for the InkPort playground.
/// Edit me: the seal0 Rust on the right regenerates as you type.
contract Counter {
    uint256 private count;

    constructor(uint256 initial) {
        count = initial;
    }

    function inc() public {
        count += 1;
    }

    function incBy(uint256 by) public {
        count += by;
    }

    function get() public view returns (uint256) {
        return count;
    }
}
`;

const EMPTY_META: Metadata = { name: 'contract', constructor: { args: [], argNames: [] }, messages: [], events: [] };

function StatusBadge({ statuses, translating, runningWhat }: { statuses: Statuses; translating: boolean; runningWhat: string | null }) {
  let cls = 'sb-idle', label = 'idle';
  if (translating) { cls = 'sb-work'; label = 'translating…'; }
  else if (runningWhat === 'compile') { cls = 'sb-work sb-pulse'; label = 'compiling…'; }
  else if (runningWhat === 'deploy') { cls = 'sb-work'; label = 'deploying…'; }
  else if (runningWhat === 'call') { cls = 'sb-work'; label = 'calling…'; }
  else if (statuses.compile === 'failed' || statuses.translate === 'failed' || statuses.deploy === 'failed') { cls = 'sb-red'; label = 'error'; }
  else if (statuses.deploy === 'done') { cls = 'sb-green'; label = 'deployed ✓'; }
  else if (statuses.compile === 'done') { cls = 'sb-blue'; label = 'compiled ✓'; }
  return <div className={'status-badge ' + cls}><span className="sb-dot" />{label}</div>;
}

export default function PlaygroundPage() {
  const [solidity, setSolidity] = useState(DEFAULT_SOL);
  const [rust, setRust] = useState('');
  const [metadata, setMetadata] = useState<Metadata | null>(null);
  const [translateError, setTranslateError] = useState<string | null>(null);
  const [translating, setTranslating] = useState(false);

  const [compile, setCompile] = useState<CompileState>({ lines: [], running: false, wasmSize: 0, error: null });
  const [deploy, setDeploy] = useState<DeployState>({ args: [], lines: [], running: false, address: null, error: null });
  const [call, setCall] = useState<CallPanelState>({ selected: 0, args: [], value: '', lines: [], running: false, result: null, events: [], error: null });
  const [counter, setCounter] = useState<CallState>({ count: 0 });

  const [active, setActive] = useState('compile');
  const [leftPct, setLeftPct] = useState(50);
  const [pipeH, setPipeH] = useState(330);

  const firstRef = useRef(true);
  const cancelRef = useRef<(() => void) | null>(null);

  /* live translate (debounced) */
  useEffect(() => {
    let cancelled = false;
    const delay = firstRef.current ? 30 : 600;
    setTranslating(true);
    const t = setTimeout(() => {
      if (cancelled) return;
      const t2 = setTimeout(() => {
        if (cancelled) return;
        const res = translate(solidity);
        if ('error' in res && res.error) {
          setTranslateError(res.error);
          setMetadata(null);
          setTranslating(false);
        } else if ('rust' in res) {
          setRust(res.rust!);
          setMetadata(res.metadata!);
          setTranslateError(null);
          setTranslating(false);
        }
        if (!firstRef.current) {
          setCompile(c => ({ ...c, lines: [], wasmSize: 0, error: null }));
          setDeploy(d => ({ ...d, lines: [], address: null, error: null }));
          setCall(c => ({ ...c, lines: [], result: null, events: [], error: null }));
          setActive('compile');
        }
        firstRef.current = false;
      }, 200 + Math.random() * 240);
      cancelRef.current = () => clearTimeout(t2);
    }, delay);
    return () => { cancelled = true; clearTimeout(t); };
  }, [solidity]);

  const meta = metadata || EMPTY_META;

  const statuses: Statuses = {
    translate: translating ? 'running' : translateError ? 'failed' : metadata ? 'done' : 'disabled',
    compile: compile.running ? 'running' : compile.error ? 'failed' : compile.wasmSize ? 'done' : (solidity.trim() ? 'ready' : 'disabled'),
    deploy: deploy.running ? 'running' : deploy.error ? 'failed' : deploy.address ? 'done' : (compile.wasmSize ? 'ready' : 'disabled'),
    call: call.running ? 'running' : call.error ? 'failed' : (call.result != null ? 'done' : (deploy.address ? 'ready' : 'disabled')),
  };
  const runningWhat = compile.running ? 'compile' : deploy.running ? 'deploy' : call.running ? 'call' : null;

  const onCompile = () => {
    if (compile.running) return;
    setActive('compile');
    const res = translate(solidity);
    if ('error' in res && res.error) {
      setCompile({ running: true, lines: [], wasmSize: 0, error: null });
      const fail: LogLine[] = [
        [['lg-cmd', '$ '], ['', 'inkport-translate contract.sol --target seal']],
        [['lg-dim', '  parsing … ']],
        [['lg-err', '  ' + res.error]],
        [['lg-err', '  ✗ translate failed — exit 1 (nothing built)']],
      ];
      stream(fail, {
        onLine: (ln: LogLine) => setCompile(c => ({ ...c, lines: [...c.lines, ln] })),
        onDone: () => setCompile(c => ({ ...c, running: false, error: res.error })),
      });
      return;
    }
    const built = buildCompile((res as { metadata: Metadata }).metadata.name);
    setCompile({ running: true, lines: [], wasmSize: 0, error: null });
    setMetadata((res as { metadata: Metadata }).metadata);
    cancelRef.current && cancelRef.current();
    cancelRef.current = stream(built.lines, {
      onLine: (ln: LogLine) => setCompile(c => ({ ...c, lines: [...c.lines, ln] })),
      onDone: () => { setCompile(c => ({ ...c, running: false, wasmSize: built.wasmSize })); setActive('deploy'); },
    });
  };

  const onDeploy = () => {
    if (deploy.running || !compile.wasmSize) return;
    setActive('deploy');
    const initial = parseInt((deploy.args[0] || '0'), 10);
    const built = buildDeploy(meta.name, deploy.args);
    setDeploy(d => ({ ...d, running: true, lines: [], address: null, error: null }));
    cancelRef.current = stream(built.lines, {
      onLine: (ln: LogLine) => setDeploy(d => ({ ...d, lines: [...d.lines, ln] })),
      onDone: () => {
        setDeploy(d => ({ ...d, running: false, address: built.address }));
        setCounter({ count: isNaN(initial) ? 0 : initial });
        setActive('call');
      },
    });
  };

  const onCall = () => {
    if (call.running || !deploy.address) return;
    const msg = meta.messages[call.selected];
    if (!msg) return;
    const built = buildCall(meta.name, msg, call.args, counter);
    setCall(c => ({ ...c, running: true, lines: [], result: null, events: [], error: null }));
    cancelRef.current = stream(built.lines, {
      onLine: (ln: LogLine) => setCall(c => ({ ...c, lines: [...c.lines, ln] })),
      onDone: () => {
        setCounter(built.newState || counter);
        setCall(c => ({ ...c, running: false, result: built.error ? null : built.result, events: built.events || [], error: built.error }));
      },
    });
  };

  const bus = {
    metadata: meta, statuses, compile, deploy, call,
    onCompile, onDeploy, onCall,
    setDeployArg: (i: number, v: string) => setDeploy(d => { const a = [...d.args]; a[i] = v; return { ...d, args: a }; }),
    setCallMsg: (i: number) => setCall(c => ({ ...c, selected: i, args: [], value: '', result: null, error: null, lines: [] })),
    setCallArg: (i: number, v: string) => setCall(c => { const a = [...c.args]; a[i] = v; return { ...c, args: a }; }),
    setCallValue: (v: string) => setCall(c => ({ ...c, value: v })),
    copy: (t: string) => { navigator.clipboard && navigator.clipboard.writeText(t); },
  };

  /* drag: editor split */
  const onDividerDown = (e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const startX = e.clientX, startPct = leftPct;
    const w = (e.target as HTMLElement).closest('.editors')!.clientWidth;
    const move = (ev: MouseEvent) => setLeftPct(Math.max(28, Math.min(72, startPct + ((ev.clientX - startX) / w) * 100)));
    const up = () => { document.removeEventListener('mousemove', move); document.removeEventListener('mouseup', up); };
    document.addEventListener('mousemove', move); document.addEventListener('mouseup', up);
  };

  const onPipeDown = (e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const startY = e.clientY, startH = pipeH;
    const move = (ev: MouseEvent) => setPipeH(Math.max(190, Math.min(560, startH - (ev.clientY - startY))));
    const up = () => { document.removeEventListener('mousemove', move); document.removeEventListener('mouseup', up); };
    document.addEventListener('mousemove', move); document.addEventListener('mouseup', up);
  };

  const ActivePanel = active === 'deploy' ? DeployPanel : active === 'call' ? CallPanel : CompilePanel;

  return (
    <div className="app">
      <div className="topbar">
        <div className="brand">
          <div className="brand-mark"><Ic.bolt width={13} height={13} /></div>
          <span className="brand-name">Ink<b>Port</b> Playground</span>
        </div>
        <span className="brand-sub">sol → seal0 → wasm</span>
        <div className="topbar-spacer" />
        <div className="node-pill"><span className="node-dot" />portaldot.philotheephilix.in</div>
        <StatusBadge statuses={statuses} translating={translating} runningWhat={runningWhat} />
      </div>

      <div className="editors">
        <div className="pane" style={{ width: leftPct + '%' }}>
          <div className="pane-header">
            <span className="pane-tab"><span className="dotfile">contracts/</span>{meta.name}.sol</span>
            <span className="lang-chip">solidity</span>
            <div className="ph-right"><span className="ro-tag">editable</span></div>
          </div>
          <FauxEditor value={solidity} onChange={setSolidity} language="solidity" readOnly={false} />
        </div>

        <div className="divider" onMouseDown={onDividerDown} />

        <div className="pane" style={{ width: (100 - leftPct) + '%', position: 'relative' }}>
          <div className="pane-header">
            <span className="pane-tab"><span className="dotfile">build/{meta.name}/src/</span>lib.rs</span>
            <span className="lang-chip">rust · seal0</span>
            <div className="ph-right">
              {translating
                ? <span className="translate-flag"><span className="mini-spinner" />translating</span>
                : <span className="ro-tag">read-only · live</span>}
            </div>
          </div>
          <div style={{ position: 'relative', flex: 1, minHeight: 0, display: 'flex' }}>
            <FauxEditor value={rust} onChange={() => {}} language="rust" readOnly={true} />
            {translateError && (
              <div className="rust-error">
                <div className="re-head"><Ic.cross width={13} height={13} /> translate error — Rust output is stale</div>
                {translateError}
                {'\n\n'}<span style={{ color: 'var(--text-faint)' }}>// editor is never blocked — fix the Solidity to regenerate</span>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="pipeline" style={{ height: pipeH }}>
        <div className="pipeline-resize" onMouseDown={onPipeDown} />
        <Stepper statuses={statuses} active={active} onSelect={setActive} />
        <ActivePanel bus={bus} />
      </div>
    </div>
  );
}
