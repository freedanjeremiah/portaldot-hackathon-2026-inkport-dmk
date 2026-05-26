use inkport_translate::ir::{Contract, Type, Mutability};

#[test]
fn ir_types_construct() {
    let c = Contract {
        name: "Token".into(),
        storage: vec![],
        events: vec![],
        errors: vec![],
        constructor: None,
        messages: vec![],
    };
    assert_eq!(c.name, "Token");
    let _ = Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128));
    let _ = Mutability::View;
}

// Task 5
use inkport_translate::parse::parse_contract_name;

#[test]
fn finds_contract_name() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    assert_eq!(parse_contract_name(&src).unwrap(), "Token");
}

// Task 6
use inkport_translate::lower::lower_storage;

#[test]
fn lowers_storage_fields() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let def = inkport_translate::parse::parse_contract(&src).unwrap();
    let fields = lower_storage(&def, "u128");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"balances"));
    assert!(names.contains(&"allowances"));
    assert!(names.contains(&"totalSupply_"));

    let balances = fields.iter().find(|f| f.name == "balances").unwrap();
    assert_eq!(balances.ty, Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128)));

    let allowances = fields.iter().find(|f| f.name == "allowances").unwrap();
    assert_eq!(allowances.ty, Type::Mapping(
        Box::new(Type::AccountId),
        Box::new(Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128)))));
}

// Task 7
use inkport_translate::lower::{lower_events, lower_errors};

#[test]
fn lowers_events_and_errors() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let def = inkport_translate::parse::parse_contract(&src).unwrap();

    let events = lower_events(&def, "u128");
    let transfer = events.iter().find(|e| e.name == "Transfer").unwrap();
    assert_eq!(transfer.fields.len(), 3);
    assert!(transfer.fields[0].indexed);
    assert!(transfer.fields[1].indexed);
    assert!(!transfer.fields[2].indexed);

    let errors = lower_errors(&def);
    let names: Vec<&str> = errors.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"InsufficientBalance"));
    assert!(names.contains(&"InsufficientAllowance"));
}

// Task 8
use inkport_translate::lower::lower_functions;

#[test]
fn lowers_function_signatures() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let def = inkport_translate::parse::parse_contract(&src).unwrap();
    let (ctor, messages) = lower_functions(&def, "u128");
    assert!(ctor.is_some());
    let balance_of = messages.iter().find(|f| f.name == "balanceOf").unwrap();
    assert_eq!(balance_of.mutability, Mutability::View);
    assert_eq!(balance_of.params.len(), 1);
    assert!(balance_of.returns.is_some());
    let transfer = messages.iter().find(|f| f.name == "transfer").unwrap();
    assert_eq!(transfer.mutability, Mutability::Mutating);
    assert_eq!(transfer.params.len(), 2);
}
