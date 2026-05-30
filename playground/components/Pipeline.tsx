'use client';
import React, { useRef, useEffect, useState } from 'react';
import type { Metadata, MetadataMessage } from '@/lib/translator';
export type LogSegment = [string, string];
export type LogLine = LogSegment[];

/* ---------- icons ---------- */
const Ic = {
  translate: (p: React.SVGProps<SVGSVGElement>) => (<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...p}><path d="M4 7h11M9 4l-5 3 5 3M20 17H9m6-3l5 3-5 3" strokeLinecap="round" strokeLinejoin="round"/></svg>),
  compile: (p: React.SVGProps<SVGSVGElement>) => (<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...p}><path d="M12 2l8 4.5v9L12 20l-8-4.5v-9L12 2z" strokeLinejoin="round"/><path d="M12 11l8-4.5M12 11v9M12 11L4 6.5" strokeLinejoin="round"/></svg>),
  deploy: (p: React.SVGProps<SVGSVGElement>) => (<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...p}><path d="M12 3c3 1.5 5 4.5 5 8 0 2-1 4-2 5H9c-1-1-2-3-2-5 0-3.5 2-6.5 5-8z" strokeLinejoin="round"/><circle cx="12" cy="10" r="1.6"/><path d="M9 16l-2 4 3-1.5M15 16l2 4-3-1.5" strokeLinejoin="round"/></svg>),
  call: (p: React.SVGProps<SVGSVGElement>) => (<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...p}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4" strokeLinecap="round" strokeLinejoin="round"/></svg>),
  check: (p: React.SVGProps<SVGSVGElement>) => (<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" {...p}><path d="M5 12l5 5L19 7" strokeLinecap="round" strokeLinejoin="round"/></svg>),
  cross: (p: React.SVGProps<SVGSVGElement>) => (<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" {...p}><path d="M6 6l12 12M18 6L6 18" strokeLinecap="round"/></svg>),
  play: (p: React.SVGProps<SVGSVGElement>) => (<svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" {...p}><path d="M7 4l13 8-13 8V4z"/></svg>),
  copy: (p: React.SVGProps<SVGSVGElement>) => (<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...p}><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15V5a2 2 0 012-2h8"/></svg>),
  chevron: (p: React.SVGProps<SVGSVGElement>) => (<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" {...p}><path d="M6 9l6 6 6-6" strokeLinecap="round" strokeLinejoin="round"/></svg>),
  bolt: (p: React.SVGProps<SVGSVGElement>) => (<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" {...p}><path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" strokeLinejoin="round"/></svg>),
};

export { Ic };

export type StageStatus = 'disabled' | 'ready' | 'running' | 'done' | 'failed';

export interface Statuses {
  translate: StageStatus;
  compile: StageStatus;
  deploy: StageStatus;
  call: StageStatus;
}

const STAGE_DEFS = [
  { key: 'translate', name: 'Translate', icon: Ic.translate, sub: 'solang → IR → seal0' },
  { key: 'compile',   name: 'Compile',   icon: Ic.compile,   sub: 'cargo build → wasm' },
  { key: 'deploy',    name: 'Deploy',    icon: Ic.deploy,    sub: 'instantiate_with_code' },
  { key: 'call',      name: 'Call',      icon: Ic.call,      sub: 'Contracts.call' },
] as const;

function stateLabel(status: StageStatus, key: string): string {
  if (key === 'translate') {
    if (status === 'running') return 'translating…';
    if (status === 'done') return 'live';
    if (status === 'failed') return 'parse error';
    return 'idle';
  }
  return { disabled: 'locked', ready: 'ready', running: 'running…', done: 'done', failed: 'failed' }[status] || 'idle';
}

export function Stepper({ statuses, active, onSelect }: { statuses: Statuses; active: string; onSelect: (k: string) => void }) {
  return (
    <div className="stepper">
      {STAGE_DEFS.map((s, i) => {
        const st = statuses[s.key as keyof Statuses];
        const clickable = s.key !== 'translate' && st !== 'disabled' && st !== 'running';
        const isActive = active === s.key;
        const cls = ['step', st, clickable ? 'clickable' : '', isActive ? 'active' : ''].join(' ');
        const NodeIcon = s.icon;
        return (
          <span key={s.key} style={{ display: 'contents' }}>
            <div className={cls} onClick={() => clickable && onSelect(s.key)}>
              <div className="step-node">
                {st === 'done' ? <Ic.check /> : st === 'failed' ? <Ic.cross /> : <NodeIcon />}
              </div>
              <div className="step-meta">
                <div className="step-name">{s.name}</div>
                <div className="step-state">{stateLabel(st, s.key)}</div>
              </div>
            </div>
            {i < STAGE_DEFS.length - 1 && (
              <div className={['step-conn', statuses[s.key as keyof Statuses] === 'done' ? 'filled' : '', statuses[STAGE_DEFS[i + 1].key as keyof Statuses] === 'running' ? 'flowing' : ''].join(' ')} />
            )}
          </span>
        );
      })}
    </div>
  );
}

export function LogConsole({ lines, running, emptyText }: { lines: LogLine[]; running: boolean; emptyText?: React.ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => { if (ref.current) ref.current.scrollTop = ref.current.scrollHeight; }, [lines, running]);
  if (!lines.length && !running) {
    return <div className="log-empty">{emptyText || '— no output yet —'}</div>;
  }
  return (
    <div className="log" ref={ref}>
      {lines.map((segs, i) => (
        <div className="log-line" key={i}>
          {segs.map((seg, j) => <span key={j} className={seg[0]}>{seg[1]}</span>)}
        </div>
      ))}
      {running && <div className="log-line"><span className="cursor-blink" /></div>}
    </div>
  );
}

function ArgInput({ name, type, value, onChange }: { name: string; type: string; value: string; onChange: (v: string) => void }) {
  function phFor(t: string): string {
    if (/u128|uint/.test(t)) return '0';
    if (/i128|int/.test(t)) return '-0';
    if (t === 'bool') return 'true';
    if (t === 'address') return '//Alice  or  5Grw…';
    if (t === 'string') return '"hello"';
    return '';
  }
  return (
    <div className="arg-row">
      <label className="arg-label"><span>{name || 'arg'}</span><span className="at">: {type}</span></label>
      <input className="arg-input" value={value || ''} placeholder={phFor(type)} onChange={e => onChange(e.target.value)} />
    </div>
  );
}

function MsgBadge({ m }: { m: MetadataMessage }) {
  if (m.payable) return <span className="badge badge-pay">payable</span>;
  if (m.mutates) return <span className="badge badge-mut">mutates</span>;
  return <span className="badge badge-view">view</span>;
}

export interface CompileState { lines: LogLine[]; running: boolean; wasmSize: number; error: string | null }
export interface DeployState { args: string[]; lines: LogLine[]; running: boolean; address: string | null; error: string | null }
export interface CallPanelState { selected: number; args: string[]; value: string; lines: LogLine[]; running: boolean; result: string | null; events: unknown[]; error: string | null }

export interface Bus {
  metadata: Metadata;
  statuses: Statuses;
  compile: CompileState;
  deploy: DeployState;
  call: CallPanelState;
  onCompile: () => void;
  onDeploy: () => void;
  onCall: () => void;
  setDeployArg: (i: number, v: string) => void;
  setCallMsg: (i: number) => void;
  setCallArg: (i: number, v: string) => void;
  setCallValue: (v: string) => void;
  copy: (t: string) => void;
}

export function CompilePanel({ bus }: { bus: Bus }) {
  const c = bus.compile;
  return (
    <div className="panel-stage">
      <div className="panel-head">
        <span className="ph-title">Build log</span>
        <span className="ph-meta">{c.wasmSize ? `${bus.metadata.name}.wasm · ${c.wasmSize.toLocaleString()} bytes` : 'cargo +stable build --release --target wasm32-unknown-unknown'}</span>
      </div>
      <div className="panel-body">
        <div className="stage-actions" style={{ marginTop: 0, marginBottom: 14 }}>
          <button className={'btn btn-primary' + (c.running ? ' disabled' : '')} onClick={bus.onCompile}>
            <Ic.compile width={14} height={14} /> {c.running ? 'Compiling…' : c.wasmSize ? 'Recompile' : 'Compile'}
          </button>
          {c.wasmSize > 0 && !c.running && (
            <span className="field-note" style={{ color: 'var(--green)' }}>✓ {bus.metadata.name}.wasm — {c.wasmSize.toLocaleString()} bytes stripped</span>
          )}
          {c.error && !c.running && <span className="field-note" style={{ color: 'var(--red)' }}>✗ build failed — see log</span>}
        </div>
        <LogConsole lines={c.lines} running={c.running}
          emptyText={<span>Press <span className="kbd">Compile</span> to run translate + cargo build + strip → Portaldot wasm</span>} />
      </div>
    </div>
  );
}

export function DeployPanel({ bus }: { bus: Bus }) {
  const d = bus.deploy, meta = bus.metadata;
  const args = meta.constructor.args || [];
  return (
    <div className="panel-stage">
      <div className="panel-head">
        <span className="ph-title">Deploy · constructor</span>
        <span className="ph-meta">{bus.statuses.compile === 'done' ? `${bus.compile.wasmSize.toLocaleString()} bytes ready` : 'compile first'}</span>
      </div>
      <div className="panel-body">
        {bus.statuses.deploy === 'disabled' ? (
          <div className="empty-hint">
            <div className="eh-ic"><Ic.deploy width={26} height={26} /></div>
            <div className="eh-t">Compile the contract to unlock deploy.</div>
            <div className="eh-s">wasm + metadata must exist in session state</div>
          </div>
        ) : (
          <>
            <div className="arg-grid">
              {args.length === 0 && <div className="field-note">constructor() — no arguments</div>}
              {args.map((t, i) => (
                <ArgInput key={i} type={t} name={meta.constructor.argNames?.[i] || `arg${i}`} value={d.args[i] || ''} onChange={v => bus.setDeployArg(i, v)} />
              ))}
            </div>
            <div className="stage-actions">
              <button className={'btn btn-primary' + (d.running ? ' disabled' : '')} onClick={bus.onDeploy}>
                <Ic.play /> {d.running ? 'Deploying…' : 'Deploy to Portaldot'}
              </button>
              <span className="field-note">endowment 10 POT · signer //Alice</span>
            </div>
            {(d.lines.length > 0 || d.running) && (
              <div className="result-card">
                <div className="rc-head">deploy stream</div>
                <div className="rc-body" style={{ maxHeight: 150, overflow: 'auto' }}>
                  <LogConsole lines={d.lines} running={d.running} />
                </div>
              </div>
            )}
            {d.address && (
              <div className="result-card">
                <div className="rc-head"><Ic.check width={13} height={13} /> contract address</div>
                <div className="rc-body">
                  <span className="addr-chip" onClick={() => bus.copy(d.address!)} title="copy">
                    {d.address}
                    <span className="copy-ic"><Ic.copy /></span>
                  </span>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ─── PipelineRail (Remix-style right sidebar) ───────────────────────────────

function RailLog({ lines, running }: { lines: LogLine[]; running: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines, running]);
  if (!lines.length && !running) return null;
  return (
    <div className="stage-log" ref={ref}>
      {lines.map((segs, i) => (
        <div className="log-line" key={i}>
          {segs.map((seg, j) => <span key={j} className={seg[0]}>{seg[1]}</span>)}
        </div>
      ))}
      {running && <div className="log-line"><span className="cursor-blink" /></div>}
    </div>
  );
}

function stageCardCls(status: StageStatus) {
  const map: Record<StageStatus, string> = {
    disabled: 'st-disabled', ready: 'st-ready',
    running: 'st-running', done: 'st-done', failed: 'st-failed',
  };
  return `stage-card ${map[status] ?? ''}`;
}

function pillCls(status: StageStatus) {
  const map: Partial<Record<StageStatus, string>> = {
    running: 'sp-running', done: 'sp-done', failed: 'sp-failed', ready: 'sp-ready',
  };
  return `stage-pill ${map[status] ?? ''}`;
}

const PILL_LABELS: Record<StageStatus, string> = {
  disabled: 'locked', ready: 'ready', running: 'running', done: 'done', failed: 'failed',
};

function CallRailBody({ bus }: { bus: Bus }) {
  const [open, setOpen] = useState(false);
  const cl = bus.call, meta = bus.metadata;
  const msgs = meta.messages || [];
  const sel = msgs[cl.selected] ?? null;

  return (
    <>
      <div className="rail-msg-select">
        <button className="rail-msg-btn" onClick={() => setOpen(o => !o)}>
          {sel ? (
            <>
              <span className="rail-msg-name">{sel.name}</span>
              <span className="rail-msg-sig">({sel.args.join(', ')}){sel.ret ? ' → ' + sel.ret : ''}</span>
              <MsgBadge m={sel} />
              <Ic.chevron style={{ marginLeft: 'auto' }} />
            </>
          ) : (
            <span style={{ color: 'var(--text-faint)', fontSize: 11, fontFamily: 'var(--mono)' }}>select a message…</span>
          )}
        </button>
        {open && (
          <div className="rail-dropdown">
            {msgs.map((m, i) => (
              <div key={i} className={`rail-msg-opt${i === cl.selected ? ' sel' : ''}`}
                onClick={() => { bus.setCallMsg(i); setOpen(false); }}>
                <span className="rail-msg-name">{m.name}</span>
                <span className="rail-msg-sig">({m.args.join(', ')}){m.ret ? ' → ' + m.ret : ''}</span>
                <MsgBadge m={m} />
              </div>
            ))}
          </div>
        )}
      </div>

      {sel && sel.args.map((t, i) => (
        <div key={i} className="rail-arg-row">
          <label className="rail-arg-label">
            <span>{sel.argNames?.[i] || `arg${i}`}</span>
            <span className="at">: {t}</span>
          </label>
          <input className="arg-input" value={cl.args[i] || ''}
            placeholder={t === 'address' ? '//Alice  or  5Grw…' : '0'}
            onChange={e => bus.setCallArg(i, e.target.value)} />
        </div>
      ))}
      {sel?.payable && (
        <div className="rail-arg-row">
          <label className="rail-arg-label">
            <span>value</span><span className="at">: POT</span>
            <span className="badge badge-pay">payable</span>
          </label>
          <input className="arg-input" value={cl.value || ''} placeholder="0.0"
            onChange={e => bus.setCallValue(e.target.value)} />
        </div>
      )}

      {sel && (
        <button
          className={`stage-btn ${sel.mutates ? 'sb-blue' : 'sb-green'}${cl.running ? ' sb-disabled' : ''}`}
          onClick={bus.onCall} disabled={cl.running}
        >
          <Ic.play />
          {cl.running ? '…' : sel.mutates ? 'Send Transaction' : 'Read'}
          <span style={{ marginLeft: 'auto', fontFamily: 'var(--mono)', fontSize: 9, opacity: 0.6 }}>{sel.selector}</span>
        </button>
      )}

      <RailLog lines={cl.lines} running={cl.running} />

      {cl.result != null && !cl.error && (
        <div className="rail-result">
          <div className="rail-result-head"><Ic.check width={12} height={12} />{sel?.mutates ? 'result' : 'returned'}</div>
          <div className="rail-result-body">
            <span className="ret-value">{cl.result}</span>
            {sel?.ret && <span className="ret-type">: {sel.ret}</span>}
          </div>
        </div>
      )}
      {cl.error && (
        <div className="rail-result" style={{ borderColor: 'var(--red)' }}>
          <div className="rail-result-head" style={{ color: 'var(--red)' }}><Ic.cross width={12} height={12} />reverted</div>
          <div className="rail-result-body" style={{ color: '#ffb4b4', fontSize: 11 }}>{cl.error}</div>
        </div>
      )}
    </>
  );
}

export function PipelineRail({ bus, active, onSelect, translateError }: {
  bus: Bus;
  active: string;
  onSelect: (k: string) => void;
  translateError: string | null;
}) {
  return (
    <div className="pipe-rail">
      <div className="rail-header">
        <Ic.bolt width={14} height={14} />
        Pipeline
      </div>

      {/* 1 — Translate */}
      <div className={stageCardCls(bus.statuses.translate)}>
        <div className="stage-hd" style={{ cursor: 'pointer' }} onClick={() => onSelect('translate')}>
          <div className="stage-ic"><Ic.translate /></div>
          <div className="stage-info">
            <div className="stage-name">Translate</div>
            <div className="stage-sub">solang → IR → seal0</div>
          </div>
          <span className={pillCls(bus.statuses.translate)}>{PILL_LABELS[bus.statuses.translate]}</span>
        </div>
        {(active === 'translate' || !!translateError) && (
          <div className="stage-body" style={{ paddingTop: 12 }}>
            {translateError ? (
              <div className="translate-note has-error">
                <span className="tn-label">translate error</span>
                {translateError}
              </div>
            ) : (
              <div className="translate-note">
                {bus.statuses.translate === 'running'
                  ? 'Translating Solidity → seal0 Rust…'
                  : 'AST → IR → seal0 Rust — runs live as you type.'}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 2 — Compile */}
      <div className={stageCardCls(bus.statuses.compile)}>
        <div className="stage-hd" style={{ cursor: 'pointer' }} onClick={() => onSelect('compile')}>
          <div className="stage-ic"><Ic.compile /></div>
          <div className="stage-info">
            <div className="stage-name">Compile</div>
            <div className="stage-sub">cargo build → wasm</div>
          </div>
          <span className={pillCls(bus.statuses.compile)}>{PILL_LABELS[bus.statuses.compile]}</span>
        </div>
        {active === 'compile' && (
          <div className="stage-body" style={{ paddingTop: 12 }}>
            <button
              className={`stage-btn sb-blue${bus.compile.running ? ' sb-disabled' : ''}`}
              onClick={bus.onCompile} disabled={bus.compile.running}
            >
              <Ic.compile width={13} height={13} />
              {bus.compile.running ? 'Compiling…' : bus.compile.wasmSize ? 'Recompile' : 'Compile'}
              {bus.compile.wasmSize > 0 && !bus.compile.running && (
                <span style={{ marginLeft: 'auto', fontFamily: 'var(--mono)', fontSize: 9.5, color: 'var(--green-2)', fontWeight: 400 }}>
                  {bus.metadata.name}.wasm · {bus.compile.wasmSize.toLocaleString()} B
                </span>
              )}
            </button>
            <RailLog lines={bus.compile.lines} running={bus.compile.running} />
            {bus.compile.error && !bus.compile.running && (
              <div className="translate-note has-error">{bus.compile.error}</div>
            )}
          </div>
        )}
      </div>

      {/* 3 — Deploy */}
      <div className={stageCardCls(bus.statuses.deploy)}>
        <div
          className="stage-hd"
          style={{ cursor: bus.statuses.deploy !== 'disabled' ? 'pointer' : 'default' }}
          onClick={() => bus.statuses.deploy !== 'disabled' && onSelect('deploy')}
        >
          <div className="stage-ic"><Ic.deploy /></div>
          <div className="stage-info">
            <div className="stage-name">Deploy</div>
            <div className="stage-sub">instantiate_with_code</div>
          </div>
          <span className={pillCls(bus.statuses.deploy)}>{PILL_LABELS[bus.statuses.deploy]}</span>
        </div>
        {active === 'deploy' && bus.statuses.deploy !== 'disabled' && (
          <div className="stage-body" style={{ paddingTop: 12 }}>
            {(bus.metadata.constructor.args ?? []).length === 0 && (
              <div className="translate-note">constructor() — no arguments</div>
            )}
            {(bus.metadata.constructor.args ?? []).map((t, i) => (
              <div key={i} className="rail-arg-row">
                <label className="rail-arg-label">
                  <span>{bus.metadata.constructor.argNames?.[i] || `arg${i}`}</span>
                  <span className="at">: {t}</span>
                </label>
                <input className="arg-input" value={bus.deploy.args[i] || ''}
                  placeholder={t === 'address' ? '//Alice  or  5Grw…' : '0'}
                  onChange={e => bus.setDeployArg(i, e.target.value)} />
              </div>
            ))}
            <button
              className={`stage-btn sb-blue${bus.deploy.running ? ' sb-disabled' : ''}`}
              onClick={bus.onDeploy} disabled={bus.deploy.running}
            >
              <Ic.play /> {bus.deploy.running ? 'Deploying…' : 'Deploy to Portaldot'}
            </button>
            <RailLog lines={bus.deploy.lines} running={bus.deploy.running} />
            {bus.deploy.address && (
              <div className="rail-result">
                <div className="rail-result-head"><Ic.check width={12} height={12} />Contract address</div>
                <div className="rail-result-body">
                  <span className="addr-chip" onClick={() => bus.copy(bus.deploy.address!)}>
                    {bus.deploy.address}<span className="copy-ic"><Ic.copy /></span>
                  </span>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* 4 — Call */}
      <div className={stageCardCls(bus.statuses.call)}>
        <div
          className="stage-hd"
          style={{ cursor: bus.statuses.call !== 'disabled' ? 'pointer' : 'default' }}
          onClick={() => bus.statuses.call !== 'disabled' && onSelect('call')}
        >
          <div className="stage-ic"><Ic.call /></div>
          <div className="stage-info">
            <div className="stage-name">Call</div>
            <div className="stage-sub">Contracts.call · dry-run</div>
          </div>
          <span className={pillCls(bus.statuses.call)}>{PILL_LABELS[bus.statuses.call]}</span>
        </div>
        {active === 'call' && bus.statuses.call !== 'disabled' && (
          <div className="stage-body" style={{ paddingTop: 12 }}>
            <CallRailBody bus={bus} />
          </div>
        )}
      </div>
    </div>
  );
}

// ─── PipelineSide (Forge / parchment accordion sidebar) ──────────────────────

const STAGE_DEFS_PS = [
  { key: 'translate', name: 'Translate', sub: 'solang → IR → seal0' },
  { key: 'compile',   name: 'Compile',   sub: 'cargo build → wasm' },
  { key: 'deploy',    name: 'Deploy',    sub: 'instantiate_with_code' },
  { key: 'call',      name: 'Call',      sub: 'Contracts.call' },
] as const;

const PS_ICONS: Record<string, (p: React.SVGProps<SVGSVGElement>) => JSX.Element> = {
  translate: Ic.translate,
  compile:   Ic.compile,
  deploy:    Ic.deploy,
  call:      Ic.call,
};

function psLabel(status: StageStatus, key: string): string {
  if (key === 'translate') {
    return ({ running: 'translating…', done: 'live', failed: 'parse error', disabled: 'idle' } as Record<string, string>)[status] ?? 'idle';
  }
  return ({ disabled: 'locked', ready: 'ready', running: 'running…', done: 'done', failed: 'failed' } as Record<string, string>)[status] ?? 'idle';
}

export function PipelineSide({ bus, active, onSelect }: {
  bus: Bus;
  active: string;
  onSelect: (k: string) => void;
}) {
  const statuses = bus.statuses;
  const Panel = active === 'deploy' ? DeployPanel : active === 'call' ? CallPanel : CompilePanel;

  return (
    <aside className="pipeline-side">
      <div className="ps-head">
        <Ic.bolt width={15} height={15} />
        Pipeline
      </div>
      <div className="ps-stages">
        {STAGE_DEFS_PS.map((s, i) => {
          const st = statuses[s.key as keyof Statuses];
          const clickable = s.key !== 'translate' && st !== 'disabled' && st !== 'running';
          const isActive = active === s.key;
          const NodeIcon = PS_ICONS[s.key] ?? Ic.compile;
          const cls = ['ps-stage', st, isActive ? 'active' : '', clickable ? 'clickable' : ''].filter(Boolean).join(' ');
          return (
            <React.Fragment key={s.key}>
              <div className={cls}>
                <div className="ps-stage-row" onClick={() => clickable && onSelect(s.key)}>
                  <div className="ps-node">
                    {st === 'done' ? <Ic.check /> : st === 'failed' ? <Ic.cross /> : <NodeIcon />}
                  </div>
                  <div className="ps-meta">
                    <div className="ps-name">{s.name}</div>
                    <div className="ps-sub">{s.sub}</div>
                  </div>
                  <div className={`ps-badge b-${st}`}>{psLabel(st, s.key)}</div>
                </div>
                {isActive && s.key !== 'translate' && (
                  <div className="ps-panel"><Panel bus={bus} /></div>
                )}
              </div>
              {i < STAGE_DEFS_PS.length - 1 && (
                <div className={`ps-conn${statuses[s.key as keyof Statuses] === 'done' ? ' filled' : ''}`} />
              )}
            </React.Fragment>
          );
        })}
      </div>
    </aside>
  );
}

// ─── Legacy panels (kept for backwards compat) ───────────────────────────────
export function CallPanel({ bus }: { bus: Bus }) {
  const cl = bus.call, meta = bus.metadata;
  const [open, setOpen] = useState(false);
  const msgs = meta.messages || [];
  const sel = msgs[cl.selected] || null;

  return (
    <div className="panel-stage">
      <div className="panel-head">
        <span className="ph-title">Call · message</span>
        <span className="ph-meta">{bus.statuses.deploy === 'done' ? (bus.deploy.address?.slice(0, 10) + '…') : 'deploy first'}</span>
      </div>
      <div className="panel-body">
        {bus.statuses.call === 'disabled' ? (
          <div className="empty-hint">
            <div className="eh-ic"><Ic.call width={26} height={26} /></div>
            <div className="eh-t">Deploy the contract to start calling messages.</div>
            <div className="eh-s">messages are read from metadata.json</div>
          </div>
        ) : (
          <>
            <div className="msg-select-wrap">
              <div className="msg-select" onClick={() => setOpen(!open)}>
                {sel ? (
                  <>
                    <span className="msg-name">{sel.name}</span>
                    <span className="msg-sig">({sel.args.join(', ')}){sel.ret ? ' → ' + sel.ret : ''}</span>
                    <MsgBadge m={sel} />
                    <span style={{ marginLeft: 'auto' }}><Ic.chevron /></span>
                  </>
                ) : <span className="msg-sig">select a message…</span>}
              </div>
              {open && (
                <div className="msg-dropdown">
                  {msgs.map((m, i) => (
                    <div key={i} className={'msg-opt' + (i === cl.selected ? ' sel' : '')} onClick={() => { bus.setCallMsg(i); setOpen(false); }}>
                      <span className="msg-name">{m.name}</span>
                      <span className="msg-sig">({m.args.join(', ')}){m.ret ? ' → ' + m.ret : ''}</span>
                      <MsgBadge m={m} />
                    </div>
                  ))}
                </div>
              )}
            </div>

            {sel && (
              <div className="arg-grid" style={{ marginTop: 14 }}>
                {sel.args.length === 0 && <div className="field-note">{sel.name}() — no arguments</div>}
                {sel.args.map((t, i) => (
                  <ArgInput key={i} type={t} name={sel.argNames?.[i] || `arg${i}`} value={cl.args[i] || ''} onChange={v => bus.setCallArg(i, v)} />
                ))}
                {sel.payable && (
                  <div className="arg-row">
                    <label className="arg-label"><span>value</span><span className="at">: POT</span><span className="badge badge-pay">payable</span></label>
                    <input className="arg-input" value={cl.value || ''} placeholder="0.0" onChange={e => bus.setCallValue(e.target.value)} />
                  </div>
                )}
              </div>
            )}

            {sel && (
              <div className="stage-actions">
                <button className={'btn ' + (sel.mutates ? 'btn-primary' : 'btn-green') + (cl.running ? ' disabled' : '')} onClick={bus.onCall}>
                  <Ic.play /> {cl.running ? '…' : sel.mutates ? 'Send' : 'Read'}
                </button>
                <span className="selector-tag">selector {sel.selector}</span>
              </div>
            )}

            {(cl.lines.length > 0 || cl.running) && (
              <div className="result-card">
                <div className="rc-head">call stream</div>
                <div className="rc-body" style={{ maxHeight: 130, overflow: 'auto' }}>
                  <LogConsole lines={cl.lines} running={cl.running} />
                </div>
              </div>
            )}

            {cl.error && (
              <div className="result-card" style={{ borderColor: 'var(--red)' }}>
                <div className="rc-head" style={{ color: 'var(--red)' }}><Ic.cross width={13} height={13} /> reverted</div>
                <div className="rc-body" style={{ color: '#ffb4b4' }}>{cl.error}</div>
              </div>
            )}

            {cl.result != null && !cl.error && (
              <div className="result-card">
                <div className="rc-head"><Ic.check width={13} height={13} /> {sel && sel.mutates ? 'result' : 'returned'}</div>
                <div className="rc-body">
                  <span className="ret-value">{cl.result}</span>
                  {sel && sel.ret && <span className="ret-type">: {sel.ret}</span>}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
