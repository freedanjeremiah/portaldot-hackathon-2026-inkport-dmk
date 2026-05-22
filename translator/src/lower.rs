use solang_parser::pt::{
    ContractDefinition, ContractPart, Expression, FunctionAttribute, FunctionTy,
    Mutability as PtMutability, Statement, Type as PtType,
};
use crate::ir::{Event, EventField, ErrorVariant, Field, Function, Mutability, Param, Type};
use crate::types::map_elementary;

/// Map a parse-tree type expression to an IR Type.
pub fn map_type(expr: &Expression, uint_strategy: &str) -> Option<Type> {
    match expr {
        Expression::Type(_, ty) => match ty {
            PtType::Bool => Some(Type::Bool),
            PtType::Address | PtType::AddressPayable => Some(Type::AccountId),
            PtType::String => Some(Type::String),
            PtType::Uint(_) => Some(if uint_strategy == "u256" { Type::U256 } else { Type::U128 }),
            PtType::Int(_) => Some(if uint_strategy == "u256" { Type::U256 } else { Type::U128 }),
            PtType::Bytes(_) | PtType::DynamicBytes => Some(Type::Bytes),
            PtType::Mapping { key, value, .. } => {
                let k = map_type(key, uint_strategy)?;
                let v = map_type(value, uint_strategy)?;
                Some(Type::Mapping(Box::new(k), Box::new(v)))
            }
            _ => None,
        },
        Expression::Variable(id) => map_elementary(&id.name, uint_strategy),
        _ => None,
    }
}

/// Lower storage variable definitions from a contract into IR Fields.
pub fn lower_storage(def: &ContractDefinition, uint_strategy: &str) -> Vec<Field> {
    let mut out = Vec::new();
    for part in &def.parts {
        if let ContractPart::VariableDefinition(v) = part {
            if let Some(ty) = map_type(&v.ty, uint_strategy) {
                if let Some(name) = &v.name {
                    out.push(Field { name: name.name.clone(), ty });
                }
            }
        }
    }
    out
}

/// Lower event definitions from a contract into IR Events.
pub fn lower_events(def: &ContractDefinition, uint_strategy: &str) -> Vec<Event> {
    let mut out = Vec::new();
    for part in &def.parts {
        if let ContractPart::EventDefinition(ev) = part {
            if let Some(name_id) = &ev.name {
                let fields: Vec<EventField> = ev
                    .fields
                    .iter()
                    .map(|f| EventField {
                        name: f.name.as_ref().map(|id| id.name.clone()).unwrap_or_default(),
                        ty: map_type(&f.ty, uint_strategy).unwrap_or(Type::Bytes),
                        indexed: f.indexed,
                    })
                    .collect();
                out.push(Event { name: name_id.name.clone(), fields });
            }
        }
    }
    out
}

/// Lower error definitions from a contract into IR ErrorVariants.
pub fn lower_errors(def: &ContractDefinition) -> Vec<ErrorVariant> {
    let mut out = Vec::new();
    for part in &def.parts {
        if let ContractPart::ErrorDefinition(err) = part {
            if let Some(name_id) = &err.name {
                out.push(ErrorVariant { name: name_id.name.clone() });
            }
        }
    }
    out
}

/// Infer IR Mutability from a function's attributes.
fn infer_mutability(attrs: &[FunctionAttribute]) -> Mutability {
    for attr in attrs {
        if let FunctionAttribute::Mutability(m) = attr {
            return match m {
                PtMutability::Pure(_) | PtMutability::View(_) | PtMutability::Constant(_) => {
                    Mutability::View
                }
                PtMutability::Payable(_) => Mutability::Payable,
            };
        }
    }
    Mutability::Mutating
}

/// Lower function definitions into (constructor, messages).
///
/// Constructor is named "new". Bodies are left empty (filled in a later task).
pub fn lower_functions(
    def: &ContractDefinition,
    uint_strategy: &str,
) -> (Option<Function>, Vec<Function>) {
    let mut constructor: Option<Function> = None;
    let mut messages: Vec<Function> = Vec::new();

    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            match f.ty {
                FunctionTy::Constructor => {
                    let params: Vec<Param> = f
                        .params
                        .iter()
                        .filter_map(|(_, opt_p)| opt_p.as_ref())
                        .filter_map(|p| {
                            let ty = map_type(&p.ty, uint_strategy)?;
                            let name = p.name.as_ref().map(|id| id.name.clone()).unwrap_or_default();
                            Some(Param { name, ty })
                        })
                        .collect();
                    let mutability = infer_mutability(&f.attributes);
                    constructor = Some(Function {
                        name: "new".into(),
                        mutability,
                        params,
                        returns: None,
                        body: vec![],
                    });
                }
                FunctionTy::Function => {
                    // Skip functions without a name (old-style fallback)
                    let name = match &f.name {
                        Some(id) => id.name.clone(),
                        None => continue,
                    };

                    let params: Vec<Param> = f
                        .params
                        .iter()
                        .filter_map(|(_, opt_p)| opt_p.as_ref())
                        .filter_map(|p| {
                            let ty = map_type(&p.ty, uint_strategy)?;
                            let pname = p.name.as_ref().map(|id| id.name.clone()).unwrap_or_default();
                            Some(Param { name: pname, ty })
                        })
                        .collect();

                    let returns: Option<Type> = f
                        .returns
                        .iter()
                        .filter_map(|(_, opt_p)| opt_p.as_ref())
                        .filter_map(|p| map_type(&p.ty, uint_strategy))
                        .next();

                    let mutability = infer_mutability(&f.attributes);

                    messages.push(Function {
                        name,
                        mutability,
                        params,
                        returns,
                        body: vec![],
                    });
                }
                // Fallback, receive, modifier — skip
                _ => {}
            }
        }
    }

    (constructor, messages)
}

// ---------------------------------------------------------------------------
// Task 9: statement / expression codegen for the ERC-20 subset.
// ---------------------------------------------------------------------------

/// Render the ink! equivalent of Solidity's `msg.sender`.
pub fn render_msg_sender() -> String {
    "self.env().caller()".to_string()
}

/// Render a read from a storage mapping: `self.{map}.get({key}).unwrap_or_default()`.
pub fn render_mapping_read(map: &str, key_expr: &str) -> String {
    format!("self.{map}.get({key_expr}).unwrap_or_default()")
}

/// Render a write to a storage mapping: `self.{map}.insert({key}, &{value});`.
pub fn render_mapping_write(map: &str, key_expr: &str, value_expr: &str) -> String {
    format!("self.{map}.insert({key_expr}, &{value_expr});")
}

/// Walks solang `Statement`/`Expression` nodes for the ERC-20 subset and
/// renders ink! Rust source lines.
///
/// Unsupported nodes push a marker into `unsupported` and render a
/// `// TODO: manual review` placeholder.
pub struct LowerCtx<'a> {
    /// Names of contract storage fields (used to distinguish storage access
    /// from local variables).
    pub storage_names: &'a [String],
    /// Event definitions, used to map positional emit args to field names.
    pub events: &'a [Event],
    /// Markers describing nodes that could not be translated.
    pub unsupported: Vec<String>,
}

impl<'a> LowerCtx<'a> {
    /// Create a new lowering context.
    pub fn new(storage_names: &'a [String], events: &'a [Event]) -> Self {
        LowerCtx { storage_names, events, unsupported: Vec::new() }
    }

    fn is_storage(&self, name: &str) -> bool {
        self.storage_names.iter().any(|n| n == name)
    }

    fn mark_unsupported(&mut self, what: &str) -> String {
        self.unsupported.push(what.to_string());
        "// TODO: manual review".to_string()
    }

    /// If `e` is an indexed access `m[k]` (possibly nested `m[a][b]`) rooted at
    /// a storage mapping, return `(map_name, [key_exprs...])`.
    fn as_storage_index(&mut self, e: &Expression) -> Option<(String, Vec<String>)> {
        if let Expression::ArraySubscript(_, base, Some(index)) = e {
            let key = self.expr(index);
            match base.as_ref() {
                Expression::Variable(id) if self.is_storage(&id.name) => {
                    Some((id.name.clone(), vec![key]))
                }
                _ => {
                    // nested: base itself is a storage index expression.
                    let (name, mut keys) = self.as_storage_index(base)?;
                    keys.push(key);
                    Some((name, keys))
                }
            }
        } else {
            None
        }
    }

    /// Render a key expression for a mapping access given its component keys.
    /// A single key renders as-is; multiple keys flatten to a tuple.
    fn render_key(keys: &[String]) -> String {
        if keys.len() == 1 {
            keys[0].clone()
        } else {
            format!("({})", keys.join(", "))
        }
    }

    /// Render an expression into an ink! Rust expression string.
    pub fn expr(&mut self, e: &Expression) -> String {
        match e {
            Expression::Parenthesis(_, inner) => format!("({})", self.expr(inner)),

            // `msg.sender` and other member accesses.
            Expression::MemberAccess(_, base, member) => {
                if let Expression::Variable(id) = base.as_ref() {
                    if id.name == "msg" && member.name == "sender" {
                        return render_msg_sender();
                    }
                }
                self.mark_unsupported(&format!("member access .{}", member.name))
            }

            Expression::Variable(id) => id.name.clone(),

            Expression::NumberLiteral(_, value, _, _) => value.clone(),
            Expression::BoolLiteral(_, b) => if *b { "true".into() } else { "false".into() },

            // Indexed read of a storage mapping.
            Expression::ArraySubscript(_, _, Some(_)) => {
                if let Some((map, keys)) = self.as_storage_index(e) {
                    render_mapping_read(&map, &Self::render_key(&keys))
                } else {
                    self.mark_unsupported("array subscript on non-storage value")
                }
            }

            Expression::Add(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("{l}.checked_add({r}).ok_or(Error::Overflow)?")
            }
            Expression::Subtract(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("{l}.checked_sub({r}).ok_or(Error::Overflow)?")
            }
            Expression::Less(_, l, r) => {
                let l = self.expr(l);
                let r = self.expr(r);
                format!("{l} < {r}")
            }

            Expression::Assign(_, lhs, rhs) => self.render_assign(lhs, rhs),

            _ => self.mark_unsupported("expression"),
        }
    }

    /// Render an assignment `lhs = rhs` as a statement-ending string.
    fn render_assign(&mut self, lhs: &Expression, rhs: &Expression) -> String {
        let value = self.expr(rhs);
        // Storage mapping write (possibly nested).
        if let Some((map, keys)) = self.as_storage_index(lhs) {
            return render_mapping_write(&map, &Self::render_key(&keys), &value);
        }
        match lhs {
            Expression::Variable(id) if self.is_storage(&id.name) => {
                format!("self.{} = {value};", id.name)
            }
            Expression::Variable(id) => format!("{} = {value};", id.name),
            _ => self.mark_unsupported("assignment target"),
        }
    }

    /// Render a statement into zero or more ink! Rust source lines.
    pub fn stmt(&mut self, s: &Statement) -> Vec<String> {
        match s {
            Statement::Block { statements, .. } => {
                statements.iter().flat_map(|st| self.stmt(st)).collect()
            }

            Statement::Return(_, Some(e)) => {
                let inner = self.expr(e);
                vec![format!("return Ok({inner});")]
            }
            Statement::Return(_, None) => vec!["return Ok(());".to_string()],

            Statement::Expression(_, e) => {
                // Assignments already render with a trailing semicolon.
                match e {
                    Expression::Assign(_, lhs, rhs) => vec![self.render_assign(lhs, rhs)],
                    _ => {
                        let rendered = self.expr(e);
                        if rendered.ends_with(';') || rendered.starts_with("//") {
                            vec![rendered]
                        } else {
                            vec![format!("{rendered};")]
                        }
                    }
                }
            }

            Statement::If(_, cond, then_branch, else_branch) => {
                let mut out = Vec::new();
                let cond = self.expr(cond);
                out.push(format!("if {cond} {{"));
                for line in self.stmt(then_branch) {
                    out.push(format!("    {line}"));
                }
                if let Some(else_b) = else_branch {
                    out.push("} else {".to_string());
                    for line in self.stmt(else_b) {
                        out.push(format!("    {line}"));
                    }
                }
                out.push("}".to_string());
                out
            }

            Statement::Emit(_, e) => vec![self.render_emit(e)],

            Statement::Revert(_, path, _args) => {
                let name = path
                    .as_ref()
                    .and_then(|p| p.identifiers.last())
                    .map(|id| id.name.clone());
                match name {
                    Some(n) => vec![format!("return Err(Error::{n});")],
                    None => vec![self.mark_unsupported("revert without error name")],
                }
            }

            _ => vec![self.mark_unsupported("statement")],
        }
    }

    /// Render an `emit E(args)` expression into an `emit_event` call.
    fn render_emit(&mut self, e: &Expression) -> String {
        if let Expression::FunctionCall(_, callee, args) = e {
            if let Expression::Variable(id) = callee.as_ref() {
                let event_name = id.name.clone();
                let field_names: Vec<String> = self
                    .events
                    .iter()
                    .find(|ev| ev.name == event_name)
                    .map(|ev| ev.fields.iter().map(|f| f.name.clone()).collect())
                    .unwrap_or_default();
                let rendered_args: Vec<String> = args.iter().map(|a| self.expr(a)).collect();
                let fields: Vec<String> = rendered_args
                    .iter()
                    .enumerate()
                    .map(|(i, val)| {
                        let fname = field_names
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("field{i}"));
                        format!("{fname}: {val}")
                    })
                    .collect();
                return format!(
                    "self.env().emit_event({event_name} {{ {} }});",
                    fields.join(", ")
                );
            }
        }
        self.mark_unsupported("emit target")
    }
}

#[cfg(test)]
mod expr_tests {
    use super::*;
    use crate::ir::{Event, EventField, Type};
    use crate::parse::parse_contract;

    #[test]
    fn msg_sender_becomes_caller() {
        assert_eq!(render_msg_sender(), "self.env().caller()");
    }

    #[test]
    fn mapping_read_uses_get_unwrap_default() {
        assert_eq!(
            render_mapping_read("balances", "self.env().caller()"),
            "self.balances.get(self.env().caller()).unwrap_or_default()"
        );
    }

    #[test]
    fn mapping_write_uses_insert() {
        assert_eq!(
            render_mapping_write("balances", "to", "value"),
            "self.balances.insert(to, &value);"
        );
    }

    /// Parse a contract, find a function by name, return its body statement.
    fn body_of(src: &str, fname: &str) -> Statement {
        let def = parse_contract(src).expect("parse");
        for part in &def.parts {
            if let ContractPart::FunctionDefinition(f) = part {
                if f.name.as_ref().map(|id| id.name.as_str()) == Some(fname) {
                    return f.body.clone().expect("function has a body");
                }
            }
        }
        panic!("function {fname} not found");
    }

    fn transfer_events() -> Vec<Event> {
        vec![Event {
            name: "Transfer".into(),
            fields: vec![
                EventField { name: "from".into(), ty: Type::AccountId, indexed: true },
                EventField { name: "to".into(), ty: Type::AccountId, indexed: true },
                EventField { name: "value".into(), ty: Type::U128, indexed: false },
            ],
        }]
    }

    #[test]
    fn emit_renders_emit_event() {
        let src = r#"
            contract C {
                event Transfer(address indexed from, address indexed to, uint256 value);
                function transfer(address to, uint256 value) public {
                    emit Transfer(msg.sender, to, value);
                }
            }
        "#;
        let events = transfer_events();
        let storage: Vec<String> = vec![];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "transfer"));
        assert!(
            lines.contains(&"self.env().emit_event(Transfer { from: self.env().caller(), to: to, value: value });".to_string()),
            "got: {lines:?}"
        );
        assert!(ctx.unsupported.is_empty(), "unsupported: {:?}", ctx.unsupported);
    }

    #[test]
    fn revert_renders_err() {
        let src = r#"
            contract C {
                error InsufficientBalance();
                function f() public {
                    revert InsufficientBalance();
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage: Vec<String> = vec![];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "f"));
        assert!(
            lines.contains(&"return Err(Error::InsufficientBalance);".to_string()),
            "got: {lines:?}"
        );
    }

    #[test]
    fn mapping_read_from_storage() {
        let src = r#"
            contract C {
                mapping(address => uint256) balances;
                function balanceOf(address who) public view returns (uint256) {
                    return balances[who];
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage = vec!["balances".to_string()];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "balanceOf"));
        assert_eq!(lines, vec!["return Ok(self.balances.get(who).unwrap_or_default());"]);
    }

    #[test]
    fn nested_mapping_read_flattens_to_tuple() {
        let src = r#"
            contract C {
                mapping(address => mapping(address => uint256)) allowances;
                function allowance(address o, address s) public view returns (uint256) {
                    return allowances[o][s];
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage = vec!["allowances".to_string()];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "allowance"));
        assert_eq!(lines, vec!["return Ok(self.allowances.get((o, s)).unwrap_or_default());"]);
    }

    #[test]
    fn mapping_write_and_nested_write() {
        let src = r#"
            contract C {
                mapping(address => uint256) balances;
                mapping(address => mapping(address => uint256)) allowances;
                function f(address to, uint256 value) public {
                    balances[to] = value;
                    allowances[msg.sender][to] = value;
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage = vec!["balances".to_string(), "allowances".to_string()];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "f"));
        assert_eq!(
            lines,
            vec![
                "self.balances.insert(to, &value);".to_string(),
                "self.allowances.insert((self.env().caller(), to), &value);".to_string(),
            ]
        );
    }

    #[test]
    fn checked_add_and_if_less() {
        let src = r#"
            contract C {
                mapping(address => uint256) balances;
                function f(address to, uint256 value) public {
                    if (balances[to] < value) {
                        balances[to] = balances[to] + value;
                    }
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage = vec!["balances".to_string()];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "f"));
        assert_eq!(
            lines,
            vec![
                "if self.balances.get(to).unwrap_or_default() < value {".to_string(),
                "    self.balances.insert(to, &self.balances.get(to).unwrap_or_default().checked_add(value).ok_or(Error::Overflow)?);".to_string(),
                "}".to_string(),
            ]
        );
    }

    #[test]
    fn local_var_assignment() {
        let src = r#"
            contract C {
                function f(uint256 value) public {
                    value = value;
                }
            }
        "#;
        let events: Vec<Event> = vec![];
        let storage: Vec<String> = vec![];
        let mut ctx = LowerCtx::new(&storage, &events);
        let lines = ctx.stmt(&body_of(src, "f"));
        assert_eq!(lines, vec!["value = value;"]);
    }
}
