# InkPort seal0 backend — codegen contract

Target: this Portaldot node runs **rent-era `pallet-contracts` (seal0 ABI)**. ink! 3/4/5
toolchains do not build here, but **raw `seal0` Rust contracts compile on stable Rust and
deploy** (proven: `onchain-contracts/counter`). The translator therefore emits raw seal0 Rust,
not ink!.

## Pipeline
`Solidity (.sol)` → solang-parser → IR → **seal0 Rust `lib.rs`** → `cargo +stable build
--release --target wasm32-unknown-unknown` → `inkport_chain/strip_wasm.py` → deploy/call via
`inkport_chain/portaldot.py`.

## Contract ABI (codegen and Python harness MUST agree)
- **Call input** = 4-byte selector ++ SCALE(args in order).
- **Constructor input** (to `deploy`) = SCALE(ctor args) (no selector).
- **Return** = SCALE(return value); no return → empty.
- **Scalar SCALE encodings used** (these equal SCALE for the types):
  - `uintN`/`uint256` → `u128` → 16 bytes little-endian.
  - `bool` → 1 byte (`00`/`01`).
  - `address` → `[u8;32]` (AccountId) → 32 bytes raw.
- **Selectors**: real keccak-256 4-byte selectors = first 4 bytes of
  `keccak256("name(t1,t2,...)")` with canonical Solidity ABI arg types
  (`uint256`,`address`,`bool`,`int256`,`string`,`bytes`). ABI-compatible with
  Ethereum tooling. Emitted in metadata; the harness/CLI uses the metadata
  selector (never assumes sequential). A 4-byte selector collision is a hard
  translate error.
- **Event topics**: first topic = `keccak256("Event(t1,t2,...)")` (canonical
  signature), precomputed at translate time and written as a byte literal into
  the event's topic buffer. (The rent-era node does not surface topics; the
  harness disambiguates events by decoding the data payload.)

## Storage model
Assign each state variable a `u8` slot index in declaration order.
- **Scalar var** at slot S → 32-byte key `[S,0,0,…]`. Value = SCALE bytes (16 for u128, 1 for bool).
- **`mapping(K=>V)`** at slot S → key = `seal_hash_blake2_256([S] ++ key_bytes)`.
- **nested `mapping(A=>mapping(B=>V))`** → key = `seal_hash_blake2_256([S] ++ a_bytes ++ b_bytes)`.

## Runtime helpers (emit in every contract; only import seal0 funcs actually used)
```rust
#![no_std]
#![no_main]
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { core::arch::wasm32::unreachable() }

#[link(wasm_import_module = "seal0")]
extern "C" {
    fn seal_input(buf: *mut u8, len: *mut u32);
    fn seal_return(flags: u32, data: *const u8, len: u32);
    fn seal_get_storage(key: *const u8, out: *mut u8, out_len: *mut u32) -> u32;
    fn seal_set_storage(key: *const u8, val: *const u8, val_len: u32);
    // include only if used:
    fn seal_caller(out: *mut u8, out_len: *mut u32);
    fn seal_value_transferred(out: *mut u8, out_len: *mut u32);
    fn seal_hash_blake2_256(input: *const u8, len: u32, out: *mut u8);
    fn seal_deposit_event(topics: *const u8, topics_len: u32, data: *const u8, data_len: u32);
}
```
Helper conventions: `input()` reads up to 512 bytes; `ret(&[u8]) -> !` returns flags=0;
`revert() -> !` returns flags=1; storage read returns zero-filled buffer when key absent
(`seal_get_storage` rc != 0). `caller() -> [u8;32]`, `value() -> u128` when used.

## Solidity → seal0 mapping
| Solidity | seal0 Rust |
|---|---|
| state var (scalar) | slot read/write helpers (`load_u128(S)`, `store_u128(S, v)`, bool variant) |
| `mapping` access `m[k]` / `m[a][b]` | `map_load(S, k)` / `map_load2(S, a, b)` (blake2 key) |
| `m[k] = v` | `map_store(S, k, v)` |
| `constructor(args)` | `deploy`: decode args from input, init storage |
| `function f() view returns(T)` | message arm: decode args, compute, `ret(encode(result))` |
| state-mutating `function` | message arm: decode args, mutate storage, `ret(&[])` or `ret(encode(result))` |
| `msg.sender` | `caller()` |
| `msg.value` | `value()` |
| `require(c)` / `revert()` | `if !c { revert() }` |
| `+ - *` | `checked_*` → `revert()` on overflow |
| `<,<=,>,>=,==,!=` | same operators |
| `emit E(args)` | `seal_deposit_event` with SCALE(args); topic = blake2(event signature) |

## Metadata artifact (per contract, JSON)
```json
{ "name": "...", "constructor": {"args": ["u128"]},
  "messages": [ {"name":"get","selector":"0x00000002","args":[],"ret":"bool","mutates":false} ] }
```
The CLI/harness uses this to encode calls and decode returns. No hardcoded per-contract logic.

## Wasm constraints (rent-era validator)
The node's `pallet-contracts` wasm validator only accepts **MVP wasm**; it rejects
post-MVP features with `"Can't decode wasm code"`. Two rules the backend enforces:
- **No `memory.fill` / `memory.copy`.** A zero-initialized stack array larger than
  32 bytes (`[0u8; N]`, N>32) makes LLVM emit `memory.fill`; >32-byte slice copies
  emit `memory.copy`. The codegen therefore reads `seal_input` into a
  `MaybeUninit::<[u8; N]>` buffer (no zeroing) and writes all multi-byte event/key
  buffers via explicit byte loops. The `target-feature=-bulk-memory` rustflag alone
  does **not** suppress these — the source must avoid the memset/memcpy.
- Input buffers are still sized to the SCALE payload (not a fixed 512).

## Round 4 (Wave C) capabilities
- **Inheritance / interface flattening**: `contract C is A, B` merges base state
  vars, functions, modifiers, events, structs, enums into C (C overrides win;
  diamond bases de-duplicated). `interface`/`abstract`/bodyless function
  declarations are NOT emitted (they are obligations). FAIL-LOUD on multiple
  unrelated deployable contracts in one file, or a base not defined in the file.
- **`constant`**: compile-time inlined literal everywhere (incl. its public
  getter); takes no storage slot. FAIL-LOUD on a non-literal initializer.
  `immutable` stays a real storage slot (written by the constructor).
- **`receive()` / `fallback()`**: dispatched from `call()`'s default arm —
  empty calldata → receive(); else fallback() (or revert if absent).
- **enums**: lowered to `uint8` numeric; `Enum.Value` inlined as its ordinal.
- **struct locals**: `Point memory p = Point(a,b); p.x` lowers each field to a
  backing scalar local `__s_<var>_<field>`. (Struct args/returns across the ABI
  boundary are out of scope; struct *storage* uses the mapping-of-struct path.)
- **cross-contract calls** `IFoo(addr).bar(args)`: seal0 `seal_call` with the
  keccak-4 selector ++ SCALE(args); the return is decoded per the interface's
  declared type. The callee interface must be declared in the same file.

## Known limitation
- **uint256 is represented as `u128`** (16-byte LE), not true 256-bit. Arithmetic
  is *checked* (`checked_add/mul/...` → `revert` on overflow), so values above
  2^128 fail loud rather than silently wrapping; the SCALE harness encodes ≤16
  bytes. True 256-bit `[u8;32]` math is deferred. `uintN`/`intN` with N≤128 map
  to `u128`/`i128`.

## Validation requirement (zero mock)
Every supported contract must: translate → `cargo build` to wasm → strip → **deploy to
`wss://portaldot.philotheephilix.in` and pass behavioral assertions via dry-run reads + real
`Contracts.call` extrinsics**. A contract is "supported" only when its on-chain test passes.
