use solang_parser::pt::{
    ContractDefinition, ContractPart, ContractTy, FunctionDefinition, FunctionTy, SourceUnitPart,
};

pub fn parse_contract_name(src: &str) -> Result<String, String> {
    let def = parse_contract(src)?;
    def.name
        .as_ref()
        .map(|id| id.name.clone())
        .ok_or_else(|| "contract has no name".into())
}

/// Parse every top-level `contract`/`interface`/`abstract`/`library` definition.
pub fn parse_all(src: &str) -> Result<Vec<ContractDefinition>, String> {
    let (unit, _comments) =
        solang_parser::parse(src, 0).map_err(|e| format!("parse error: {:?}", e))?;
    let mut out = Vec::new();
    for part in unit.0 {
        if let SourceUnitPart::ContractDefinition(def) = part {
            out.push(*def);
        }
    }
    if out.is_empty() {
        return Err("no contract definition found".into());
    }
    Ok(out)
}

/// Is this contract a concrete `contract` (i.e. deployable), as opposed to an
/// `interface`, `abstract`, or `library`?
fn is_concrete(def: &ContractDefinition) -> bool {
    matches!(def.ty, ContractTy::Contract(_))
}

fn name_of(def: &ContractDefinition) -> Option<String> {
    def.name.as_ref().map(|i| i.name.clone())
}

/// Whether `f` is a real, emittable function/modifier/constructor definition
/// (i.e. has a body). Interface/abstract declarations have `body == None` and
/// must NOT be emitted — they are contract obligations, not code.
fn has_body(f: &FunctionDefinition) -> bool {
    f.body.is_some()
}

/// Parse the Solidity source and return a single, *flattened* contract
/// definition ready for lowering. Inheritance (`contract C is A, B`) is
/// resolved by merging base state vars, functions, modifiers, events, structs,
/// and errors into the derived contract (derived members override base members
/// of the same name). `interface`/`abstract` function declarations without
/// bodies are dropped.
///
/// FAIL-LOUD: if the file declares more than one concrete deployable contract
/// (that are not in an inheritance relationship), or names a base contract that
/// is not present in the file, translation hard-errors rather than silently
/// emitting only the first contract.
pub fn parse_contract(src: &str) -> Result<ContractDefinition, String> {
    let all = parse_all(src)?;

    // Index every contract by name (concrete + interface + abstract + library).
    let by_name: std::collections::BTreeMap<String, &ContractDefinition> = all
        .iter()
        .filter_map(|d| name_of(d).map(|n| (n, d)))
        .collect();

    // The set of names referenced as a base by *some* contract.
    let mut used_as_base: std::collections::BTreeSet<String> = Default::default();
    for d in &all {
        for b in &d.base {
            if let Some(id) = b.name.identifiers.last() {
                used_as_base.insert(id.name.clone());
            }
        }
    }

    // Candidate deployable contracts: concrete contracts that no other concrete
    // contract derives from. (A concrete base that is also derived-from is a
    // building block, not the deployment target.)
    let candidates: Vec<&ContractDefinition> = all
        .iter()
        .filter(|d| is_concrete(d))
        .filter(|d| {
            name_of(d)
                .map(|n| !used_as_base.contains(&n))
                .unwrap_or(false)
        })
        .collect();

    let target = match candidates.len() {
        0 => {
            return Err(
                "no deployable contract found (only interface/abstract/library or all bases)"
                    .into(),
            )
        }
        1 => candidates[0],
        _ => {
            let names: Vec<String> = candidates.iter().filter_map(|d| name_of(d)).collect();
            return Err(format!(
                "multiple deployable contracts in one file: {} — split them into separate files \
                 (inheritance is flattened, but two unrelated contracts are ambiguous)",
                names.join(", ")
            ));
        }
    };

    flatten(target, &by_name)
}

/// Recursively flatten `target` against its base contracts.
fn flatten(
    target: &ContractDefinition,
    by_name: &std::collections::BTreeMap<String, &ContractDefinition>,
) -> Result<ContractDefinition, String> {
    // Linearize bases depth-first (C3-ish: bases first, then derived overrides).
    // We build an ordered list of contracts from least-derived to most-derived,
    // so that later (more-derived) parts override earlier ones.
    let mut chain: Vec<&ContractDefinition> = Vec::new();
    collect_chain(target, by_name, &mut chain, &mut Vec::new())?;

    // Merge parts. Derived overrides base by (kind, name). We accumulate in a
    // way that preserves declaration order while letting later definitions win.
    let mut funcs: Vec<(String, ContractPart)> = Vec::new(); // keyed by func name
    let mut ctor: Option<ContractPart> = None;
    let mut modifiers: Vec<(String, ContractPart)> = Vec::new();
    let mut vars: Vec<(String, ContractPart)> = Vec::new();
    let mut events: Vec<(String, ContractPart)> = Vec::new();
    let mut errors: Vec<(String, ContractPart)> = Vec::new();
    let mut structs: Vec<(String, ContractPart)> = Vec::new();
    let mut enums: Vec<(String, ContractPart)> = Vec::new();
    let mut others: Vec<ContractPart> = Vec::new();

    let upsert = |list: &mut Vec<(String, ContractPart)>, key: String, part: ContractPart| {
        if let Some(slot) = list.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = part;
        } else {
            list.push((key, part));
        }
    };

    for c in &chain {
        for part in &c.parts {
            match part {
                ContractPart::FunctionDefinition(f) => match f.ty {
                    FunctionTy::Constructor => {
                        // Most-derived constructor wins (only one supported).
                        if f.body.is_some() {
                            ctor = Some(part.clone());
                        }
                    }
                    FunctionTy::Modifier => {
                        if let Some(id) = &f.name {
                            upsert(&mut modifiers, id.name.clone(), part.clone());
                        }
                    }
                    FunctionTy::Function => {
                        // Drop bodyless (interface/abstract) declarations: they
                        // are obligations, not code. A derived override with a
                        // body replaces them.
                        if !has_body(f) {
                            continue;
                        }
                        if f.name.is_some() {
                            // Key by full signature so overloads coexist while a
                            // same-signature override still replaces its base.
                            upsert(&mut funcs, fn_sig_key(f), part.clone());
                        }
                    }
                    // receive() / fallback() — keyed by their kind name.
                    FunctionTy::Receive => {
                        if has_body(f) {
                            upsert(&mut funcs, "<receive>".into(), part.clone());
                        }
                    }
                    FunctionTy::Fallback => {
                        if has_body(f) {
                            upsert(&mut funcs, "<fallback>".into(), part.clone());
                        }
                    }
                },
                ContractPart::VariableDefinition(v) => {
                    if let Some(id) = &v.name {
                        upsert(&mut vars, id.name.clone(), part.clone());
                    }
                }
                ContractPart::EventDefinition(e) => {
                    if let Some(id) = &e.name {
                        upsert(&mut events, id.name.clone(), part.clone());
                    }
                }
                ContractPart::ErrorDefinition(e) => {
                    if let Some(id) = &e.name {
                        upsert(&mut errors, id.name.clone(), part.clone());
                    }
                }
                ContractPart::StructDefinition(s) => {
                    if let Some(id) = &s.name {
                        upsert(&mut structs, id.name.clone(), part.clone());
                    }
                }
                ContractPart::EnumDefinition(e) => {
                    if let Some(id) = &e.name {
                        upsert(&mut enums, id.name.clone(), part.clone());
                    }
                }
                other => others.push(other.clone()),
            }
        }
    }

    // Reassemble parts in a stable order: structs, enums, errors, events, vars,
    // modifiers, constructor, functions, others.
    let mut parts: Vec<ContractPart> = Vec::new();
    parts.extend(structs.into_iter().map(|(_, p)| p));
    parts.extend(enums.into_iter().map(|(_, p)| p));
    parts.extend(errors.into_iter().map(|(_, p)| p));
    parts.extend(events.into_iter().map(|(_, p)| p));
    parts.extend(vars.into_iter().map(|(_, p)| p));
    parts.extend(modifiers.into_iter().map(|(_, p)| p));
    if let Some(c) = ctor {
        parts.push(c);
    }
    parts.extend(funcs.into_iter().map(|(_, p)| p));
    parts.extend(others);

    Ok(ContractDefinition {
        loc: target.loc,
        ty: target.ty.clone(),
        name: target.name.clone(),
        base: Vec::new(),
        parts,
    })
}

/// Depth-first collect the inheritance chain (bases before the derived
/// contract). `stack` guards against cyclic inheritance.
fn collect_chain<'a>(
    def: &'a ContractDefinition,
    by_name: &std::collections::BTreeMap<String, &'a ContractDefinition>,
    out: &mut Vec<&'a ContractDefinition>,
    stack: &mut Vec<String>,
) -> Result<(), String> {
    let this_name = name_of(def).unwrap_or_default();
    if stack.contains(&this_name) {
        return Err(format!("cyclic inheritance involving `{this_name}`"));
    }
    stack.push(this_name.clone());
    for b in &def.base {
        let bname = b
            .name
            .identifiers
            .last()
            .map(|i| i.name.clone())
            .unwrap_or_default();
        let bdef = by_name.get(&bname).ok_or_else(|| {
            format!(
                "contract `{this_name}` inherits from `{bname}`, which is not defined in this file"
            )
        })?;
        collect_chain(bdef, by_name, out, stack)?;
    }
    stack.pop();
    // Append self after bases, de-duplicated (diamond inheritance).
    if !out.iter().any(|d| name_of(d) == Some(this_name.clone())) {
        out.push(def);
    }
    Ok(())
}

/// Stable canonical key for a function used during inheritance flattening.
///
/// Solidity allows function overloading: several functions sharing a name but
/// differing in parameter arity/types. They are distinct functions and each
/// has its own ABI selector. Flattening must therefore key by the *full
/// signature* (`name(type,type,...)`), not by name alone — keying by name would
/// let one overload silently overwrite another (a silent miscompile).
///
/// At the same time, an `override` in a derived contract has the *same*
/// signature as the base it replaces, so it produces the same key and correctly
/// upserts over the base. Param types are canonicalized through the same type
/// mapper the codegen/selector path uses, so the key agrees with the eventual
/// 4-byte selector grouping. Unmappable/unknown param types fall back to their
/// source identifier (struct/enum names) so they still distinguish overloads.
fn fn_sig_key(f: &FunctionDefinition) -> String {
    use solang_parser::pt::Expression;
    let name = f.name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
    let mut tys: Vec<String> = Vec::new();
    for (_, opt_p) in &f.params {
        let Some(p) = opt_p.as_ref() else { continue };
        // Canonicalize via the shared type mapper (uint256/uint collapse, etc.).
        let key = if let Some(t) = crate::lower::map_type_structs(&p.ty, "u128") {
            format!("{t:?}")
        } else if let Expression::Variable(id) = &p.ty {
            // Unknown named type (struct/enum/interface): use its identifier.
            id.name.clone()
        } else {
            // Last resort: a structural debug of the type expression. This is
            // only reached for exotic param types; it still differs across
            // genuinely different types, so no overload is dropped.
            format!("{:?}", p.ty)
        };
        tys.push(key);
    }
    format!("{name}({})", tys.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_contract_unchanged() {
        let src = "contract A { uint256 x; function f() public {} }";
        let def = parse_contract(src).unwrap();
        assert_eq!(name_of(&def).as_deref(), Some("A"));
    }

    #[test]
    fn errors_on_two_unrelated_contracts() {
        let src = "contract A { uint256 x; } contract B { uint256 y; }";
        let err = parse_contract(src).unwrap_err();
        assert!(err.contains("multiple deployable contracts"), "got: {err}");
    }

    #[test]
    fn flattens_is_base() {
        let src = r#"
            contract Base { address owner; constructor() { owner = msg.sender; }
                modifier onlyOwner() { require(msg.sender == owner); _; } }
            contract Token is Base {
                uint256 public total;
                function mint(uint256 n) public onlyOwner { total += n; }
            }
        "#;
        let def = parse_contract(src).unwrap();
        assert_eq!(name_of(&def).as_deref(), Some("Token"));
        // Base's `owner` var and `onlyOwner` modifier are merged in.
        let has_owner = def.parts.iter().any(|p| matches!(p,
            ContractPart::VariableDefinition(v) if v.name.as_ref().map(|i| i.name.as_str()) == Some("owner")));
        assert!(has_owner, "owner var not flattened in");
        let has_mod = def.parts.iter().any(|p| matches!(p,
            ContractPart::FunctionDefinition(f) if matches!(f.ty, FunctionTy::Modifier)));
        assert!(has_mod, "onlyOwner modifier not flattened in");
    }

    #[test]
    fn interface_decls_without_body_are_dropped() {
        let src = r#"
            interface IFoo { function bar() external returns (uint256); }
            contract Foo is IFoo { function bar() public pure returns (uint256) { return 1; } }
        "#;
        let def = parse_contract(src).unwrap();
        // Only the concrete `bar` with a body should remain (one bar fn).
        let bars = def.parts.iter().filter(|p| matches!(p,
            ContractPart::FunctionDefinition(f) if f.name.as_ref().map(|i| i.name.as_str()) == Some("bar"))).count();
        assert_eq!(bars, 1, "expected exactly one bar fn after flattening");
    }

    #[test]
    fn errors_on_missing_base() {
        let src = "contract Token is Missing { uint256 x; }";
        let err = parse_contract(src).unwrap_err();
        assert!(err.contains("not defined in this file"), "got: {err}");
    }
}
