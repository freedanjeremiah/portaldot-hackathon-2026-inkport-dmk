/* Lightweight tokenizer-based syntax highlighter for Solidity + Rust.
   Returns HTML string with <span class="tk-*"> wrappers. */

function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

const SOL = {
  kw: new Set(('pragma solidity contract interface library abstract function constructor modifier ' +
    'event emit struct enum mapping returns return public private internal external view pure payable ' +
    'memory storage calldata if else for while do break continue require assert revert unchecked new is ' +
    'import using indexed virtual override immutable constant delete try catch receive fallback ' +
    'true false this super').split(' ')),
  type: new Set(('bool address string bytes uint int uint8 uint16 uint32 uint64 uint128 uint256 ' +
    'int8 int16 int32 int64 int128 int256 bytes1 bytes4 bytes8 bytes32 byte').split(' ')),
  builtin: new Set(('msg sender value block timestamp number keccak256 sha256 ecrecover abi ' +
    'origin gasleft').split(' '))
};

const RUST = {
  kw: new Set(('as break const continue crate dyn else enum extern false fn for if impl in let loop ' +
    'match mod move mut pub ref return self Self static struct super trait true type unsafe use where ' +
    'while async await').split(' ')),
  type: new Set(('u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 bool char str String Vec ' +
    'Option Result Box Some None Ok Err AccountId Balance').split(' ')),
  builtin: new Set<string>([])
};

function classifyIdent(word: string, spec: typeof SOL, nextIsParen: boolean, isAttr: boolean): string {
  if (isAttr) return 'tk-attr';
  if (spec.kw.has(word)) return 'tk-kw';
  if (spec.type.has(word)) return 'tk-type';
  if (spec.builtin && spec.builtin.has(word)) return 'tk-id';
  if (nextIsParen) return 'tk-fn';
  if (/^[A-Z][A-Z0-9_]+$/.test(word)) return 'tk-const';
  if (/^[A-Z]/.test(word)) return 'tk-type';
  return 'tk-id';
}

export function highlight(code: string, lang: 'solidity' | 'rust'): string {
  const spec = lang === 'rust' ? RUST : SOL;
  let out = '';
  let i = 0;
  const n = code.length;
  while (i < n) {
    const c = code[i];
    if (c === '/' && code[i + 1] === '/') {
      let j = code.indexOf('\n', i); if (j < 0) j = n;
      out += '<span class="tk-com">' + esc(code.slice(i, j)) + '</span>'; i = j; continue;
    }
    if (c === '/' && code[i + 1] === '*') {
      let k = code.indexOf('*/', i + 2); k = k < 0 ? n : k + 2;
      out += '<span class="tk-com">' + esc(code.slice(i, k)) + '</span>'; i = k; continue;
    }
    if (lang === 'rust' && c === '#' && (code[i + 1] === '[' || (code[i + 1] === '!' && code[i + 2] === '['))) {
      let b = code.indexOf(']', i); b = b < 0 ? n : b + 1;
      out += '<span class="tk-attr">' + esc(code.slice(i, b)) + '</span>'; i = b; continue;
    }
    if (c === '"' || c === "'") {
      const q = c; let m = i + 1;
      while (m < n && code[m] !== q) { if (code[m] === '\\') m++; m++; }
      m = Math.min(m + 1, n);
      out += '<span class="tk-str">' + esc(code.slice(i, m)) + '</span>'; i = m; continue;
    }
    if (/[0-9]/.test(c) || (c === '.' && /[0-9]/.test(code[i + 1] || ''))) {
      const nm = /^(0x[0-9a-fA-F_]+|[0-9][0-9_]*(\.[0-9_]+)?(e[+-]?[0-9]+)?)([a-zA-Z0-9]*)/.exec(code.slice(i));
      const tok = nm ? nm[0] : c;
      out += '<span class="tk-num">' + esc(tok) + '</span>'; i += tok.length; continue;
    }
    if (/[A-Za-z_$]/.test(c)) {
      const w = /^[A-Za-z_$][A-Za-z0-9_$]*/.exec(code.slice(i))![0];
      let p = i + w.length;
      while (p < n && /\s/.test(code[p])) p++;
      const nextParen = code[p] === '(';
      const isMacro = code[i + w.length] === '!';
      const cls = classifyIdent(w, spec as typeof SOL, nextParen || isMacro, false);
      out += '<span class="' + cls + '">' + esc(w) + '</span>'; i += w.length; continue;
    }
    if (/[+\-*/%=&|<>!^~?:]/.test(c)) {
      const om = /^[+\-*/%=&|<>!^~?:]+/.exec(code.slice(i))![0];
      out += '<span class="tk-op">' + esc(om) + '</span>'; i += om.length; continue;
    }
    out += esc(c); i++;
  }
  return out;
}
