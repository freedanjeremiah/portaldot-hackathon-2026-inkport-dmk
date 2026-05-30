//! seal0 codegen backend.
//!
//! Emits raw `seal0` Rust (`lib.rs`) plus a metadata JSON. Two tiers:
//!
//! * **scalar tier** — `bool`, `uintN`→`u128`, `address`→`[u8;32]` state
//!   variables; constructor; view/mutating messages; arithmetic; require/revert.
//! * **mapping/event/payable tier** — `mapping(address=>uintN)` and
//!   `mapping(address=>mapping(address=>uintN))` storage (blake2-256 keys),
//!   `msg.sender`/`msg.value`, events via `seal_deposit_event`, `payable`
//!   functions, POT transfers via `seal_transfer`, and `onlyOwner`-style
//!   modifiers inlined as entry guards.
//!
//! Follows `docs/seal-backend-spec.md` and mirrors the shape of the proven
//! `onchain-contracts/counter` contract.
//!
//! Unlike the ink! path (which leaves IR `Function.body` empty and renders via
//! `codegen.rs`), this backend re-parses the Solidity AST with solang and does
//! seal0-specific statement lowering directly off the parse tree.

use std::collections::BTreeMap;

use solang_parser::pt::{
    ContractDefinition, ContractPart, Expression, FunctionAttribute, FunctionDefinition,
    FunctionTy, Statement, Type as PtType,
};

use crate::ir::{Contract, Mutability, Type};
use crate::lower::map_type;

/// The runtime value kind an expression evaluates to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValTy {
    /// `u128` numeric value (uintN).
    Num,
    /// `i128` signed numeric value (intN). Stored as 16-byte LE two's complement.
    SNum,
    /// `bool`.
    Bool,
    /// `[u8; 32]` AccountId / address.
    Addr,
    /// `string` / `bytes`: SCALE = compact(len) ++ raw bytes. Handled by a
    /// dedicated narrow path (storage blob, param slice, return blob).
    Str,
}

impl ValTy {
    fn from_type(t: &Type) -> ValTy {
        match t {
            Type::Bool => ValTy::Bool,
            Type::AccountId => ValTy::Addr,
            Type::I128 => ValTy::SNum,
            Type::String | Type::Bytes => ValTy::Str,
            _ => ValTy::Num,
        }
    }

    /// Is this a numeric (signed or unsigned) kind?
    fn is_numeric(self) -> bool {
        matches!(self, ValTy::Num | ValTy::SNum)
    }
}

/// The kind of a storage slot.
#[derive(Debug, Clone)]
enum SlotKind {
    /// A scalar value of the given runtime kind.
    Scalar(ValTy),
    /// `mapping(K => V)`. `key` is the key's runtime kind (Addr = 32 bytes,
    /// Num/SNum = 16-byte LE, Bool = 1 byte).
    Map { key: ValTy, val: ValTy },
    /// `mapping(A => mapping(B => V))`.
    Map2 { key1: ValTy, key2: ValTy, val: ValTy },
    /// Dynamic array `T[]` of a scalar element kind. Length stored at the var's
    /// scalar slot key; element `i` at `blake2(slot ++ u32_le(i))`.
    Array { elem: ValTy },
    /// `mapping(K => Struct)`. `fields` lists each struct field's kind in
    /// declaration order; field `f` of `m[k]` lives at
    /// `blake2(slot ++ key_bytes ++ [field_index])`.
    MapStruct { key: ValTy, fields: Vec<(String, ValTy)> },
}

/// A storage field with its assigned slot index.
struct Slot {
    name: String,
    index: u8,
    kind: SlotKind,
}

/// Result of seal0 codegen: the Rust source and the metadata JSON.
#[derive(Debug)]
pub struct SealArtifacts {
    pub lib_rs: String,
    pub metadata_json: String,
    pub cargo_toml: String,
    pub cargo_config_toml: String,
    /// snake_case crate name derived from the contract name.
    pub crate_name: String,
}

/// Which seal0 host functions a contract actually uses (gates the imports).
#[derive(Default, Clone, Copy)]
struct Uses {
    caller: bool,
    value: bool,
    blake2: bool,
    deposit_event: bool,
    transfer: bool,
    now: bool,
    block_number: bool,
    balance: bool,
    /// Cross-contract `seal_call`.
    call: bool,
}

/// An external function signature usable as a cross-contract call target
/// (declared on an `interface`/contract in the same file).
#[derive(Debug, Clone)]
pub struct ExternalFn {
    /// keccak-256 4-byte selector of the canonical signature.
    selector: [u8; 4],
    /// Argument kinds in declaration order (for SCALE-encoding the call input).
    args: Vec<ValTy>,
    /// Return kind, if any. (Single-value returns only.)
    ret: Option<ValTy>,
}

/// Lowering context for seal0 statement/expression generation.
struct SealCtx<'a> {
    slots: &'a [Slot],
    events: &'a [crate::ir::Event],
    /// Local variable -> runtime kind (params).
    locals: BTreeMap<String, ValTy>,
    /// Variable name -> declared *narrow* bit-width N (1..=127) for numeric
    /// types. Wide numeric types (uint128/uint256/uint, int128/int256/int) are
    /// intentionally absent: absence means "fail-safe wide" (the existing
    /// checked_* at the u128/i128 boundary already reverts correctly). Covers
    /// storage scalars, params and locals.
    widths: BTreeMap<String, u32>,
    /// When true, arithmetic is being lowered inside a Solidity `unchecked { }`
    /// block: narrow ops wrap (mask to the type width) instead of reverting.
    unchecked: bool,
    /// `constant` variables inlined at compile time: name -> (rust literal, kind).
    constants: &'a BTreeMap<String, (String, ValTy)>,
    /// Enum value names -> their u8 ordinal (rendered as a u128 literal).
    enum_values: &'a BTreeMap<String, u8>,
    /// Struct definitions: name -> ordered (field, kind).
    structs: &'a BTreeMap<String, Vec<(String, ValTy)>>,
    /// In-scope struct-typed locals: var name -> (struct name).
    local_structs: BTreeMap<String, String>,
    /// External function signatures (by name) for cross-contract calls.
    external_fns: &'a BTreeMap<String, ExternalFn>,
    uses: Uses,
    errors: Vec<String>,
    /// Declared return kinds, used to lower tuple `return (a, b)` statements
    /// for multi-return functions.
    ret_kinds: Vec<ValTy>,
}

impl<'a> SealCtx<'a> {
    fn new(
        slots: &'a [Slot],
        events: &'a [crate::ir::Event],
        constants: &'a BTreeMap<String, (String, ValTy)>,
        enum_values: &'a BTreeMap<String, u8>,
        structs: &'a BTreeMap<String, Vec<(String, ValTy)>>,
        external_fns: &'a BTreeMap<String, ExternalFn>,
    ) -> Self {
        SealCtx {
            slots,
            events,
            locals: BTreeMap::new(),
            widths: BTreeMap::new(),
            unchecked: false,
            constants,
            enum_values,
            structs,
            local_structs: BTreeMap::new(),
            external_fns,
            uses: Uses::default(),
            errors: Vec::new(),
            ret_kinds: Vec::new(),
        }
    }

    /// The Rust local name backing struct local `var`'s `field`.
    fn struct_local(var: &str, field: &str) -> String {
        format!("__s_{var}_{field}")
    }

    fn slot_of(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.name == name)
    }

    fn err(&mut self, msg: &str) -> String {
        self.errors.push(msg.to_string());
        format!("/* UNSUPPORTED: {msg} */ 0u128")
    }

    /// Render an expression, returning the Rust source and its runtime kind.
    fn expr_ty(&mut self, e: &Expression) -> (String, ValTy) {
        match e {
            Expression::Parenthesis(_, inner) => {
                let (s, t) = self.expr_ty(inner);
                (format!("({s})"), t)
            }

            // Storage scalar read or local variable.
            Expression::Variable(id) => {
                // Compile-time inlined `constant`.
                if let Some((lit, vt)) = self.constants.get(&id.name) {
                    return (lit.clone(), *vt);
                }
                // Bare enum value name (e.g. an enum used without qualifier).
                if let Some(ord) = self.enum_values.get(&id.name) {
                    return (format!("{ord}u128"), ValTy::Num);
                }
                if let Some(slot) = self.slot_of(&id.name) {
                    if let SlotKind::Scalar(vt) = slot.kind {
                        // Signed scalars are stored as u128 bit patterns.
                        let read = match vt {
                            ValTy::SNum => format!("(load_slot_{}() as i128)", slot.index),
                            _ => format!("load_slot_{}()", slot.index),
                        };
                        return (read, vt);
                    }
                    // A bare mapping name in value position is unsupported.
                    return (self.err("mapping used as value"), ValTy::Num);
                }
                let t = self.locals.get(&id.name).copied().unwrap_or(ValTy::Num);
                (id.name.clone(), t)
            }

            Expression::NumberLiteral(_, value, _, _) => (format!("{value}u128"), ValTy::Num),
            Expression::HexNumberLiteral(_, value, _) => {
                // 0x... numeric literal.
                (format!("{value}u128"), ValTy::Num)
            }
            Expression::BoolLiteral(_, b) => {
                ((if *b { "true" } else { "false" }).to_string(), ValTy::Bool)
            }

            Expression::Not(_, inner) => {
                let (s, _) = self.expr_ty(inner);
                (format!("!({s})"), ValTy::Bool)
            }

            Expression::Add(_, l, r) => self.arith(l, r, "checked_add"),
            Expression::Subtract(_, l, r) => self.arith(l, r, "checked_sub"),
            Expression::Multiply(_, l, r) => self.arith(l, r, "checked_mul"),
            Expression::Divide(_, l, r) => self.arith(l, r, "checked_div"),
            Expression::Modulo(_, l, r) => self.arith(l, r, "checked_rem"),

            Expression::Less(_, l, r) => (self.cmp(l, r, "<"), ValTy::Bool),
            Expression::More(_, l, r) => (self.cmp(l, r, ">"), ValTy::Bool),
            Expression::LessEqual(_, l, r) => (self.cmp(l, r, "<="), ValTy::Bool),
            Expression::MoreEqual(_, l, r) => (self.cmp(l, r, ">="), ValTy::Bool),
            Expression::Equal(_, l, r) => (self.cmp(l, r, "=="), ValTy::Bool),
            Expression::NotEqual(_, l, r) => (self.cmp(l, r, "!="), ValTy::Bool),

            Expression::And(_, l, r) => {
                let (l, _) = self.expr_ty(l);
                let (r, _) = self.expr_ty(r);
                (format!("({l} && {r})"), ValTy::Bool)
            }
            Expression::Or(_, l, r) => {
                let (l, _) = self.expr_ty(l);
                let (r, _) = self.expr_ty(r);
                (format!("({l} || {r})"), ValTy::Bool)
            }

            // Mapping read: m[k] or m[a][b]. Or dynamic-array element a[i].
            Expression::ArraySubscript(_, base, Some(index)) => {
                // Dynamic array element read?
                if let Expression::Variable(id) = base.as_ref() {
                    if let Some(slot) = self.slot_of(&id.name) {
                        if let SlotKind::Array { elem } = slot.kind {
                            let s = slot.index;
                            let (ix, it) = self.expr_ty(index);
                            let ix = coerce_num(&ix, it, ValTy::Num);
                            let read = match elem {
                                ValTy::Bool => format!("arr_get_bool_{s}({ix})"),
                                ValTy::Addr => format!("arr_get_addr_{s}({ix})"),
                                ValTy::SNum => format!("(arr_get_u128_{s}({ix}) as i128)"),
                                _ => format!("arr_get_u128_{s}({ix})"),
                            };
                            return (read, elem);
                        }
                    }
                }
                if let Some((slot_idx, val, keys)) = self.as_map_access(e) {
                    let key_expr = self.map_key_call(slot_idx, &keys);
                    let getter = match val {
                        ValTy::Bool => format!("map_get_bool({key_expr})"),
                        ValTy::Addr => format!("map_get_addr({key_expr})"),
                        ValTy::SNum => format!("(map_get_u128({key_expr}) as i128)"),
                        _ => format!("map_get_u128({key_expr})"),
                    };
                    (getter, val)
                } else {
                    (self.err("array subscript on non-mapping"), ValTy::Num)
                }
            }

            Expression::MemberAccess(_, base, member) => {
                if let Expression::Variable(id) = base.as_ref() {
                    if id.name == "msg" && member.name == "sender" {
                        self.uses.caller = true;
                        return ("caller()".to_string(), ValTy::Addr);
                    }
                    if id.name == "msg" && member.name == "value" {
                        self.uses.value = true;
                        return ("value()".to_string(), ValTy::Num);
                    }
                    // block.timestamp / block.number
                    if id.name == "block" {
                        if member.name == "timestamp" {
                            self.uses.now = true;
                            return ("block_timestamp()".to_string(), ValTy::Num);
                        }
                        if member.name == "number" {
                            self.uses.block_number = true;
                            return ("block_number()".to_string(), ValTy::Num);
                        }
                    }
                }
                // address(this).balance
                if member.name == "balance" {
                    if let Expression::FunctionCall(_, callee, cargs) = base.as_ref() {
                        if is_address_cast(callee) {
                            if let Some(Expression::Variable(inner)) = cargs.first() {
                                if inner.name == "this" {
                                    self.uses.balance = true;
                                    return ("self_balance()".to_string(), ValTy::Num);
                                }
                            }
                        }
                    }
                }
                // `arr.length` on a dynamic array storage var.
                if member.name == "length" {
                    if let Expression::Variable(id) = base.as_ref() {
                        if let Some(slot) = self.slot_of(&id.name) {
                            if let SlotKind::Array { .. } = slot.kind {
                                return (format!("arr_len_{}()", slot.index), ValTy::Num);
                            }
                        }
                    }
                }
                // `localStruct.field` read.
                if let Expression::Variable(id) = base.as_ref() {
                    if let Some((v, t)) = self.local_struct_field_read(&id.name, &member.name) {
                        return (v, t);
                    }
                }
                // `EnumName.Value` -> the variant's u8 ordinal.
                if let Expression::Variable(_) = base.as_ref() {
                    if let Some(ord) = self.enum_values.get(&member.name) {
                        return (format!("{ord}u128"), ValTy::Num);
                    }
                }
                // `m[k].field` struct-field read.
                if let Some((read, vt)) = self.struct_field_read(base, &member.name) {
                    return (read, vt);
                }
                (self.err(&format!("member access .{}", member.name)), ValTy::Num)
            }

            // `address(x)` / `payable(x)` casts: pass through the inner value.
            Expression::FunctionCall(_, callee, args) => {
                // Cross-contract call `IFoo(addr).bar(args)` in value position.
                if let Some((expr, vt)) = self.cross_call(callee, args) {
                    return (expr, vt);
                }
                if let Expression::Type(_, PtType::Payable) = callee.as_ref() {
                    if let Some(a) = args.first() {
                        return self.expr_ty(a);
                    }
                }
                if let Expression::Type(_, PtType::Address)
                | Expression::Type(_, PtType::AddressPayable) = callee.as_ref()
                {
                    if let Some(a) = args.first() {
                        if is_zero_literal(a) {
                            return ("[0u8; 32]".to_string(), ValTy::Addr);
                        }
                        return self.expr_ty(a);
                    }
                }
                // `uintN(x)` / `intN(x)` where solang renders the callee as a
                // `Type` node rather than a `Variable`. Narrowing casts (N<128)
                // must truncate to N bits, matching Solidity's modulo-2^N
                // semantics (`uint8(256) == 0`, `int8(200) == -56`).
                if let Expression::Type(_, PtType::Uint(n)) = callee.as_ref() {
                    if let Some(a) = args.first() {
                        let (s, t) = self.expr_ty(a);
                        let v = coerce_num(&s, t, ValTy::Num);
                        return (mask_narrow(&v, narrow_width(*n)), ValTy::Num);
                    }
                }
                if let Expression::Type(_, PtType::Int(n)) = callee.as_ref() {
                    if let Some(a) = args.first() {
                        let (s, t) = self.expr_ty(a);
                        let v = coerce_num(&s, t, ValTy::SNum);
                        return (sign_extend_narrow(&v, narrow_width(*n)), ValTy::SNum);
                    }
                }
                if let Expression::Variable(id) = callee.as_ref() {
                    if id.name == "address" {
                        if let Some(a) = args.first() {
                            // `address(0)` -> the 32-byte zero AccountId.
                            if is_zero_literal(a) {
                                return ("[0u8; 32]".to_string(), ValTy::Addr);
                            }
                            return self.expr_ty(a);
                        }
                    }
                    // intN(x) / uintN(x) casts: coerce kind and truncate to the
                    // target width N for narrowing casts (N<128).
                    if id.name.starts_with("uint") {
                        if let Some(a) = args.first() {
                            let (s, t) = self.expr_ty(a);
                            let v = coerce_num(&s, t, ValTy::Num);
                            return (mask_narrow(&v, cast_name_width(&id.name)), ValTy::Num);
                        }
                    }
                    if id.name.starts_with("int") {
                        if let Some(a) = args.first() {
                            let (s, t) = self.expr_ty(a);
                            let v = coerce_num(&s, t, ValTy::SNum);
                            return (sign_extend_narrow(&v, cast_name_width(&id.name)), ValTy::SNum);
                        }
                    }
                }
                (self.err("function call in value position"), ValTy::Num)
            }

            // Bitwise binary ops.
            Expression::BitwiseAnd(_, l, r) => (self.bitop(l, r, "&"), ValTy::Num),
            Expression::BitwiseOr(_, l, r) => (self.bitop(l, r, "|"), ValTy::Num),
            Expression::BitwiseXor(_, l, r) => (self.bitop(l, r, "^"), ValTy::Num),
            Expression::ShiftLeft(_, l, r) => (self.shift(l, r, "wrapping_shl"), ValTy::Num),
            Expression::ShiftRight(_, l, r) => (self.shift(l, r, "wrapping_shr"), ValTy::Num),
            Expression::BitwiseNot(_, inner) => {
                let (s, t) = self.expr_ty(inner);
                (format!("(!({s}))"), if t.is_numeric() { t } else { ValTy::Num })
            }

            // Unary minus on a numeric literal/expr -> signed.
            Expression::Negate(_, inner) => {
                let (s, _) = self.expr_ty(inner);
                (format!("(-({s} as i128))"), ValTy::SNum)
            }

            // Pre/post increment & decrement used in value position (e.g. i++).
            Expression::PreIncrement(_, inner)
            | Expression::PostIncrement(_, inner)
            | Expression::PreDecrement(_, inner)
            | Expression::PostDecrement(_, inner) => {
                // Value-position inc/dec is uncommon; handle as the bare value
                // (the mutation is realized by the statement-level handler).
                self.expr_ty(inner)
            }

            _ => (self.err("expression"), ValTy::Num),
        }
    }

    /// Render an expression (value context), discarding the kind.
    fn expr(&mut self, e: &Expression) -> String {
        self.expr_ty(e).0
    }

    /// Infer the Solidity *declared* bit-width of a numeric expression.
    ///
    /// Returns `Some(N)` only for a *narrow* width (1..=127) whose overflow
    /// semantics differ from the u128/i128 boundary; returns `None` for wide
    /// types (uint128/uint256/uint, int128/int256/int) and non-numeric or
    /// width-unknown shapes — the caller treats `None` as the fail-safe wide
    /// case (existing checked_* at 128 bits already reverts correctly, and
    /// `unchecked` wraps at 128 bits via `wrapping_*`).
    ///
    /// Width composition mirrors Solidity: a literal has no intrinsic width
    /// (`None`) and adopts its sibling operand's width; a binary op's width is
    /// the wider of its operands; an explicit `uintN(x)`/`intN(x)` cast yields
    /// width N.
    fn expr_width(&self, e: &Expression) -> Option<u32> {
        match e {
            Expression::Parenthesis(_, inner) => self.expr_width(inner),

            // Variables: storage scalar / param / local declared width.
            Expression::Variable(id) => self.widths.get(&id.name).copied(),

            // Literals carry no intrinsic width.
            Expression::NumberLiteral(..)
            | Expression::HexNumberLiteral(..)
            | Expression::BoolLiteral(..) => None,

            // Binary numeric ops: result width = max of operand widths.
            Expression::Add(_, l, r)
            | Expression::Subtract(_, l, r)
            | Expression::Multiply(_, l, r)
            | Expression::Divide(_, l, r)
            | Expression::Modulo(_, l, r)
            | Expression::BitwiseAnd(_, l, r)
            | Expression::BitwiseOr(_, l, r)
            | Expression::BitwiseXor(_, l, r) => max_width(self.expr_width(l), self.expr_width(r)),

            // Shift result width = the left operand's width.
            Expression::ShiftLeft(_, l, _) | Expression::ShiftRight(_, l, _) => self.expr_width(l),

            Expression::BitwiseNot(_, inner) | Expression::Not(_, inner) => self.expr_width(inner),

            // Explicit width casts `uintN(x)` / `intN(x)`.
            Expression::FunctionCall(_, callee, _) => match callee.as_ref() {
                Expression::Type(_, PtType::Uint(n)) | Expression::Type(_, PtType::Int(n)) => {
                    narrow_width(*n)
                }
                Expression::Variable(id) => cast_name_width(&id.name),
                _ => None,
            },

            // Mapping/array element read: width is the declared value width,
            // which we don't currently thread (stored as u128/i128). Treat as
            // wide — fail-safe, never a silent narrow miscompile.
            _ => None,
        }
    }

    /// Render a checked arithmetic op, propagating signedness and applying the
    /// Solidity width semantics:
    ///
    /// * default (checked) mode + narrow width N: revert if the u128/i128
    ///   result falls outside the type's range (`[0, 2^N-1]` unsigned;
    ///   `[-2^(N-1), 2^(N-1)-1]` signed).
    /// * `unchecked { }` + narrow width N: wrap to the type width
    ///   (`& MASK_N` unsigned, sign-extend-from-N signed).
    /// * wide (≥128): existing `checked_*` (checked) / `wrapping_*` (unchecked).
    fn arith(&mut self, l: &Expression, r: &Expression, op: &str) -> (String, ValTy) {
        let width = max_width(self.expr_width(l), self.expr_width(r));
        let (ls, lt) = self.expr_ty(l);
        let (rs, rt) = self.expr_ty(r);
        let signed = lt == ValTy::SNum || rt == ValTy::SNum;
        if signed {
            let lc = coerce_num(&ls, lt, ValTy::SNum);
            let rc = coerce_num(&rs, rt, ValTy::SNum);
            (self.num_binop(&lc, &rc, op, width, true), ValTy::SNum)
        } else {
            (self.num_binop(&ls, &rs, op, width, false), ValTy::Num)
        }
    }

    /// Emit a width-aware numeric binary op. `op` is a `checked_*` method name.
    /// `width` is the result's narrow width (`None` = wide/fail-safe). `signed`
    /// selects i128 vs u128 semantics. Honors `self.unchecked`.
    fn num_binop(&self, lc: &str, rc: &str, op: &str, width: Option<u32>, signed: bool) -> String {
        match (self.unchecked, width) {
            // ---- checked (default) ----
            (false, None) => {
                // Wide: existing fail-safe checked at the 128-bit boundary.
                format!("({lc}).{op}({rc}).unwrap_or_else(|| revert())")
            }
            (false, Some(n)) if signed => {
                let (lo, hi) = signed_bounds(n);
                format!(
                    "{{ let __r = ({lc}).{op}({rc}).unwrap_or_else(|| revert()); \
                     if __r < {lo}i128 || __r > {hi}i128 {{ revert() }} __r }}"
                )
            }
            (false, Some(n)) => {
                let mask = unsigned_mask(n);
                format!(
                    "{{ let __r = ({lc}).{op}({rc}).unwrap_or_else(|| revert()); \
                     if __r > {mask}u128 {{ revert() }} __r }}"
                )
            }
            // ---- unchecked (wrap) ----
            (true, None) => {
                let wrap = wrapping_of(op);
                format!("({lc}).{wrap}({rc})")
            }
            (true, Some(n)) if signed => {
                let wrap = wrapping_of(op);
                // Wrap at the 128-bit op, then reduce modulo 2^N into the
                // signed range by sign-extending the low N bits: shift the
                // sign bit (bit N-1) up to bit 127 then arithmetic-shift back.
                let sh = 128 - n;
                format!(
                    "((((({lc}).{wrap}({rc})) << {sh}u32) >> {sh}u32))"
                )
            }
            (true, Some(n)) => {
                let wrap = wrapping_of(op);
                let mask = unsigned_mask(n);
                format!("(({lc}).{wrap}({rc}) & {mask}u128)")
            }
        }
    }

    fn cmp(&mut self, l: &Expression, r: &Expression, op: &str) -> String {
        let (ls, lt) = self.expr_ty(l);
        let (rs, rt) = self.expr_ty(r);
        // Coerce to a common numeric type for signed compares.
        if lt == ValTy::SNum || rt == ValTy::SNum {
            let lc = coerce_num(&ls, lt, ValTy::SNum);
            let rc = coerce_num(&rs, rt, ValTy::SNum);
            return format!("({lc} {op} {rc})");
        }
        format!("({ls} {op} {rs})")
    }

    /// Bitwise `& | ^` on numeric operands.
    fn bitop(&mut self, l: &Expression, r: &Expression, op: &str) -> String {
        let (ls, _) = self.expr_ty(l);
        let (rs, _) = self.expr_ty(r);
        format!("(({ls}) {op} ({rs}))")
    }

    /// Bitwise shift `<< >>` (wrapping on the shift amount as u32). Solidity's
    /// `<<`/`>>` never revert — a left shift truncates to the type width — so a
    /// narrow left operand masks the result to its width regardless of
    /// checked/unchecked context.
    fn shift(&mut self, l: &Expression, r: &Expression, method: &str) -> String {
        let width = self.expr_width(l);
        let (ls, _) = self.expr_ty(l);
        let (rs, _) = self.expr_ty(r);
        let raw = format!("(({ls}).{method}(({rs}) as u32))");
        mask_narrow(&raw, width)
    }

    /// If `e` is a mapping access `m[k]` or `m[a][b]` rooted at a storage
    /// mapping, return `(slot_index, value_kind, [(key_expr, key_kind)...])`.
    fn as_map_access(&mut self, e: &Expression) -> Option<(u8, ValTy, Vec<(String, ValTy)>)> {
        if let Expression::ArraySubscript(_, base, Some(index)) = e {
            match base.as_ref() {
                Expression::Variable(id) => {
                    let (idx, key_kind, val) = match self.slot_of(&id.name)?.kind {
                        SlotKind::Map { key, val } => (self.slot_of(&id.name)?.index, key, val),
                        _ => return None,
                    };
                    let key = self.encode_key(index, key_kind);
                    Some((idx, val, vec![(key, key_kind)]))
                }
                Expression::ArraySubscript(_, inner_base, Some(inner_idx)) => {
                    // nested: base is m[a], so this is m[a][b].
                    if let Expression::Variable(id) = inner_base.as_ref() {
                        let slot = self.slot_of(&id.name)?;
                        if let SlotKind::Map2 { key1, key2, val } = slot.kind {
                            let idx = slot.index;
                            let a = self.encode_key(inner_idx, key1);
                            let b = self.encode_key(index, key2);
                            return Some((idx, val, vec![(a, key1), (b, key2)]));
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// If `base` is `m[k]` over a `mapping(K=>Struct)` slot and `field` is a
    /// known struct field, return the storage-key call computing
    /// `blake2(slot ++ key_bytes ++ [field_index])` and the field's kind.
    fn struct_field_key(&mut self, base: &Expression, field: &str) -> Option<(String, ValTy)> {
        if let Expression::ArraySubscript(_, mbase, Some(index)) = base {
            if let Expression::Variable(id) = mbase.as_ref() {
                let slot = self.slot_of(&id.name)?;
                if let SlotKind::MapStruct { key, fields } = &slot.kind {
                    let key_kind = *key;
                    let fields = fields.clone();
                    let slot_idx = slot.index;
                    let (fi, (_, fkind)) =
                        fields.iter().enumerate().find(|(_, (n, _))| n == field)?;
                    let key_str = self.encode_key(index, key_kind);
                    self.uses.blake2 = true;
                    let kpart = match key_kind {
                        ValTy::Addr => format!("MapKey::Addr(&{key_str})"),
                        ValTy::Bool => format!("MapKey::Byte({key_str} as u8)"),
                        _ => format!("MapKey::Word(({key_str}) as u128)"),
                    };
                    let call = format!(
                        "map_key(&[{kpart}, MapKey::Byte({fi}u8)], {slot_idx})"
                    );
                    return Some((call, *fkind));
                }
            }
        }
        None
    }

    /// If `ty` names a known struct, return its name.
    fn struct_type_name(&self, ty: &Expression) -> Option<String> {
        if let Expression::Variable(id) = ty {
            if self.structs.contains_key(&id.name) {
                return Some(id.name.clone());
            }
        }
        None
    }

    /// Read `local.field` for a struct local. Returns the backing local + kind.
    fn local_struct_field_read(&self, var: &str, field: &str) -> Option<(String, ValTy)> {
        let sname = self.local_structs.get(var)?;
        let fields = self.structs.get(sname)?;
        let (_, fkind) = fields.iter().find(|(n, _)| n == field)?;
        Some((SealCtx::struct_local(var, field), *fkind))
    }

    /// Declare a struct local from `Point(a, b)` (or another struct local),
    /// binding each field to a backing scalar local `__s_<var>_<field>`.
    fn decl_struct_local(
        &mut self,
        name: &str,
        sname: &str,
        init: Option<&Expression>,
    ) -> Vec<String> {
        let fields = match self.structs.get(sname) {
            Some(f) => f.clone(),
            None => return vec![format!("// {}", self.err("unknown struct type"))],
        };
        self.local_structs.insert(name.to_string(), sname.to_string());
        let mut out = Vec::new();
        match init {
            // `Point(a, b)` positional constructor.
            Some(Expression::FunctionCall(_, callee, args)) if Self::is_struct_ctor(callee, sname) => {
                if args.len() != fields.len() {
                    return vec![format!(
                        "// {}",
                        self.err("struct constructor arity mismatch")
                    )];
                }
                for (i, (fname, fkind)) in fields.iter().enumerate() {
                    let (v, t) = self.expr_ty(&args[i]);
                    let v = match fkind {
                        ValTy::Num | ValTy::SNum => coerce_num(&v, t, *fkind),
                        _ => v,
                    };
                    out.push(format!("let mut {} = {v};", SealCtx::struct_local(name, fname)));
                }
            }
            // `Point memory q = p;` copy from another struct local.
            Some(Expression::Variable(id)) if self.local_structs.contains_key(&id.name) => {
                let src = id.name.clone();
                for (fname, _) in &fields {
                    out.push(format!(
                        "let mut {} = {};",
                        SealCtx::struct_local(name, fname),
                        SealCtx::struct_local(&src, fname)
                    ));
                }
            }
            None => {
                // Zero-initialized struct local.
                for (fname, fkind) in &fields {
                    let zero = match fkind {
                        ValTy::Bool => "false",
                        ValTy::Addr => "[0u8; 32]",
                        ValTy::SNum => "0i128",
                        _ => "0u128",
                    };
                    out.push(format!("let mut {} = {zero};", SealCtx::struct_local(name, fname)));
                }
            }
            _ => {
                out.push(format!("// {}", self.err("unsupported struct local initializer")));
            }
        }
        out
    }

    /// Is `callee` a constructor call for struct `sname`?
    fn is_struct_ctor(callee: &Expression, sname: &str) -> bool {
        matches!(callee, Expression::Variable(id) if id.name == sname)
    }

    /// Recognize a cross-contract call `IFoo(addr).method(args)`. `outer_callee`
    /// is the `MemberAccess(FunctionCall(Variable(IFoo), [addr]), method)` node;
    /// `args` are the method arguments. Returns the rendered expression (an
    /// inline block performing `seal_call` and decoding the return) + its kind.
    fn cross_call(
        &mut self,
        outer_callee: &Expression,
        args: &[Expression],
    ) -> Option<(String, ValTy)> {
        let (base, method) = match outer_callee {
            Expression::MemberAccess(_, base, member) => (base.as_ref(), member.name.clone()),
            _ => return None,
        };
        // base must be `IFoo(addr)` — a cast of an address through an interface.
        let addr_expr = match base {
            Expression::FunctionCall(_, callee, cargs) => match callee.as_ref() {
                Expression::Variable(_) if cargs.len() == 1 => &cargs[0],
                _ => return None,
            },
            _ => return None,
        };
        let sig = self.external_fns.get(&method)?.clone();
        // Encode the address operand.
        let (addr_s, _) = self.expr_ty(addr_expr);
        // Build the call-input prelude: 4-byte selector ++ SCALE(args).
        let mut input_len = 4usize;
        let mut writes: Vec<String> = Vec::new();
        let s = sig.selector;
        writes.push(format!(
            "__ci[0]={}u8; __ci[1]={}u8; __ci[2]={}u8; __ci[3]={}u8;",
            s[0], s[1], s[2], s[3]
        ));
        for (i, a) in args.iter().enumerate() {
            let want = sig.args.get(i).copied().unwrap_or(ValTy::Num);
            let (av, at) = self.expr_ty(a);
            match want {
                ValTy::Addr => {
                    writes.push(format!(
                        "{{ let __a: [u8;32] = {av}; let mut __k=0usize; while __k<32 {{ __ci[{input_len}+__k]=__a[__k]; __k+=1; }} }}"
                    ));
                    input_len += 32;
                }
                ValTy::Bool => {
                    writes.push(format!("__ci[{input_len}] = ({av}) as u8;"));
                    input_len += 1;
                }
                _ => {
                    let v = coerce_num(&av, at, ValTy::Num);
                    writes.push(format!(
                        "{{ let __le = ({v}).to_le_bytes(); let mut __k=0usize; while __k<16 {{ __ci[{input_len}+__k]=__le[__k]; __k+=1; }} }}"
                    ));
                    input_len += 16;
                }
            }
        }
        self.uses.call = true;
        // Output decode.
        let (decode, vt) = match sig.ret {
            Some(ValTy::Addr) => (
                "{ let mut __o=[0u8;32]; let __n=do_call(&__dest,&__ci[..__cil],__o.as_mut_ptr(),32); let _=__n; __o }".to_string(),
                ValTy::Addr,
            ),
            Some(ValTy::Bool) => (
                "{ let mut __o=[0u8;1]; do_call(&__dest,&__ci[..__cil],__o.as_mut_ptr(),1); __o[0]!=0 }".to_string(),
                ValTy::Bool,
            ),
            Some(_) => (
                "{ let mut __o=[0u8;16]; do_call(&__dest,&__ci[..__cil],__o.as_mut_ptr(),16); u128::from_le_bytes(__o) }".to_string(),
                ValTy::Num,
            ),
            None => (
                "{ let mut __o=[0u8;1]; do_call(&__dest,&__ci[..__cil],__o.as_mut_ptr(),0); }".to_string(),
                ValTy::Num,
            ),
        };
        // Assemble the full inline block.
        let mut block = String::from("{ ");
        block.push_str(&format!("let __dest: [u8;32] = {addr_s}; "));
        block.push_str(&format!(
            "let mut __ci_u = MaybeUninit::<[u8; {}]>::uninit(); let __ci = unsafe {{ &mut *__ci_u.as_mut_ptr() }}; ",
            input_len.max(4)
        ));
        for w in &writes {
            block.push_str(w);
            block.push(' ');
        }
        block.push_str(&format!("let __cil = {input_len}usize; "));
        block.push_str(&decode);
        block.push_str(" }");
        Some((block, vt))
    }

    /// Render a `m[k].field` read.
    fn struct_field_read(&mut self, base: &Expression, field: &str) -> Option<(String, ValTy)> {
        let (key_call, fkind) = self.struct_field_key(base, field)?;
        let read = match fkind {
            ValTy::Bool => format!("map_get_bool({key_call})"),
            ValTy::Addr => format!("map_get_addr({key_call})"),
            ValTy::SNum => format!("(map_get_u128({key_call}) as i128)"),
            _ => format!("map_get_u128({key_call})"),
        };
        Some((read, fkind))
    }

    /// Render a key expression coerced to the slot's declared key kind (so a
    /// numeric literal indexing a `mapping(uint=>..)` is `u128`, etc).
    fn encode_key(&mut self, e: &Expression, kind: ValTy) -> String {
        let (s, t) = self.expr_ty(e);
        match kind {
            ValTy::Addr | ValTy::Bool | ValTy::Str => s,
            ValTy::Num => coerce_num(&s, t, ValTy::Num),
            ValTy::SNum => coerce_num(&s, t, ValTy::SNum),
        }
    }

    /// Emit the call that computes a mapping storage key for the given slot and
    /// key expressions. Key bytes are the SCALE/raw encoding of each key: an
    /// address key is 32 raw bytes, a u128/i128 key is 16 LE bytes, a bool key
    /// is 1 byte. The preimage is `[slot] ++ key_bytes...` hashed with blake2.
    fn map_key_call(&mut self, slot: u8, keys: &[(String, ValTy)]) -> String {
        self.uses.blake2 = true;
        let parts: Vec<String> = keys
            .iter()
            .map(|(expr, kind)| match kind {
                ValTy::Addr => format!("MapKey::Addr(&{expr})"),
                ValTy::Bool => format!("MapKey::Byte({expr} as u8)"),
                _ => format!("MapKey::Word(({expr}) as u128)"),
            })
            .collect();
        format!("map_key(&[{}], {slot})", parts.join(", "))
    }

    /// Render a statement into Rust source lines.
    fn stmt(&mut self, s: &Statement) -> Vec<String> {
        match s {
            Statement::Block { statements, unchecked, .. } => {
                // Honor Solidity `unchecked { }`: arithmetic inside wraps to the
                // type width instead of reverting. Nested blocks inherit the
                // flag; restore the prior state on exit.
                let prev = self.unchecked;
                if *unchecked {
                    self.unchecked = true;
                }
                let out: Vec<String> =
                    statements.iter().flat_map(|st| self.stmt(st)).collect();
                self.unchecked = prev;
                out
            }

            Statement::Expression(_, e) => self.expr_stmt(e),

            Statement::Return(_, Some(e)) => {
                // Multi-return: `return (a, b, ...)` -> SCALE-concat and `ret()`
                // inline (diverges, so control flow matches Solidity's return).
                if self.ret_kinds.len() > 1 {
                    if let Some(items) = list_items(e) {
                        let kinds = self.ret_kinds.clone();
                        let mut vals: Vec<(String, ValTy)> = Vec::new();
                        for (i, item) in items.iter().enumerate() {
                            let want = kinds.get(i).copied().unwrap_or(ValTy::Num);
                            let (v, t) = self.expr_ty(item);
                            let coerced = match want {
                                ValTy::Num | ValTy::SNum => coerce_num(&v, t, want),
                                _ => v,
                            };
                            vals.push((coerced, want));
                        }
                        return emit_tuple_ret(&vals);
                    }
                    self.err("multi-return expects a tuple `return (a, b)`");
                    return vec![];
                }
                // string/bytes return: read the stored SCALE blob and `ret` it
                // directly (it is already compact(len) ++ bytes). Diverges.
                if self.ret_kinds.first() == Some(&ValTy::Str) {
                    if let Expression::Variable(id) = e {
                        if let Some(slot) = self.slot_of(&id.name) {
                            if matches!(slot.kind, SlotKind::Scalar(ValTy::Str)) {
                                return vec![format!("return_str_{}();", slot.index)];
                            }
                        }
                    }
                    self.err("string return must return a string storage variable");
                    return vec![];
                }
                let (v, t) = self.expr_ty(e);
                vec![format!("__ret = Some({});", wrap_ret(&v, t))]
            }
            Statement::Return(_, None) => vec!["return;".to_string()],

            Statement::If(_, cond, then_b, else_b) => {
                let mut out = Vec::new();
                let c = self.expr(cond);
                out.push(format!("if {c} {{"));
                for l in self.stmt(then_b) {
                    out.push(format!("    {l}"));
                }
                if let Some(eb) = else_b {
                    out.push("} else {".to_string());
                    for l in self.stmt(eb) {
                        out.push(format!("    {l}"));
                    }
                }
                out.push("}".to_string());
                out
            }

            // `for (init; cond; post) { body }` -> Rust `while` loop.
            Statement::For(_, init, cond, post, body) => {
                let mut out = Vec::new();
                out.push("{".to_string());
                if let Some(init) = init {
                    for l in self.stmt(init) {
                        out.push(format!("    {l}"));
                    }
                }
                let c = match cond {
                    Some(c) => self.expr(c),
                    None => "true".to_string(),
                };
                out.push(format!("    while {c} {{"));
                if let Some(body) = body {
                    for l in self.stmt(body) {
                        out.push(format!("        {l}"));
                    }
                }
                if let Some(post) = post {
                    // `post` is an expression (e.g. i++); reuse statement lowering.
                    for l in self.expr_stmt(post) {
                        out.push(format!("        {l}"));
                    }
                }
                out.push("    }".to_string());
                out.push("}".to_string());
                out
            }

            Statement::While(_, cond, body) => {
                let mut out = Vec::new();
                let c = self.expr(cond);
                out.push(format!("while {c} {{"));
                for l in self.stmt(body) {
                    out.push(format!("    {l}"));
                }
                out.push("}".to_string());
                out
            }

            Statement::DoWhile(_, body, cond) => {
                let mut out = Vec::new();
                out.push("loop {".to_string());
                for l in self.stmt(body) {
                    out.push(format!("    {l}"));
                }
                let c = self.expr(cond);
                out.push(format!("    if !({c}) {{ break; }}"));
                out.push("}".to_string());
                out
            }

            Statement::Emit(_, e) => self.emit_stmt(e),

            Statement::Revert(_, _path, _args) => vec!["revert();".to_string()],

            // Local variable declaration `T name = expr;`.
            Statement::VariableDefinition(_, decl, init) => {
                let name = decl.name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                // Struct local: `Point memory p = Point(a, b);`.
                if let Some(sname) = self.struct_type_name(&decl.ty) {
                    return self.decl_struct_local(&name, &sname, init.as_ref());
                }
                // Locals are `mut` (loop counters / reassigned vars); the crate
                // sets `#![allow(unused_mut)]`.
                // Record the local's declared narrow width (if any) so later
                // arithmetic on it applies the correct overflow semantics.
                if let Some(w) = pt_type_width(&decl.ty) {
                    self.widths.insert(name.clone(), w);
                } else {
                    self.widths.remove(&name);
                }
                if let Some(rhs) = init {
                    let (v, t) = self.expr_ty(rhs);
                    self.locals.insert(name.clone(), t);
                    vec![format!("let mut {name} = {v};")]
                } else {
                    self.locals.insert(name.clone(), ValTy::Num);
                    vec![format!("let mut {name};")]
                }
            }

            _ => vec![format!("// {}", self.err("statement"))],
        }
    }

    /// Lower an expression used in statement position.
    fn expr_stmt(&mut self, e: &Expression) -> Vec<String> {
        match e {
            Expression::Assign(_, lhs, rhs) => self.assign(lhs, rhs),
            // Compound assignment: a += b, a -= b, ...
            Expression::AssignAdd(_, lhs, rhs) => self.compound(lhs, rhs, "checked_add"),
            Expression::AssignSubtract(_, lhs, rhs) => self.compound(lhs, rhs, "checked_sub"),
            Expression::AssignMultiply(_, lhs, rhs) => self.compound(lhs, rhs, "checked_mul"),
            Expression::AssignDivide(_, lhs, rhs) => self.compound(lhs, rhs, "checked_div"),
            Expression::AssignModulo(_, lhs, rhs) => self.compound(lhs, rhs, "checked_rem"),
            Expression::AssignOr(_, lhs, rhs) => self.compound_bit(lhs, rhs, "|"),
            Expression::AssignAnd(_, lhs, rhs) => self.compound_bit(lhs, rhs, "&"),
            Expression::AssignXor(_, lhs, rhs) => self.compound_bit(lhs, rhs, "^"),
            Expression::AssignShiftLeft(_, lhs, rhs) => self.compound_shift(lhs, rhs, "wrapping_shl"),
            Expression::AssignShiftRight(_, lhs, rhs) => {
                self.compound_shift(lhs, rhs, "wrapping_shr")
            }
            // Increment / decrement statements: n++, ++n, n--, --n.
            Expression::PostIncrement(_, inner) | Expression::PreIncrement(_, inner) => {
                self.incdec(inner, "checked_add")
            }
            Expression::PostDecrement(_, inner) | Expression::PreDecrement(_, inner) => {
                self.incdec(inner, "checked_sub")
            }
            Expression::FunctionCall(_, callee, args) => self.call_stmt(callee, args),
            _ => {
                let r = self.expr(e);
                vec![format!("let _ = {r};")]
            }
        }
    }

    /// Compound arithmetic assignment `lhs OP= rhs` (checked).
    fn compound(&mut self, lhs: &Expression, rhs: &Expression, op: &str) -> Vec<String> {
        // Read current value, apply checked op, write back. Implemented by
        // synthesizing `lhs = (lhs).op(rhs)` through the normal read/write paths.
        // The result is bounded by the lvalue's declared width (Solidity
        // evaluates `a OP= b` at the type of `a`).
        let width = self.expr_width(lhs);
        let (cur, ct) = self.expr_ty(lhs);
        let (val, vt) = self.expr_ty(rhs);
        let signed = ct == ValTy::SNum || vt == ValTy::SNum;
        let target = if signed { ValTy::SNum } else { ValTy::Num };
        let lc = coerce_num(&cur, ct, target);
        let rc = coerce_num(&val, vt, target);
        let combined = self.num_binop(&lc, &rc, op, width, signed);
        self.write_back(lhs, &combined, target)
    }

    /// Compound bitwise assignment `lhs OP= rhs`.
    fn compound_bit(&mut self, lhs: &Expression, rhs: &Expression, op: &str) -> Vec<String> {
        let (cur, ct) = self.expr_ty(lhs);
        let (val, _) = self.expr_ty(rhs);
        let combined = format!("(({cur}) {op} ({val}))");
        self.write_back(lhs, &combined, ct)
    }

    /// Compound shift assignment `lhs <<= rhs` / `lhs >>= rhs`.
    fn compound_shift(&mut self, lhs: &Expression, rhs: &Expression, method: &str) -> Vec<String> {
        let width = self.expr_width(lhs);
        let (cur, ct) = self.expr_ty(lhs);
        let (val, _) = self.expr_ty(rhs);
        let raw = format!("(({cur}).{method}(({val}) as u32))");
        let combined = mask_narrow(&raw, width);
        self.write_back(lhs, &combined, ct)
    }

    /// `n++` / `n--` as a statement: read, checked +/-1, write back.
    fn incdec(&mut self, target: &Expression, op: &str) -> Vec<String> {
        let width = self.expr_width(target);
        let (cur, ct) = self.expr_ty(target);
        let signed = ct == ValTy::SNum;
        let one = if signed { "1i128" } else { "1u128" };
        let combined = self.num_binop(&cur, one, op, width, signed);
        self.write_back(target, &combined, ct)
    }

    /// Write `value` (of kind `vt`) back to an lvalue (storage scalar, mapping
    /// element, or local).
    fn write_back(&mut self, lhs: &Expression, value: &str, vt: ValTy) -> Vec<String> {
        // `localStruct.field = value` write to a struct local.
        if let Expression::MemberAccess(_, base, member) = lhs {
            if let Expression::Variable(id) = base.as_ref() {
                if let Some((target, fkind)) =
                    self.local_struct_field_read(&id.name, &member.name)
                {
                    let coerced = match fkind {
                        ValTy::Num | ValTy::SNum => coerce_num(value, vt, fkind),
                        _ => value.to_string(),
                    };
                    return vec![format!("{target} = {coerced};")];
                }
            }
        }
        // `m[k].field = value` struct-field write.
        if let Expression::MemberAccess(_, base, member) = lhs {
            if let Some((key_call, fkind)) = self.struct_field_key(base, &member.name) {
                let setter = match fkind {
                    ValTy::Bool => format!("map_set_bool({key_call}, {value});"),
                    ValTy::Addr => format!("map_set_addr({key_call}, &{value});"),
                    _ => {
                        let coerced = coerce_num(value, vt, fkind);
                        format!("map_set_u128({key_call}, ({coerced}) as u128);")
                    }
                };
                return vec![setter];
            }
        }
        // `a[i] = value` dynamic-array element write.
        if let Expression::ArraySubscript(_, base, Some(index)) = lhs {
            if let Expression::Variable(id) = base.as_ref() {
                if let Some(slot) = self.slot_of(&id.name) {
                    if let SlotKind::Array { elem } = slot.kind {
                        let s = slot.index;
                        let (ix, it) = self.expr_ty(index);
                        let ix = coerce_num(&ix, it, ValTy::Num);
                        let setter = match elem {
                            ValTy::Bool => format!("arr_set_bool_{s}({ix}, {value});"),
                            ValTy::Addr => format!("arr_set_addr_{s}({ix}, &{value});"),
                            _ => {
                                let coerced = coerce_num(value, vt, elem);
                                format!("arr_set_u128_{s}({ix}, ({coerced}) as u128);")
                            }
                        };
                        return vec![setter];
                    }
                }
            }
        }
        if let Expression::ArraySubscript(..) = lhs {
            if let Some((slot_idx, val, keys)) = self.as_map_access(lhs) {
                let key_expr = self.map_key_call(slot_idx, &keys);
                let setter = match val {
                    ValTy::Bool => format!("map_set_bool({key_expr}, {value});"),
                    ValTy::Addr => format!("map_set_addr({key_expr}, &{value});"),
                    _ => {
                        let coerced = coerce_num(value, vt, val);
                        format!("map_set_u128({key_expr}, ({coerced}) as u128);")
                    }
                };
                return vec![setter];
            }
        }
        if let Expression::Variable(id) = lhs {
            if let Some(slot) = self.slot_of(&id.name) {
                if let SlotKind::Scalar(svt) = slot.kind {
                    let idx = slot.index;
                    let coerced = coerce_num(value, vt, svt);
                    let store = match svt {
                        ValTy::SNum => format!("store_slot_{idx}(({coerced}) as u128);"),
                        _ => format!("store_slot_{idx}({coerced});"),
                    };
                    return vec![store];
                }
            }
            return vec![format!("{} = {value};", id.name)];
        }
        vec![format!("// {}", self.err("compound assignment target"))]
    }

    /// `lhs = rhs;` — scalar storage write, mapping write, or local.
    fn assign(&mut self, lhs: &Expression, rhs: &Expression) -> Vec<String> {
        // string/bytes storage write: `strvar = strparam`.
        if let Expression::Variable(lid) = lhs {
            if let Some(slot) = self.slot_of(&lid.name) {
                if matches!(slot.kind, SlotKind::Scalar(ValTy::Str)) {
                    let sidx = slot.index;
                    if let Expression::Variable(rid) = rhs {
                        if self.locals.get(&rid.name) == Some(&ValTy::Str) {
                            let n = &rid.name;
                            return vec![format!(
                                "store_str_{sidx}(__str_{n}_ptr, __str_{n}_total);"
                            )];
                        }
                    }
                    return vec![format!("// {}", self.err("string assignment requires a string parameter source"))];
                }
            }
        }
        let (value, vt) = self.expr_ty(rhs);
        // Struct-field write `m[k].field = v` or `localStruct.field = v`.
        if let Expression::MemberAccess(_, base, member) = lhs {
            if self.struct_field_key(base, &member.name).is_some() {
                return self.write_back(lhs, &value, vt);
            }
            if let Expression::Variable(id) = base.as_ref() {
                if self.local_struct_field_read(&id.name, &member.name).is_some() {
                    return self.write_back(lhs, &value, vt);
                }
            }
        }
        // Array-element or mapping write `a[i] = v` / `m[k] = v`.
        if let Expression::ArraySubscript(_, base, Some(_)) = lhs {
            if let Expression::Variable(id) = base.as_ref() {
                if matches!(
                    self.slot_of(&id.name).map(|s| &s.kind),
                    Some(SlotKind::Array { .. })
                ) {
                    return self.write_back(lhs, &value, vt);
                }
            }
            if self.as_map_access(lhs).is_some() {
                return self.write_back(lhs, &value, vt);
            }
        }
        if let Expression::Variable(id) = lhs {
            if let Some(slot) = self.slot_of(&id.name) {
                if let SlotKind::Scalar(_) = slot.kind {
                    return self.write_back(lhs, &value, vt);
                }
            }
            // Reassign an already-declared local with plain `=` (so the update
            // is visible after a loop); declare a fresh `let mut` otherwise.
            if self.locals.contains_key(&id.name) {
                return vec![format!("{} = {value};", id.name)];
            }
            self.locals.insert(id.name.clone(), vt);
            return vec![format!("let mut {} = {value};", id.name)];
        }
        vec![format!("// {}", self.err("assignment target"))]
    }

    /// A bare call statement: `require(...)`, `revert()`, `addr.transfer(x)`.
    fn call_stmt(&mut self, callee: &Expression, args: &[Expression]) -> Vec<String> {
        // Cross-contract call as a statement: `IFoo(addr).bar(args);`.
        if let Some((expr, _)) = self.cross_call(callee, args) {
            return vec![format!("let _ = {expr};")];
        }
        // arr.push(x) on a dynamic-array storage var.
        if let Expression::MemberAccess(_, base, member) = callee {
            if member.name == "push" {
                if let Expression::Variable(id) = base.as_ref() {
                    if let Some(slot) = self.slot_of(&id.name) {
                        if let SlotKind::Array { elem } = slot.kind {
                            let s = slot.index;
                            let (v, t) = args
                                .first()
                                .map(|a| self.expr_ty(a))
                                .unwrap_or(("0u128".to_string(), ValTy::Num));
                            let push = match elem {
                                ValTy::Bool => format!("arr_push_bool_{s}({v});"),
                                ValTy::Addr => format!("arr_push_addr_{s}(&{v});"),
                                _ => {
                                    let c = coerce_num(&v, t, elem);
                                    format!("arr_push_u128_{s}(({c}) as u128);")
                                }
                            };
                            return vec![push];
                        }
                    }
                }
            }
        }
        // payable(addr).transfer(amount) / addr.transfer(amount)
        if let Expression::MemberAccess(_, base, member) = callee {
            if member.name == "transfer" || member.name == "send" {
                let (acct, _) = self.expr_ty(base);
                let amount = args.first().map(|a| self.expr(a)).unwrap_or_else(|| "0u128".into());
                self.uses.transfer = true;
                return vec![format!("do_transfer(&{acct}, {amount});")];
            }
        }
        if let Expression::Variable(id) = callee {
            match id.name.as_str() {
                "require" => {
                    if let Some(cond) = args.first() {
                        let c = self.expr(cond);
                        return vec![format!("if !({c}) {{ revert(); }}")];
                    }
                    return vec!["// require() with no condition".to_string()];
                }
                "revert" => return vec!["revert();".to_string()],
                "assert" => {
                    if let Some(cond) = args.first() {
                        let c = self.expr(cond);
                        return vec![format!("if !({c}) {{ revert(); }}")];
                    }
                }
                _ => {}
            }
        }
        vec![format!("// {}", self.err("call statement"))]
    }

    /// `emit E(args)` -> `seal_deposit_event`.
    fn emit_stmt(&mut self, e: &Expression) -> Vec<String> {
        if let Expression::FunctionCall(_, callee, args) = e {
            if let Expression::Variable(id) = callee.as_ref() {
                let ev = match self.events.iter().find(|ev| ev.name == id.name) {
                    Some(ev) => ev.clone(),
                    None => return vec![format!("// {}", self.err("unknown event"))],
                };
                self.uses.deposit_event = true;
                let mut lines = Vec::new();
                // Evaluate each field expression into a typed local.
                let mut field_vars: Vec<(String, ValTy)> = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let (v, t) = self.expr_ty(a);
                    let var = format!("__ev{i}");
                    lines.push(format!("let {var} = {v};"));
                    field_vars.push((var, t));
                }
                // topics: Vec<[u8;32]> of indexed-field values, plus event sig
                // hash as first topic. SCALE = compact(len) ++ each 32 bytes.
                let indexed: Vec<&(String, ValTy)> = field_vars
                    .iter()
                    .zip(ev.fields.iter())
                    .filter(|(_, f)| f.indexed)
                    .map(|(v, _)| v)
                    .collect();
                let n_topics = indexed.len() + 1; // + signature topic
                let topics_bytes = 1 + n_topics * 32; // 1-byte compact len (n<64)
                // Uninitialized buffer (avoids memory.fill); all bytes written below.
                lines.push(format!(
                    "let mut __topics_u = MaybeUninit::<[u8; {topics_bytes}]>::uninit();"
                ));
                lines.push("let __topics = unsafe { &mut *__topics_u.as_mut_ptr() };".to_string());
                lines.push(format!("__topics[0] = {};", (n_topics as u32) << 2));
                // first topic = keccak256(canonical event signature), the real
                // ABI event topic, precomputed at translate time and written as
                // a byte literal (no in-wasm keccak needed; the node ignores
                // topics but this keeps the emitted payload ABI-faithful).
                let topic = keccak256(event_sig_string(&ev).as_bytes());
                for (j, b) in topic.iter().enumerate() {
                    lines.push(format!("__topics[1 + {j}] = {b}u8;"));
                }
                let mut off = 1 + 32;
                for (var, vt) in &indexed {
                    match vt {
                        ValTy::Addr => {
                            lines.push(format!(
                                "{{ let mut __i = 0usize; while __i < 32 {{ __topics[{off} + __i] = {var}[__i]; __i += 1; }} }}"
                            ));
                        }
                        _ => {
                            // numeric/bool indexed: left-justify LE bytes into 32.
                            lines.push(format!(
                                "{{ let __le = ({var} as u128).to_le_bytes(); let mut __i = 0usize; while __i < 16 {{ __topics[{off} + __i] = __le[__i]; __i += 1; }} let mut __z = 16usize; while __z < 32 {{ __topics[{off} + __z] = 0; __z += 1; }} }}"
                            ));
                        }
                    }
                    off += 32;
                }
                // data: SCALE of all fields in order.
                let data_len: usize = field_vars
                    .iter()
                    .map(|(_, t)| match t {
                        ValTy::Addr => 32,
                        ValTy::Bool => 1,
                        ValTy::Num | ValTy::SNum => 16,
                        ValTy::Str => 0,
                    })
                    .sum();
                lines.push(format!(
                    "let mut __data_u = MaybeUninit::<[u8; {}]>::uninit();",
                    data_len.max(1)
                ));
                lines.push("let __data = unsafe { &mut *__data_u.as_mut_ptr() };".to_string());
                let mut doff = 0usize;
                for (var, vt) in &field_vars {
                    match vt {
                        ValTy::Addr => {
                            lines.push(format!(
                                "{{ let mut __i = 0usize; while __i < 32 {{ __data[{doff} + __i] = {var}[__i]; __i += 1; }} }}"
                            ));
                            doff += 32;
                        }
                        ValTy::Bool => {
                            lines.push(format!("__data[{doff}] = {var} as u8;"));
                            doff += 1;
                        }
                        ValTy::Num | ValTy::SNum => {
                            lines.push(format!(
                                "{{ let __le = ({var} as u128).to_le_bytes(); let mut __i = 0usize; while __i < 16 {{ __data[{doff} + __i] = __le[__i]; __i += 1; }} }}"
                            ));
                            doff += 16;
                        }
                        ValTy::Str => {
                            self.err("string/bytes field in event not supported");
                        }
                    }
                }
                lines.push(format!(
                    "unsafe {{ seal_deposit_event(__topics.as_ptr(), {topics_bytes}, __data.as_ptr(), {}); }}",
                    data_len
                ));
                return lines;
            }
        }
        vec![format!("// {}", self.err("emit target"))]
    }
}

/// Wrap a return expression so all returns are encoded uniformly as bytes.
fn wrap_ret(v: &str, _t: ValTy) -> String {
    v.to_string()
}

/// Coerce a rendered numeric expression of kind `from` to kind `to`.
fn coerce_num(src: &str, from: ValTy, to: ValTy) -> String {
    if from == to {
        return src.to_string();
    }
    match to {
        ValTy::SNum => format!("({src} as i128)"),
        ValTy::Num => format!("({src} as u128)"),
        _ => src.to_string(),
    }
}

/// The wider of two optional narrow widths (`None` = wide, dominates only when
/// both are `None`; any concrete narrow width is "narrower" than wide and is
/// what bounds the Solidity result type).
fn max_width(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// Narrow width predicate: solang reports `uintN`/`intN` bit-widths in `n`.
/// Only widths strictly below 128 have semantics differing from the u128/i128
/// boundary; 128 and above are fail-safe wide (`None`).
fn narrow_width(n: u16) -> Option<u32> {
    let n = n as u32;
    if (1..128).contains(&n) {
        Some(n)
    } else {
        None
    }
}

/// Narrow width from a textual `uintN`/`intN`/`uint`/`int` cast name.
fn cast_name_width(name: &str) -> Option<u32> {
    let digits = if let Some(rest) = name.strip_prefix("uint") {
        rest
    } else if let Some(rest) = name.strip_prefix("int") {
        rest
    } else {
        return None;
    };
    if digits.is_empty() {
        // `uint`/`int` == 256-bit -> wide.
        return None;
    }
    digits.parse::<u16>().ok().and_then(narrow_width)
}

/// `2^N - 1` as a decimal string, the inclusive upper bound of `uintN`.
fn unsigned_mask(n: u32) -> String {
    (((1u128) << n) - 1).to_string()
}

/// Inclusive `(lo, hi)` two's-complement bounds of `intN` as decimal strings.
fn signed_bounds(n: u32) -> (String, String) {
    let hi = ((1i128) << (n - 1)) - 1;
    let lo = -((1i128) << (n - 1));
    (lo.to_string(), hi.to_string())
}

/// Mask a rendered numeric expression to a narrow width (`& (2^N-1)`), used for
/// shift results which truncate rather than revert. Wide width (`None`) is a
/// no-op.
fn mask_narrow(expr: &str, width: Option<u32>) -> String {
    match width {
        Some(n) => format!("(({expr}) & {}u128)", unsigned_mask(n)),
        None => expr.to_string(),
    }
}

/// Sign-extend a rendered i128 expression from its low N bits, used for signed
/// narrowing casts `intN(x)` (N<128) which truncate modulo 2^N then reinterpret
/// the low N bits as a two's-complement value (`int8(200) == -56`). Shift the
/// sign bit (bit N-1) up to bit 127 then arithmetic-shift back. Wide width
/// (`None`) is a no-op.
fn sign_extend_narrow(expr: &str, width: Option<u32>) -> String {
    match width {
        Some(n) => {
            let sh = 128 - n;
            format!("((({expr}) << {sh}u32) >> {sh}u32)")
        }
        None => expr.to_string(),
    }
}

/// The `wrapping_*` counterpart of a `checked_*` method name.
fn wrapping_of(op: &str) -> &'static str {
    match op {
        "checked_add" => "wrapping_add",
        "checked_sub" => "wrapping_sub",
        "checked_mul" => "wrapping_mul",
        "checked_div" => "wrapping_div",
        "checked_rem" => "wrapping_rem",
        _ => "wrapping_add",
    }
}

/// If `e` is a tuple/list `(a, b, ...)`, return its element expressions.
fn list_items(e: &Expression) -> Option<Vec<Expression>> {
    match e {
        Expression::List(_, params) => {
            let items: Vec<Expression> = params
                .iter()
                .filter_map(|(_, p)| p.as_ref().map(|p| p.ty.clone()))
                .collect();
            if items.is_empty() {
                None
            } else {
                Some(items)
            }
        }
        _ => None,
    }
}

/// Emit an inline SCALE-concat `ret()` for a `return (a, b, ...)` tuple. Each
/// rendered value `vals[i]` is appended LE (numeric 16 bytes, bool 1, addr 32).
/// `ret()` diverges, so this faithfully exits the function.
fn emit_tuple_ret(vals: &[(String, ValTy)]) -> Vec<String> {
    let total: usize = vals.iter().map(|(_, t)| val_len(*t)).sum();
    let mut lines = Vec::new();
    lines.push("{".to_string());
    // Bind each value first so side-effecting expressions evaluate once.
    for (i, (v, t)) in vals.iter().enumerate() {
        match t {
            ValTy::Addr => lines.push(format!("let __r{i}: [u8; 32] = {v};")),
            ValTy::Bool => lines.push(format!("let __r{i}: bool = {v};")),
            ValTy::SNum => lines.push(format!("let __r{i}: i128 = {v};")),
            ValTy::Num => lines.push(format!("let __r{i}: u128 = {v};")),
            ValTy::Str => unreachable!("string in a multi-return tuple is rejected earlier"),
        }
    }
    lines.push(format!(
        "let mut __mout = MaybeUninit::<[u8; {}]>::uninit();",
        total.max(1)
    ));
    lines.push("let __mo = unsafe { &mut *__mout.as_mut_ptr() };".to_string());
    let mut off = 0usize;
    for (i, (_, t)) in vals.iter().enumerate() {
        match t {
            ValTy::Addr => {
                lines.push(format!(
                    "{{ let mut __j = 0usize; while __j < 32 {{ __mo[{off} + __j] = __r{i}[__j]; __j += 1; }} }}"
                ));
                off += 32;
            }
            ValTy::Bool => {
                lines.push(format!("__mo[{off}] = __r{i} as u8;"));
                off += 1;
            }
            ValTy::Num | ValTy::SNum => {
                lines.push(format!(
                    "{{ let __le = (__r{i} as u128).to_le_bytes(); let mut __j = 0usize; while __j < 16 {{ __mo[{off} + __j] = __le[__j]; __j += 1; }} }}"
                ));
                off += 16;
            }
            ValTy::Str => unreachable!("string in a multi-return tuple is rejected earlier"),
        }
    }
    lines.push(format!("ret(&__mo[..{off}]);"));
    lines.push("}".to_string());
    lines
}

/// Is `e` a numeric literal equal to zero (used for `address(0)`)?
fn is_zero_literal(e: &Expression) -> bool {
    match e {
        Expression::NumberLiteral(_, v, _, _) => v == "0",
        Expression::HexNumberLiteral(_, v, _) => {
            let t = v.trim_start_matches("0x").trim_start_matches("0X");
            t.chars().all(|c| c == '0') && !t.is_empty()
        }
        _ => false,
    }
}

/// Is `e` an `address(...)` cast callee?
fn is_address_cast(e: &Expression) -> bool {
    match e {
        Expression::Variable(id) => id.name == "address",
        Expression::Type(_, PtType::Address) | Expression::Type(_, PtType::AddressPayable) => true,
        _ => false,
    }
}

/// Canonical Solidity ABI type for a parameter/field/return type. Used to build
/// the function-signature and event-signature strings that are keccak-hashed for
/// 4-byte selectors / event topics (ABI compatibility).
fn abi_ty(t: &Type) -> &'static str {
    match t {
        Type::Bool => "bool",
        Type::AccountId => "address",
        Type::I128 => "int256",
        Type::String => "string",
        Type::Bytes => "bytes",
        // uintN (incl. enums lowered to numeric) canonicalize to uint256, the
        // Solidity default width for the contracts in scope.
        _ => "uint256",
    }
}

/// Canonical function signature string `name(t1,t2,...)`.
fn fn_sig_string(name: &str, params: &[crate::ir::Param]) -> String {
    let parts: Vec<&str> = params.iter().map(|p| abi_ty(&p.ty)).collect();
    format!("{}({})", name, parts.join(","))
}

/// keccak256 of `s`.
fn keccak256(s: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Keccak};
    let mut k = Keccak::v256();
    let mut out = [0u8; 32];
    k.update(s);
    k.finalize(&mut out);
    out
}

/// The 4-byte function selector = first 4 bytes of keccak256(canonical sig).
fn selector4(name: &str, params: &[crate::ir::Param]) -> [u8; 4] {
    let h = keccak256(fn_sig_string(name, params).as_bytes());
    [h[0], h[1], h[2], h[3]]
}

/// Canonical event signature string `Name(t1,t2,...)`.
fn event_sig_string(ev: &crate::ir::Event) -> String {
    let parts: Vec<&str> = ev.fields.iter().map(|f| abi_ty(&f.ty)).collect();
    format!("{}({})", ev.name, parts.join(","))
}

/// keccak256 of the canonical event signature (the ABI event topic).
fn event_topic_hex(ev: &crate::ir::Event) -> String {
    let h = keccak256(event_sig_string(ev).as_bytes());
    let mut s = String::from("0x");
    for b in h.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Convert a contract name to a snake_case crate name.
fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            // Insert '_' only at a lower->Upper boundary or an Upper->Upper
            // boundary that begins a new word (next char is lowercase), e.g.
            // "SimpleStorage"->"simple_storage", "ERC20"->"erc20".
            let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if i != 0 && (prev_lower || (prev_upper && next_lower)) {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    // Avoid Rust reserved keywords as crate names (e.g. `pub`).
    if is_rust_keyword(&out) {
        out.push_str("_contract");
    }
    out
}

/// Whether `s` is a Rust reserved keyword that cannot be a crate/package name.
fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break" | "const" | "continue" | "crate" | "dyn" | "else" | "enum" | "extern"
            | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod"
            | "move" | "mut" | "pub" | "ref" | "return" | "self" | "static" | "struct" | "super"
            | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async"
            | "await" | "abstract"
    )
}

/// Max byte length of a string/bytes blob (compact prefix + payload) handled by
/// the no-std fixed-buffer string path. Inputs/returns larger than this revert.
const STR_MAX: usize = 256;

/// Number of SCALE/storage bytes for a scalar value kind. Strings are variable
/// length and never appear in fixed-layout contexts (tuples/events), so they
/// are reported with their max buffer size for sizing fallbacks only.
fn val_len(t: ValTy) -> usize {
    match t {
        ValTy::Bool => 1,
        ValTy::Num | ValTy::SNum => 16,
        ValTy::Addr => 32,
        ValTy::Str => STR_MAX,
    }
}

/// SCALE bytes for a parameter type.
fn param_len(t: &Type) -> usize {
    val_len(ValTy::from_type(t))
}

/// Metadata type string for a parameter/return/field type.
fn meta_ty(t: &Type) -> &'static str {
    match t {
        Type::Bool => "bool",
        Type::AccountId => "address",
        Type::I128 => "i128",
        Type::String => "string",
        Type::Bytes => "bytes",
        _ => "u128",
    }
}

/// Find a function/constructor body in the parse tree by name & kind.
fn find_function<'a>(
    def: &'a ContractDefinition,
    is_ctor: bool,
    name: &str,
) -> Option<&'a FunctionDefinition> {
    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            let kind_ok = if is_ctor {
                matches!(f.ty, FunctionTy::Constructor)
            } else {
                matches!(f.ty, FunctionTy::Function)
                    && f.name.as_ref().map(|i| i.name.as_str()) == Some(name)
            };
            if kind_ok {
                return Some(f);
            }
        }
    }
    None
}

/// Find the parse-tree body for a specific (possibly overloaded) message.
///
/// Overloaded Solidity functions share a name but differ in arity/types. The
/// lowered `Function` (`msg`) carries the disambiguating parameter list, so we
/// match on name AND on the mapped parameter signature (count + value-kind).
/// Without this an overload would silently bind to the wrong body — a silent
/// miscompile. Falls back to name-only matching for the non-overloaded case so
/// minor type-mapping mismatches don't lose a unique body.
fn find_function_overload<'a>(
    def: &'a ContractDefinition,
    name: &str,
    params: &[crate::ir::Param],
    uint_strategy: &str,
) -> Option<&'a FunctionDefinition> {
    let want: Vec<ValTy> = params.iter().map(|p| ValTy::from_type(&p.ty)).collect();
    let mut name_matches = 0usize;
    let mut name_only: Option<&'a FunctionDefinition> = None;
    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            if !matches!(f.ty, FunctionTy::Function) {
                continue;
            }
            if f.name.as_ref().map(|i| i.name.as_str()) != Some(name) {
                continue;
            }
            name_matches += 1;
            name_only.get_or_insert(f);
            // Map this candidate's params the same way lowering does and compare
            // the value-kind signature.
            let cand: Vec<ValTy> = f
                .params
                .iter()
                .filter_map(|(_, opt_p)| opt_p.as_ref())
                .filter_map(|p| crate::lower::map_type(&p.ty, uint_strategy))
                .map(|t| ValTy::from_type(&t))
                .collect();
            if cand == want {
                return Some(f);
            }
        }
    }
    // Only one candidate by name → no ambiguity; use it even if the mapped
    // signature didn't compare equal (e.g. exotic param the comparator can't
    // model). With overloads present and no exact match, refuse to guess.
    if name_matches <= 1 {
        name_only
    } else {
        None
    }
}

/// Collect modifier definitions by name (their guard statements minus `_`).
fn collect_modifiers(def: &ContractDefinition) -> BTreeMap<String, Vec<Statement>> {
    let mut out = BTreeMap::new();
    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            if matches!(f.ty, FunctionTy::Modifier) {
                if let (Some(name), Some(Statement::Block { statements, .. })) =
                    (f.name.as_ref(), f.body.as_ref())
                {
                    // Keep all statements except the `_` placeholder.
                    let guards: Vec<Statement> = statements
                        .iter()
                        .filter(|s| !is_placeholder(s))
                        .cloned()
                        .collect();
                    out.insert(name.name.clone(), guards);
                }
            }
        }
    }
    out
}

/// Collect struct definitions: name -> ordered (field_name, field_kind). Only
/// scalar fields are supported; a non-scalar field is left out (and a later
/// `m[k].field` access on it fails loud as an unknown member).
fn collect_structs(
    def: &ContractDefinition,
    uint_strategy: &str,
) -> BTreeMap<String, Vec<(String, ValTy)>> {
    let mut out = BTreeMap::new();
    for part in &def.parts {
        if let ContractPart::StructDefinition(sd) = part {
            if let Some(name) = &sd.name {
                let mut fields = Vec::new();
                for f in &sd.fields {
                    if let (Some(fname), Some(ty)) =
                        (&f.name, crate::lower::map_type_structs(&f.ty, uint_strategy))
                    {
                        if is_scalar(&ty) {
                            fields.push((fname.name.clone(), ValTy::from_type(&ty)));
                        }
                    }
                }
                out.insert(name.name.clone(), fields);
            }
        }
    }
    out
}

/// Collect enum definitions: every enum *value* name -> its u8 ordinal. Enums
/// lower to `uint8` (a numeric kind). Solidity forbids reusing a value name, so
/// a flat name->ordinal map is unambiguous within a contract.
fn collect_enum_values(def: &ContractDefinition) -> BTreeMap<String, u8> {
    let mut out = BTreeMap::new();
    for part in &def.parts {
        if let ContractPart::EnumDefinition(ed) = part {
            for (i, v) in ed.values.iter().enumerate() {
                if let Some(id) = v {
                    out.insert(id.name.clone(), i as u8);
                }
            }
        }
    }
    out
}

/// The set of enum type names declared in the contract (used to resolve an
/// enum-typed storage var / param to a numeric kind).
fn collect_enum_types(def: &ContractDefinition) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for part in &def.parts {
        if let ContractPart::EnumDefinition(ed) = part {
            if let Some(id) = &ed.name {
                out.insert(id.name.clone());
            }
        }
    }
    out
}

/// Render a `constant` initializer expression to a Rust literal of the given
/// kind. Only compile-time-evaluable literals are accepted; anything else is a
/// hard error (returned as `Err`) so the constant is never silently lost.
fn const_literal(init: &Expression, vt: ValTy) -> Result<String, String> {
    match init {
        Expression::NumberLiteral(_, v, _, _) => match vt {
            ValTy::SNum => Ok(format!("{v}i128")),
            _ => Ok(format!("{v}u128")),
        },
        Expression::HexNumberLiteral(_, v, _) => Ok(format!("{v}u128")),
        Expression::BoolLiteral(_, b) => Ok((if *b { "true" } else { "false" }).to_string()),
        Expression::Negate(_, inner) => {
            let inner = const_literal(inner, ValTy::SNum)?;
            Ok(format!("(-({inner}))"))
        }
        // Simple constant arithmetic (e.g. `10 ** 18` is common). Only handle
        // the forms we can fold to a literal; otherwise fail loud.
        Expression::Power(_, b, e) => {
            if let (Expression::NumberLiteral(_, bs, _, _), Expression::NumberLiteral(_, es, _, _)) =
                (b.as_ref(), e.as_ref())
            {
                let base: u128 = bs.parse().map_err(|_| "constant power base too large".to_string())?;
                let exp: u32 = es.parse().map_err(|_| "constant power exponent too large".to_string())?;
                let val = base
                    .checked_pow(exp)
                    .ok_or_else(|| "constant power overflows u128".to_string())?;
                Ok(format!("{val}u128"))
            } else {
                Err("constant initializer must be a compile-time literal".into())
            }
        }
        _ => Err("constant initializer must be a compile-time literal".into()),
    }
}

/// Collect `constant` variables for compile-time inlining. Returns a map
/// name -> (rust literal, kind). `immutable` variables are NOT inlined (they
/// are written by the constructor and stay as real storage slots).
///
/// FAIL-LOUD: a `constant` with a non-literal initializer is an error.
fn collect_constants(
    def: &ContractDefinition,
    enum_types: &std::collections::BTreeSet<String>,
    uint_strategy: &str,
) -> Result<BTreeMap<String, (String, ValTy)>, String> {
    let mut out = BTreeMap::new();
    for part in &def.parts {
        if let ContractPart::VariableDefinition(v) = part {
            let is_const = v
                .attrs
                .iter()
                .any(|a| matches!(a, solang_parser::pt::VariableAttribute::Constant(_)));
            if !is_const {
                continue;
            }
            let name = match &v.name {
                Some(id) => id.name.clone(),
                None => continue,
            };
            // Resolve the declared kind (enum -> numeric).
            let vt = resolve_kind(&v.ty, enum_types, uint_strategy);
            let init = v.initializer.as_ref().ok_or_else(|| {
                format!("constant `{name}` has no initializer")
            })?;
            let lit = const_literal(init, vt)
                .map_err(|e| format!("constant `{name}`: {e}"))?;
            out.insert(name, (lit, vt));
        }
    }
    Ok(out)
}

/// Resolve a type expression to a `ValTy`, mapping enum types to numeric.
fn resolve_kind(
    ty: &Expression,
    enum_types: &std::collections::BTreeSet<String>,
    uint_strategy: &str,
) -> ValTy {
    if let Expression::Variable(id) = ty {
        if enum_types.contains(&id.name) {
            return ValTy::Num;
        }
    }
    match crate::lower::map_type_structs(ty, uint_strategy) {
        Some(t) => ValTy::from_type(&t),
        None => ValTy::Num,
    }
}

/// Is this statement the modifier placeholder `_;`?
fn is_placeholder(s: &Statement) -> bool {
    matches!(
        s,
        Statement::Expression(_, Expression::Variable(id)) if id.name == "_"
    )
}

/// Modifier names applied to a function (in declaration order).
fn function_modifiers(f: &FunctionDefinition) -> Vec<String> {
    let mut out = Vec::new();
    for attr in &f.attributes {
        if let FunctionAttribute::BaseOrModifier(_, base) = attr {
            if let Some(id) = base.name.identifiers.last() {
                out.push(id.name.clone());
            }
        }
    }
    out
}

/// Generate seal0 artifacts from the IR contract + the parsed AST.
pub fn emit_seal(
    c: &Contract,
    def: &ContractDefinition,
    uint_strategy: &str,
    external_fns: &BTreeMap<String, ExternalFn>,
) -> Result<SealArtifacts, String> {
    let _ = uint_strategy;

    // Resolve struct definitions (name -> ordered scalar fields).
    let structs = collect_structs(def, uint_strategy);
    let enum_values = collect_enum_values(def);
    let enum_types = collect_enum_types(def);
    // Compile-time inlined constants (excluded from storage slots).
    let constants = collect_constants(def, &enum_types, uint_strategy)?;

    // Storage fields, excluding `constant` vars (inlined) — they must not take
    // a slot or their initializer value would be silently lost.
    let storage: Vec<&crate::ir::Field> = c
        .storage
        .iter()
        .filter(|f| !constants.contains_key(&f.name))
        .collect();

    // Assign slots, validating supported storage shapes.
    let mut slots: Vec<Slot> = Vec::new();
    for (i, f) in storage.iter().enumerate() {
        let kind = match &f.ty {
            Type::Bool => SlotKind::Scalar(ValTy::Bool),
            Type::U128 => SlotKind::Scalar(ValTy::Num),
            Type::I128 => SlotKind::Scalar(ValTy::SNum),
            Type::AccountId => SlotKind::Scalar(ValTy::Addr),
            Type::U256 => {
                return Err(format!(
                    "field `{}`: u256 not supported (use u128 strategy)",
                    f.name
                ))
            }
            // string / bytes: stored as a SCALE blob under the slot key.
            Type::String | Type::Bytes => SlotKind::Scalar(ValTy::Str),
            // Dynamic array of a scalar element.
            Type::Array(elem) => {
                if is_scalar(elem) {
                    SlotKind::Array { elem: ValTy::from_type(elem) }
                } else {
                    return Err(format!(
                        "field `{}`: only arrays of scalar elements (uint/int/address/bool) supported",
                        f.name
                    ));
                }
            }
            Type::Mapping(k, v) => match (k.as_ref(), v.as_ref()) {
                // mapping(scalar-key => scalar)
                (kt, inner) if is_scalar_key(kt) && is_scalar(inner) => SlotKind::Map {
                    key: ValTy::from_type(kt),
                    val: ValTy::from_type(inner),
                },
                // mapping(scalar-key => Struct{scalar fields...})
                (kt, Type::Struct(sname)) if is_scalar_key(kt) => {
                    let fields = structs.get(sname).ok_or_else(|| {
                        format!("field `{}`: unknown struct `{sname}`", f.name)
                    })?;
                    SlotKind::MapStruct { key: ValTy::from_type(kt), fields: fields.clone() }
                }
                // mapping(scalar-key => mapping(scalar-key => scalar))
                (kt, Type::Mapping(k2, v2))
                    if is_scalar_key(kt) && is_scalar_key(k2) && is_scalar(v2) =>
                {
                    SlotKind::Map2 {
                        key1: ValTy::from_type(kt),
                        key2: ValTy::from_type(k2),
                        val: ValTy::from_type(v2),
                    }
                }
                _ => {
                    return Err(format!(
                        "field `{}`: only mapping(scalar=>scalar) and \
                         mapping(scalar=>mapping(scalar=>scalar)) supported",
                        f.name
                    ))
                }
            },
            // An enum-typed storage var lowers to a numeric (uint8) scalar slot.
            Type::Struct(sname) if enum_types.contains(sname) => SlotKind::Scalar(ValTy::Num),
            other => {
                return Err(format!("field `{}`: unsupported storage type {other:?}", f.name))
            }
        };
        slots.push(Slot { name: f.name.clone(), index: i as u8, kind });
    }

    // A string/bytes parameter must be the LAST parameter (its variable length
    // breaks the static offsets of any following params).
    let check_str_last = |params: &[crate::ir::Param], who: &str| -> Result<(), String> {
        for (i, p) in params.iter().enumerate() {
            if ValTy::from_type(&p.ty) == ValTy::Str && i + 1 != params.len() {
                return Err(format!(
                    "{who}: a string/bytes parameter must be the last parameter (`{}`)",
                    p.name
                ));
            }
        }
        Ok(())
    };
    if let Some(ctor) = &c.constructor {
        check_str_last(&ctor.params, "constructor")?;
    }
    for msg in &c.messages {
        check_str_last(&msg.params, &format!("function `{}`", msg.name))?;
        if msg.returns.len() > 1
            && msg.returns.iter().any(|t| ValTy::from_type(t) == ValTy::Str)
        {
            return Err(format!(
                "function `{}`: string/bytes in a multi-value return is not supported",
                msg.name
            ));
        }
    }

    let modifiers = collect_modifiers(def);
    let mut uses = Uses::default();
    let mut all_errors: Vec<String> = Vec::new();

    // ----- Constructor body -----
    let ctor_body_lines: Vec<String> = if let Some(ctor) = &c.constructor {
        let fdef = find_function(def, true, "");
        let mut ctx = SealCtx::new(&slots, &c.events, &constants, &enum_values, &structs, &external_fns);
        register_storage_widths(&mut ctx, def);
        register_params(&mut ctx, &ctor.params);
        if let Some(fdef) = fdef {
            register_param_widths(&mut ctx, fdef);
        }
        let mut lines = decode_params_prelude(&ctor.params, false);
        if let Some(fdef) = fdef {
            if let Some(body) = &fdef.body {
                for l in ctx.stmt(body) {
                    lines.push(l);
                }
            }
        }
        merge_uses(&mut uses, ctx.uses);
        all_errors.extend(ctx.errors);
        lines
    } else {
        Vec::new()
    };

    // ----- receive() / fallback() -----
    // Solidity dispatch for the `call()` default (no selector match):
    //   * empty calldata + `receive` defined  -> run receive()
    //   * otherwise                            -> run fallback() if defined
    // Both bodies are lowered here; if neither exists the default reverts (as
    // before). A bodyless receive/fallback declaration is ignored.
    let mut receive_body: Option<Vec<String>> = None;
    let mut fallback_body: Option<Vec<String>> = None;
    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            let kind = match f.ty {
                FunctionTy::Receive => Some(false),
                FunctionTy::Fallback => Some(true),
                _ => None,
            };
            if let Some(is_fallback) = kind {
                if let Some(body) = &f.body {
                    let mut ctx = SealCtx::new(&slots, &c.events, &constants, &enum_values, &structs, &external_fns);
                    register_storage_widths(&mut ctx, def);
                    register_param_widths(&mut ctx, f);
                    let mut lines: Vec<String> = Vec::new();
                    for mname in function_modifiers(f) {
                        if let Some(guards) = modifiers.get(&mname) {
                            for g in guards {
                                lines.extend(ctx.stmt(g));
                            }
                        }
                    }
                    lines.extend(ctx.stmt(body));
                    lines.push("ret(&[]);".to_string());
                    merge_uses(&mut uses, ctx.uses);
                    all_errors.extend(ctx.errors);
                    if is_fallback {
                        fallback_body = Some(lines);
                    } else {
                        receive_body = Some(lines);
                    }
                }
            }
        }
    }

    // ----- Messages -----
    let mut arms: Vec<String> = Vec::new();
    let mut meta_messages: Vec<String> = Vec::new();

    // Real keccak-256 4-byte selectors (ABI-compatible). Detect any collision
    // (e.g. genuine 4-byte clash or duplicate signature) and fail loud rather
    // than silently dispatching the wrong function.
    //
    // ANY duplicate selector is fatal — including the same name + same
    // signature (a genuine duplicate function) which would otherwise emit two
    // identical, conflicting dispatch arms. Overloads (same name, different
    // signature) hash to *different* selectors and so pass this check, each
    // keeping its own arm + metadata entry. Nothing is ever silently dropped.
    let mut seen_selectors: BTreeMap<[u8; 4], String> = BTreeMap::new();
    for msg in &c.messages {
        let sel = selector4(&msg.name, &msg.params);
        let sig = fn_sig_string(&msg.name, &msg.params);
        if let Some(prev) = seen_selectors.get(&sel) {
            return Err(format!(
                "selector collision: `{}` and `{}` hash to the same 4-byte selector \
                 0x{:02x}{:02x}{:02x}{:02x}",
                prev, sig, sel[0], sel[1], sel[2], sel[3]
            ));
        }
        seen_selectors.insert(sel, sig);
    }

    for (_i, msg) in c.messages.iter().enumerate() {
        let sel_bytes = selector4(&msg.name, &msg.params);
        let selector =
            u32::from_be_bytes(sel_bytes);
        let pat = format!(
            "[{}, {}, {}, {}]",
            sel_bytes[0], sel_bytes[1], sel_bytes[2], sel_bytes[3]
        );

        let fdef = find_function_overload(def, &msg.name, &msg.params, uint_strategy);
        let mut ctx = SealCtx::new(&slots, &c.events, &constants, &enum_values, &structs, &external_fns);
        register_storage_widths(&mut ctx, def);
        register_params(&mut ctx, &msg.params);
        if let Some(fdef) = fdef {
            register_param_widths(&mut ctx, fdef);
        }
        ctx.ret_kinds = msg.returns.iter().map(ValTy::from_type).collect();

        let mut body_lines: Vec<String> = decode_params_prelude(&msg.params, true);

        // Single-return functions use an `Option<T> __ret` accumulator set by
        // `Statement::Return`. Multi-return uses per-element `__mretN` vars.
        // String returns diverge via `return_str_S()`, so they use no
        // accumulator.
        let single_ret = msg.returns.len() == 1;
        let str_ret = single_ret && ValTy::from_type(&msg.returns[0]) == ValTy::Str;
        if single_ret && !str_ret {
            let rt = ret_rust_ty(&msg.returns[0]);
            body_lines.push(format!("let mut __ret: Option<{rt}> = None;"));
        }

        // Auto-getter synthesis: a public storage var with no explicit function
        // body. Emit `return <slot>;`.
        let is_auto_getter = fdef.is_none()
            && msg.params.is_empty()
            && single_ret
            && slots.iter().any(|s| s.name == msg.name && matches!(s.kind, SlotKind::Scalar(_)));

        // Auto-getter for a `public constant`: return the inlined literal.
        let const_getter = fdef.is_none()
            && msg.params.is_empty()
            && single_ret
            && constants.contains_key(&msg.name);

        if is_auto_getter {
            let slot = slots.iter().find(|s| s.name == msg.name).unwrap();
            if let SlotKind::Scalar(vt) = slot.kind {
                if vt == ValTy::Str {
                    body_lines.push(format!("return_str_{}();", slot.index));
                } else {
                    let read = match vt {
                        ValTy::SNum => format!("(load_slot_{}() as i128)", slot.index),
                        _ => format!("load_slot_{}()", slot.index),
                    };
                    body_lines.push(format!("__ret = Some({read});"));
                }
            }
        } else if const_getter {
            let (lit, vt) = constants.get(&msg.name).unwrap();
            let read = match vt {
                ValTy::SNum => format!("({lit} as i128)"),
                _ => lit.clone(),
            };
            body_lines.push(format!("__ret = Some({read});"));
        } else if let Some(fdef) = fdef {
            // Inline modifier guards (e.g. onlyOwner) at function entry.
            for mname in function_modifiers(fdef) {
                if let Some(guards) = modifiers.get(&mname) {
                    for g in guards {
                        for l in ctx.stmt(g) {
                            body_lines.push(l);
                        }
                    }
                }
            }
            if let Some(body) = &fdef.body {
                for l in ctx.stmt(body) {
                    body_lines.push(l);
                }
            }
        } else if !msg.returns.is_empty() {
            // No body, not an auto-getter, but declares a return: cannot emit.
            ctx.err(&format!("function `{}` has no translatable body", msg.name));
        }
        merge_uses(&mut uses, ctx.uses);
        all_errors.extend(ctx.errors);

        // Return emission.
        if str_ret {
            // The body diverges via `return_str_S()`. If control still reaches
            // here (no return on some path), return an empty string blob.
            body_lines.push("ret(&[0u8]);".to_string());
        } else if single_ret {
            match ValTy::from_type(&msg.returns[0]) {
                ValTy::Bool => {
                    body_lines.push("let __v = __ret.unwrap_or(false);".to_string());
                    body_lines.push("let __out = [__v as u8];".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
                ValTy::Addr => {
                    body_lines.push("let __v = __ret.unwrap_or([0u8; 32]);".to_string());
                    body_lines.push("ret(&__v);".to_string());
                }
                ValTy::SNum => {
                    body_lines.push("let __v = __ret.unwrap_or(0i128);".to_string());
                    body_lines.push("let __out = (__v as u128).to_le_bytes();".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
                ValTy::Num => {
                    body_lines.push("let __v = __ret.unwrap_or(0u128);".to_string());
                    body_lines.push("let __out = __v.to_le_bytes();".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
                ValTy::Str => unreachable!("str_ret handled above"),
            }
        } else if msg.returns.len() > 1 {
            // Multi-return values are emitted inline at each `return (a, b)` via
            // a diverging `ret()`. If control reaches here the body had no
            // return on some path; revert (mirrors a missing-return contract).
            body_lines.push("revert();".to_string());
        } else {
            body_lines.push("ret(&[]);".to_string());
        }

        let mut arm = String::new();
        arm.push_str(&format!("        {pat} => {{\n"));
        for l in &body_lines {
            arm.push_str(&format!("            {l}\n"));
        }
        arm.push_str("        }\n");
        arms.push(arm);

        // Metadata.
        let args_meta: Vec<String> = msg
            .params
            .iter()
            .map(|p| format!("\"{}\"", meta_ty(&p.ty)))
            .collect();
        // `ret` is a single type string for 0/1 returns (back-compat), or a
        // JSON list of type strings for multi-return.
        let ret_meta = match msg.returns.len() {
            0 => "null".to_string(),
            1 => format!("\"{}\"", meta_ty(&msg.returns[0])),
            _ => {
                let parts: Vec<String> =
                    msg.returns.iter().map(|t| format!("\"{}\"", meta_ty(t))).collect();
                format!("[{}]", parts.join(", "))
            }
        };
        let mutates = !matches!(msg.mutability, Mutability::View);
        let payable = matches!(msg.mutability, Mutability::Payable);
        meta_messages.push(format!(
            "    {{ \"name\": \"{}\", \"selector\": \"0x{:08x}\", \"args\": [{}], \"ret\": {}, \"mutates\": {}, \"payable\": {} }}",
            msg.name,
            selector,
            args_meta.join(", "),
            ret_meta,
            mutates,
            payable
        ));
    }

    // ----- FAIL-LOUD: any unsupported construct collected during lowering is a
    // hard error. We never emit a silently mis-translated contract.
    if !all_errors.is_empty() {
        let mut dedup: Vec<String> = Vec::new();
        for e in &all_errors {
            if !dedup.contains(e) {
                dedup.push(e.clone());
            }
        }
        let listed = dedup
            .iter()
            .map(|e| format!("  - {e}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "{} unsupported construct(s) in contract `{}`:\n{}",
            dedup.len(),
            c.name,
            listed
        ));
    }

    // ----- Buffer sizes (size to actual payload; node rejects big buffers) ---
    let ctor_bytes: usize = c
        .constructor
        .as_ref()
        .map(|ct| ct.params.iter().map(|p| param_len(&p.ty)).sum())
        .unwrap_or(0);
    let deploy_buf = ctor_bytes.max(1);
    let max_msg_args: usize = c
        .messages
        .iter()
        .map(|m| m.params.iter().map(|p| param_len(&p.ty)).sum::<usize>())
        .max()
        .unwrap_or(0);
    let call_buf = 4 + max_msg_args.max(1);

    // Build the `call()` default-match arm from receive/fallback bodies.
    let default_arm: String = {
        let mut s = String::new();
        match (&receive_body, &fallback_body) {
            (None, None) => {
                s.push_str("        _ => { revert(); }\n");
            }
            (recv, fb) => {
                s.push_str("        _ => {\n");
                if let Some(lines) = recv {
                    s.push_str("            if in_len == 0 {\n");
                    for l in lines {
                        s.push_str(&format!("                {l}\n"));
                    }
                    s.push_str("            }\n");
                }
                if let Some(lines) = fb {
                    for l in lines {
                        s.push_str(&format!("            {l}\n"));
                    }
                } else {
                    s.push_str("            revert();\n");
                }
                s.push_str("        }\n");
            }
        }
        s
    };

    let lib_rs = render_lib_rs(&slots, &ctor_body_lines, &arms, uses, deploy_buf, call_buf, &default_arm);

    // ----- metadata.json (incl. events layout) -----
    let ctor_args_meta: Vec<String> = c
        .constructor
        .as_ref()
        .map(|ct| ct.params.iter().map(|p| format!("\"{}\"", meta_ty(&p.ty))).collect())
        .unwrap_or_default();

    let events_meta: Vec<String> = c
        .events
        .iter()
        .map(|ev| {
            let fields: Vec<String> = ev
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "{{ \"name\": \"{}\", \"type\": \"{}\", \"indexed\": {} }}",
                        f.name,
                        meta_ty(&f.ty),
                        f.indexed
                    )
                })
                .collect();
            format!(
                "    {{ \"name\": \"{}\", \"sig\": \"{}\", \"topic\": \"{}\", \"fields\": [{}] }}",
                ev.name,
                event_sig_string(ev),
                event_topic_hex(ev),
                fields.join(", ")
            )
        })
        .collect();

    let metadata_json = format!(
        "{{\n  \"name\": \"{}\",\n  \"constructor\": {{ \"args\": [{}] }},\n  \"messages\": [\n{}\n  ],\n  \"events\": [\n{}\n  ]\n}}\n",
        c.name,
        ctor_args_meta.join(", "),
        meta_messages.join(",\n"),
        events_meta.join(",\n")
    );

    let crate_name = snake(&c.name);
    let cargo_toml = render_cargo_toml(&crate_name);
    let cargo_config_toml = CARGO_CONFIG.to_string();

    Ok(SealArtifacts { lib_rs, metadata_json, cargo_toml, cargo_config_toml, crate_name })
}

fn is_scalar(t: &Type) -> bool {
    matches!(t, Type::Bool | Type::U128 | Type::I128 | Type::AccountId)
}

/// Types usable as a mapping key (address, numeric, bool).
fn is_scalar_key(t: &Type) -> bool {
    matches!(t, Type::AccountId | Type::U128 | Type::I128 | Type::Bool)
}

fn ret_rust_ty(t: &Type) -> &'static str {
    match ValTy::from_type(t) {
        ValTy::Bool => "bool",
        ValTy::Addr => "[u8; 32]",
        ValTy::SNum => "i128",
        ValTy::Num => "u128",
        // String returns diverge directly and never use the `__ret` accumulator.
        ValTy::Str => "u128",
    }
}

fn register_params(ctx: &mut SealCtx, params: &[crate::ir::Param]) {
    for p in params {
        ctx.locals.insert(p.name.clone(), ValTy::from_type(&p.ty));
    }
}

/// Narrow width of a parse-tree type expression, if any (`uintN`/`intN`,
/// N<128). Wide / non-numeric types yield `None`.
fn pt_type_width(ty: &Expression) -> Option<u32> {
    match ty {
        Expression::Type(_, PtType::Uint(n)) | Expression::Type(_, PtType::Int(n)) => {
            narrow_width(*n)
        }
        Expression::Variable(id) => cast_name_width(&id.name),
        Expression::Parenthesis(_, inner) => pt_type_width(inner),
        _ => None,
    }
}

/// Register declared narrow widths for a contract's storage scalar variables.
fn register_storage_widths(ctx: &mut SealCtx, def: &ContractDefinition) {
    for part in &def.parts {
        if let ContractPart::VariableDefinition(v) = part {
            if let (Some(name), Some(w)) = (&v.name, pt_type_width(&v.ty)) {
                ctx.widths.insert(name.name.clone(), w);
            }
        }
    }
}

/// Register declared narrow widths for a function's parameters from the raw
/// parse-tree definition (the IR has already collapsed widths to u128/i128).
fn register_param_widths(ctx: &mut SealCtx, fdef: &FunctionDefinition) {
    for (_, opt_p) in &fdef.params {
        if let Some(p) = opt_p {
            if let (Some(id), Some(w)) = (&p.name, pt_type_width(&p.ty)) {
                ctx.widths.insert(id.name.clone(), w);
            }
        }
    }
}

fn merge_uses(acc: &mut Uses, u: Uses) {
    acc.caller |= u.caller;
    acc.value |= u.value;
    acc.blake2 |= u.blake2;
    acc.deposit_event |= u.deposit_event;
    acc.transfer |= u.transfer;
    acc.now |= u.now;
    acc.block_number |= u.block_number;
    acc.balance |= u.balance;
    acc.call |= u.call;
}

/// Prelude that decodes parameters from the input buffer.
fn decode_params_prelude(params: &[crate::ir::Param], after_selector: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut off: usize = if after_selector { 4 } else { 0 };
    for p in params {
        match ValTy::from_type(&p.ty) {
            ValTy::Bool => {
                lines.push(format!("let {} = input[{}] != 0;", p.name, off));
                off += 1;
            }
            ValTy::Addr => {
                lines.push(format!(
                    "let mut {n} = [0u8; 32]; {n}.copy_from_slice(&input[{a}..{b}]);",
                    n = p.name,
                    a = off,
                    b = off + 32
                ));
                off += 32;
            }
            ValTy::Num => {
                lines.push(format!(
                    "let mut __b_{n} = [0u8; 16]; __b_{n}.copy_from_slice(&input[{a}..{b}]); let {n} = u128::from_le_bytes(__b_{n});",
                    n = p.name,
                    a = off,
                    b = off + 16
                ));
                off += 16;
            }
            ValTy::SNum => {
                // Signed param: decode 16-byte LE two's-complement -> i128.
                lines.push(format!(
                    "let mut __b_{n} = [0u8; 16]; __b_{n}.copy_from_slice(&input[{a}..{b}]); let {n} = i128::from_le_bytes(__b_{n});",
                    n = p.name,
                    a = off,
                    b = off + 16
                ));
                off += 16;
            }
            ValTy::Str => {
                // string/bytes param: SCALE = compact(len) ++ bytes, contiguous
                // in the input buffer. Decode the compact prefix to find the
                // payload length and the total blob length; expose both plus a
                // pointer to the blob start. A string must be the LAST param so
                // the following params keep a static offset (enforced earlier).
                lines.push(format!(
                    "let __str_{n}_b0 = input[{a}] as u32;",
                    n = p.name, a = off
                ));
                lines.push(format!(
                    "let (__str_{n}_plen, __str_{n}_len) = if (__str_{n}_b0 & 3) == 0 {{ (1usize, (__str_{n}_b0 >> 2) as usize) }} else if (__str_{n}_b0 & 3) == 1 {{ (2usize, (((__str_{n}_b0 >> 2) | ((input[{a1}] as u32) << 6)) as usize)) }} else {{ revert() }};",
                    n = p.name, a1 = off + 1
                ));
                lines.push(format!(
                    "let __str_{n}_total = __str_{n}_plen + __str_{n}_len;",
                    n = p.name
                ));
                lines.push(format!(
                    "let __str_{n}_ptr = unsafe {{ (input.as_ptr() as *const u8).add({a}) }};",
                    n = p.name, a = off
                ));
                // off is no longer static; a string must be the last param.
                off += 1;
            }
        }
    }
    lines
}

/// Render the per-field scalar storage load/store helpers.
fn render_slot_helpers(slots: &[Slot]) -> String {
    let mut out = String::new();
    for slot in slots {
        let s = slot.index;
        if let SlotKind::Scalar(vt) = slot.kind {
            out.push_str(&format!(
                "static KEY_{s}: [u8; 32] = {{ let mut k = [0u8; 32]; k[0] = {s}; k }};\n"
            ));
            match vt {
                ValTy::Bool => {
                    out.push_str(&format!(
                        "fn store_slot_{s}(v: bool) {{ let b = [v as u8]; unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), b.as_ptr(), 1); }} }}\n"
                    ));
                    out.push_str(&format!(
                        "fn load_slot_{s}() -> bool {{ let mut buf = [0u8; 1]; let mut len: u32 = 1; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 1 {{ buf[0] != 0 }} else {{ false }} }}\n"
                    ));
                }
                ValTy::Addr => {
                    out.push_str(&format!(
                        "fn store_slot_{s}(v: [u8; 32]) {{ unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), v.as_ptr(), 32); }} }}\n"
                    ));
                    out.push_str(&format!(
                        "fn load_slot_{s}() -> [u8; 32] {{ let mut buf = [0u8; 32]; let mut len: u32 = 32; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; let _ = rc; buf }}\n"
                    ));
                }
                // Both unsigned and signed numerics are stored as 16-byte LE
                // u128 bit patterns (two's-complement for signed). Signed reads
                // cast `load_slot_N() as i128` at the use site.
                ValTy::Num | ValTy::SNum => {
                    out.push_str(&format!(
                        "fn store_slot_{s}(v: u128) {{ let b = v.to_le_bytes(); unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), b.as_ptr(), 16); }} }}\n"
                    ));
                    out.push_str(&format!(
                        "fn load_slot_{s}() -> u128 {{ let mut buf = [0u8; 16]; let mut len: u32 = 16; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 16 {{ u128::from_le_bytes(buf) }} else {{ 0 }} }}\n"
                    ));
                }
                // string/bytes: the whole SCALE blob (compact(len) ++ bytes) is
                // stored under the slot key. `store_str_S` writes a blob given a
                // pointer + total byte length; `return_str_S` reads it back into
                // a fixed buffer and `ret`s it verbatim (already SCALE).
                ValTy::Str => {
                    out.push_str(&format!(
                        "fn store_str_{s}(ptr: *const u8, total: usize) {{ unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), ptr, total as u32); }} }}\n"
                    ));
                    out.push_str(&format!(
                        "fn return_str_{s}() -> ! {{ let mut buf = MaybeUninit::<[u8; {max}]>::uninit(); let bp = buf.as_mut_ptr() as *mut u8; let mut len: u32 = {max}; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), bp, &mut len as *mut u32) }}; if rc != 0 {{ ret(&[0u8]); }} let sl = unsafe {{ core::slice::from_raw_parts(bp as *const u8, len as usize) }}; ret(sl); }}\n",
                        max = STR_MAX
                    ));
                }
            }
        } else if let SlotKind::Array { elem } = slot.kind {
            out.push_str(&render_array_helpers(s, elem));
        }
    }
    out
}

/// Per-slot dynamic-array helpers. Length stored at the scalar slot key
/// `[S,0,..]` (16-byte LE); element `i` at `blake2([S] ++ u32_le(i))`.
fn render_array_helpers(s: u8, elem: ValTy) -> String {
    let mut out = String::new();
    // Length store/load at the plain slot key.
    out.push_str(&format!(
        "static AKEY_{s}: [u8; 32] = {{ let mut k = [0u8; 32]; k[0] = {s}; k }};\n"
    ));
    out.push_str(&format!(
        "fn arr_len_{s}() -> u128 {{ let mut buf = [0u8; 16]; let mut len: u32 = 16; let rc = unsafe {{ seal_get_storage(AKEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 16 {{ u128::from_le_bytes(buf) }} else {{ 0 }} }}\n"
    ));
    out.push_str(&format!(
        "fn arr_setlen_{s}(n: u128) {{ let b = n.to_le_bytes(); unsafe {{ seal_set_storage(AKEY_{s}.as_ptr(), b.as_ptr(), 16); }} }}\n"
    ));
    // Element key: blake2([S] ++ u32_le(i)). Built byte-by-byte (no memory.copy).
    out.push_str(&format!(
        "fn arr_key_{s}(i: u128) -> [u8; 32] {{ let mut pre = MaybeUninit::<[u8; 5]>::uninit(); let pp = pre.as_mut_ptr() as *mut u8; let le = (i as u32).to_le_bytes(); unsafe {{ *pp = {s}; *pp.add(1) = le[0]; *pp.add(2) = le[1]; *pp.add(3) = le[2]; *pp.add(4) = le[3]; }} let mut out = [0u8; 32]; unsafe {{ seal_hash_blake2_256(pp as *const u8, 5, out.as_mut_ptr()); }} out }}\n"
    ));
    match elem {
        ValTy::Bool => {
            out.push_str(&format!(
                "fn arr_get_bool_{s}(i: u128) -> bool {{ map_get_bool(arr_key_{s}(i)) }}\n"
            ));
            out.push_str(&format!(
                "fn arr_set_bool_{s}(i: u128, v: bool) {{ map_set_bool(arr_key_{s}(i), v); }}\n"
            ));
            out.push_str(&format!(
                "fn arr_push_bool_{s}(v: bool) {{ let n = arr_len_{s}(); arr_set_bool_{s}(n, v); arr_setlen_{s}(n + 1); }}\n"
            ));
        }
        ValTy::Addr => {
            out.push_str(&format!(
                "fn arr_get_addr_{s}(i: u128) -> [u8; 32] {{ map_get_addr(arr_key_{s}(i)) }}\n"
            ));
            out.push_str(&format!(
                "fn arr_set_addr_{s}(i: u128, v: &[u8; 32]) {{ map_set_addr(arr_key_{s}(i), v); }}\n"
            ));
            out.push_str(&format!(
                "fn arr_push_addr_{s}(v: &[u8; 32]) {{ let n = arr_len_{s}(); arr_set_addr_{s}(n, v); arr_setlen_{s}(n + 1); }}\n"
            ));
        }
        _ => {
            out.push_str(&format!(
                "fn arr_get_u128_{s}(i: u128) -> u128 {{ map_get_u128(arr_key_{s}(i)) }}\n"
            ));
            out.push_str(&format!(
                "fn arr_set_u128_{s}(i: u128, v: u128) {{ map_set_u128(arr_key_{s}(i), v); }}\n"
            ));
            out.push_str(&format!(
                "fn arr_push_u128_{s}(v: u128) {{ let n = arr_len_{s}(); arr_set_u128_{s}(n, v); arr_setlen_{s}(n + 1); }}\n"
            ));
        }
    }
    out
}

/// Whether the contract needs the blake2 key + typed get/set helper block
/// (mappings, dynamic arrays, and mapping-of-struct slots all use it).
fn has_mapping(slots: &[Slot]) -> bool {
    slots.iter().any(|s| {
        matches!(
            s.kind,
            SlotKind::Map { .. }
                | SlotKind::Map2 { .. }
                | SlotKind::Array { .. }
                | SlotKind::MapStruct { .. }
        )
    })
}

/// Render the complete `lib.rs`.
fn render_lib_rs(
    slots: &[Slot],
    ctor_body: &[String],
    arms: &[String],
    uses: Uses,
    deploy_buf: usize,
    call_buf: usize,
    default_arm: &str,
) -> String {
    let mut out = String::new();
    out.push_str("#![no_std]\n#![no_main]\n#![allow(dead_code, non_snake_case, unused_mut, unused_assignments)]\nuse core::panic::PanicInfo;\nuse core::mem::MaybeUninit;\n\n");
    out.push_str("#[panic_handler]\nfn panic(_: &PanicInfo) -> ! { core::arch::wasm32::unreachable() }\n\n");

    // seal0 imports — only those used.
    out.push_str("#[link(wasm_import_module = \"seal0\")]\n");
    out.push_str("extern \"C\" {\n");
    out.push_str("    fn seal_input(buf: *mut u8, len: *mut u32);\n");
    out.push_str("    fn seal_return(flags: u32, data: *const u8, len: u32) -> !;\n");
    out.push_str("    fn seal_get_storage(key: *const u8, out: *mut u8, out_len: *mut u32) -> u32;\n");
    out.push_str("    fn seal_set_storage(key: *const u8, val: *const u8, val_len: u32);\n");
    if uses.caller {
        out.push_str("    fn seal_caller(out: *mut u8, out_len: *mut u32);\n");
    }
    if uses.value {
        out.push_str("    fn seal_value_transferred(out: *mut u8, out_len: *mut u32);\n");
    }
    if uses.blake2 || has_mapping(slots) {
        out.push_str("    fn seal_hash_blake2_256(input: *const u8, len: u32, out: *mut u8);\n");
    }
    if uses.deposit_event {
        out.push_str(
            "    fn seal_deposit_event(topics: *const u8, topics_len: u32, data: *const u8, data_len: u32);\n",
        );
    }
    if uses.transfer {
        out.push_str(
            "    fn seal_transfer(acct: *const u8, acct_len: u32, val: *const u8, val_len: u32) -> u32;\n",
        );
    }
    if uses.now {
        out.push_str("    fn seal_now(out: *mut u8, out_len: *mut u32);\n");
    }
    if uses.block_number {
        out.push_str("    fn seal_block_number(out: *mut u8, out_len: *mut u32);\n");
    }
    if uses.balance {
        out.push_str("    fn seal_balance(out: *mut u8, out_len: *mut u32);\n");
    }
    if uses.call {
        out.push_str("    fn seal_call(callee: *const u8, callee_len: u32, gas: u64, value: *const u8, value_len: u32, input: *const u8, input_len: u32, output: *mut u8, output_len: *mut u32) -> u32;\n");
    }
    out.push_str("}\n\n");

    // Runtime helpers.
    out.push_str("#[inline(never)]\nfn ret(data: &[u8]) -> ! { unsafe { seal_return(0, data.as_ptr(), data.len() as u32) } }\n");
    out.push_str("#[inline(never)]\nfn revert() -> ! { unsafe { seal_return(1, core::ptr::null(), 0) } }\n");
    if uses.caller {
        out.push_str("fn caller() -> [u8; 32] { let mut buf = [0u8; 32]; let mut len: u32 = 32; unsafe { seal_caller(buf.as_mut_ptr(), &mut len as *mut u32); } buf }\n");
    }
    if uses.value {
        // msg.value: seal_value_transferred yields a u128 LE balance.
        out.push_str("fn value() -> u128 { let mut buf = [0u8; 16]; let mut len: u32 = 16; unsafe { seal_value_transferred(buf.as_mut_ptr(), &mut len as *mut u32); } u128::from_le_bytes(buf) }\n");
    }
    if uses.transfer {
        out.push_str("fn do_transfer(acct: &[u8; 32], amount: u128) { let v = amount.to_le_bytes(); let rc = unsafe { seal_transfer(acct.as_ptr(), 32, v.as_ptr(), 16) }; if rc != 0 { revert(); } }\n");
    }
    if uses.now {
        // block.timestamp: seal_now returns the block time in milliseconds (LE).
        out.push_str("fn block_timestamp() -> u128 { let mut buf = [0u8; 16]; let mut len: u32 = 16; unsafe { seal_now(buf.as_mut_ptr(), &mut len as *mut u32); } u128::from_le_bytes(buf) }\n");
    }
    if uses.block_number {
        out.push_str("fn block_number() -> u128 { let mut buf = [0u8; 16]; let mut len: u32 = 16; unsafe { seal_block_number(buf.as_mut_ptr(), &mut len as *mut u32); } u128::from_le_bytes(buf) }\n");
    }
    if uses.balance {
        out.push_str("fn self_balance() -> u128 { let mut buf = [0u8; 16]; let mut len: u32 = 16; unsafe { seal_balance(buf.as_mut_ptr(), &mut len as *mut u32); } u128::from_le_bytes(buf) }\n");
    }
    if uses.call {
        // Cross-contract call: selector++SCALE(args) already in `input`. Forwards
        // zero value. Reverts on callee trap. Writes up to `out_cap` return bytes.
        out.push_str("fn do_call(dest: &[u8; 32], input: &[u8], out: *mut u8, out_cap: u32) -> u32 { let val = [0u8; 16]; let mut out_len: u32 = out_cap; let rc = unsafe { seal_call(dest.as_ptr(), 32, 0u64, val.as_ptr(), 16, input.as_ptr(), input.len() as u32, out, &mut out_len as *mut u32) }; if rc != 0 { revert(); } out_len }\n");
    }
    out.push('\n');

    // Mapping key + access helpers.
    if has_mapping(slots) {
        out.push_str(MAP_HELPERS);
        out.push('\n');
    }

    // Scalar storage helpers.
    out.push_str(&render_slot_helpers(slots));
    out.push('\n');

    // deploy()
    //
    // The input buffer is read through `MaybeUninit` rather than a zeroed
    // `[0u8; N]`. A zero-initialized stack array larger than 32 bytes makes
    // rustc/LLVM emit a `memory.fill` (bulk-memory) instruction, which the
    // rent-era wasm validator rejects ("Can't decode wasm code"). `MaybeUninit`
    // skips the zeroing; `seal_input` fills the bytes we read.
    out.push_str("#[no_mangle]\npub extern \"C\" fn deploy() {\n");
    out.push_str(&format!("    let mut __raw = MaybeUninit::<[u8; {deploy_buf}]>::uninit();\n"));
    out.push_str("    let __p = __raw.as_mut_ptr() as *mut u8;\n");
    out.push_str(&format!("    let mut in_len: u32 = {deploy_buf};\n"));
    out.push_str("    unsafe { seal_input(__p, &mut in_len as *mut u32); }\n");
    out.push_str(&format!("    let input: &[u8; {deploy_buf}] = unsafe {{ &*(__p as *const [u8; {deploy_buf}]) }};\n"));
    out.push_str("    let _ = in_len;\n");
    out.push_str("    let _ = &input;\n");
    for l in ctor_body {
        out.push_str(&format!("    {l}\n"));
    }
    out.push_str("}\n\n");

    // call()
    out.push_str("#[no_mangle]\npub extern \"C\" fn call() {\n");
    out.push_str(&format!("    let mut __raw = MaybeUninit::<[u8; {call_buf}]>::uninit();\n"));
    out.push_str("    let __p = __raw.as_mut_ptr() as *mut u8;\n");
    out.push_str(&format!("    let mut in_len: u32 = {call_buf};\n"));
    out.push_str("    unsafe { seal_input(__p, &mut in_len as *mut u32); }\n");
    out.push_str(&format!("    let input: &[u8; {call_buf}] = unsafe {{ &*(__p as *const [u8; {call_buf}]) }};\n"));
    out.push_str("    let _ = in_len;\n");
    out.push_str("    let sel = [input[0], input[1], input[2], input[3]];\n");
    out.push_str("    match sel {\n");
    for arm in arms {
        out.push_str(arm);
    }
    out.push_str(default_arm);
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

/// Mapping storage helpers: blake2-256 keys + typed get/set.
///
/// `map_key(&[MapKey], slot)` = blake2_256( [slot] ++ each key's bytes ).
/// Keys may be an address (32 raw bytes), a 16-byte LE u128/i128 word, or a
/// single byte (bool). The preimage is built into a 65-byte `MaybeUninit`
/// buffer (max: slot + 2 * 32) byte-by-byte to avoid `memory.fill`/`memory.copy`.
const MAP_HELPERS: &str = r#"enum MapKey<'a> { Addr(&'a [u8; 32]), Word(u128), Byte(u8) }
fn map_key(keys: &[MapKey], slot: u8) -> [u8; 32] {
    let mut pre = MaybeUninit::<[u8; 65]>::uninit();
    let pp = pre.as_mut_ptr() as *mut u8;
    let mut n = 0usize;
    unsafe {
        *pp = slot; n = 1;
        let mut ki = 0usize;
        while ki < keys.len() {
            match &keys[ki] {
                MapKey::Addr(a) => {
                    let mut i = 0usize;
                    while i < 32 { *pp.add(n + i) = a[i]; i += 1; }
                    n += 32;
                }
                MapKey::Word(w) => {
                    let le = w.to_le_bytes();
                    let mut i = 0usize;
                    while i < 16 { *pp.add(n + i) = le[i]; i += 1; }
                    n += 16;
                }
                MapKey::Byte(b) => { *pp.add(n) = *b; n += 1; }
            }
            ki += 1;
        }
    }
    let mut out = [0u8; 32];
    unsafe { seal_hash_blake2_256(pp as *const u8, n as u32, out.as_mut_ptr()); }
    out
}
fn map_get_u128(key: [u8; 32]) -> u128 {
    let mut buf = [0u8; 16]; let mut len: u32 = 16;
    let rc = unsafe { seal_get_storage(key.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) };
    if rc == 0 && len >= 16 { u128::from_le_bytes(buf) } else { 0 }
}
fn map_set_u128(key: [u8; 32], v: u128) {
    let b = v.to_le_bytes();
    unsafe { seal_set_storage(key.as_ptr(), b.as_ptr(), 16); }
}
fn map_get_bool(key: [u8; 32]) -> bool {
    let mut buf = [0u8; 1]; let mut len: u32 = 1;
    let rc = unsafe { seal_get_storage(key.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) };
    if rc == 0 && len >= 1 { buf[0] != 0 } else { false }
}
fn map_set_bool(key: [u8; 32], v: bool) {
    let b = [v as u8];
    unsafe { seal_set_storage(key.as_ptr(), b.as_ptr(), 1); }
}
fn map_get_addr(key: [u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 32]; let mut len: u32 = 32;
    let rc = unsafe { seal_get_storage(key.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) };
    let _ = rc; buf
}
fn map_set_addr(key: [u8; 32], v: &[u8; 32]) {
    unsafe { seal_set_storage(key.as_ptr(), v.as_ptr(), 32); }
}
"#;

fn render_cargo_toml(crate_name: &str) -> String {
    format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[profile.release]\npanic = \"abort\"\nlto = true\nopt-level = \"z\"\noverflow-checks = false\n"
    )
}

// The rent-era pallet-contracts wasm validator only accepts MVP wasm: it
// rejects post-MVP features (bulk-memory `memory.copy`, sign-extension, etc.)
// with "Can't decode wasm code". `copy_from_slice` on the 32-byte AccountId
// arrays of the mapping/event tier compiles to `memory.copy`, so those
// features must be disabled to keep the output deployable.
const CARGO_CONFIG: &str = "[target.wasm32-unknown-unknown]\nrustflags = [\n  \"-C\", \"target-feature=-bulk-memory,-sign-ext,-reference-types,-multivalue,-nontrapping-fptoint\",\n  \"-C\", \"link-arg=--import-memory\",\n  \"-C\", \"link-arg=--initial-memory=65536\",\n  \"-C\", \"link-arg=--max-memory=1048576\",\n  \"-C\", \"link-arg=-zstack-size=32768\",\n]\n";

/// Convenience: parse + lower + emit seal artifacts from Solidity source.
pub fn translate_seal(src: &str) -> Result<SealArtifacts, String> {
    let def = crate::parse::parse_contract(src)?;
    let uint_strategy = "u128";
    let name = def.name.as_ref().map(|i| i.name.clone()).unwrap_or_else(|| "Contract".into());
    let storage = crate::lower::lower_storage_structs(&def, uint_strategy);
    let events = crate::lower::lower_events(&def, uint_strategy);
    let errors = crate::lower::lower_errors(&def);
    let (constructor, messages) = crate::lower::lower_functions(&def, uint_strategy);
    let contract = Contract { name, storage, events, errors, constructor, messages };
    // External call targets: every function declared on any contract/interface
    // in the file (incl. bodyless interface decls), keyed by name.
    let external_fns = collect_external_fns(src, uint_strategy);
    emit_seal(&contract, &def, uint_strategy, &external_fns)
}

/// Collect external function signatures from all contracts/interfaces in the
/// source, for cross-contract `IFoo(addr).bar(..)` calls. Keyed by function
/// name (last writer wins; overloads by name are not distinguished here).
fn collect_external_fns(src: &str, uint_strategy: &str) -> BTreeMap<String, ExternalFn> {
    let mut out = BTreeMap::new();
    let all = match crate::parse::parse_all(src) {
        Ok(a) => a,
        Err(_) => return out,
    };
    for c in &all {
        for part in &c.parts {
            if let ContractPart::FunctionDefinition(f) = part {
                if !matches!(f.ty, FunctionTy::Function) {
                    continue;
                }
                let name = match &f.name {
                    Some(id) => id.name.clone(),
                    None => continue,
                };
                let params: Vec<crate::ir::Param> = f
                    .params
                    .iter()
                    .filter_map(|(_, p)| p.as_ref())
                    .filter_map(|p| {
                        crate::lower::map_type_structs(&p.ty, uint_strategy).map(|ty| {
                            crate::ir::Param {
                                name: p.name.as_ref().map(|i| i.name.clone()).unwrap_or_default(),
                                ty,
                            }
                        })
                    })
                    .collect();
                let arg_kinds: Vec<ValTy> = params.iter().map(|p| ValTy::from_type(&p.ty)).collect();
                let ret = f
                    .returns
                    .iter()
                    .filter_map(|(_, p)| p.as_ref())
                    .filter_map(|p| crate::lower::map_type_structs(&p.ty, uint_strategy))
                    .map(|t| ValTy::from_type(&t))
                    .next();
                let selector = selector4(&name, &params);
                out.insert(name, ExternalFn { selector, args: arg_kinds, ret });
            }
        }
    }
    out
}

// keep map_type referenced so future non-scalar detection compiles.
#[allow(unused_imports)]
use map_type as _map_type_ref;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flipper_emits_bool_storage_and_two_messages() {
        let src = r#"
            contract Flipper {
                bool value;
                constructor(bool initial) { value = initial; }
                function flip() public { value = !value; }
                function get() public view returns (bool) { return value; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("pub extern \"C\" fn deploy()"));
        assert!(art.lib_rs.contains("pub extern \"C\" fn call()"));
        assert!(art.lib_rs.contains("store_slot_0"));
        assert!(art.lib_rs.contains("fn load_slot_0() -> bool"));
        // Real keccak-256 4-byte selectors: flip()=0xcde4efa9, get()=0x6d4ce63c.
        assert!(art.lib_rs.contains("[205, 228, 239, 169]"));
        assert!(art.lib_rs.contains("[109, 76, 230, 60]"));
        assert!(art.lib_rs.contains("input[0] != 0"));
        assert!(art.metadata_json.contains("\"selector\": \"0xcde4efa9\""));
        assert!(art.metadata_json.contains("\"selector\": \"0x6d4ce63c\""));
        assert!(art.metadata_json.contains("\"name\": \"get\""));
    }

    #[test]
    fn overloaded_functions_both_emitted_with_distinct_selectors() {
        // Two functions share the name `add` but differ in arity. Each must get
        // its own metadata entry, its own ABI selector, and its own dispatch arm
        // bound to the correct body. Previously the later overload silently
        // dropped the earlier one (silent miscompile).
        let src = r#"
            contract Overload {
                uint256 s;
                function add(uint256 a) public { s += a; }
                function add(uint256 a, uint256 b) public { s += a + b; }
                function get() public view returns (uint256) { return s; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        // Canonical keccak4 selectors: add(uint256)=0x1003e2d2,
        // add(uint256,uint256)=0x771602f7, get()=0x6d4ce63c.
        assert!(
            art.metadata_json.contains("\"selector\": \"0x1003e2d2\""),
            "add(uint256) overload missing from metadata: {}",
            art.metadata_json
        );
        assert!(
            art.metadata_json.contains("\"selector\": \"0x771602f7\""),
            "add(uint256,uint256) overload missing from metadata: {}",
            art.metadata_json
        );
        // Both `add` entries present (two name matches), plus `get`.
        assert_eq!(
            art.metadata_json.matches("\"name\": \"add\"").count(),
            2,
            "expected exactly two `add` overloads in metadata"
        );
        // A distinct dispatch arm per selector.
        assert!(art.lib_rs.contains("[16, 3, 226, 210] =>")); // 0x1003e2d2
        assert!(art.lib_rs.contains("[119, 22, 2, 247] =>")); // 0x771602f7
        // The 1-arg arm reads only `a`; the 2-arg arm also reads `b`. This
        // proves each arm bound to its own body, not the last-writer's.
        assert!(art.lib_rs.contains("input[20..36]"), "2-arg overload body must decode a second param");
    }

    #[test]
    fn counter_uses_checked_add_and_u128() {
        let src = r#"
            contract Counter {
                uint256 count;
                constructor(uint256 initial) { count = initial; }
                function inc() public { count = count + 1; }
                function incBy(uint256 n) public { count = count + n; }
                function get() public view returns (uint256) { return count; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("checked_add"));
        assert!(art.lib_rs.contains("fn load_slot_0() -> u128"));
        assert!(art.lib_rs.contains("u128::from_le_bytes"));
        assert!(art.lib_rs.contains("input[4..20]"));
    }

    #[test]
    fn simplestorage_require_becomes_revert_guard() {
        let src = r#"
            contract SimpleStorage {
                uint256 data;
                constructor(uint256 initial) { data = initial; }
                function set(uint256 x) public { data = x; }
                function setIfPositive(uint256 x) public { require(x > 0); data = x; }
                function get() public view returns (uint256) { return data; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("revert();"));
        assert!(art.lib_rs.contains("if !((x > 0u128))"));
    }

    #[test]
    fn mapping_storage_now_supported() {
        let src = r#"
            contract M {
                mapping(address => uint256) balances;
                function get(address a) public view returns (uint256) { return balances[a]; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("map_key("));
        assert!(art.lib_rs.contains("MapKey::Addr"));
        assert!(art.lib_rs.contains("seal_hash_blake2_256"));
        assert!(art.lib_rs.contains("map_get_u128"));
        // address param decoded as 32 bytes at offset 4.
        assert!(art.lib_rs.contains("input[4..36]"));
        assert!(art.metadata_json.contains("\"address\""));
    }

    #[test]
    fn nested_mapping_uses_map_key2() {
        let src = r#"
            contract M {
                mapping(address => mapping(address => uint256)) allowances;
                function allowance(address o, address s) public view returns (uint256) {
                    return allowances[o][s];
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        // nested mapping access builds a 2-key blake2 preimage.
        assert!(art.lib_rs.contains("MapKey::Addr"));
        assert!(art.lib_rs.matches("MapKey::Addr").count() >= 2);
    }

    #[test]
    fn address_storage_and_msg_sender() {
        let src = r#"
            contract Own {
                address owner;
                constructor() { owner = msg.sender; }
                function owner() public view returns (address) { return owner; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn load_slot_0() -> [u8; 32]"));
        assert!(art.lib_rs.contains("fn caller()"));
        assert!(art.lib_rs.contains("seal_caller"));
        // address return path
        assert!(art.lib_rs.contains("__ret.unwrap_or([0u8; 32])"));
        assert!(art.metadata_json.contains("\"ret\": \"address\""));
    }

    #[test]
    fn modifier_inlined_as_guard() {
        let src = r#"
            contract Own {
                address owner;
                modifier onlyOwner() { require(msg.sender == owner); _; }
                constructor() { owner = msg.sender; }
                function transferOwnership(address n) public onlyOwner { owner = n; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        // guard appears in transferOwnership arm before the body assignment.
        assert!(art.lib_rs.contains("if !((caller() == load_slot_0()))"));
        assert!(art.lib_rs.contains("store_slot_0(n);"));
    }

    #[test]
    fn payable_and_value_and_transfer() {
        let src = r#"
            contract Bank {
                mapping(address => uint256) deposits;
                function deposit() public payable {
                    deposits[msg.sender] = deposits[msg.sender] + msg.value;
                }
                function withdraw(uint256 amount) public {
                    require(deposits[msg.sender] >= amount);
                    deposits[msg.sender] = deposits[msg.sender] - amount;
                    payable(msg.sender).transfer(amount);
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn value()"));
        assert!(art.lib_rs.contains("seal_value_transferred"));
        assert!(art.lib_rs.contains("do_transfer"));
        assert!(art.lib_rs.contains("seal_transfer"));
        assert!(art.metadata_json.contains("\"payable\": true"));
    }

    #[test]
    fn event_emits_deposit_event() {
        let src = r#"
            contract T {
                mapping(address => uint256) balances;
                event Transfer(address indexed from, address indexed to, uint256 value);
                function transfer(address to, uint256 value) public returns (bool) {
                    balances[to] = value;
                    emit Transfer(msg.sender, to, value);
                    return true;
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("seal_deposit_event"));
        assert!(art.metadata_json.contains("\"events\""));
        assert!(art.metadata_json.contains("\"indexed\": true"));
        assert!(art.metadata_json.contains("Transfer(address,address,uint256)"));
    }

    #[test]
    fn snake_case_crate_name() {
        assert_eq!(snake("SimpleStorage"), "simple_storage");
        assert_eq!(snake("Counter"), "counter");
        assert_eq!(snake("Flipper"), "flipper");
        assert_eq!(snake("ERC20"), "erc20");
        assert_eq!(snake("Ownable"), "ownable");
    }

    #[test]
    fn reserved_keyword_crate_name_suffixed() {
        assert_eq!(snake("Pub"), "pub_contract");
    }

    // ---- Round 2, Wave A features ----

    #[test]
    fn fail_loud_on_unsupported_construct() {
        // An unsupported member access must surface as a hard error, not a
        // silently mis-translated contract.
        let src = r#"
            contract Bad {
                function f() public view returns (uint256) { return block.coinbase.balance; }
            }
        "#;
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("unsupported"), "got: {err}");
        assert!(err.contains("member access"), "got: {err}");
    }

    #[test]
    fn signed_int_uses_i128_and_twos_complement_storage() {
        let src = r#"
            contract Signed {
                int256 x;
                constructor(int256 v) { x = v; }
                function dec() public { x = x - 1; }
                function add(int256 d) public { x = x + d; }
                function get() public view returns (int256) { return x; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("i128::from_le_bytes"));
        assert!(art.lib_rs.contains("load_slot_0() as i128"));
        assert!(art.lib_rs.contains("checked_sub"));
        assert!(art.metadata_json.contains("\"ret\": \"i128\""));
        assert!(art.metadata_json.contains("\"args\": [\"i128\"]"));
    }

    #[test]
    fn public_var_synthesizes_getter() {
        let src = r#"
            contract Pub {
                uint256 public count;
                constructor(uint256 c) { count = c; }
                function bump() public { count = count + 1; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.metadata_json.contains("\"name\": \"count\""));
        // getter reads the slot directly.
        assert!(art.lib_rs.contains("__ret = Some(load_slot_0());"));
    }

    #[test]
    fn multi_return_encodes_tuple_and_lists_ret() {
        let src = r#"
            contract MinMax {
                function minmax(uint a, uint b) public pure returns (uint, uint) {
                    if (a < b) { return (a, b); }
                    return (b, a);
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.metadata_json.contains("\"ret\": [\"u128\", \"u128\"]"));
        // two LE words written into the output buffer, then a sliced ret().
        assert!(art.lib_rs.contains("ret(&__mo[..32]);"));
    }

    #[test]
    fn for_loop_lowers_to_while() {
        let src = r#"
            contract Sum {
                function sumTo(uint n) public pure returns (uint) {
                    uint s = 0;
                    for (uint i = 0; i < n; i++) { s = s + i; }
                    return s;
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("while (i < n)"));
        // loop counter mutated via plain assignment (visible after the loop).
        assert!(art.lib_rs.contains("i = (i).checked_add(1u128)"));
        assert!(art.lib_rs.contains("s = (s).checked_add(i)"));
    }

    #[test]
    fn compound_and_incdec() {
        let src = r#"
            contract Inc {
                uint256 n;
                function bump() public { n++; }
                function addmul(uint a) public { n += a; }
                function get() public view returns (uint256) { return n; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("store_slot_0((load_slot_0()).checked_add(1u128)"));
        assert!(art.lib_rs.contains("store_slot_0((load_slot_0()).checked_add(a)"));
    }

    #[test]
    fn bitwise_ops() {
        let src = r#"
            contract Bits {
                function mask(uint x) public pure returns (uint) { return x & 0xff; }
                function shl(uint x) public pure returns (uint) { return x << 2; }
                function inv(uint x) public pure returns (uint) { return ~x; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("(x) & (0xffu128)"));
        assert!(art.lib_rs.contains("(x).wrapping_shl((2u128) as u32)"));
        assert!(art.lib_rs.contains("(!(x))"));
    }

    #[test]
    fn block_context_host_fns() {
        let src = r#"
            contract Timed {
                uint256 start;
                constructor() { start = block.timestamp; }
                function elapsed() public view returns (uint256) { return block.timestamp - start; }
                function afterStart() public view returns (bool) { return block.timestamp >= start; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn seal_now"));
        assert!(art.lib_rs.contains("fn block_timestamp()"));
        assert!(art.lib_rs.contains("block_timestamp() >= "));
    }

    #[test]
    fn balance_and_block_number() {
        let src = r#"
            contract B {
                function bal() public view returns (uint256) { return address(this).balance; }
                function bn() public view returns (uint256) { return block.number; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn seal_balance"));
        assert!(art.lib_rs.contains("self_balance()"));
        assert!(art.lib_rs.contains("fn seal_block_number"));
        assert!(art.lib_rs.contains("block_number()"));
    }

    #[test]
    fn scalar_map_keys_use_word_encoding() {
        let src = r#"
            contract IdStore {
                mapping(uint256 => uint256) byId;
                function set(uint256 id, uint256 v) public { byId[id] = v; }
                function get(uint256 id) public view returns (uint256) { return byId[id]; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("MapKey::Word((id) as u128)"));
        assert!(art.lib_rs.contains("map_set_u128"));
    }

    // ---- Round 3, Wave B: aggregate types ----

    #[test]
    fn address_map_value_uses_addr_helpers() {
        let src = r#"
            contract NFT {
                mapping(uint256 => address) owners;
                function ownerOf(uint256 id) public view returns (address) { return owners[id]; }
                function set(uint256 id, address to) public { owners[id] = to; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn map_get_addr"));
        assert!(art.lib_rs.contains("fn map_set_addr"));
        assert!(art.lib_rs.contains("map_get_addr(map_key("));
        assert!(art.lib_rs.contains("map_set_addr(map_key("));
    }

    #[test]
    fn address_zero_literal_is_zero_account() {
        let src = r#"
            contract Z {
                mapping(uint256 => address) owners;
                function mint(uint256 id) public { require(owners[id] == address(0)); }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("== [0u8; 32]"));
    }

    #[test]
    fn dynamic_array_push_length_index() {
        let src = r#"
            contract IntList {
                uint256[] items;
                function add(uint256 x) public { items.push(x); }
                function len() public view returns (uint256) { return items.length; }
                function get(uint256 i) public view returns (uint256) { return items[i]; }
                function set(uint256 i, uint256 x) public { items[i] = x; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        // length at the plain slot key; elements at blake2(slot ++ u32_le(i)).
        assert!(art.lib_rs.contains("fn arr_len_0()"));
        assert!(art.lib_rs.contains("fn arr_push_u128_0("));
        assert!(art.lib_rs.contains("fn arr_key_0("));
        assert!(art.lib_rs.contains("arr_push_u128_0("));
        assert!(art.lib_rs.contains("arr_len_0()"));
        assert!(art.lib_rs.contains("arr_get_u128_0(i)"));
        assert!(art.lib_rs.contains("arr_set_u128_0(i,"));
    }

    #[test]
    fn mapping_struct_field_access() {
        let src = r#"
            contract Voting {
                struct Proposal { uint256 votes; }
                mapping(uint256 => Proposal) proposals;
                uint256 public count;
                function vote(uint256 id) public { proposals[id].votes += 1; }
                function votesOf(uint256 id) public view returns (uint256) { return proposals[id].votes; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        // struct field key = blake2(slot ++ key_bytes ++ [field_index]).
        assert!(art.lib_rs.contains("MapKey::Byte(0u8)"));
        assert!(art.lib_rs.contains("map_get_u128(map_key(&[MapKey::Word((id) as u128), MapKey::Byte(0u8)], 0))"));
    }

    #[test]
    fn string_storage_param_and_return() {
        let src = r#"
            contract Greeter {
                string greeting;
                constructor(string memory g) { greeting = g; }
                function setGreeting(string memory g) public { greeting = g; }
                function greet() public view returns (string memory) { return greeting; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn store_str_0("));
        assert!(art.lib_rs.contains("fn return_str_0()"));
        assert!(art.lib_rs.contains("store_str_0(__str_g_ptr, __str_g_total);"));
        assert!(art.lib_rs.contains("return_str_0();"));
        assert!(art.metadata_json.contains("\"ret\": \"string\""));
        assert!(art.metadata_json.contains("\"args\": [\"string\"]"));
    }

    #[test]
    fn string_param_must_be_last() {
        let src = r#"
            contract Bad {
                string s;
                function f(string memory a, uint256 b) public { s = a; }
            }
        "#;
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("last parameter"), "got: {err}");
    }

    #[test]
    fn require_with_string_reason_parses() {
        let src = r#"
            contract Req {
                uint256 x;
                function setp(uint256 v) public { require(v > 0, "must be positive"); x = v; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("if !((v > 0u128)) { revert(); }"));
    }

    // ---- Round 4, Wave C ----

    #[test]
    fn keccak_selectors_match_known_abi() {
        // get() and flip() have well-known 4-byte selectors.
        let src = r#"contract F { bool v; function flip() public { v = !v; }
            function get() public view returns (bool) { return v; } }"#;
        let art = translate_seal(src).expect("translate");
        assert!(art.metadata_json.contains("\"selector\": \"0xcde4efa9\""), "flip selector");
        assert!(art.metadata_json.contains("\"selector\": \"0x6d4ce63c\""), "get selector");
    }

    #[test]
    fn event_topic_is_keccak_in_metadata() {
        let src = r#"contract T { mapping(address=>uint256) b;
            event Transfer(address indexed from, address indexed to, uint256 value);
            function f(address to, uint256 v) public { b[to]=v; emit Transfer(msg.sender,to,v); } }"#;
        let art = translate_seal(src).expect("translate");
        // keccak256("Transfer(address,address,uint256)") well-known topic.
        assert!(art.metadata_json.contains(
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        ), "ERC20 Transfer topic");
    }

    #[test]
    fn fail_loud_on_multiple_contracts() {
        let src = "contract A { uint256 x; } contract B { uint256 y; }";
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("multiple deployable contracts"), "got: {err}");
    }

    #[test]
    fn fail_loud_on_missing_base() {
        let src = "contract C is Missing { uint256 x; }";
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("not defined in this file"), "got: {err}");
    }

    #[test]
    fn constant_is_inlined_not_a_slot() {
        let src = r#"contract C { uint256 constant CAP = 1000; uint256 total;
            function setTotal(uint256 n) public { require(n <= CAP); total = n; }
            function get() public view returns (uint256) { return total; } }"#;
        let art = translate_seal(src).expect("translate");
        // CAP inlined; `total` is slot 0 (CAP took no slot).
        assert!(art.lib_rs.contains("n <= 1000u128"));
        assert!(art.lib_rs.contains("fn load_slot_0() -> u128"));
        assert!(!art.lib_rs.contains("load_slot_1"), "CAP must not take a slot");
    }

    #[test]
    fn fail_loud_on_non_literal_constant() {
        let src = r#"contract C { uint256 constant X = block.timestamp;
            function f() public view returns (uint256) { return X; } }"#;
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("compile-time literal"), "got: {err}");
    }

    #[test]
    fn receive_dispatched_on_empty_calldata() {
        let src = r#"contract V { uint256 public got;
            receive() external payable { got = got + msg.value; } }"#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("if in_len == 0 {"));
        assert!(art.lib_rs.contains("value()"));
    }

    #[test]
    fn inheritance_flattens_base_members() {
        let src = r#"
            contract Base { address owner; constructor(){ owner = msg.sender; }
                modifier onlyOwner(){ require(msg.sender==owner); _; } }
            contract Token is Base { uint256 public total;
                function mint(uint256 n) public onlyOwner { total += n; } }
        "#;
        let art = translate_seal(src).expect("translate");
        // Base owner -> slot 0, total -> slot 1; onlyOwner guard inlined into mint.
        assert!(art.lib_rs.contains("store_slot_0(caller());"));
        assert!(art.lib_rs.contains("if !((caller() == load_slot_0())) { revert(); }"));
        assert!(art.metadata_json.contains("\"name\": \"Token\""));
    }

    #[test]
    fn enum_value_is_inlined_ordinal() {
        let src = r#"contract E { enum S { A, B, C } S s;
            function setB() public { s = S.B; }
            function isC() public view returns (bool) { return s == S.C; } }"#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("store_slot_0(1u128);"));   // S.B == 1
        assert!(art.lib_rs.contains("load_slot_0() == 2u128"));  // S.C == 2
    }

    #[test]
    fn struct_local_lowers_to_field_locals() {
        let src = r#"contract S { struct P { uint256 x; uint256 y; }
            function sum(uint256 a, uint256 b) public pure returns (uint256) {
                P memory p = P(a, b); return p.x + p.y; } }"#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("let mut __s_p_x = a;"));
        assert!(art.lib_rs.contains("let mut __s_p_y = b;"));
    }

    #[test]
    fn cross_contract_call_emits_seal_call() {
        let src = r#"
            interface IT { function getValue() external view returns (uint256);
                function setValue(uint256 v) external; }
            contract C {
                function rd(address t) public view returns (uint256) { return IT(t).getValue(); }
                function wr(address t, uint256 v) public { IT(t).setValue(v); }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("fn seal_call("));
        assert!(art.lib_rs.contains("fn do_call("));
        // getValue() selector 0x20965255 -> bytes 32,150,82,85.
        assert!(art.lib_rs.contains("__ci[0]=32u8; __ci[1]=150u8; __ci[2]=82u8; __ci[3]=85u8;"));
    }

    #[test]
    fn cross_contract_call_unknown_method_fails_loud() {
        // `t.nope()` where `nope` is not a declared external fn is not a
        // recognized cross-call and must not silently compile to a no-op.
        let src = r#"contract C {
            function f(address t) public view returns (uint256) { return INope(t).nope(); } }"#;
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("unsupported") || err.contains("function call"), "got: {err}");
    }

    // ---- integer bit-width semantics (Wave E) ----

    #[test]
    fn width_helpers_compute_bounds() {
        assert_eq!(unsigned_mask(8), "255");
        assert_eq!(unsigned_mask(16), "65535");
        assert_eq!(unsigned_mask(32), "4294967295");
        assert_eq!(signed_bounds(8), ("-128".to_string(), "127".to_string()));
        assert_eq!(narrow_width(8), Some(8));
        assert_eq!(narrow_width(128), None);
        assert_eq!(narrow_width(256), None);
        assert_eq!(cast_name_width("uint8"), Some(8));
        assert_eq!(cast_name_width("uint256"), None);
        assert_eq!(cast_name_width("uint"), None);
        assert_eq!(max_width(Some(8), None), Some(8));
        assert_eq!(max_width(Some(8), Some(16)), Some(16));
        assert_eq!(max_width(None, None), None);
    }

    #[test]
    fn uint8_checked_add_reverts_out_of_range() {
        // uint8 + must revert when the result exceeds 255.
        let src = r#"
            contract C {
                uint8 x;
                function addc(uint8 n) public { x = x + n; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(
            art.lib_rs.contains("if __r > 255u128 { revert() }"),
            "expected uint8 range check, got body:\n{}",
            art.lib_rs
        );
        assert!(art.lib_rs.contains("checked_add(n)"));
    }

    #[test]
    fn uint16_checked_add_uses_65535_bound() {
        let src = r#"
            contract C {
                function f(uint16 a, uint16 b) public pure returns (uint16) { return a + b; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("if __r > 65535u128 { revert() }"), "{}", art.lib_rs);
    }

    #[test]
    fn uint8_unchecked_add_wraps_and_masks() {
        let src = r#"
            contract C {
                function wrap(uint8 a) public pure returns (uint8) {
                    unchecked { return a + 1; }
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(
            art.lib_rs.contains("(a).wrapping_add(1u128) & 255u128"),
            "expected wrapping mask, got:\n{}",
            art.lib_rs
        );
        // The checked range-check form must NOT appear inside an unchecked block.
        assert!(!art.lib_rs.contains("if __r > 255u128"));
    }

    #[test]
    fn explicit_narrow_uint_cast_masks_to_width() {
        // uint8(x) must truncate to 8 bits (& 255), matching Solidity modulo-256.
        let src = r#"
            contract C {
                function down(uint256 x) public pure returns (uint8) { return uint8(x); }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(
            art.lib_rs.contains("& 255u128"),
            "expected uint8 cast to mask with 255, got body:\n{}",
            art.lib_rs
        );
    }

    #[test]
    fn explicit_narrow_int_cast_sign_extends() {
        // int8(x) must sign-extend the low 8 bits: (<x> << 120) >> 120 on i128.
        let src = r#"
            contract C {
                function down(int256 x) public pure returns (int8) { return int8(x); }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(
            art.lib_rs.contains("<< 120u32) >> 120u32"),
            "expected int8 cast to sign-extend from 8 bits, got body:\n{}",
            art.lib_rs
        );
    }

    #[test]
    fn explicit_wide_uint256_cast_does_not_mask() {
        // uint256(x) is a widening/identity cast and must not add a width mask.
        let src = r#"
            contract C {
                function up(uint8 a) public pure returns (uint256) { return uint256(a); }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(
            !art.lib_rs.contains("& 255u128"),
            "uint256 cast must not mask, got body:\n{}",
            art.lib_rs
        );
    }

    #[test]
    fn cast_width_helpers_round_trip() {
        assert_eq!(mask_narrow("v", Some(8)), "((v) & 255u128)");
        assert_eq!(mask_narrow("v", None), "v");
        assert_eq!(sign_extend_narrow("v", Some(8)), "(((v) << 120u32) >> 120u32)");
        assert_eq!(sign_extend_narrow("v", None), "v");
    }

    #[test]
    fn wide_uint256_keeps_plain_checked_add() {
        // uint256/uint -> wide: no narrow range check, fail-safe checked_* at 128.
        let src = r#"
            contract C {
                uint256 t;
                function inc(uint256 n) public { t = t + n; }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("checked_add(n).unwrap_or_else(|| revert())"));
        assert!(!art.lib_rs.contains("if __r >"), "wide type must not get a narrow check");
    }

    #[test]
    fn uint8_unchecked_mul_wraps() {
        let src = r#"
            contract C {
                function m(uint8 a, uint8 b) public pure returns (uint8) {
                    unchecked { return a * b; }
                }
            }
        "#;
        let art = translate_seal(src).expect("translate");
        assert!(art.lib_rs.contains("wrapping_mul"), "{}", art.lib_rs);
        assert!(art.lib_rs.contains("& 255u128"), "{}", art.lib_rs);
    }
}
