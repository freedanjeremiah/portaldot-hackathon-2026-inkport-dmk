//! seal0 codegen backend.
//!
//! Emits raw `seal0` Rust (`lib.rs`) plus a metadata JSON for the scalar tier:
//! contracts using only scalar state (`bool`, `uintN`→`u128`). No mappings,
//! no events. Follows `docs/seal-backend-spec.md` and mirrors the shape of the
//! proven `onchain-contracts/counter` contract.
//!
//! Unlike the ink! path (which leaves IR `Function.body` empty and renders via
//! `codegen.rs`), this backend re-parses the Solidity AST with solang and does
//! seal0-specific statement lowering directly off the parse tree.

use solang_parser::pt::{
    ContractDefinition, ContractPart, Expression, FunctionTy, Statement,
};

use crate::ir::{Contract, Mutability, Type};
use crate::lower::map_type;

/// A scalar storage field with its assigned slot index.
struct Slot {
    name: String,
    ty: Type,
    index: u8,
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

/// Lowering context for seal0 statement/expression generation.
struct SealCtx<'a> {
    slots: &'a [Slot],
    /// Whether `seal_input` style overflow helpers (checked arith) were used —
    /// always emitted; tracked here only for clarity.
    used_caller: bool,
    errors: Vec<String>,
}

impl<'a> SealCtx<'a> {
    fn new(slots: &'a [Slot]) -> Self {
        SealCtx { slots, used_caller: false, errors: Vec::new() }
    }

    fn slot_of(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.name == name)
    }

    fn err(&mut self, msg: &str) -> String {
        self.errors.push(msg.to_string());
        format!("/* UNSUPPORTED: {msg} */ 0")
    }

    /// Render a Solidity expression to a Rust expression string (value context).
    ///
    /// All numeric values are `u128`; bool values are `bool`.
    fn expr(&mut self, e: &Expression) -> String {
        match e {
            Expression::Parenthesis(_, inner) => format!("({})", self.expr(inner)),

            Expression::Variable(id) => {
                if let Some(slot) = self.slot_of(&id.name) {
                    format!("load_slot_{}()", slot.index)
                } else {
                    id.name.clone()
                }
            }

            Expression::NumberLiteral(_, value, _, _) => {
                // Decimal integer literal -> u128 literal.
                format!("{value}u128")
            }
            Expression::BoolLiteral(_, b) => {
                if *b { "true".into() } else { "false".into() }
            }

            Expression::Not(_, inner) => format!("!({})", self.expr(inner)),

            Expression::Add(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("({l}).checked_add({r}).unwrap_or_else(|| revert())")
            }
            Expression::Subtract(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("({l}).checked_sub({r}).unwrap_or_else(|| revert())")
            }
            Expression::Multiply(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("({l}).checked_mul({r}).unwrap_or_else(|| revert())")
            }

            Expression::Less(_, l, r) => self.cmp(l, r, "<"),
            Expression::More(_, l, r) => self.cmp(l, r, ">"),
            Expression::LessEqual(_, l, r) => self.cmp(l, r, "<="),
            Expression::MoreEqual(_, l, r) => self.cmp(l, r, ">="),
            Expression::Equal(_, l, r) => self.cmp(l, r, "=="),
            Expression::NotEqual(_, l, r) => self.cmp(l, r, "!="),

            Expression::And(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("({l} && {r})")
            }
            Expression::Or(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("({l} || {r})")
            }

            Expression::MemberAccess(_, base, member) => {
                if let Expression::Variable(id) = base.as_ref() {
                    if id.name == "msg" && member.name == "sender" {
                        self.used_caller = true;
                        return "caller()".to_string();
                    }
                }
                self.err(&format!("member access .{}", member.name))
            }

            _ => self.err("expression"),
        }
    }

    fn cmp(&mut self, l: &Expression, r: &Expression, op: &str) -> String {
        let l = self.expr(l);
        let r = self.expr(r);
        format!("({l} {op} {r})")
    }

    /// Render a Solidity statement into Rust source lines (no trailing indent).
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
                let v = self.expr(e);
                vec![format!("__ret = Some({v});")]
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

            Statement::Revert(_, _path, _args) => vec!["revert();".to_string()],

            _ => vec![format!("// {}", self.err("statement"))],
        }
    }

    /// Handle `name = value;` (storage scalar write or local).
    fn assign(&mut self, lhs: &Expression, rhs: &Expression) -> Vec<String> {
        let value = self.expr(rhs);
        if let Expression::Variable(id) = lhs {
            if let Some(slot) = self.slot_of(&id.name) {
                return vec![format!("store_slot_{}({value});", slot.index)];
            }
            return vec![format!("let mut {} = {value};", id.name)];
        }
        vec![format!("// {}", self.err("assignment target"))]
    }

    /// Handle a bare call statement: `require(...)` / `revert()`.
    fn call_stmt(&mut self, callee: &Expression, args: &[Expression]) -> Vec<String> {
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
}

/// Convert a contract name to a snake_case crate name.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Number of SCALE/storage bytes for a scalar type.
fn scalar_len(t: &Type) -> usize {
    match t {
        Type::Bool => 1,
        _ => 16, // uintN -> u128
    }
}

/// Metadata type string for a scalar type.
fn meta_ty(t: &Type) -> &'static str {
    match t {
        Type::Bool => "bool",
        _ => "u128",
    }
}

/// Find a function body statement in the parse tree by name & kind.
fn find_function<'a>(
    def: &'a ContractDefinition,
    is_ctor: bool,
    name: &str,
) -> Option<&'a solang_parser::pt::FunctionDefinition> {
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

/// Generate seal0 artifacts from the IR contract + the parsed AST.
///
/// `uint_strategy` should be "u128" (the scalar tier only supports u128).
pub fn emit_seal(
    c: &Contract,
    def: &ContractDefinition,
    uint_strategy: &str,
) -> Result<SealArtifacts, String> {
    // Reject non-scalar tier features.
    for f in &c.storage {
        match f.ty {
            Type::Bool | Type::U128 => {}
            Type::U256 => {
                return Err(format!(
                    "field `{}`: u256 not supported in scalar tier (use u128)",
                    f.name
                ))
            }
            _ => {
                return Err(format!(
                    "field `{}`: only scalar bool/uintN storage supported in scalar tier",
                    f.name
                ))
            }
        }
    }
    if !c.events.is_empty() {
        return Err("events not supported in scalar tier".into());
    }

    // Assign slots.
    let slots: Vec<Slot> = c
        .storage
        .iter()
        .enumerate()
        .map(|(i, f)| Slot { name: f.name.clone(), ty: f.ty.clone(), index: i as u8 })
        .collect();

    let mut used_caller = false;
    let mut all_errors: Vec<String> = Vec::new();

    // ----- Constructor body -----
    let ctor_body_lines: Vec<String> = if let Some(_ctor) = &c.constructor {
        let fdef = find_function(def, true, "");
        let mut ctx = SealCtx::new(&slots);
        // Decode ctor params from input in order, then run body.
        let mut lines = Vec::new();
        if let Some(ctor) = &c.constructor {
            // Param decode prelude.
            lines.extend(decode_params_prelude(&ctor.params, false));
        }
        if let Some(fdef) = fdef {
            if let Some(body) = &fdef.body {
                for l in ctx.stmt(body) {
                    lines.push(l);
                }
            }
        }
        used_caller |= ctx.used_caller;
        all_errors.extend(ctx.errors);
        lines
    } else {
        Vec::new()
    };

    // ----- Messages: build dispatch arms -----
    let mut arms: Vec<String> = Vec::new();
    let mut meta_messages: Vec<String> = Vec::new();

    for (i, msg) in c.messages.iter().enumerate() {
        let selector = (i + 1) as u32; // 1-based
        let sel_bytes = selector.to_be_bytes();
        let pat = format!(
            "[{}, {}, {}, {}]",
            sel_bytes[0], sel_bytes[1], sel_bytes[2], sel_bytes[3]
        );

        let fdef = find_function(def, false, &msg.name);
        let mut ctx = SealCtx::new(&slots);

        let mut body_lines: Vec<String> = Vec::new();
        // Decode message params (after the 4-byte selector).
        body_lines.extend(decode_params_prelude(&msg.params, true));
        // __ret holder for returns.
        let has_ret = msg.returns.is_some();
        if has_ret {
            let ret_ty = msg.returns.as_ref().unwrap();
            let rt = match ret_ty {
                Type::Bool => "bool",
                _ => "u128",
            };
            body_lines.push(format!("let mut __ret: Option<{rt}> = None;"));
        }
        if let Some(fdef) = fdef {
            if let Some(body) = &fdef.body {
                for l in ctx.stmt(body) {
                    body_lines.push(l);
                }
            }
        }
        used_caller |= ctx.used_caller;
        all_errors.extend(ctx.errors);

        // Return emission.
        if has_ret {
            let ret_ty = msg.returns.as_ref().unwrap();
            match ret_ty {
                Type::Bool => {
                    body_lines.push("let __v = __ret.unwrap_or(false);".to_string());
                    body_lines.push("let __out = [__v as u8];".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
                _ => {
                    body_lines.push("let __v = __ret.unwrap_or(0u128);".to_string());
                    body_lines.push("let __out = __v.to_le_bytes();".to_string());
                    body_lines.push("ret(&__out);".to_string());
                }
            }
        } else {
            body_lines.push("ret(&[]);".to_string());
        }

        // Build the arm.
        let mut arm = String::new();
        arm.push_str(&format!("        {pat} => {{\n"));
        for l in &body_lines {
            arm.push_str(&format!("            {l}\n"));
        }
        arm.push_str("        }\n");
        arms.push(arm);

        // Metadata entry.
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
        meta_messages.push(format!(
            "    {{ \"name\": \"{}\", \"selector\": \"0x{:08x}\", \"args\": [{}], \"ret\": {}, \"mutates\": {} }}",
            msg.name,
            selector,
            args_meta.join(", "),
            ret_meta,
            mutates
        ));
    }

    let _ = uint_strategy;

    // ----- Buffer sizes (keep stack tiny; the node rejects a large fixed
    // input buffer in `deploy`, so size to the actual payload). -----
    let ctor_bytes: usize = c
        .constructor
        .as_ref()
        .map(|ct| ct.params.iter().map(|p| scalar_len(&p.ty)).sum())
        .unwrap_or(0);
    let deploy_buf = ctor_bytes.max(1);
    let max_msg_args: usize = c
        .messages
        .iter()
        .map(|m| m.params.iter().map(|p| scalar_len(&p.ty)).sum::<usize>())
        .max()
        .unwrap_or(0);
    let call_buf = 4 + max_msg_args;

    // ----- Assemble lib.rs -----
    let lib_rs = render_lib_rs(&slots, &ctor_body_lines, &arms, used_caller, c, deploy_buf, call_buf);

    // ----- Assemble metadata.json -----
    let ctor_args_meta: Vec<String> = c
        .constructor
        .as_ref()
        .map(|ct| ct.params.iter().map(|p| format!("\"{}\"", meta_ty(&p.ty))).collect())
        .unwrap_or_default();
    let metadata_json = format!(
        "{{\n  \"name\": \"{}\",\n  \"constructor\": {{ \"args\": [{}] }},\n  \"messages\": [\n{}\n  ]\n}}\n",
        c.name,
        ctor_args_meta.join(", "),
        meta_messages.join(",\n")
    );

    let crate_name = snake(&c.name);
    let cargo_toml = render_cargo_toml(&crate_name);
    let cargo_config_toml = CARGO_CONFIG.to_string();

    Ok(SealArtifacts { lib_rs, metadata_json, cargo_toml, cargo_config_toml, crate_name })
}

/// Emit the prelude lines that decode parameters from the input buffer.
///
/// For messages, decoding starts at offset 4 (after selector); for ctor at 0.
/// All numeric params are u128 (16 bytes LE); bools are 1 byte.
fn decode_params_prelude(params: &[crate::ir::Param], after_selector: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut off: usize = if after_selector { 4 } else { 0 };
    for p in params {
        match p.ty {
            Type::Bool => {
                lines.push(format!("let {} = input[{}] != 0;", p.name, off));
                off += 1;
            }
            _ => {
                // u128 little-endian, 16 bytes.
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

/// Render the per-field storage load/store helpers.
fn render_slot_helpers(slots: &[Slot]) -> String {
    let mut out = String::new();
    for slot in slots {
        let s = slot.index;
        out.push_str(&format!(
            "static KEY_{s}: [u8; 32] = {{ let mut k = [0u8; 32]; k[0] = {s}; k }};\n"
        ));
        match slot.ty {
            Type::Bool => {
                out.push_str(&format!(
                    "fn store_slot_{s}(v: bool) {{ let b = [v as u8]; unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), b.as_ptr(), 1); }} }}\n"
                ));
                out.push_str(&format!(
                    "fn load_slot_{s}() -> bool {{ let mut buf = [0u8; 1]; let mut len: u32 = 1; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 1 {{ buf[0] != 0 }} else {{ false }} }}\n"
                ));
            }
            _ => {
                let _ = scalar_len(&slot.ty);
                out.push_str(&format!(
                    "fn store_slot_{s}(v: u128) {{ let b = v.to_le_bytes(); unsafe {{ seal_set_storage(KEY_{s}.as_ptr(), b.as_ptr(), 16); }} }}\n"
                ));
                out.push_str(&format!(
                    "fn load_slot_{s}() -> u128 {{ let mut buf = [0u8; 16]; let mut len: u32 = 16; let rc = unsafe {{ seal_get_storage(KEY_{s}.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) }}; if rc == 0 && len >= 16 {{ u128::from_le_bytes(buf) }} else {{ 0 }} }}\n"
                ));
            }
        }
    }
    out
}

/// Render the complete `lib.rs`.
fn render_lib_rs(
    slots: &[Slot],
    ctor_body: &[String],
    arms: &[String],
    used_caller: bool,
    _c: &Contract,
    deploy_buf: usize,
    call_buf: usize,
) -> String {
    let mut out = String::new();
    out.push_str("#![no_std]\n#![no_main]\nuse core::panic::PanicInfo;\n\n");
    out.push_str("#[panic_handler]\nfn panic(_: &PanicInfo) -> ! { core::arch::wasm32::unreachable() }\n\n");

    // seal0 imports — only those used.
    out.push_str("#[link(wasm_import_module = \"seal0\")]\n");
    out.push_str("extern \"C\" {\n");
    out.push_str("    fn seal_input(buf: *mut u8, len: *mut u32);\n");
    out.push_str("    fn seal_return(flags: u32, data: *const u8, len: u32) -> !;\n");
    out.push_str("    fn seal_get_storage(key: *const u8, out: *mut u8, out_len: *mut u32) -> u32;\n");
    out.push_str("    fn seal_set_storage(key: *const u8, val: *const u8, val_len: u32);\n");
    if used_caller {
        out.push_str("    fn seal_caller(out: *mut u8, out_len: *mut u32);\n");
    }
    out.push_str("}\n\n");

    // Runtime helpers.
    out.push_str("#[inline(never)]\nfn ret(data: &[u8]) -> ! { unsafe { seal_return(0, data.as_ptr(), data.len() as u32) } }\n");
    out.push_str("#[inline(never)]\nfn revert() -> ! { unsafe { seal_return(1, core::ptr::null(), 0) } }\n");
    if used_caller {
        out.push_str("fn caller() -> [u8; 32] { let mut buf = [0u8; 32]; let mut len: u32 = 32; unsafe { seal_caller(buf.as_mut_ptr(), &mut len as *mut u32); } buf }\n");
    }
    out.push('\n');

    // Storage helpers.
    out.push_str(&render_slot_helpers(slots));
    out.push('\n');

    // deploy()
    out.push_str("#[no_mangle]\npub extern \"C\" fn deploy() {\n");
    // Read input for ctor args (deploy input = SCALE(ctor args), no selector).
    // NOTE: a large fixed stack buffer makes instantiate fail on this
    // rent-era node, so the buffer is sized to the ctor payload.
    out.push_str(&format!("    let mut input = [0u8; {deploy_buf}];\n"));
    out.push_str(&format!("    let mut in_len: u32 = {deploy_buf};\n"));
    out.push_str("    unsafe { seal_input(input.as_mut_ptr(), &mut in_len as *mut u32); }\n");
    out.push_str("    let _ = in_len;\n");
    out.push_str("    let _ = &input;\n");
    for l in ctor_body {
        out.push_str(&format!("    {l}\n"));
    }
    out.push_str("}\n\n");

    // call()
    out.push_str("#[no_mangle]\npub extern \"C\" fn call() {\n");
    out.push_str(&format!("    let mut input = [0u8; {call_buf}];\n"));
    out.push_str(&format!("    let mut in_len: u32 = {call_buf};\n"));
    out.push_str("    unsafe { seal_input(input.as_mut_ptr(), &mut in_len as *mut u32); }\n");
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

fn render_cargo_toml(crate_name: &str) -> String {
    format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[profile.release]\npanic = \"abort\"\nlto = true\nopt-level = \"z\"\noverflow-checks = false\n"
    )
}

const CARGO_CONFIG: &str = "[target.wasm32-unknown-unknown]\nrustflags = [\n  \"-C\", \"link-arg=--import-memory\",\n  \"-C\", \"link-arg=--initial-memory=65536\",\n  \"-C\", \"link-arg=--max-memory=1048576\",\n  \"-C\", \"link-arg=-zstack-size=32768\",\n]\n";

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
        // flip selector 1, get selector 2.
        assert!(art.lib_rs.contains("[0, 0, 0, 1]"));
        assert!(art.lib_rs.contains("[0, 0, 0, 2]"));
        // ctor decodes bool at offset 0.
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
        // incBy decodes a u128 param after the selector (offset 4).
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
    fn rejects_mapping_storage() {
        let src = r#"
            contract M {
                mapping(address => uint256) balances;
                function get(address a) public view returns (uint256) { return balances[a]; }
            }
        "#;
        let err = translate_seal(src).unwrap_err();
        assert!(err.contains("scalar"), "got: {err}");
    }

    #[test]
    fn snake_case_crate_name() {
        assert_eq!(snake("SimpleStorage"), "simple_storage");
        assert_eq!(snake("Counter"), "counter");
        assert_eq!(snake("Flipper"), "flipper");
    }
}
