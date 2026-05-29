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
    /// `bool`.
    Bool,
    /// `[u8; 32]` AccountId / address.
    Addr,
}

impl ValTy {
    fn from_type(t: &Type) -> ValTy {
        match t {
            Type::Bool => ValTy::Bool,
            Type::AccountId => ValTy::Addr,
            _ => ValTy::Num,
        }
    }
}

/// The kind of a storage slot.
#[derive(Debug, Clone)]
enum SlotKind {
    /// A scalar value of the given runtime kind.
    Scalar(ValTy),
    /// `mapping(K => V)`.
    Map { val: ValTy },
    /// `mapping(A => mapping(B => V))`.
    Map2 { val: ValTy },
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
}

/// Lowering context for seal0 statement/expression generation.
struct SealCtx<'a> {
    slots: &'a [Slot],
    events: &'a [crate::ir::Event],
    /// Local variable -> runtime kind (params).
    locals: BTreeMap<String, ValTy>,
    uses: Uses,
    errors: Vec<String>,
}

impl<'a> SealCtx<'a> {
    fn new(slots: &'a [Slot], events: &'a [crate::ir::Event]) -> Self {
        SealCtx {
            slots,
            events,
            locals: BTreeMap::new(),
            uses: Uses::default(),
            errors: Vec::new(),
        }
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
                if let Some(slot) = self.slot_of(&id.name) {
                    if let SlotKind::Scalar(vt) = slot.kind {
                        return (format!("load_slot_{}()", slot.index), vt);
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

            Expression::Add(_, l, r) => (self.arith(l, r, "checked_add"), ValTy::Num),
            Expression::Subtract(_, l, r) => (self.arith(l, r, "checked_sub"), ValTy::Num),
            Expression::Multiply(_, l, r) => (self.arith(l, r, "checked_mul"), ValTy::Num),
            Expression::Divide(_, l, r) => (self.arith(l, r, "checked_div"), ValTy::Num),
            Expression::Modulo(_, l, r) => (self.arith(l, r, "checked_rem"), ValTy::Num),

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

            // Mapping read: m[k] or m[a][b].
            Expression::ArraySubscript(_, _, Some(_)) => {
                if let Some((slot_idx, val, keys)) = self.as_map_access(e) {
                    let key_expr = self.map_key_call(slot_idx, &keys);
                    let getter = match val {
                        ValTy::Bool => format!("map_get_bool({key_expr})"),
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
                }
                (self.err(&format!("member access .{}", member.name)), ValTy::Num)
            }

            // `address(x)` / `payable(x)` casts: pass through the inner value.
            Expression::FunctionCall(_, callee, args) => {
                if let Expression::Type(_, PtType::Payable) = callee.as_ref() {
                    if let Some(a) = args.first() {
                        return self.expr_ty(a);
                    }
                }
                if let Expression::Variable(id) = callee.as_ref() {
                    if id.name == "address" {
                        if let Some(a) = args.first() {
                            return self.expr_ty(a);
                        }
                    }
                }
                (self.err("function call in value position"), ValTy::Num)
            }

            _ => (self.err("expression"), ValTy::Num),
        }
    }

    /// Render an expression (value context), discarding the kind.
    fn expr(&mut self, e: &Expression) -> String {
        self.expr_ty(e).0
    }

    fn arith(&mut self, l: &Expression, r: &Expression, op: &str) -> String {
        let l = self.expr(l);
        let r = self.expr(r);
        format!("({l}).{op}({r}).unwrap_or_else(|| revert())")
    }

    fn cmp(&mut self, l: &Expression, r: &Expression, op: &str) -> String {
        let (l, _) = self.expr_ty(l);
        let (r, _) = self.expr_ty(r);
        format!("({l} {op} {r})")
    }

    /// If `e` is a mapping access `m[k]` or `m[a][b]` rooted at a storage
    /// mapping, return `(slot_index, value_kind, key_exprs)`.
    fn as_map_access(&mut self, e: &Expression) -> Option<(u8, ValTy, Vec<String>)> {
        if let Expression::ArraySubscript(_, base, Some(index)) = e {
            let key = self.expr(index);
            match base.as_ref() {
                Expression::Variable(id) => {
                    let slot = self.slot_of(&id.name)?;
                    match slot.kind {
                        SlotKind::Map { val } => Some((slot.index, val, vec![key])),
                        _ => None,
                    }
                }
                Expression::ArraySubscript(..) => {
                    // nested: base is m[a], so this is m[a][b].
                    if let Expression::ArraySubscript(_, inner_base, Some(inner_idx)) =
                        base.as_ref()
                    {
                        if let Expression::Variable(id) = inner_base.as_ref() {
                            let slot = self.slot_of(&id.name)?;
                            let info = match slot.kind {
                                SlotKind::Map2 { val } => Some((slot.index, val)),
                                _ => None,
                            };
                            if let Some((idx, val)) = info {
                                let a = self.expr(inner_idx);
                                return Some((idx, val, vec![a, key]));
                            }
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

    /// Emit the call that computes a mapping storage key for the given slot and
    /// key expressions. Key bytes are the SCALE/raw encoding of each key: a
    /// u128 key is 16 LE bytes, an address key is 32 raw bytes. To keep the
    /// generated helper simple we restrict map keys to `address` (32 bytes),
    /// which covers all spec fixtures; numeric keys fall back to 16-byte LE.
    fn map_key_call(&mut self, slot: u8, keys: &[String]) -> String {
        self.uses.blake2 = true;
        match keys.len() {
            1 => format!("map_key1({slot}, &{})", keys[0]),
            _ => format!("map_key2({slot}, &{}, &{})", keys[0], keys[1]),
        }
    }

    /// Render a statement into Rust source lines.
    fn stmt(&mut self, s: &Statement) -> Vec<String> {
        match s {
            Statement::Block { statements, .. } => {
                statements.iter().flat_map(|st| self.stmt(st)).collect()
            }

            Statement::Expression(_, e) => match e {
                Expression::Assign(_, lhs, rhs) => self.assign(lhs, rhs),
                Expression::FunctionCall(_, callee, args) => self.call_stmt(callee, args),
                _ => {
                    let r = self.expr(e);
                    vec![format!("let _ = {r};")]
                }
            },

            Statement::Return(_, Some(e)) => {
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

            Statement::Emit(_, e) => self.emit_stmt(e),

            Statement::Revert(_, _path, _args) => vec!["revert();".to_string()],

            // Local variable declaration `T name = expr;`.
            Statement::VariableDefinition(_, decl, init) => {
                let name = decl.name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                if let Some(rhs) = init {
                    let (v, t) = self.expr_ty(rhs);
                    self.locals.insert(name.clone(), t);
                    vec![format!("let {name} = {v};")]
                } else {
                    self.locals.insert(name.clone(), ValTy::Num);
                    vec![format!("let {name};")]
                }
            }

            _ => vec![format!("// {}", self.err("statement"))],
        }
    }

    /// `lhs = rhs;` — scalar storage write, mapping write, or local.
    fn assign(&mut self, lhs: &Expression, rhs: &Expression) -> Vec<String> {
        // Mapping write?
        if let Expression::ArraySubscript(..) = lhs {
            if let Some((slot_idx, val, keys)) = self.as_map_access(lhs) {
                let (value, _) = self.expr_ty(rhs);
                let key_expr = self.map_key_call(slot_idx, &keys);
                let setter = match val {
                    ValTy::Bool => format!("map_set_bool({key_expr}, {value});"),
                    _ => format!("map_set_u128({key_expr}, {value});"),
                };
                return vec![setter];
            }
        }
        let (value, vt) = self.expr_ty(rhs);
        if let Expression::Variable(id) = lhs {
            if let Some(slot) = self.slot_of(&id.name) {
                if let SlotKind::Scalar(_) = slot.kind {
                    return vec![format!("store_slot_{}({value});", slot.index)];
                }
            }
            self.locals.insert(id.name.clone(), vt);
            return vec![format!("let {} = {value};", id.name)];
        }
        vec![format!("// {}", self.err("assignment target"))]
    }

    /// A bare call statement: `require(...)`, `revert()`, `addr.transfer(x)`.
    fn call_stmt(&mut self, callee: &Expression, args: &[Expression]) -> Vec<String> {
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
                // first topic = event signature hash (blake2 of name bytes)
                self.uses.blake2 = true;
                let sig = event_sig_string(&ev);
                lines.push(format!(
                    "{{ let __sig = b\"{sig}\"; unsafe {{ seal_hash_blake2_256(__sig.as_ptr(), {} , __topics.as_mut_ptr().add(1)); }} }}",
                    sig.len()
                ));
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
                        ValTy::Num => 16,
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
                        ValTy::Num => {
                            lines.push(format!(
                                "{{ let __le = ({var} as u128).to_le_bytes(); let mut __i = 0usize; while __i < 16 {{ __data[{doff} + __i] = __le[__i]; __i += 1; }} }}"
                            ));
                            doff += 16;
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

/// Canonical event signature string `Name(t1,t2,...)`.
fn event_sig_string(ev: &crate::ir::Event) -> String {
    let parts: Vec<&str> = ev
        .fields
        .iter()
        .map(|f| match f.ty {
            Type::AccountId => "address",
            Type::Bool => "bool",
            _ => "uint256",
        })
        .collect();
    format!("{}({})", ev.name, parts.join(","))
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
    out
}

/// Number of SCALE/storage bytes for a scalar value kind.
fn val_len(t: ValTy) -> usize {
    match t {
        ValTy::Bool => 1,
        ValTy::Num => 16,
        ValTy::Addr => 32,
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
) -> Result<SealArtifacts, String> {
    let _ = uint_strategy;

    // Assign slots, validating supported storage shapes.
    let mut slots: Vec<Slot> = Vec::new();
    for (i, f) in c.storage.iter().enumerate() {
        let kind = match &f.ty {
            Type::Bool => SlotKind::Scalar(ValTy::Bool),
            Type::U128 => SlotKind::Scalar(ValTy::Num),
            Type::AccountId => SlotKind::Scalar(ValTy::Addr),
            Type::U256 => {
                return Err(format!(
                    "field `{}`: u256 not supported (use u128 strategy)",
                    f.name
                ))
            }
            Type::Mapping(k, v) => match (k.as_ref(), v.as_ref()) {
                // mapping(address => scalar)
                (Type::AccountId, inner) if is_scalar(inner) => {
                    SlotKind::Map { val: ValTy::from_type(inner) }
                }
                // mapping(address => mapping(address => scalar))
                (Type::AccountId, Type::Mapping(k2, v2))
                    if matches!(k2.as_ref(), Type::AccountId) && is_scalar(v2) =>
                {
                    SlotKind::Map2 { val: ValTy::from_type(v2) }
                }
                _ => {
                    return Err(format!(
                        "field `{}`: only mapping(address=>scalar) and \
                         mapping(address=>mapping(address=>scalar)) supported",
                        f.name
                    ))
                }
            },
            other => {
                return Err(format!("field `{}`: unsupported storage type {other:?}", f.name))
            }
        };
        slots.push(Slot { name: f.name.clone(), index: i as u8, kind });
    }

    let modifiers = collect_modifiers(def);
    let mut uses = Uses::default();
    let mut all_errors: Vec<String> = Vec::new();

    // ----- Constructor body -----
    let ctor_body_lines: Vec<String> = if let Some(ctor) = &c.constructor {
        let fdef = find_function(def, true, "");
        let mut ctx = SealCtx::new(&slots, &c.events);
        register_params(&mut ctx, &ctor.params);
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

    // ----- Messages -----
    let mut arms: Vec<String> = Vec::new();
    let mut meta_messages: Vec<String> = Vec::new();

    for (i, msg) in c.messages.iter().enumerate() {
        let selector = (i + 1) as u32;
        let sel_bytes = selector.to_be_bytes();
        let pat = format!(
            "[{}, {}, {}, {}]",
            sel_bytes[0], sel_bytes[1], sel_bytes[2], sel_bytes[3]
        );

        let fdef = find_function(def, false, &msg.name);
        let mut ctx = SealCtx::new(&slots, &c.events);
        register_params(&mut ctx, &msg.params);

        let mut body_lines: Vec<String> = decode_params_prelude(&msg.params, true);

        let has_ret = msg.returns.is_some();
        if has_ret {
            let rt = ret_rust_ty(msg.returns.as_ref().unwrap());
            body_lines.push(format!("let mut __ret: Option<{rt}> = None;"));
        }

        // Inline modifier guards (e.g. onlyOwner) at function entry.
        if let Some(fdef) = fdef {
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
        }
        merge_uses(&mut uses, ctx.uses);
        all_errors.extend(ctx.errors);

        // Return emission.
        if has_ret {
            match ValTy::from_type(msg.returns.as_ref().unwrap()) {
                ValTy::Bool => {
                    body_lines.push("let __v = __ret.unwrap_or(false);".to_string());
                    body_lines.push("let __out = [__v as u8];".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
                ValTy::Addr => {
                    body_lines.push("let __v = __ret.unwrap_or([0u8; 32]);".to_string());
                    body_lines.push("ret(&__v);".to_string());
                }
                ValTy::Num => {
                    body_lines.push("let __v = __ret.unwrap_or(0u128);".to_string());
                    body_lines.push("let __out = __v.to_le_bytes();".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
            }
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
        let ret_meta = match &msg.returns {
            Some(t) => format!("\"{}\"", meta_ty(t)),
            None => "null".to_string(),
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

    let lib_rs = render_lib_rs(&slots, &ctor_body_lines, &arms, uses, deploy_buf, call_buf);

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
                "    {{ \"name\": \"{}\", \"sig\": \"{}\", \"fields\": [{}] }}",
                ev.name,
                event_sig_string(ev),
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
    matches!(t, Type::Bool | Type::U128 | Type::AccountId)
}

fn ret_rust_ty(t: &Type) -> &'static str {
    match ValTy::from_type(t) {
        ValTy::Bool => "bool",
        ValTy::Addr => "[u8; 32]",
        ValTy::Num => "u128",
    }
}

fn register_params(ctx: &mut SealCtx, params: &[crate::ir::Param]) {
    for p in params {
        ctx.locals.insert(p.name.clone(), ValTy::from_type(&p.ty));
    }
}

fn merge_uses(acc: &mut Uses, u: Uses) {
    acc.caller |= u.caller;
    acc.value |= u.value;
    acc.blake2 |= u.blake2;
    acc.deposit_event |= u.deposit_event;
    acc.transfer |= u.transfer;
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
                ValTy::Num => {
                    out.push_str(&format!(
                        "fn store_slot_{s}(v: u128) {{ let b = v.to_le_bytes(); unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), b.as_ptr(), 16); }} }}\n"
                    ));
                    out.push_str(&format!(
                        "fn load_slot_{s}() -> u128 {{ let mut buf = [0u8; 16]; let mut len: u32 = 16; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 16 {{ u128::from_le_bytes(buf) }} else {{ 0 }} }}\n"
                    ));
                }
            }
        }
    }
    out
}

/// Whether the contract has any mapping slot (needs blake2 key helpers).
fn has_mapping(slots: &[Slot]) -> bool {
    slots
        .iter()
        .any(|s| matches!(s.kind, SlotKind::Map { .. } | SlotKind::Map2 { .. }))
}

/// Render the complete `lib.rs`.
fn render_lib_rs(
    slots: &[Slot],
    ctor_body: &[String],
    arms: &[String],
    uses: Uses,
    deploy_buf: usize,
    call_buf: usize,
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
    out.push_str("        _ => { revert(); }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

/// Mapping storage helpers: blake2-256 keys + typed get/set.
///
/// `map_key1(slot, key32)` = blake2_256( [slot] ++ key ) ; key is a 32-byte
/// AccountId. `map_key2` chains two keys.
const MAP_HELPERS: &str = r#"fn map_key1(slot: u8, k: &[u8; 32]) -> [u8; 32] {
    // `pre` is left uninitialized (MaybeUninit) to avoid a `memory.fill`; every
    // byte is written before the hash reads it.
    let mut pre = MaybeUninit::<[u8; 33]>::uninit();
    let pp = pre.as_mut_ptr() as *mut u8;
    unsafe {
        *pp = slot;
        let mut i = 0usize;
        while i < 32 { *pp.add(1 + i) = k[i]; i += 1; }
    }
    let mut out = [0u8; 32];
    unsafe { seal_hash_blake2_256(pp as *const u8, 33, out.as_mut_ptr()); }
    out
}
fn map_key2(slot: u8, a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut pre = MaybeUninit::<[u8; 65]>::uninit();
    let pp = pre.as_mut_ptr() as *mut u8;
    unsafe {
        *pp = slot;
        let mut i = 0usize;
        while i < 32 { *pp.add(1 + i) = a[i]; i += 1; }
        let mut j = 0usize;
        while j < 32 { *pp.add(33 + j) = b[j]; j += 1; }
    }
    let mut out = [0u8; 32];
    unsafe { seal_hash_blake2_256(pp as *const u8, 65, out.as_mut_ptr()); }
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
    let storage = crate::lower::lower_storage(&def, uint_strategy);
    let events = crate::lower::lower_events(&def, uint_strategy);
    let errors = crate::lower::lower_errors(&def);
    let (constructor, messages) = crate::lower::lower_functions(&def, uint_strategy);
    let contract = Contract { name, storage, events, errors, constructor, messages };
    emit_seal(&contract, &def, uint_strategy)
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
        assert!(art.lib_rs.contains("[0, 0, 0, 1]"));
        assert!(art.lib_rs.contains("[0, 0, 0, 2]"));
        assert!(art.lib_rs.contains("input[0] != 0"));
        assert!(art.metadata_json.contains("\"selector\": \"0x00000001\""));
        assert!(art.metadata_json.contains("\"name\": \"get\""));
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
        assert!(art.lib_rs.contains("map_key1"));
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
        assert!(art.lib_rs.contains("map_key2"));
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
}
