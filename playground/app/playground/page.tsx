'use client';
import { useState, useEffect, useRef } from 'react';
import { translate } from '@/lib/translator';
import { readSSE } from '@/lib/sse';
import type { SSEPayload } from '@/lib/sse';
import FauxEditor from '@/components/FauxEditor';
import { PipelineSide, Ic } from '@/components/Pipeline';
import type { LogLine } from '@/components/Pipeline';
import type { Metadata } from '@/lib/translator';
import type { Statuses, CompileState, DeployState, CallPanelState } from '@/components/Pipeline';

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
  const [wasmB64, setWasmB64] = useState<string | null>(null);

  const [active, setActive] = useState('compile');

  const firstRef = useRef(true);
  const sessionIdRef = useRef<string>('');

  useEffect(() => {
    if (!sessionIdRef.current) {
      const stored = sessionStorage.getItem('inkport-session-id');
      const id = stored ?? crypto.randomUUID();
      if (!stored) sessionStorage.setItem('inkport-session-id', id);
      sessionIdRef.current = id;
    }
  }, []);

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

  const onCompile = async () => {
    if (compile.running) return;
    setActive('compile');
    setCompile({ lines: [], running: true, wasmSize: 0, error: null });
    setWasmB64(null);

    try {
      await readSSE(
        '/api/compile',
        { solidity, sessionId: sessionIdRef.current },
        {
          onLine: (payload: SSEPayload) => {
            const seg: LogLine = [[payload.cls ?? 'lg-dim', payload.text ?? '']];
            setCompile(c => ({ ...c, lines: [...c.lines, seg] }));
          },
          onDone: (payload: SSEPayload) => {
            if (payload.type === 'wasm') {
              const wasm = payload.data as string;
              const metaPayload = payload.metadata as Metadata;
              const sz = (payload.size as number) ?? 0;
              setWasmB64(wasm);
              setMetadata(metaPayload);
              setCompile(c => ({ ...c, running: false, wasmSize: sz }));
              // Pre-fill deploy args with type-appropriate defaults so the
              // user can click Deploy immediately without hitting an arg-count error.
              const ctorArgs = metaPayload.constructor?.args ?? [];
              const defaultFor = (t: string) =>
                t === 'bool' ? 'false' : t === 'address' ? '//Alice' : t === 'string' ? '' : '0';
              setDeploy(d => ({ ...d, args: ctorArgs.map(defaultFor) }));
              setActive('deploy');
            }
          },
          onError: (payload: SSEPayload) => {
            setCompile(c => ({ ...c, running: false, error: (payload.log as string) ?? 'compile failed' }));
          },
        }
      );
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setCompile(c => ({ ...c, running: false, error: msg }));
    }
  };

  const onDeploy = async () => {
    if (deploy.running || !wasmB64) return;
    setActive('deploy');
    setDeploy(d => ({ ...d, running: true, lines: [], address: null, error: null }));

    try {
      await readSSE(
        '/api/deploy',
        { wasmB64, metadata: meta, args: deploy.args, sessionId: sessionIdRef.current },
        {
          onLine: (payload: SSEPayload) => {
            const seg: LogLine = [[payload.cls ?? 'lg-dim', payload.text ?? '']];
            setDeploy(d => ({ ...d, lines: [...d.lines, seg] }));
          },
          onDone: (payload: SSEPayload) => {
            if (payload.type === 'address') {
              setDeploy(d => ({ ...d, running: false, address: payload.address as string }));
              setActive('call');
            }
          },
          onError: (payload: SSEPayload) => {
            setDeploy(d => ({ ...d, running: false, error: (payload.log as string) ?? 'deploy failed' }));
          },
        }
      );
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setDeploy(d => ({ ...d, running: false, error: msg }));
    }
  };

  const onCall = async () => {
    if (call.running || !deploy.address) return;
    const msg = meta.messages[call.selected];
    if (!msg) return;

    setCall(c => ({ ...c, running: true, lines: [], result: null, events: [], error: null }));

    try {
      const response = await fetch('/api/call', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          metadata: meta,
          message: msg.name,
          args: call.args,
          sessionId: sessionIdRef.current,
        }),
      });

      const data = await response.json() as { result?: unknown; events?: unknown[]; error?: string };

      if (!response.ok || data.error) {
        setCall(c => ({ ...c, running: false, error: data.error ?? 'call failed' }));
      } else {
        setCall(c => ({
          ...c,
          running: false,
          result: String(data.result ?? 'ok'),
          events: data.events ?? [],
          error: null,
        }));
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setCall(c => ({ ...c, running: false, error: msg }));
    }
  };

  const bus = {
    metadata: meta, statuses, compile, deploy, call,
    onCompile, onDeploy, onCall,
    setDeployArg: (i: number, v: string) => setDeploy(d => { const a = [...d.args]; a[i] = v; return { ...d, args: a }; }),
    setCallMsg: (i: number) => setCall(c => ({ ...c, selected: i, args: [], value: '', result: null, error: null, lines: [] })),
    setCallArg: (i: number, v: string) => setCall(c => { const a = [...c.args]; a[i] = v; return { ...c, args: a }; }),
    setCallValue: (v: string) => setCall(c => ({ ...c, value: v })),
    copy: (t: string) => { navigator.clipboard?.writeText(t); },
  };

  return (
    <div className="app">
      <svg width="0" height="0" style={{ position: 'absolute', width: 0, height: 0 }}>
        <defs>
          <filter id="burn" x="-20%" y="-20%" width="140%" height="140%">
            <feTurbulence type="fractalNoise" baseFrequency="0.04 0.09" numOctaves="3" seed="11" result="n"/>
            <feDisplacementMap in="SourceGraphic" in2="n" scale="8" xChannelSelector="R" yChannelSelector="G"/>
          </filter>
        </defs>
      </svg>

      <div className="pg-vignette" />

      <div className="topbar">
        <a href="/" className="topbar-back">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M15 6l-6 6 6 6"/>
          </svg>
          InkPort
        </a>
        <div className="topbar-sep" />
        <div className="brand">
          <div className="brand-mark"><Ic.bolt width={13} height={13} /></div>
          <span className="brand-name">Ink<b>Port</b> Playground</span>
        </div>
        <span className="brand-sub">sol → seal0 → wasm</span>
        <div className="topbar-spacer" />
        <div className="node-pill"><span className="node-dot" />portaldot.philotheephilix.in</div>
        <StatusBadge statuses={statuses} translating={translating} runningWhat={runningWhat} />
      </div>

      <div className="workspace">
        <div className="pane">
          <div className="pane-header">
            <span className="pane-tab"><span className="dotfile">contracts/</span>{meta.name}.sol</span>
            <span className="lang-chip">solidity</span>
            <div className="ph-right"><span className="ro-tag">editable</span></div>
          </div>
          <FauxEditor value={solidity} onChange={setSolidity} language="solidity" readOnly={false} />
        </div>

        <div className="pane" style={{ position: 'relative' }}>
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

        <PipelineSide bus={bus} active={active} onSelect={setActive} />
      </div>
    </div>
  );
}
