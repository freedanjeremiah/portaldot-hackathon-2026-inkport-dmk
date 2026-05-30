/* InkPort translate simulator: parses a subset of Solidity and emits believable
   seal0 Rust + metadata.json, mirroring what the real inkport-translate step produces.
   This is a plausibility engine for the playground UI — not a real compiler. */

export interface MetadataField {
  name: string;
  type: string;
}

export interface MetadataMessage {
  name: string;
  selector: string;
  args: string[];
  argNames: string[];
  ret: string | null;
  mutates: boolean;
  payable: boolean;
  getter?: boolean;
}

export interface MetadataEvent {
  name: string;
  fields: MetadataField[];
}

export interface Metadata {
  name: string;
  constructor: { args: string[]; argNames: string[] };
  messages: MetadataMessage[];
  events: MetadataEvent[];
}

export type TranslateResult =
  | { rust: string; metadata: Metadata; error?: never; line?: never }
  | { error: string; line: number | null; rust?: never; metadata?: never };

// Rejected constructs (mirror spec §7)
const REJECTED: [RegExp, string][] = [
  [/\bassembly\s*\{/, 'inline `assembly` is not supported by the seal0 target'],
  [/\bdelegatecall\b/, '`delegatecall` is rejected (no host function on rent-era pallet-contracts)'],
  [/\btx\.origin\b/, '`tx.origin` is not available under seal0'],
  [/\babi\.encodePacked\b/, '`abi.encodePacked` is rejected; use typed args'],
  [/\blibrary\s+\w+/, '`library` definitions are not supported (flatten into the contract)'],
  [/[^=!<>]=\s*[^;]*\?[^;:]*:/, 'ternary `?:` is rejected; use if/else'],
  [/\bnew\s+[A-Z]\w*\s*\(/, '`new` factory deployment is not supported'],
];

const SOL_TO_ABI: Record<string, string> = {
  bool: 'bool', address: 'address', string: 'string', bytes: 'bytes', bytes32: 'bytes32'
};

function mapType(t: string): string {
  t = t.trim();
  if (SOL_TO_ABI[t]) return SOL_TO_ABI[t];
  if (/^uint\d*$/.test(t)) return 'u128';
  if (/^int\d*$/.test(t)) return 'i128';
  if (/^bytes\d+$/.test(t)) return 'bytes32';
  if (/\]$/.test(t)) return mapType(t.replace(/\[\]$/, '')) + '[]';
  if (/^mapping/.test(t)) return 'mapping';
  return t;
}

function selector(sig: string): string {
  const known: Record<string, string> = {
    'inc()': '0x371303c0',
    'incBy(u128)': '0x70119d06',
    'get()': '0x6d4ce63c',
  };
  if (known[sig]) return known[sig];
  let h = 0x811c9dc5;
  for (let i = 0; i < sig.length; i++) { h ^= sig.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; }
  return '0x' + ('00000000' + h.toString(16)).slice(-8);
}

function stripComments(src: string): string {
  return src.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/[^\n]*/g, '');
}

function splitArgs(s: string): string[] {
  s = s.trim(); if (!s) return [];
  const out: string[] = [];
  let depth = 0, cur = '';
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === '(' || c === '[') depth++;
    if (c === ')' || c === ']') depth--;
    if (c === ',' && depth === 0) { out.push(cur); cur = ''; } else cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out;
}

function parseParam(p: string): { soltype: string; name: string; type: string } {
  p = p.trim().replace(/\b(memory|storage|calldata)\b/g, '').replace(/\s+/g, ' ').trim();
  const m = /^(.+?)\s+([A-Za-z_]\w*)$/.exec(p);
  if (m) return { soltype: m[1].trim(), name: m[2], type: mapType(m[1].trim()) };
  return { soltype: p, name: '', type: mapType(p) };
}

function lineOf(src: string, re: RegExp): number | null {
  const m = re.exec(src); if (!m) return null;
  return src.slice(0, m.index).split('\n').length;
}

function parse(src: string) {
  const code = stripComments(src);

  for (const [re, msg] of REJECTED) {
    if (re.test(code)) {
      return { error: 'error: ' + msg, line: lineOf(src, re) };
    }
  }

  const cm = /contract\s+([A-Za-z_]\w*)/.exec(code);
  const iname = /interface\s+([A-Za-z_]\w*)/.exec(code);
  if (!cm && !iname) return { error: 'error: no `contract` definition found', line: null };
  const name = (cm || iname)![1];

  const ctorArgs: ReturnType<typeof parseParam>[] = [];
  const cc = /constructor\s*\(([^)]*)\)/.exec(code);
  if (cc) ctorArgs.push(...splitArgs(cc[1]).map(parseParam));

  const messages: MetadataMessage[] = [];
  const fnRe = /function\s+([A-Za-z_]\w*)\s*\(([^)]*)\)([^{};]*)(\{|;)/g;
  let fm: RegExpExecArray | null;
  while ((fm = fnRe.exec(code))) {
    const fname = fm[1];
    const params = splitArgs(fm[2]).map(parseParam);
    const mods = fm[3] || '';
    const isView = /\b(view|pure)\b/.test(mods);
    const isPayable = /\bpayable\b/.test(mods);
    const retM = /returns\s*\(([^)]*)\)/.exec(mods);
    const rets = retM ? splitArgs(retM[1]).map(parseParam) : [];
    const ret = rets.length ? rets[0].type : null;
    const canon = '(' + params.map(p => p.type).join(',') + ')';
    messages.push({
      name: fname,
      selector: selector(fname + canon),
      args: params.map(p => p.type),
      argNames: params.map(p => p.name),
      ret, mutates: !isView, payable: isPayable
    });
  }

  const stateVars: { soltype: string; name: string; type: string; public: boolean }[] = [];
  const bodyStart = code.indexOf('{');
  const body = bodyStart >= 0 ? code.slice(bodyStart + 1) : code;

  // Match all visibility modifiers so private/internal vars get their name captured correctly
  const varRe = /\b(uint\d*|int\d*|bool|address|bytes\d*|string)\s+(public\s+|private\s+|internal\s+|external\s+)?(constant\s+|immutable\s+)?([A-Za-z_]\w*)\s*(=|;)/g;
  let vm: RegExpExecArray | null;
  while ((vm = varRe.exec(body))) {
    const isPublic = !!vm[2] && vm[2].trim() === 'public';
    stateVars.push({ soltype: vm[1], name: vm[4], type: mapType(vm[1]), public: isPublic });
    if (isPublic && !messages.some(mm => mm.name === vm![4])) {
      messages.push({ name: vm[4], selector: selector(vm[4] + '()'), args: [], argNames: [], ret: mapType(vm[1]), mutates: false, payable: false, getter: true });
    }
  }

  const mapRe = /\bmapping\s*\(\s*([A-Za-z_]\w*)\s*=>\s*([A-Za-z_]\w*[\[\]]*)\s*\)\s*(public\s+)?([A-Za-z_]\w*)\s*;/g;
  let mp: RegExpExecArray | null;
  while ((mp = mapRe.exec(body))) {
    stateVars.push({ soltype: 'mapping(' + mp[1] + '=>' + mp[2] + ')', name: mp[4], type: 'mapping', public: !!mp[3] });
    if (mp[3] && !messages.some(mm => mm.name === mp![4])) {
      messages.push({ name: mp[4], selector: selector(mp[4] + '(' + mapType(mp[1]) + ')'), args: [mapType(mp[1])], argNames: ['key'], ret: mapType(mp[2]), mutates: false, payable: false, getter: true });
    }
  }

  const events: MetadataEvent[] = [];
  const evRe = /event\s+([A-Za-z_]\w*)\s*\(([^)]*)\)/g;
  let em: RegExpExecArray | null;
  while ((em = evRe.exec(code))) {
    const fields = splitArgs(em[2]).map(f => {
      const pp = parseParam(f.replace(/\bindexed\b/g, ''));
      return { name: pp.name || 'arg', type: pp.type };
    });
    events.push({ name: em[1], fields });
  }

  if (messages.length === 0 && ctorArgs.length === 0) {
    return { error: 'error: contract `' + name + '` exposes no callable messages', line: null };
  }

  const metadata: Metadata = {
    name,
    constructor: { args: ctorArgs.map(a => a.type), argNames: ctorArgs.map(a => a.name) },
    messages: messages.map(m => ({ name: m.name, selector: m.selector, args: m.args, argNames: m.argNames, ret: m.ret, mutates: m.mutates, payable: m.payable })),
    events,
  };

  return { name, metadata, ctorArgs, messages, stateVars, events };
}

function genRust(p: NonNullable<ReturnType<typeof parse> & { name: string }>): string {
  const L: string[] = [];
  L.push('#![no_std]');
  L.push('#![no_main]');
  L.push('');
  L.push('extern crate alloc;');
  L.push('use alloc::vec::Vec;');
  L.push('use core::panic::PanicInfo;');
  L.push('');
  L.push('// ---------------------------------------------------------------------------');
  L.push('//  ' + p.name + '  ->  seal0 raw Rust   (generated by inkport-translate)');
  L.push('//  target: rent-era pallet-contracts, seal0 ABI (~Substrate 2021)');
  L.push('// ---------------------------------------------------------------------------');
  L.push('');
  L.push('mod seal0 {');
  L.push('    #[link(wasm_import_module = "seal0")]');
  L.push('    extern "C" {');
  L.push('        pub fn seal_input(buf: *mut u8, len: *mut u32);');
  L.push('        pub fn seal_return(flags: u32, data: *const u8, len: u32) -> !;');
  L.push('        pub fn seal_get_storage(key: *const u8, out: *mut u8, len: *mut u32) -> u32;');
  L.push('        pub fn seal_set_storage(key: *const u8, value: *const u8, len: u32);');
  L.push('        pub fn seal_caller(out: *mut u8, len: *mut u32);');
  L.push('        pub fn seal_value_transferred(out: *mut u8, len: *mut u32);');
  if (p.events && p.events.length) L.push('        pub fn seal_deposit_event(topics: *const u8, t_len: u32, data: *const u8, d_len: u32);');
  L.push('    }');
  L.push('}');
  L.push('');

  const slots = (p.stateVars && p.stateVars.length) ? p.stateVars : [{ name: 'state', type: 'u128' }];
  slots.forEach((_s: { name: string }, idx: number) => {
    L.push('const SLOT_' + _s.name.toUpperCase() + ': [u8; 32] = [0u8; 32];');
  });
  L.push('');

  L.push('#[no_mangle]');
  L.push('pub extern "C" fn deploy() {');
  if (p.ctorArgs && p.ctorArgs.length) {
    L.push('    let mut input = [0u8; 1024];');
    L.push('    let mut len = 1024u32;');
    L.push('    unsafe { seal0::seal_input(input.as_mut_ptr(), &mut len) };');
    L.push('    let mut off = 0usize;');
    p.ctorArgs.forEach((a: { name: string; type: string }) => {
      L.push('    let ' + (a.name || 'arg') + ' = ' + decodeExpr(a.type) + ';');
    });
    const firstSlot = slots[0].name.toUpperCase();
    L.push('    write_u128(&SLOT_' + firstSlot + ', ' + ((p.ctorArgs[0] && p.ctorArgs[0].name) || 'arg') + ' as u128);');
  } else {
    L.push('    // no constructor args');
  }
  L.push('    ret(&[]);');
  L.push('}');
  L.push('');

  L.push('#[no_mangle]');
  L.push('pub extern "C" fn call() {');
  L.push('    let mut input = [0u8; 1024];');
  L.push('    let mut len = 1024u32;');
  L.push('    unsafe { seal0::seal_input(input.as_mut_ptr(), &mut len) };');
  L.push('    let selector = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);');
  L.push('    match selector {');
  (p.messages || []).forEach((m: MetadataMessage) => {
    L.push('        ' + m.selector + ' => { // ' + m.name + '(' + m.args.join(', ') + ')');
    if (m.mutates) {
      if (m.args.length) {
        L.push('            let mut off = 4usize;');
        m.argNames.forEach((an: string, idx: number) => {
          L.push('            let ' + (an || ('a' + idx)) + ' = ' + decodeExpr(m.args[idx]) + ';');
        });
      }
      L.push('            let cur = read_u128(&SLOT_' + slots[0].name.toUpperCase() + ');');
      L.push('            let next = cur.checked_add(' + (m.args.length ? (m.argNames[0] || 'a0') + ' as u128' : '1') + ')');
      L.push('                .unwrap_or_else(|| revert(b"overflow"));');
      L.push('            write_u128(&SLOT_' + slots[0].name.toUpperCase() + ', next);');
      L.push('            ret(&[]);');
    } else if (m.ret) {
      L.push('            let v = read_u128(&SLOT_' + slots[0].name.toUpperCase() + ');');
      L.push('            ret(&v.to_le_bytes());');
    } else {
      L.push('            ret(&[]);');
    }
    L.push('        }');
  });
  L.push('        _ => revert(b"unknown selector"),');
  L.push('    }');
  L.push('}');
  L.push('');

  L.push('#[inline] fn read_u128(key: &[u8; 32]) -> u128 {');
  L.push('    let mut buf = [0u8; 16]; let mut l = 16u32;');
  L.push('    unsafe { seal0::seal_get_storage(key.as_ptr(), buf.as_mut_ptr(), &mut l) };');
  L.push('    u128::from_le_bytes(buf)');
  L.push('}');
  L.push('#[inline] fn write_u128(key: &[u8; 32], v: u128) {');
  L.push('    let b = v.to_le_bytes();');
  L.push('    unsafe { seal0::seal_set_storage(key.as_ptr(), b.as_ptr(), 16) };');
  L.push('}');
  L.push('fn ret(data: &[u8]) -> ! {');
  L.push('    unsafe { seal0::seal_return(0, data.as_ptr(), data.len() as u32) }');
  L.push('}');
  L.push('fn revert(_msg: &[u8]) -> ! {');
  L.push('    unsafe { seal0::seal_return(1, _msg.as_ptr(), _msg.len() as u32) }');
  L.push('}');
  L.push('');
  L.push('#[panic_handler]');
  L.push('fn panic(_: &PanicInfo) -> ! { revert(b"panic") }');
  L.push('');

  return L.join('\n');
}

function decodeExpr(type: string): string {
  if (type === 'bool') return '{ let v = input[off] != 0; off += 1; v }';
  if (type === 'address') return '{ let mut a = [0u8;32]; a.copy_from_slice(&input[off..off+32]); off += 32; a }';
  if (type === 'i128') return '{ let mut b=[0u8;16]; b.copy_from_slice(&input[off..off+16]); off+=16; i128::from_le_bytes(b) }';
  if (type === 'string' || type === 'bytes') return 'decode_bytes(&input[off..], &mut off)';
  return '{ let mut b=[0u8;16]; b.copy_from_slice(&input[off..off+16]); off+=16; u128::from_le_bytes(b) }';
}

export function translate(src: string): TranslateResult {
  const p = parse(src);
  if ('error' in p && p.error) return { error: p.error, line: p.line ?? null };
  const parsed = p as NonNullable<ReturnType<typeof parse> & { name: string }>;
  return { rust: genRust(parsed), metadata: parsed.metadata };
}
