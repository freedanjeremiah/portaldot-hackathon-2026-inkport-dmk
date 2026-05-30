/* Simulated InkPort backend: builds believable SSE-style log streams + results
   for compile / deploy / call. No network — pure timing + canned plausibility. */

export type LogSegment = [string, string];
export type LogLine = LogSegment[];

function rand(a: number, b: number): number { return a + Math.random() * (b - a); }

function ss58(): string {
  const abc = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  let s = '5';
  for (let i = 0; i < 47; i++) s += abc[Math.floor(Math.random() * abc.length)];
  return s;
}

function gasWeight(): string {
  return 'Compact<Weight> { ref_time: ' + (1200000000 + Math.floor(rand(0, 8e8))).toLocaleString().replace(/,/g, '_') + ', proof_size: 0 }';
}

const UUID = (typeof crypto !== 'undefined' && crypto.randomUUID) ? crypto.randomUUID() : 'a1b2c3d4-0000-4000-8000-000000000000';
const DIR = '/tmp/inkport-playground/' + UUID;

export function buildCompile(name: string): { lines: LogLine[]; wasmSize: number } {
  const crate = name.toLowerCase();
  const size = 3600 + Math.floor(rand(0, 2400));
  const raw = size + Math.floor(rand(900, 1600));
  const secs = rand(2.8, 5.4).toFixed(2);
  const lines: LogLine[] = [
    [['lg-cmd', '$ '], ['', 'inkport-translate ' + name + '.sol --target seal --out '], ['lg-path', 'build/' + name + '/']],
    [['lg-dim', '  parsing '], ['lg-path', name + '.sol'], ['lg-dim', ' … '], ['lg-ok', 'ok']],
    [['lg-dim', '  solang-parser → IR … '], ['lg-ok', 'ok']],
    [['lg-dim', '  seal0 codegen … '], ['lg-ok', 'ok']],
    [['lg-dim', '  wrote '], ['lg-path', 'build/' + name + '/src/lib.rs']],
    [['lg-dim', '  wrote '], ['lg-path', 'build/' + name + '/metadata.json']],
    [['', '']],
    [['lg-cmd', '$ '], ['', 'cargo +stable build --release --target wasm32-unknown-unknown']],
    [['lg-dim', '   Compiling '], ['', crate + ' v0.1.0 '], ['lg-path', '(' + DIR + '/build/' + name + ')']],
    [['lg-dim', '    Compiling parity-scale-codec v3.6.9']],
    [['lg-warn', '    Finished '], ['', 'release [optimized] target(s) in '], ['lg-num', secs + 's']],
    [['', '']],
    [['lg-cmd', '$ '], ['', 'strip_wasm '], ['lg-path', name + '.wasm']],
    [['lg-dim', '  raw wasm '], ['lg-num', raw.toLocaleString()], ['lg-dim', ' bytes']],
    [['lg-dim', '  strip memory.fill / memory.copy → MVP-wasm … '], ['lg-ok', 'ok']],
    [['lg-dim', '  exports: '], ['lg-path', 'call'], ['lg-dim', ', '], ['lg-path', 'deploy'], ['lg-dim', '  + imported memory']],
    [['', '']],
    [['lg-ok', '✓ '], ['', name + '.wasm — '], ['lg-num', size.toLocaleString()], ['', ' bytes stripped (Portaldot-compatible)']],
  ];
  return { lines, wasmSize: size };
}

export function buildDeploy(name: string, argVals: string[]): { lines: LogLine[]; address: string } {
  const addr = ss58();
  const lines: LogLine[] = [
    [['lg-cmd', '$ '], ['', 'inkport deploy '], ['lg-path', name + '.wasm'], ['', ' --endowment 10POT --signer '], ['lg-warn', '//Alice']],
    [['lg-dim', '  connecting '], ['lg-path', 'wss://portaldot.philotheephilix.in'], ['lg-dim', ' … '], ['lg-ok', 'ok']],
    [['lg-dim', '  ss58_format=42  registry=substrate-node-template']],
    [['lg-dim', '  ctor input = SCALE'], ['', '(' + (argVals && argVals.length ? argVals.join(', ') : '') + ')']],
    [['lg-dim', '  call: '], ['lg-path', 'Contracts.instantiate_with_code']],
    [['lg-dim', '  gas: '], ['', gasWeight()]],
    [['lg-dim', '  endowment: '], ['lg-num', '10.0000 POT']],
    [['lg-dim', '  uploading code … '], ['lg-ok', 'ok']],
    [['lg-warn', '  ⛏  '], ['lg-dim', 'included in block '], ['lg-num', '#' + (1840000 + Math.floor(rand(0, 9000)))]],
    [['', '']],
    [['lg-ok', '✓ '], ['', 'instantiated '], ['lg-path', name], ['', ' at '], ['lg-ok', addr.slice(0, 8) + '…' + addr.slice(-6)]],
  ];
  return { lines, address: addr };
}

export interface CallState { count: number }

export function buildCall(name: string, msg: { name: string; selector: string; args: string[]; mutates: boolean; ret: string | null }, argVals: string[], state: CallState): { lines: LogLine[]; result: string | null; events: unknown[]; error: string | null; newState: CallState } {
  const lines: LogLine[] = [];
  lines.push([['lg-cmd', '$ '], ['', 'inkport call '], ['lg-path', name], ['', ' '], ['lg-warn', msg.name],
    ...(argVals && argVals.length ? [['', ' ' + argVals.map(v => '--arg ' + v).join(' ')] as LogSegment] : [['', ''] as LogSegment])]);
  lines.push([['lg-dim', '  selector '], ['lg-num', msg.selector], ['lg-dim', '  (' + msg.name + '(' + msg.args.join(',') + '))']]);

  let newCount = state.count;

  if (msg.mutates) {
    let delta = 0;
    if (/^inc$/i.test(msg.name)) delta = 1;
    else if (argVals && argVals.length && /^-?\d+$/.test((argVals[0] || '').trim())) delta = parseInt(argVals[0], 10);
    else if (/^dec/i.test(msg.name)) delta = -1;
    newCount = state.count + (isNaN(delta) ? 0 : delta);
    if (newCount < 0) {
      lines.push([['lg-dim', '  Contracts.call (signed //Alice) … ']]);
      lines.push([['lg-err', '  ✗ ContractTrapped — reverted: "overflow"']]);
      return { lines, result: null, events: [], error: 'ContractReverted: arithmetic overflow (checked sub)', newState: state };
    }
    lines.push([['lg-dim', '  Contracts.call (signed '], ['lg-warn', '//Alice'], ['lg-dim', ') … '], ['lg-ok', 'ok']]);
    lines.push([['lg-dim', '  gas used: '], ['', gasWeight()]]);
    lines.push([['lg-warn', '  ⛏  '], ['lg-dim', 'finalized in block '], ['lg-num', '#' + (1840000 + Math.floor(rand(0, 9000)))]]);
    lines.push([['lg-ok', '✓ '], ['', 'result: '], ['lg-ok', 'ok']]);
    return { lines, result: 'ok', events: [], error: null, newState: { count: newCount } };
  } else {
    lines.push([['lg-dim', '  Contracts.call dry-run (no signer) … '], ['lg-ok', 'ok']]);
    let val: string;
    if (msg.ret === 'bool') val = 'true';
    else if (msg.ret === 'address') val = '5GrwvaEF…AWjWMvBj';
    else if (msg.ret === 'string') val = '"' + name + '"';
    else if (msg.ret == null) val = '()';
    else val = String(state.count);
    lines.push([['lg-dim', '  decoded SCALE('], ['', msg.ret || '()'], ['lg-dim', ') = '], ['lg-num', val]]);
    lines.push([['lg-ok', '✓ '], ['', 'result: '], ['lg-num', val]]);
    return { lines, result: val, events: [], error: null, newState: state };
  }
}

export function stream(lines: LogLine[], opts: { onLine?: (line: LogLine, i: number) => void; onDone?: () => void; speed?: number }): () => void {
  const onLine = opts.onLine || (() => {});
  const onDone = opts.onDone || (() => {});
  const speed = opts.speed || 1;
  let i = 0, cancelled = false, timer: ReturnType<typeof setTimeout> | null = null;
  function next() {
    if (cancelled) return;
    if (i >= lines.length) { onDone(); return; }
    onLine(lines[i], i); i++;
    const d = lines[i - 1] && lines[i - 1].length === 1 && lines[i - 1][0][1] === '' ? 40 : rand(55, 150);
    timer = setTimeout(next, d / speed);
  }
  timer = setTimeout(next, 120 / speed);
  return () => { cancelled = true; if (timer) clearTimeout(timer); };
}
