use crate::ir::*;

/// Render an IR Type as its ink!/Rust source representation.
///
/// Nested mappings (`Mapping<K, Mapping<K2, V>>`) are flattened into a
/// tuple-key mapping (`Mapping<(K, K2), V>`) as required by ink! 5.x.
pub fn render_type(t: &Type) -> String {
    match t {
        Type::Bool => "bool".into(),
        Type::U128 => "u128".into(),
        Type::I128 => "i128".into(),
        Type::U256 => "U256".into(),
        Type::AccountId => "AccountId".into(),
        Type::String => "String".into(),
        Type::Bytes => "Vec<u8>".into(),
        Type::Mapping(k, v) => match &**v {
            Type::Mapping(k2, v2) => format!(
                "Mapping<({}, {}), {}>",
                render_type(k),
                render_type(k2),
                render_type(v2)
            ),
            _ => format!("Mapping<{}, {}>", render_type(k), render_type(v)),
        },
        Type::Array(elem) => format!("Vec<{}>", render_type(elem)),
        Type::Struct(name) => name.clone(),
    }
}

/// Emit a complete ink! 5.x contract module as a Rust source string.
///
/// Identifiers are used VERBATIM (no snake_case conversion); the generated
/// module, struct, and impl blocks carry `#[allow(non_snake_case)]` so that
/// Solidity-style names compile without warnings.
pub fn emit_contract(c: &Contract) -> String {
    let mut out = String::new();

    // File-level attribute required for no_std ink! contracts.
    out.push_str("#![cfg_attr(not(feature = \"std\"), no_std, no_main)]\n\n");

    // Module name is the contract name lowercased.
    let mod_name = c.name.to_lowercase();

    out.push_str("#[ink::contract]\n");
    out.push_str(&format!("#[allow(non_snake_case)]\n"));
    out.push_str(&format!("pub mod {mod_name} {{\n"));

    // Imports inside the module.
    out.push_str("    use ink::storage::Mapping;\n\n");

    // -----------------------------------------------------------------------
    // Storage struct
    // -----------------------------------------------------------------------
    out.push_str("    #[ink(storage)]\n");
    out.push_str("    #[derive(Default)]\n");
    out.push_str("    #[allow(non_snake_case)]\n");
    out.push_str(&format!("    pub struct {} {{\n", c.name));
    for field in &c.storage {
        out.push_str(&format!(
            "        {}: {},\n",
            field.name,
            render_type(&field.ty)
        ));
    }
    out.push_str("    }\n\n");

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------
    for event in &c.events {
        out.push_str("    #[ink(event)]\n");
        out.push_str(&format!("    pub struct {} {{\n", event.name));
        for f in &event.fields {
            if f.indexed {
                out.push_str("        #[ink(topic)]\n");
            }
            out.push_str(&format!(
                "        {}: {},\n",
                f.name,
                render_type(&f.ty)
            ));
        }
        out.push_str("    }\n\n");
    }

    // -----------------------------------------------------------------------
    // Error enum — always includes `Overflow`.
    // -----------------------------------------------------------------------
    out.push_str("    #[derive(Debug, PartialEq, Eq)]\n");
    out.push_str("    #[ink::scale_derive(Encode, Decode, TypeInfo)]\n");
    out.push_str("    pub enum Error {\n");
    for variant in &c.errors {
        out.push_str(&format!("        {},\n", variant.name));
    }
    // Append `Overflow` only if not already declared.
    let has_overflow = c.errors.iter().any(|e| e.name == "Overflow");
    if !has_overflow {
        out.push_str("        Overflow,\n");
    }
    out.push_str("    }\n\n");

    // -----------------------------------------------------------------------
    // impl block
    // -----------------------------------------------------------------------
    out.push_str("    #[allow(non_snake_case)]\n");
    out.push_str(&format!("    impl {} {{\n", c.name));

    // Constructor
    if let Some(ctor) = &c.constructor {
        let params_rendered = render_params(&ctor.params);
        let param_names: Vec<&str> = ctor.params.iter().map(|p| p.name.as_str()).collect();

        // Public ink! constructor delegates to __init so `self.` is valid.
        out.push_str("        #[ink(constructor)]\n");
        out.push_str(&format!(
            "        pub fn new({params_rendered}) -> Self {{\n"
        ));
        out.push_str("            let mut instance = Self::default();\n");
        if param_names.is_empty() {
            out.push_str("            instance.__init();\n");
        } else {
            out.push_str(&format!(
                "            instance.__init({});\n",
                param_names.join(", ")
            ));
        }
        out.push_str("            instance\n");
        out.push_str("        }\n\n");

        // Private __init carries the actual body.
        if param_names.is_empty() {
            out.push_str("        fn __init(&mut self) {\n");
        } else {
            out.push_str(&format!(
                "        fn __init(&mut self, {params_rendered}) {{\n"
            ));
        }
        for line in &ctor.body {
            out.push_str(&format!("            {line}\n"));
        }
        out.push_str("        }\n\n");
    }

    // Messages
    for msg in &c.messages {
        // Attribute
        let attr = match msg.mutability {
            Mutability::Payable => "        #[ink(message, payable)]\n",
            _ => "        #[ink(message)]\n",
        };
        out.push_str(attr);

        // Self receiver
        let self_recv = match msg.mutability {
            Mutability::View => "&self",
            _ => "&mut self",
        };

        // Return type (ink! path supports a single return; tuple for multi).
        let ret_ty = match msg.returns.as_slice() {
            [] => "()".into(),
            [t] => render_type(t),
            many => format!(
                "({})",
                many.iter().map(render_type).collect::<Vec<_>>().join(", ")
            ),
        };

        let params_rendered = render_params(&msg.params);
        let full_params = if params_rendered.is_empty() {
            self_recv.to_string()
        } else {
            format!("{self_recv}, {params_rendered}")
        };

        out.push_str(&format!(
            "        pub fn {}({full_params}) -> Result<{ret_ty}, Error> {{\n",
            msg.name
        ));
        for line in &msg.body {
            out.push_str(&format!("            {line}\n"));
        }
        out.push_str("        }\n\n");
    }

    out.push_str("    }\n"); // close impl
    out.push_str("}\n"); // close mod

    out
}

/// Render a parameter list as `name: type, name: type, ...`.
fn render_params(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| format!("{}: {}", p.name, render_type(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Type;

    #[test]
    fn renders_bool() {
        assert_eq!(render_type(&Type::Bool), "bool");
    }

    #[test]
    fn renders_u128() {
        assert_eq!(render_type(&Type::U128), "u128");
    }

    #[test]
    fn renders_u256() {
        assert_eq!(render_type(&Type::U256), "U256");
    }

    #[test]
    fn renders_account_id() {
        assert_eq!(render_type(&Type::AccountId), "AccountId");
    }

    #[test]
    fn renders_string() {
        assert_eq!(render_type(&Type::String), "String");
    }

    #[test]
    fn renders_bytes() {
        assert_eq!(render_type(&Type::Bytes), "Vec<u8>");
    }

    #[test]
    fn renders_mapping_type() {
        let t = Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128));
        assert_eq!(render_type(&t), "Mapping<AccountId, u128>");
    }

    #[test]
    fn renders_nested_mapping_as_tuple_key() {
        let t = Type::Mapping(
            Box::new(Type::AccountId),
            Box::new(Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128))),
        );
        assert_eq!(render_type(&t), "Mapping<(AccountId, AccountId), u128>");
    }
}

#[cfg(test)]
mod module_tests {
    use super::*;
    fn sample() -> Contract {
        Contract {
            name: "Token".into(),
            storage: vec![Field { name: "totalSupply_".into(), ty: Type::U128, public: false }],
            events: vec![Event {
                name: "Transfer".into(),
                fields: vec![
                    EventField { name: "from".into(), ty: Type::AccountId, indexed: true },
                    EventField { name: "value".into(), ty: Type::U128, indexed: false },
                ],
            }],
            errors: vec![ErrorVariant { name: "InsufficientBalance".into() }],
            constructor: Some(Function {
                name: "new".into(),
                mutability: Mutability::Mutating,
                params: vec![Param { name: "initialSupply".into(), ty: Type::U128 }],
                returns: vec![],
                body: vec!["self.totalSupply_ = initialSupply;".into()],
            }),
            messages: vec![Function {
                name: "balanceOf".into(),
                mutability: Mutability::View,
                params: vec![Param { name: "who".into(), ty: Type::AccountId }],
                returns: vec![Type::U128],
                body: vec!["return Ok(0);".into()],
            }],
        }
    }

    #[test]
    fn emits_ink_module() {
        let src = emit_contract(&sample());
        assert!(src.contains("#[ink::contract]"), "missing #[ink::contract]");
        assert!(src.contains("pub struct Token"), "missing pub struct Token");
        assert!(src.contains("#[ink(storage)]"), "missing #[ink(storage)]");
        assert!(src.contains("#[ink(event)]"), "missing #[ink(event)]");
        assert!(src.contains("pub enum Error"), "missing pub enum Error");
        assert!(src.contains("#[ink(constructor)]"), "missing #[ink(constructor)]");
        assert!(src.contains("#[ink(message)]"), "missing #[ink(message)]");
        assert!(src.contains("totalSupply_"), "missing verbatim field name");
        assert!(src.contains("Overflow"), "missing auto-added Overflow variant");
        assert!(src.contains("#[ink(topic)]"), "missing #[ink(topic)] on indexed field");
    }

    #[test]
    fn overflow_not_duplicated_when_already_present() {
        let mut c = sample();
        c.errors.push(ErrorVariant { name: "Overflow".into() });
        let src = emit_contract(&c);
        // Count occurrences of "Overflow," to ensure it appears exactly once.
        let count = src.matches("Overflow,").count();
        assert_eq!(count, 1, "Overflow should appear exactly once, got {count}");
    }

    #[test]
    fn payable_message_gets_payable_attr() {
        let mut c = sample();
        c.messages.push(Function {
            name: "deposit".into(),
            mutability: Mutability::Payable,
            params: vec![],
            returns: vec![],
            body: vec![],
        });
        let src = emit_contract(&c);
        assert!(src.contains("#[ink(message, payable)]"), "missing payable attr");
    }

    #[test]
    fn view_message_uses_shared_ref() {
        let src = emit_contract(&sample());
        // balanceOf is View, so must use &self.
        assert!(src.contains("pub fn balanceOf(&self,"), "balanceOf should have &self");
    }

    #[test]
    fn mutating_message_uses_mut_ref() {
        let mut c = sample();
        c.messages.push(Function {
            name: "mint".into(),
            mutability: Mutability::Mutating,
            params: vec![Param { name: "amount".into(), ty: Type::U128 }],
            returns: vec![],
            body: vec![],
        });
        let src = emit_contract(&c);
        assert!(src.contains("pub fn mint(&mut self,"), "mint should have &mut self");
    }

    #[test]
    fn file_starts_with_cfg_attr() {
        let src = emit_contract(&sample());
        assert!(
            src.starts_with("#![cfg_attr(not(feature = \"std\"), no_std, no_main)]"),
            "file must start with no_std cfg_attr"
        );
    }
}
