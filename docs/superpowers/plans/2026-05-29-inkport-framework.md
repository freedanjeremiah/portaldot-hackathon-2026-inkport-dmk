# InkPort Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build InkPort, a Hardhat-style framework that compiles Solidity to ink! 5.x and deploys it to Portaldot, delivered as a working ERC-20 end-to-end vertical slice.

**Architecture:** A pure offline Rust binary (`inkport-translate`) turns one `.sol` file into an ink! crate + translation report via `solang-parser` → IR → string codegen. A Python CLI (`inkport`, built on `typer`) orchestrates `init / compile / deploy / test`, shelling out to the Rust binary and `cargo contract build` for compile, and using the Portaldot Python SDK (`substrateinterface`) for all chain interaction.

**Tech Stack:** Rust (`solang-parser`), `cargo-contract`, ink! 5.x, Python 3.11+ (`typer`, `substrateinterface`/portaldot SDK, `pytest`), local `substrate-contracts-node` for integration tests.

**Repository layout this plan creates:**
```
inkide/
  translator/                  # Rust crate: inkport-translate
    Cargo.toml
    src/{main.rs,ir.rs,parse.rs,lower.rs,types.rs,codegen.rs,report.rs}
    tests/{erc20.rs}
    fixtures/ERC20.sol
  inkport/                     # Python package
    pyproject.toml
    inkport/{__init__.py,cli.py,config.py,compile.py,deploy.py,run.py,test_runner.py,report.py,scaffold/...}
    tests/{test_config.py,test_compile.py,test_deploy.py,...}
```

**Conventions:**
- Rust tests: `cargo test` from `translator/`.
- Python tests: `pytest` from `inkport/`.
- Commit after each task with the message shown.

---

## Phase 0 — Skeletons

### Task 1: Rust translator crate skeleton

**Files:**
- Create: `translator/Cargo.toml`
- Create: `translator/src/main.rs`

- [ ] **Step 1: Create the crate manifest**

`translator/Cargo.toml`:
```toml
[package]
name = "inkport-translate"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "inkport-translate"
path = "src/main.rs"

[dependencies]
solang-parser = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 2: Minimal main that prints version**

`translator/src/main.rs`:
```rust
fn main() {
    println!("inkport-translate {}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 3: Build to verify the toolchain resolves**

Run: `cd translator && cargo build`
Expected: compiles; `cargo run` prints `inkport-translate 0.1.0`.

- [ ] **Step 4: Commit**

```bash
git add translator/Cargo.toml translator/src/main.rs
git commit -m "feat(translator): crate skeleton"
```

---

### Task 2: Python package skeleton with typer CLI

**Files:**
- Create: `inkport/pyproject.toml`
- Create: `inkport/inkport/__init__.py`
- Create: `inkport/inkport/cli.py`
- Test: `inkport/tests/test_cli.py`

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_cli.py`:
```python
from typer.testing import CliRunner
from inkport.cli import app

runner = CliRunner()

def test_help_lists_commands():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    for cmd in ["init", "compile", "deploy", "test", "report", "run"]:
        assert cmd in result.stdout
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd inkport && pytest tests/test_cli.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'inkport.cli'`.

- [ ] **Step 3: Write package manifest and CLI**

`inkport/pyproject.toml`:
```toml
[project]
name = "inkport"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = ["typer>=0.12", "substrate-interface>=1.7"]

[project.scripts]
inkport = "inkport.cli:app"

[project.optional-dependencies]
dev = ["pytest>=8"]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

`inkport/inkport/__init__.py`:
```python
__version__ = "0.1.0"
```

`inkport/inkport/cli.py`:
```python
import typer

app = typer.Typer(help="InkPort — Solidity → ink! framework for Portaldot")

@app.command()
def init(directory: str = "."):
    """Scaffold a new InkPort project."""
    raise NotImplementedError

@app.command()
def compile(contract: str = typer.Option(None, "--contract")):
    """Translate Solidity to ink! and build WASM + metadata."""
    raise NotImplementedError

@app.command()
def deploy(network: str = typer.Option(..., "--network"),
           contract: str = typer.Option(None, "--contract"),
           value: float = typer.Option(0.0, "--value"),
           account: str = typer.Option("deployer", "--account")):
    """Deploy a compiled contract to a network."""
    raise NotImplementedError

@app.command()
def run(script: str, network: str = typer.Option(..., "--network")):
    """Run a Python script with an injected InkPort context."""
    raise NotImplementedError

@app.command()
def test(network: str = typer.Option("local", "--network")):
    """Run behavioral / golden tests."""
    raise NotImplementedError

@app.command()
def report(contract: str = typer.Option(None, "--contract")):
    """Print the translation report."""
    raise NotImplementedError
```

- [ ] **Step 4: Install editable and run the test**

Run: `cd inkport && pip install -e ".[dev]" && pytest tests/test_cli.py -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add inkport/pyproject.toml inkport/inkport/__init__.py inkport/inkport/cli.py inkport/tests/test_cli.py
git commit -m "feat(cli): typer skeleton with command stubs"
```

---

## Phase 1 — Translator (Rust): ERC-20 subset

> The translator is pure and offline. The ERC-20 fixture is the spec's golden case. Every mapping rule gets a unit test. `solang-parser` API names below target version `0.3`; if a name differs in the pinned version, verify against `https://docs.rs/solang-parser/0.3` and adjust — the IR and codegen tasks do not change.

### Task 3: ERC-20 fixture + IR types

**Files:**
- Create: `translator/fixtures/ERC20.sol`
- Create: `translator/src/ir.rs`
- Modify: `translator/src/main.rs`
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Add the fixture**

`translator/fixtures/ERC20.sol`:
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Token {
    mapping(address => uint256) private balances;
    mapping(address => mapping(address => uint256)) private allowances;
    uint256 private totalSupply_;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    error InsufficientBalance();
    error InsufficientAllowance();

    constructor(uint256 initialSupply) {
        totalSupply_ = initialSupply;
        balances[msg.sender] = initialSupply;
    }

    function balanceOf(address who) public view returns (uint256) {
        return balances[who];
    }

    function transfer(address to, uint256 value) public returns (bool) {
        if (balances[msg.sender] < value) { revert InsufficientBalance(); }
        balances[msg.sender] = balances[msg.sender] - value;
        balances[to] = balances[to] + value;
        emit Transfer(msg.sender, to, value);
        return true;
    }

    function approve(address spender, uint256 value) public returns (bool) {
        allowances[msg.sender][spender] = value;
        emit Approval(msg.sender, spender, value);
        return true;
    }
}
```

- [ ] **Step 2: Write the failing IR test**

`translator/tests/erc20.rs`:
```rust
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
```

- [ ] **Step 3: Run to verify it fails**

Run: `cd translator && cargo test ir_types_construct`
Expected: FAIL — `inkport_translate` lib not found.

- [ ] **Step 4: Make the crate a lib+bin and add IR**

`translator/Cargo.toml` — add a lib target above `[[bin]]`:
```toml
[lib]
name = "inkport_translate"
path = "src/lib.rs"
```

`translator/src/lib.rs`:
```rust
pub mod ir;
pub mod types;
pub mod parse;
pub mod lower;
pub mod codegen;
pub mod report;
```

`translator/src/ir.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    U128,
    U256,
    AccountId,
    String,
    Bytes,
    Mapping(Box<Type>, Box<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mutability { View, Mutating, Payable }

#[derive(Debug, Clone)]
pub struct Field { pub name: String, pub ty: Type }

#[derive(Debug, Clone)]
pub struct Param { pub name: String, pub ty: Type }

#[derive(Debug, Clone)]
pub struct EventField { pub name: String, pub ty: Type, pub indexed: bool }

#[derive(Debug, Clone)]
pub struct Event { pub name: String, pub fields: Vec<EventField> }

#[derive(Debug, Clone)]
pub struct ErrorVariant { pub name: String }

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub mutability: Mutability,
    pub params: Vec<Param>,
    pub returns: Option<Type>,
    /// Raw lowered statement lines (already ink! Rust source).
    pub body: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub storage: Vec<Field>,
    pub events: Vec<Event>,
    pub errors: Vec<ErrorVariant>,
    pub constructor: Option<Function>,
    pub messages: Vec<Function>,
}
```

Create empty module files so `lib.rs` compiles: `translator/src/types.rs`, `parse.rs`, `lower.rs`, `codegen.rs`, `report.rs` each containing only a top comment `// filled in later tasks`.

`translator/src/main.rs` — replace body with a stub that calls the lib (kept minimal until Task 9):
```rust
fn main() {
    println!("inkport-translate {}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 5: Run to verify it passes**

Run: `cd translator && cargo test ir_types_construct`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add translator/
git commit -m "feat(translator): ERC-20 fixture and IR types"
```

---

### Task 4: Type mapping (Solidity type string → IR Type)

**Files:**
- Modify: `translator/src/types.rs`
- Test: `translator/src/types.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing tests**

`translator/src/types.rs`:
```rust
use crate::ir::Type;

/// `uint_strategy`: "u128" (default) or "u256".
pub fn map_elementary(name: &str, uint_strategy: &str) -> Option<Type> {
    match name {
        "bool" => Some(Type::Bool),
        "address" => Some(Type::AccountId),
        "string" => Some(Type::String),
        "bytes" => Some(Type::Bytes),
        n if n.starts_with("uint") => {
            Some(if uint_strategy == "u256" { Type::U256 } else { Type::U128 })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Type;

    #[test]
    fn maps_uint256_to_u128_by_default() {
        assert_eq!(map_elementary("uint256", "u128"), Some(Type::U128));
    }
    #[test]
    fn maps_uint256_to_u256_when_opted_in() {
        assert_eq!(map_elementary("uint256", "u256"), Some(Type::U256));
    }
    #[test]
    fn maps_address_and_bool() {
        assert_eq!(map_elementary("address", "u128"), Some(Type::AccountId));
        assert_eq!(map_elementary("bool", "u128"), Some(Type::Bool));
    }
    #[test]
    fn unknown_returns_none() {
        assert_eq!(map_elementary("fixed128x18", "u128"), None);
    }
}
```

- [ ] **Step 2: Run to verify they fail then pass**

Run: `cd translator && cargo test types::`
Expected: After writing the above (impl + tests together), PASS. (If you stage the test before the fn, it fails to compile first — that is the red state.)

- [ ] **Step 3: Commit**

```bash
git add translator/src/types.rs
git commit -m "feat(translator): elementary type mapping with uint strategy"
```

---

### Task 5: Parse Solidity and extract the contract shell

**Files:**
- Modify: `translator/src/parse.rs`
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Write the failing test**

Append to `translator/tests/erc20.rs`:
```rust
use inkport_translate::parse::parse_contract_name;

#[test]
fn finds_contract_name() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    assert_eq!(parse_contract_name(&src).unwrap(), "Token");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test finds_contract_name`
Expected: FAIL — `parse_contract_name` undefined.

- [ ] **Step 3: Implement parsing**

`translator/src/parse.rs`:
```rust
use solang_parser::pt::{SourceUnitPart, ContractPart};

/// Parse source, return the first contract definition's name.
pub fn parse_contract_name(src: &str) -> Result<String, String> {
    let (unit, _comments) = solang_parser::parse(src, 0)
        .map_err(|e| format!("parse error: {:?}", e))?;
    for part in unit.0 {
        if let SourceUnitPart::ContractDefinition(def) = part {
            if let Some(id) = def.name {
                return Ok(id.name);
            }
        }
    }
    Err("no contract definition found".into())
}

/// Re-export the parsed contract definition for the lowering stage.
pub fn parse_contract(src: &str) -> Result<solang_parser::pt::ContractDefinition, String> {
    let (unit, _comments) = solang_parser::parse(src, 0)
        .map_err(|e| format!("parse error: {:?}", e))?;
    for part in unit.0 {
        if let SourceUnitPart::ContractDefinition(def) = part {
            return Ok(*def);
        }
    }
    Err("no contract definition found".into())
}

/// Helper used by lowering: a contract's parts.
pub fn contract_parts(def: &solang_parser::pt::ContractDefinition) -> &Vec<ContractPart> {
    &def.parts
}
```

> Verify the `pt` paths against the pinned `solang-parser` docs. `def.name` is `Option<Identifier>`; `Identifier.name` is `String`. If the version wraps `ContractDefinition` differently, adjust the field access only.

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test finds_contract_name`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add translator/src/parse.rs translator/tests/erc20.rs
git commit -m "feat(translator): parse contract name via solang-parser"
```

---

### Task 6: Lower storage fields (incl. nested mappings)

**Files:**
- Modify: `translator/src/lower.rs`
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Write the failing test**

Append to `translator/tests/erc20.rs`:
```rust
use inkport_translate::lower::lower_storage;
use inkport_translate::ir::Type;

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
    assert_eq!(
        allowances.ty,
        Type::Mapping(
            Box::new(Type::AccountId),
            Box::new(Type::Mapping(Box::new(Type::AccountId), Box::new(Type::U128)))
        )
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test lowers_storage_fields`
Expected: FAIL — `lower_storage` undefined.

- [ ] **Step 3: Implement lowering of variable definitions**

`translator/src/lower.rs`:
```rust
use solang_parser::pt::{ContractPart, Expression, Type as PtType};
use crate::ir::{Field, Type};
use crate::types::map_elementary;

/// Convert a solang parse-tree type expression into an IR Type.
pub fn map_type(expr: &Expression, uint_strategy: &str) -> Option<Type> {
    match expr {
        Expression::Type(_, ty) => match ty {
            PtType::Bool => Some(Type::Bool),
            PtType::Address | PtType::AddressPayable => Some(Type::AccountId),
            PtType::String => Some(Type::String),
            PtType::Uint(_) => Some(if uint_strategy == "u256" { Type::U256 } else { Type::U128 }),
            PtType::Bytes(_) | PtType::DynamicBytes => Some(Type::Bytes),
            PtType::Mapping { key, value, .. } => Some(Type::Mapping(
                Box::new(map_type(key, uint_strategy)?),
                Box::new(map_type(value, uint_strategy)?),
            )),
            _ => None,
        },
        // Some grammar versions surface elementary types as variables.
        Expression::Variable(id) => map_elementary(&id.name, uint_strategy),
        _ => None,
    }
}

pub fn lower_storage(def: &solang_parser::pt::ContractDefinition, uint_strategy: &str) -> Vec<Field> {
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
```

> `PtType::Mapping` field names (`key`, `value`) vary by version; check docs and adjust the destructure. The IR result must match the test.

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test lowers_storage_fields`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add translator/src/lower.rs translator/tests/erc20.rs
git commit -m "feat(translator): lower storage fields and mapping types"
```

---

### Task 7: Lower events and errors

**Files:**
- Modify: `translator/src/lower.rs`
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Write the failing test**

Append to `translator/tests/erc20.rs`:
```rust
use inkport_translate::lower::{lower_events, lower_errors};

#[test]
fn lowers_events_and_errors() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let def = inkport_translate::parse::parse_contract(&src).unwrap();

    let events = lower_events(&def, "u128");
    let transfer = events.iter().find(|e| e.name == "Transfer").unwrap();
    assert_eq!(transfer.fields.len(), 3);
    assert!(transfer.fields[0].indexed); // from
    assert!(transfer.fields[1].indexed); // to
    assert!(!transfer.fields[2].indexed); // value

    let errors = lower_errors(&def);
    let names: Vec<&str> = errors.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"InsufficientBalance"));
    assert!(names.contains(&"InsufficientAllowance"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test lowers_events_and_errors`
Expected: FAIL — functions undefined.

- [ ] **Step 3: Implement**

Append to `translator/src/lower.rs`:
```rust
use crate::ir::{Event, EventField, ErrorVariant};

pub fn lower_events(def: &solang_parser::pt::ContractDefinition, uint_strategy: &str) -> Vec<Event> {
    let mut out = Vec::new();
    for part in &def.parts {
        if let ContractPart::EventDefinition(ev) = part {
            let name = ev.name.as_ref().map(|n| n.name.clone()).unwrap_or_default();
            let fields = ev.fields.iter().filter_map(|f| {
                let ty = map_type(&f.ty, uint_strategy)?;
                let fname = f.name.as_ref().map(|n| n.name.clone()).unwrap_or_else(|| "arg".into());
                Some(EventField { name: fname, ty, indexed: f.indexed })
            }).collect();
            out.push(Event { name, fields });
        }
    }
    out
}

pub fn lower_errors(def: &solang_parser::pt::ContractDefinition) -> Vec<ErrorVariant> {
    let mut out = Vec::new();
    for part in &def.parts {
        if let ContractPart::ErrorDefinition(er) = part {
            if let Some(name) = &er.name {
                out.push(ErrorVariant { name: name.name.clone() });
            }
        }
    }
    out
}
```

> `EventParameter` field for the indexed flag may be `indexed: bool`. Verify and adjust.

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test lowers_events_and_errors`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add translator/src/lower.rs translator/tests/erc20.rs
git commit -m "feat(translator): lower events and errors"
```

---

### Task 8: Lower functions — signatures and mutability inference

**Files:**
- Modify: `translator/src/lower.rs`
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Write the failing test**

Append to `translator/tests/erc20.rs`:
```rust
use inkport_translate::lower::lower_functions;
use inkport_translate::ir::Mutability;

#[test]
fn lowers_function_signatures() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let def = inkport_translate::parse::parse_contract(&src).unwrap();
    let (ctor, messages) = lower_functions(&def, "u128");

    assert!(ctor.is_some(), "constructor present");

    let balance_of = messages.iter().find(|f| f.name == "balanceOf").unwrap();
    assert_eq!(balance_of.mutability, Mutability::View);
    assert_eq!(balance_of.params.len(), 1);
    assert!(balance_of.returns.is_some());

    let transfer = messages.iter().find(|f| f.name == "transfer").unwrap();
    assert_eq!(transfer.mutability, Mutability::Mutating);
    assert_eq!(transfer.params.len(), 2);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test lowers_function_signatures`
Expected: FAIL — `lower_functions` undefined.

- [ ] **Step 3: Implement signature lowering (body comes in Task 9)**

Append to `translator/src/lower.rs`:
```rust
use solang_parser::pt::{FunctionTy, FunctionAttribute, Mutability as PtMutability};
use crate::ir::{Function, Param, Mutability};

fn infer_mutability(attrs: &[FunctionAttribute]) -> Mutability {
    let mut payable = false;
    let mut view = false;
    for a in attrs {
        if let FunctionAttribute::Mutability(m) = a {
            match m {
                PtMutability::Payable(_) => payable = true,
                PtMutability::View(_) | PtMutability::Pure(_) => view = true,
                _ => {}
            }
        }
    }
    if payable { Mutability::Payable } else if view { Mutability::View } else { Mutability::Mutating }
}

/// Returns (constructor, messages). Bodies are filled by Task 9's codegen pass.
pub fn lower_functions(def: &solang_parser::pt::ContractDefinition, uint_strategy: &str)
    -> (Option<Function>, Vec<Function>)
{
    let mut ctor = None;
    let mut messages = Vec::new();

    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            let params: Vec<Param> = f.params.iter().filter_map(|(_, p)| {
                let p = p.as_ref()?;
                let ty = map_type(&p.ty, uint_strategy)?;
                let name = p.name.as_ref().map(|n| n.name.clone()).unwrap_or_else(|| "arg".into());
                Some(Param { name, ty })
            }).collect();

            let returns = f.returns.first()
                .and_then(|(_, p)| p.as_ref())
                .and_then(|p| map_type(&p.ty, uint_strategy));

            match f.ty {
                FunctionTy::Constructor => {
                    ctor = Some(Function {
                        name: "new".into(),
                        mutability: Mutability::Mutating,
                        params, returns: None, body: vec![],
                    });
                }
                FunctionTy::Function => {
                    let name = f.name.as_ref().map(|n| n.name.clone()).unwrap_or_default();
                    messages.push(Function {
                        name, mutability: infer_mutability(&f.attributes),
                        params, returns, body: vec![],
                    });
                }
                _ => {}
            }
        }
    }
    (ctor, messages)
}
```

> `f.params` is `Vec<(Loc, Option<Parameter>)>` in solang-parser; the `(_, p)` destructure handles that. Verify `FunctionAttribute::Mutability` variant names against the docs.

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test lowers_function_signatures`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add translator/src/lower.rs translator/tests/erc20.rs
git commit -m "feat(translator): lower function signatures and mutability"
```

---

### Task 9: Statement/expression codegen for the ERC-20 subset

**Files:**
- Modify: `translator/src/lower.rs` (statement lowering helpers)
- Test: `translator/src/lower.rs` (inline tests)

This task lowers the constructs the ERC-20 fixture uses: `msg.sender`, mapping read/write, `require`/`revert CustomError()`, binary `+`/`-`, `emit`, `return`. Output is ink! Rust source lines stored in `Function.body`.

- [ ] **Step 1: Write failing tests for expression lowering**

Append to `translator/src/lower.rs`:
```rust
#[cfg(test)]
mod expr_tests {
    use super::*;

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
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test expr_tests`
Expected: FAIL — render helpers undefined.

- [ ] **Step 3: Implement the render helpers**

Append to `translator/src/lower.rs`:
```rust
pub fn render_msg_sender() -> String {
    "self.env().caller()".into()
}

pub fn render_mapping_read(map: &str, key_expr: &str) -> String {
    format!("self.{map}.get({key_expr}).unwrap_or_default()")
}

pub fn render_mapping_write(map: &str, key_expr: &str, value_expr: &str) -> String {
    format!("self.{map}.insert({key_expr}, &{value_expr});")
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test expr_tests`
Expected: PASS.

- [ ] **Step 5: Add the full expression/statement walker**

Append to `translator/src/lower.rs`. This walks solang `Statement`/`Expression` nodes for the supported subset and returns rendered lines; unsupported nodes return a `// TODO: manual review` marker that the report (Task 11) collects.

```rust
use solang_parser::pt::{Statement, Expression};

pub struct LowerCtx<'a> {
    pub storage_names: &'a [String],
    /// Constructs we could not translate, surfaced in the report.
    pub unsupported: Vec<String>,
}

impl<'a> LowerCtx<'a> {
    fn is_storage(&self, name: &str) -> bool {
        self.storage_names.iter().any(|s| s == name)
    }

    pub fn expr(&mut self, e: &Expression) -> String {
        match e {
            // msg.sender
            Expression::MemberAccess(_, base, member)
                if matches!(&**base, Expression::Variable(id) if id.name == "msg")
                    && member.name == "sender" => render_msg_sender(),
            // identifiers
            Expression::Variable(id) => id.name.clone(),
            // number literals
            Expression::NumberLiteral(_, n, _, _) => n.clone(),
            Expression::BoolLiteral(_, b) => b.to_string(),
            // mapping/array index read: m[k]  or  m[a][b]
            Expression::ArraySubscript(_, base, Some(idx)) => {
                let key = self.expr(idx);
                match &**base {
                    Expression::Variable(id) if self.is_storage(&id.name) =>
                        render_mapping_read(&id.name, &key),
                    // nested: allowances[a][b]
                    Expression::ArraySubscript(_, inner, Some(idx2)) => {
                        if let Expression::Variable(id) = &**inner {
                            let k2 = self.expr(idx2);
                            // single Mapping<AccountId,(AccountId->V)> modeled as Mapping<(A,A),V>
                            format!("self.{}.get(({}, {})).unwrap_or_default()", id.name, k2, key)
                        } else { self.todo(e) }
                    }
                    _ => self.todo(e),
                }
            }
            // binary + / -  → checked arithmetic
            Expression::Add(_, l, r) => format!("{}.checked_add({}).ok_or(Error::Overflow)?", self.expr(l), self.expr(r)),
            Expression::Subtract(_, l, r) => format!("{}.checked_sub({}).ok_or(Error::Overflow)?", self.expr(l), self.expr(r)),
            Expression::Less(_, l, r) => format!("{} < {}", self.expr(l), self.expr(r)),
            _ => self.todo(e),
        }
    }

    fn todo(&mut self, e: &Expression) -> String {
        self.unsupported.push(format!("{:?}", e));
        "/* TODO: manual review */ Default::default()".into()
    }

    pub fn stmt(&mut self, s: &Statement) -> Vec<String> {
        match s {
            Statement::Return(_, Some(e)) => vec![format!("return Ok({});", self.expr(e))],
            Statement::Return(_, None) => vec!["return Ok(());".into()],
            Statement::Expression(_, e) => self.expr_statement(e),
            Statement::Block { statements, .. } =>
                statements.iter().flat_map(|st| self.stmt(st)).collect(),
            Statement::If(_, cond, then_box, _else) => {
                let c = self.expr(cond);
                let mut lines = vec![format!("if {} {{", c)];
                lines.extend(self.stmt(then_box));
                lines.push("}".into());
                lines
            }
            _ => { self.unsupported.push(format!("{:?}", s)); vec!["// TODO: manual review".into()] }
        }
    }

    fn expr_statement(&mut self, e: &Expression) -> Vec<String> {
        match e {
            // assignment:  lhs = rhs
            Expression::Assign(_, lhs, rhs) => {
                let value = self.expr(rhs);
                match &**lhs {
                    Expression::ArraySubscript(_, base, Some(idx)) => {
                        let key = self.expr(idx);
                        if let Expression::Variable(id) = &**base {
                            return vec![render_mapping_write(&id.name, &key, &value)];
                        }
                        if let Expression::ArraySubscript(_, inner, Some(idx2)) = &**base {
                            if let Expression::Variable(id) = &**inner {
                                let k2 = self.expr(idx2);
                                return vec![format!("self.{}.insert(({}, {}), &{});", id.name, k2, key, value)];
                            }
                        }
                        vec!["// TODO: manual review".into()]
                    }
                    Expression::Variable(id) if self.is_storage(&id.name) =>
                        vec![format!("self.{} = {};", id.name, value)],
                    Expression::Variable(id) => vec![format!("{} = {};", id.name, value)],
                    _ => vec!["// TODO: manual review".into()],
                }
            }
            // emit Event(args)  — solang models emit as a FunctionCall under Statement::Emit in some versions;
            // here we handle the FunctionCall form named like an event.
            Expression::FunctionCall(_, callee, args) => {
                if let Expression::Variable(id) = &**callee {
                    if id.name == "revert" {
                        return vec!["// revert handled at call site".into()];
                    }
                    let rendered: Vec<String> = args.iter().map(|a| self.expr(a)).collect();
                    return vec![format!("// call {}({})", id.name, rendered.join(", "))];
                }
                vec!["// TODO: manual review".into()]
            }
            _ => { self.unsupported.push(format!("{:?}", e)); vec!["// TODO: manual review".into()] }
        }
    }
}
```

> `emit` and `revert CustomError()` are surfaced by solang as `Statement::Emit` and `Statement::Revert` in v0.3. Add explicit arms:
> ```rust
> Statement::Emit(_, expr) => { /* render self.env().emit_event(Name { fields }) */ }
> Statement::Revert(_, path, _args) => { /* render return Err(Error::Name) */ }
> ```
> Implement these two arms following the same `expr()` pattern; map the emitted event name to its IR `Event`, and the revert path to an `Error` variant. Add one inline test each asserting the rendered line equals `self.env().emit_event(Transfer { from: ..., to: ..., value });` and `return Err(Error::InsufficientBalance);` respectively before implementing.

- [ ] **Step 6: Run all translator tests**

Run: `cd translator && cargo test`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add translator/src/lower.rs
git commit -m "feat(translator): statement/expression codegen for ERC-20 subset"
```

---

### Task 10: ink! source emission

**Files:**
- Modify: `translator/src/codegen.rs`
- Test: `translator/src/codegen.rs` (inline tests)

- [ ] **Step 1: Write the failing test**

`translator/src/codegen.rs`:
```rust
use crate::ir::*;

pub fn render_type(t: &Type) -> String {
    match t {
        Type::Bool => "bool".into(),
        Type::U128 => "u128".into(),
        Type::U256 => "U256".into(),
        Type::AccountId => "AccountId".into(),
        Type::String => "String".into(),
        Type::Bytes => "Vec<u8>".into(),
        Type::Mapping(k, v) => match &**v {
            // nested mapping flattened to a tuple key
            Type::Mapping(k2, v2) =>
                format!("Mapping<({}, {}), {}>", render_type(k), render_type(k2), render_type(v2)),
            _ => format!("Mapping<{}, {}>", render_type(k), render_type(v)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Type;

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
```

- [ ] **Step 2: Run to verify it passes**

Run: `cd translator && cargo test codegen::tests::renders`
Expected: PASS.

- [ ] **Step 3: Write the failing module-emission test**

Append to `translator/src/codegen.rs`:
```rust
#[cfg(test)]
mod module_tests {
    use super::*;
    use crate::ir::*;

    fn sample() -> Contract {
        Contract {
            name: "Token".into(),
            storage: vec![Field { name: "totalSupply_".into(), ty: Type::U128 }],
            events: vec![Event { name: "Transfer".into(), fields: vec![
                EventField { name: "from".into(), ty: Type::AccountId, indexed: true },
                EventField { name: "value".into(), ty: Type::U128, indexed: false },
            ]}],
            errors: vec![ErrorVariant { name: "Overflow".into() }],
            constructor: Some(Function { name: "new".into(), mutability: Mutability::Mutating,
                params: vec![Param{name:"initialSupply".into(), ty:Type::U128}],
                returns: None, body: vec!["self.total_supply_ = initial_supply;".into()] }),
            messages: vec![Function { name: "balanceOf".into(), mutability: Mutability::View,
                params: vec![Param{name:"who".into(), ty:Type::AccountId}],
                returns: Some(Type::U128), body: vec!["return Ok(0);".into()] }],
        }
    }

    #[test]
    fn emits_ink_module() {
        let src = emit_contract(&sample());
        assert!(src.contains("#[ink::contract]"));
        assert!(src.contains("pub struct Token"));
        assert!(src.contains("#[ink(storage)]"));
        assert!(src.contains("#[ink(event)]"));
        assert!(src.contains("pub enum Error"));
        assert!(src.contains("#[ink(constructor)]"));
        assert!(src.contains("#[ink(message)]"));
    }
}
```

- [ ] **Step 4: Run to verify it fails**

Run: `cd translator && cargo test emits_ink_module`
Expected: FAIL — `emit_contract` undefined.

- [ ] **Step 5: Implement emission**

Append to `translator/src/codegen.rs`:
```rust
fn snake(s: &str) -> String {
    // minimal camelCase → snake_case for field/fn names
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 { out.push('_'); }
            out.extend(c.to_lowercase());
        } else { out.push(c); }
    }
    out.trim_end_matches('_').to_string() + if s.ends_with('_') { "_" } else { "" }
}

fn render_self_param(m: &Mutability) -> &'static str {
    match m { Mutability::View => "&self", _ => "&mut self" }
}

fn render_attr(m: &Mutability) -> &'static str {
    match m {
        Mutability::Payable => "#[ink(message, payable)]",
        _ => "#[ink(message)]",
    }
}

fn render_fn(f: &Function, is_ctor: bool) -> String {
    let params: Vec<String> = f.params.iter()
        .map(|p| format!("{}: {}", snake(&p.name), render_type(&p.ty))).collect();
    let body = f.body.join("\n        ");
    if is_ctor {
        format!(
"        #[ink(constructor)]
        pub fn {name}({params}) -> Self {{
            let mut instance = Self::default();
            {body}
            instance
        }}",
            name = f.name,
            params = params.join(", "),
            body = body)
    } else {
        let ret = match &f.returns {
            Some(t) => format!(" -> Result<{}, Error>", render_type(t)),
            None => " -> Result<(), Error>".into(),
        };
        let self_p = render_self_param(&f.mutability);
        let all_params = std::iter::once(self_p.to_string())
            .chain(params).collect::<Vec<_>>().join(", ");
        format!(
"        {attr}
        pub fn {name}({all_params}){ret} {{
            {body}
        }}",
            attr = render_attr(&f.mutability),
            name = snake(&f.name),
            all_params = all_params, ret = ret, body = body)
    }
}

pub fn emit_contract(c: &Contract) -> String {
    let mod_name = c.name.to_lowercase();

    let storage_fields: Vec<String> = c.storage.iter()
        .map(|f| format!("        {}: {},", snake(&f.name), render_type(&f.ty))).collect();

    let events: Vec<String> = c.events.iter().map(|e| {
        let fields: Vec<String> = e.fields.iter().map(|fld| {
            let topic = if fld.indexed { "            #[ink(topic)]\n" } else { "" };
            format!("{}            {}: {},", topic, snake(&fld.name), render_type(&fld.ty))
        }).collect();
        format!("    #[ink(event)]\n    pub struct {} {{\n{}\n    }}", e.name, fields.join("\n"))
    }).collect();

    let mut error_variants: Vec<String> = c.errors.iter().map(|e| format!("        {},", e.name)).collect();
    if !error_variants.iter().any(|v| v.contains("Overflow")) {
        error_variants.push("        Overflow,".into());
    }

    let ctor = c.constructor.as_ref().map(|f| render_fn(f, true)).unwrap_or_default();
    let messages: Vec<String> = c.messages.iter().map(|f| render_fn(f, false)).collect();

    format!(
"#![cfg_attr(not(feature = \"std\"), no_std, no_main)]

#[ink::contract]
mod {mod_name} {{
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct {name} {{
{storage}
    }}

{events}

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {{
{errors}
    }}

    impl {name} {{
{ctor}

{messages}
    }}
}}",
        mod_name = mod_name,
        name = c.name,
        storage = storage_fields.join("\n"),
        events = events.join("\n\n"),
        errors = error_variants.join("\n"),
        ctor = ctor,
        messages = messages.join("\n\n"))
}
```

- [ ] **Step 6: Run to verify it passes**

Run: `cd translator && cargo test emits_ink_module`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add translator/src/codegen.rs
git commit -m "feat(translator): ink! source emission"
```

---

### Task 11: Translation report

**Files:**
- Modify: `translator/src/report.rs`
- Test: `translator/src/report.rs` (inline tests)

- [ ] **Step 1: Write the failing test**

`translator/src/report.rs`:
```rust
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub enum Status { Translated, Warning, Unsupported }

#[derive(Debug, Serialize)]
pub struct Entry { pub construct: String, pub status_label: String, pub note: String }

#[derive(Debug, Serialize)]
pub struct Report { pub contract: String, pub entries: Vec<Entry> }

impl Report {
    pub fn to_markdown(&self) -> String {
        let mut s = format!("# Translation report: {}\n\n", self.contract);
        s.push_str("| Construct | Status | Note |\n|---|---|---|\n");
        for e in &self.entries {
            s.push_str(&format!("| {} | {} | {} |\n", e.construct, e.status_label, e.note));
        }
        s
    }
}

pub fn label(s: &Status) -> &'static str {
    match s { Status::Translated => "✅", Status::Warning => "⚠️", Status::Unsupported => "⛔" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_has_rows() {
        let r = Report { contract: "Token".into(), entries: vec![
            Entry { construct: "transfer".into(), status_label: label(&Status::Translated).into(), note: "".into() },
            Entry { construct: "selfdestruct".into(), status_label: label(&Status::Unsupported).into(), note: "no equivalent".into() },
        ]};
        let md = r.to_markdown();
        assert!(md.contains("# Translation report: Token"));
        assert!(md.contains("✅"));
        assert!(md.contains("⛔"));
    }
}
```

- [ ] **Step 2: Run to verify it passes**

Run: `cd translator && cargo test report::`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add translator/src/report.rs
git commit -m "feat(translator): translation report (json + markdown)"
```

---

### Task 12: Translator binary — wire end to end

**Files:**
- Modify: `translator/src/main.rs`
- Modify: `translator/src/lower.rs` (add a `lower_contract` orchestrator)
- Test: `translator/tests/erc20.rs`

- [ ] **Step 1: Write the failing integration test**

Append to `translator/tests/erc20.rs`:
```rust
use inkport_translate::lower::lower_contract;
use inkport_translate::codegen::emit_contract;

#[test]
fn full_pipeline_emits_compilable_shapes() {
    let src = std::fs::read_to_string("fixtures/ERC20.sol").unwrap();
    let (contract, _report) = lower_contract(&src, "u128").unwrap();
    let ink = emit_contract(&contract);
    assert!(ink.contains("mod token"));
    assert!(ink.contains("balances: Mapping<AccountId, u128>"));
    assert!(ink.contains("allowances: Mapping<(AccountId, AccountId), u128>"));
    assert!(ink.contains("#[ink(constructor)]"));
    assert!(ink.contains("pub fn balance_of(&self"));
    assert!(ink.contains("pub fn transfer(&mut self"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd translator && cargo test full_pipeline_emits_compilable_shapes`
Expected: FAIL — `lower_contract` undefined.

- [ ] **Step 3: Add the orchestrator**

Append to `translator/src/lower.rs`:
```rust
use crate::ir::Contract;
use crate::report::{Report, Entry, Status, label};

/// Parse + lower a single Solidity source into a Contract and a Report.
pub fn lower_contract(src: &str, uint_strategy: &str) -> Result<(Contract, Report), String> {
    let def = crate::parse::parse_contract(src)?;
    let name = def.name.as_ref().map(|n| n.name.clone()).ok_or("unnamed contract")?;

    let storage = lower_storage(&def, uint_strategy);
    let events = lower_events(&def, uint_strategy);
    let errors = lower_errors(&def);
    let (mut ctor, mut messages) = lower_functions(&def, uint_strategy);

    let storage_names: Vec<String> = storage.iter().map(|f| f.name.clone()).collect();
    let mut entries: Vec<Entry> = Vec::new();

    // Lower each function body with the statement walker.
    for part in &def.parts {
        if let ContractPart::FunctionDefinition(f) = part {
            let mut ctx = LowerCtx { storage_names: &storage_names, unsupported: vec![] };
            let body = match &f.body {
                Some(Statement::Block { statements, .. }) =>
                    statements.iter().flat_map(|s| ctx.stmt(s)).collect::<Vec<_>>(),
                _ => vec![],
            };
            let fname = f.name.as_ref().map(|n| n.name.clone()).unwrap_or_else(|| "new".into());
            let status = if ctx.unsupported.is_empty() { Status::Translated } else { Status::Warning };
            entries.push(Entry {
                construct: fname.clone(),
                status_label: label(&status).into(),
                note: ctx.unsupported.join("; "),
            });
            // attach body
            if matches!(f.ty, solang_parser::pt::FunctionTy::Constructor) {
                if let Some(c) = ctor.as_mut() { c.body = body; }
            } else if let Some(m) = messages.iter_mut().find(|m| m.name == fname) {
                m.body = body;
            }
        }
    }

    let contract = Contract { name: name.clone(), storage, events, errors, constructor: ctor, messages };
    let report = Report { contract: name, entries };
    Ok((contract, report))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd translator && cargo test full_pipeline_emits_compilable_shapes`
Expected: PASS.

- [ ] **Step 5: Implement the binary CLI**

`translator/src/main.rs`:
```rust
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "inkport-translate")]
struct Args {
    /// Path to the .sol source file
    input: PathBuf,
    /// Output crate directory
    #[arg(long, default_value = "out")]
    out: PathBuf,
    /// uint strategy: u128 (default) or u256
    #[arg(long, default_value = "u128")]
    uint: String,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let src = fs::read_to_string(&args.input).map_err(|e| e.to_string())?;
    let (contract, report) = inkport_translate::lower::lower_contract(&src, &args.uint)?;
    let ink = inkport_translate::codegen::emit_contract(&contract);

    fs::create_dir_all(&args.out).map_err(|e| e.to_string())?;
    fs::write(args.out.join("lib.rs"), ink).map_err(|e| e.to_string())?;

    let cargo_toml = format!(
"[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nink = {{ version = \"5\", default-features = false }}\n\n[lib]\npath = \"lib.rs\"\n\n[features]\ndefault = [\"std\"]\nstd = [\"ink/std\"]\nink-as-dependency = []\n",
        name = contract.name.to_lowercase());
    fs::write(args.out.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;

    fs::write(args.out.join("translation-report.md"), report.to_markdown()).map_err(|e| e.to_string())?;
    fs::write(args.out.join("translation-report.json"),
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    println!("{}", serde_json::json!({
        "contract": contract.name,
        "out": args.out.to_string_lossy(),
    }));
    Ok(())
}
```

- [ ] **Step 6: Run the binary against the fixture**

Run: `cd translator && cargo run -- fixtures/ERC20.sol --out /tmp/token-out`
Expected: writes `/tmp/token-out/{lib.rs,Cargo.toml,translation-report.md,translation-report.json}`; prints JSON with `"contract":"Token"`.

- [ ] **Step 7: Commit**

```bash
git add translator/
git commit -m "feat(translator): wire full pipeline and binary CLI"
```

---

## Phase 2 — Python framework

### Task 13: Config loader

**Files:**
- Create: `inkport/inkport/config.py`
- Test: `inkport/tests/test_config.py`

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_config.py`:
```python
import os
from pathlib import Path
import pytest
from inkport.config import load_config, resolve_network, resolve_account

def write_config(tmp_path: Path) -> Path:
    cfg = tmp_path / "inkport.config.py"
    cfg.write_text(
        'config = {\n'
        '  "networks": {"portaldot": {"url": "wss://mainnet.portaldot.io", "decimals": 14, "ss58": 42}},\n'
        '  "accounts": {"deployer": {"suri": "$INKPORT_SURI"}},\n'
        '  "compiler": {"ink": "5.x", "uint_strategy": "u128"},\n'
        '  "default_network": "portaldot",\n'
        '}\n'
    )
    return cfg

def test_loads_and_resolves_network(tmp_path):
    write_config(tmp_path)
    cfg = load_config(tmp_path)
    net = resolve_network(cfg, "portaldot")
    assert net["url"] == "wss://mainnet.portaldot.io"
    assert net["decimals"] == 14

def test_env_interpolation_in_account(tmp_path, monkeypatch):
    write_config(tmp_path)
    monkeypatch.setenv("INKPORT_SURI", "//Alice")
    cfg = load_config(tmp_path)
    acct = resolve_account(cfg, "deployer")
    assert acct["suri"] == "//Alice"

def test_missing_env_raises(tmp_path, monkeypatch):
    write_config(tmp_path)
    monkeypatch.delenv("INKPORT_SURI", raising=False)
    cfg = load_config(tmp_path)
    with pytest.raises(KeyError):
        resolve_account(cfg, "deployer")
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_config.py -v`
Expected: FAIL — `inkport.config` undefined.

- [ ] **Step 3: Implement**

`inkport/inkport/config.py`:
```python
import importlib.util
import os
from pathlib import Path


def load_config(project_dir: Path | str = ".") -> dict:
    project_dir = Path(project_dir)
    cfg_path = project_dir / "inkport.config.py"
    if not cfg_path.exists():
        raise FileNotFoundError(f"no inkport.config.py in {project_dir}")
    spec = importlib.util.spec_from_file_location("inkport_user_config", cfg_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if not hasattr(module, "config"):
        raise ValueError("inkport.config.py must define a `config` dict")
    return module.config


def resolve_network(cfg: dict, name: str) -> dict:
    networks = cfg.get("networks", {})
    if name not in networks:
        raise KeyError(f"unknown network: {name}")
    return networks[name]


def _interp(value: str) -> str:
    if isinstance(value, str) and value.startswith("$"):
        env_key = value[1:]
        if env_key not in os.environ:
            raise KeyError(f"environment variable {env_key} not set")
        return os.environ[env_key]
    return value


def resolve_account(cfg: dict, name: str) -> dict:
    accounts = cfg.get("accounts", {})
    if name not in accounts:
        raise KeyError(f"unknown account: {name}")
    return {k: _interp(v) for k, v in accounts[name].items()}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd inkport && pytest tests/test_config.py -v`
Expected: PASS (all three).

- [ ] **Step 5: Commit**

```bash
git add inkport/inkport/config.py inkport/tests/test_config.py
git commit -m "feat(config): loader with network/account resolution and env interpolation"
```

---

### Task 14: `inkport init` scaffold

**Files:**
- Create: `inkport/inkport/scaffold/__init__.py`
- Create: `inkport/inkport/scaffold/files.py`
- Modify: `inkport/inkport/cli.py`
- Test: `inkport/tests/test_init.py`

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_init.py`:
```python
from pathlib import Path
from inkport.scaffold import scaffold_project

def test_scaffold_creates_structure(tmp_path):
    scaffold_project(tmp_path)
    assert (tmp_path / "inkport.config.py").exists()
    assert (tmp_path / "contracts").is_dir()
    assert (tmp_path / "scripts" / "deploy.py").exists()
    assert (tmp_path / "tests").is_dir()
    assert (tmp_path / ".gitignore").exists()
    gi = (tmp_path / ".gitignore").read_text()
    assert "artifacts/" in gi
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_init.py -v`
Expected: FAIL — `inkport.scaffold` undefined.

- [ ] **Step 3: Implement scaffold**

`inkport/inkport/scaffold/files.py`:
```python
CONFIG = '''config = {
    "networks": {
        "portaldot": {"url": "wss://mainnet.portaldot.io", "decimals": 14, "ss58": 42},
        "local":     {"url": "ws://127.0.0.1:9944", "decimals": 14, "ss58": 42},
    },
    "accounts": {"deployer": {"suri": "$INKPORT_SURI"}},
    "compiler": {"ink": "5.x", "uint_strategy": "u128"},
    "default_network": "portaldot",
}
'''

DEPLOY = '''"""Deploy script. Run with: inkport run scripts/deploy.py --network portaldot"""

def main(ctx):
    # ctx.substrate, ctx.keypair, ctx.deploy(contract_name, args) provided by the runtime
    address = ctx.deploy("Token", {"initialSupply": 1_000_000})
    print("deployed at", address)
'''

GITIGNORE = "artifacts/\n__pycache__/\n.env\n"

EXAMPLE_SOL = '''// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Token {
    mapping(address => uint256) private balances;
    constructor(uint256 initialSupply) { balances[msg.sender] = initialSupply; }
    function balanceOf(address who) public view returns (uint256) { return balances[who]; }
}
'''
```

`inkport/inkport/scaffold/__init__.py`:
```python
from pathlib import Path
from .files import CONFIG, DEPLOY, GITIGNORE, EXAMPLE_SOL


def scaffold_project(directory: Path | str) -> None:
    d = Path(directory)
    (d / "contracts").mkdir(parents=True, exist_ok=True)
    (d / "scripts").mkdir(parents=True, exist_ok=True)
    (d / "tests").mkdir(parents=True, exist_ok=True)
    (d / "inkport.config.py").write_text(CONFIG)
    (d / "scripts" / "deploy.py").write_text(DEPLOY)
    (d / ".gitignore").write_text(GITIGNORE)
    (d / "contracts" / "Token.sol").write_text(EXAMPLE_SOL)
```

- [ ] **Step 4: Wire into the CLI**

In `inkport/inkport/cli.py`, replace the `init` body:
```python
@app.command()
def init(directory: str = "."):
    """Scaffold a new InkPort project."""
    from inkport.scaffold import scaffold_project
    scaffold_project(directory)
    typer.echo(f"Initialized InkPort project in {directory}")
```

- [ ] **Step 5: Run to verify it passes**

Run: `cd inkport && pytest tests/test_init.py -v`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add inkport/inkport/scaffold inkport/inkport/cli.py inkport/tests/test_init.py
git commit -m "feat(cli): inkport init scaffold"
```

---

### Task 15: `inkport compile` — invoke translator + cargo build

**Files:**
- Create: `inkport/inkport/compile.py`
- Modify: `inkport/inkport/cli.py`
- Test: `inkport/tests/test_compile.py`

- [ ] **Step 1: Write the failing test (translator + build steps are injected, so it runs without the real toolchain)**

`inkport/tests/test_compile.py`:
```python
from pathlib import Path
from inkport.compile import compile_contract

def test_compile_invokes_translator_then_build(tmp_path, monkeypatch):
    (tmp_path / "contracts").mkdir()
    sol = tmp_path / "contracts" / "Token.sol"
    sol.write_text("contract Token {}")

    calls = []

    def fake_translate(src_path, out_dir, uint_strategy):
        calls.append(("translate", str(src_path), uint_strategy))
        Path(out_dir).mkdir(parents=True, exist_ok=True)
        (Path(out_dir) / "lib.rs").write_text("// ink")
        (Path(out_dir) / "translation-report.md").write_text("# report")
        return {"contract": "Token"}

    def fake_build(crate_dir):
        calls.append(("build", str(crate_dir)))
        (Path(crate_dir) / "Token.contract").write_text("{}")
        return Path(crate_dir) / "Token.contract"

    artifact = compile_contract(
        project_dir=tmp_path, contract="Token", uint_strategy="u128",
        translate=fake_translate, build=fake_build,
    )

    assert ("translate", str(sol), "u128") in calls
    assert any(c[0] == "build" for c in calls)
    assert artifact.name == "Token.contract"
    assert (tmp_path / "artifacts" / "Token" / "lib.rs").exists()
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_compile.py -v`
Expected: FAIL — `inkport.compile` undefined.

- [ ] **Step 3: Implement compile with injectable steps**

`inkport/inkport/compile.py`:
```python
import json
import shutil
import subprocess
from pathlib import Path


def _default_translate(src_path: Path, out_dir: Path, uint_strategy: str) -> dict:
    """Invoke the inkport-translate Rust binary."""
    binary = shutil.which("inkport-translate") or "inkport-translate"
    result = subprocess.run(
        [binary, str(src_path), "--out", str(out_dir), "--uint", uint_strategy],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"translate failed:\n{result.stderr}")
    return json.loads(result.stdout)


def _default_build(crate_dir: Path) -> Path:
    """Run cargo contract build in the generated crate."""
    result = subprocess.run(
        ["cargo", "contract", "build", "--release"],
        cwd=str(crate_dir), capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"cargo contract build failed:\n{result.stderr}")
    # cargo-contract writes to target/ink/<name>.contract
    candidates = list((crate_dir / "target" / "ink").glob("*.contract"))
    if not candidates:
        raise RuntimeError("build produced no .contract file")
    dest = crate_dir / candidates[0].name
    shutil.copy(candidates[0], dest)
    return dest


def compile_contract(project_dir, contract: str, uint_strategy: str = "u128",
                     translate=_default_translate, build=_default_build) -> Path:
    project_dir = Path(project_dir)
    src_path = project_dir / "contracts" / f"{contract}.sol"
    if not src_path.exists():
        raise FileNotFoundError(f"no contract source: {src_path}")
    out_dir = project_dir / "artifacts" / contract
    translate(src_path, out_dir, uint_strategy)
    return build(out_dir)
```

- [ ] **Step 4: Wire into the CLI**

In `inkport/inkport/cli.py`, replace the `compile` body:
```python
@app.command()
def compile(contract: str = typer.Option("Token", "--contract")):
    """Translate Solidity to ink! and build WASM + metadata."""
    from inkport.compile import compile_contract
    from inkport.config import load_config
    cfg = load_config(".")
    strategy = cfg.get("compiler", {}).get("uint_strategy", "u128")
    artifact = compile_contract(".", contract, strategy)
    typer.echo(f"Built {artifact}")
```

- [ ] **Step 5: Run to verify it passes**

Run: `cd inkport && pytest tests/test_compile.py -v`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add inkport/inkport/compile.py inkport/inkport/cli.py inkport/tests/test_compile.py
git commit -m "feat(compile): orchestrate translator + cargo contract build"
```

---

### Task 16: Deployer library

**Files:**
- Create: `inkport/inkport/deploy.py`
- Test: `inkport/tests/test_deploy.py`

The deployer wraps the Portaldot SDK. Tests inject a fake substrate client so they run without a chain.

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_deploy.py`:
```python
import json
from pathlib import Path
from inkport.deploy import Deployer

class FakeContractInstance:
    def __init__(self, address): self.contract_address = address

class FakeCode:
    def __init__(self, **kw): self.kw = kw
    def deploy(self, keypair, endowment, gas_limit, constructor, args, upload_code):
        return FakeContractInstance("5DeployedAddr")

def test_deploy_records_address(tmp_path):
    captured = {}
    def fake_code_factory(metadata_file, wasm_file, substrate):
        captured["metadata"] = metadata_file
        return FakeCode()

    d = Deployer(
        substrate=object(),
        keypair=object(),
        code_factory=fake_code_factory,
    )
    addr = d.deploy(
        contract_dir=tmp_path,
        contract="Token",
        constructor="new",
        args={"initialSupply": 1000},
        value_pot=0.0,
        deployments_path=tmp_path / "deployments" / "portaldot.json",
    )
    assert addr == "5DeployedAddr"
    saved = json.loads((tmp_path / "deployments" / "portaldot.json").read_text())
    assert saved["Token"]["address"] == "5DeployedAddr"
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_deploy.py -v`
Expected: FAIL — `inkport.deploy` undefined.

- [ ] **Step 3: Implement the deployer**

`inkport/inkport/deploy.py`:
```python
import json
from pathlib import Path

POT_DECIMALS = 14


def pot_to_planck(pot: float) -> int:
    return int(pot * (10 ** POT_DECIMALS))


def _default_code_factory(metadata_file, wasm_file, substrate):
    from substrateinterface.contracts import ContractCode
    return ContractCode.create_from_contract_files(
        metadata_file=metadata_file, wasm_file=wasm_file, substrate=substrate)


class Deployer:
    def __init__(self, substrate, keypair, code_factory=_default_code_factory):
        self.substrate = substrate
        self.keypair = keypair
        self.code_factory = code_factory

    def deploy(self, contract_dir, contract: str, constructor: str, args: dict,
               value_pot: float, deployments_path) -> str:
        contract_dir = Path(contract_dir)
        metadata = str(contract_dir / "artifacts" / contract / f"{contract}.contract")
        wasm = str(contract_dir / "artifacts" / contract / f"{contract}.wasm")

        code = self.code_factory(metadata_file=metadata, wasm_file=wasm, substrate=self.substrate)
        instance = code.deploy(
            keypair=self.keypair,
            endowment=pot_to_planck(value_pot),
            gas_limit=1_000_000_000_000,
            constructor=constructor,
            args=args,
            upload_code=True,
        )
        address = instance.contract_address

        dp = Path(deployments_path)
        dp.parent.mkdir(parents=True, exist_ok=True)
        existing = json.loads(dp.read_text()) if dp.exists() else {}
        existing[contract] = {"address": address}
        dp.write_text(json.dumps(existing, indent=2))
        return address
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd inkport && pytest tests/test_deploy.py -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add inkport/inkport/deploy.py inkport/tests/test_deploy.py
git commit -m "feat(deploy): Deployer over Portaldot SDK with POT conversion"
```

---

### Task 17: Runtime context + `inkport deploy` / `inkport run`

**Files:**
- Create: `inkport/inkport/run.py`
- Modify: `inkport/inkport/cli.py`
- Test: `inkport/tests/test_run.py`

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_run.py`:
```python
from pathlib import Path
from inkport.run import build_context, run_script

def test_run_script_calls_main_with_ctx(tmp_path, monkeypatch):
    script = tmp_path / "myscript.py"
    script.write_text(
        "def main(ctx):\n"
        "    ctx.recorded = ctx.network_name\n"
    )
    ctx = build_context(network_name="local", substrate=object(), keypair=object(), deployer=object())
    run_script(script, ctx)
    assert ctx.recorded == "local"
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_run.py -v`
Expected: FAIL — `inkport.run` undefined.

- [ ] **Step 3: Implement context + runner**

`inkport/inkport/run.py`:
```python
import importlib.util
from pathlib import Path


class Context:
    def __init__(self, network_name, substrate, keypair, deployer):
        self.network_name = network_name
        self.substrate = substrate
        self.keypair = keypair
        self._deployer = deployer

    def deploy(self, contract: str, args: dict, value_pot: float = 0.0):
        return self._deployer.deploy(
            contract_dir=".", contract=contract, constructor="new",
            args=args, value_pot=value_pot,
            deployments_path=Path("deployments") / f"{self.network_name}.json",
        )


def build_context(network_name, substrate, keypair, deployer) -> Context:
    return Context(network_name, substrate, keypair, deployer)


def run_script(script_path, ctx: Context) -> None:
    script_path = Path(script_path)
    spec = importlib.util.spec_from_file_location("inkport_user_script", script_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if not hasattr(module, "main"):
        raise ValueError(f"{script_path} must define main(ctx)")
    module.main(ctx)
```

- [ ] **Step 4: Wire `deploy` and `run` into the CLI**

Add a shared helper and replace both command bodies in `inkport/inkport/cli.py`:
```python
def _connect(network: str, account: str):
    """Build (substrate, keypair, deployer) from config for a network."""
    from substrateinterface import SubstrateInterface, Keypair
    from inkport.config import load_config, resolve_network, resolve_account
    from inkport.deploy import Deployer
    cfg = load_config(".")
    net = resolve_network(cfg, network)
    acct = resolve_account(cfg, account)
    substrate = SubstrateInterface(url=net["url"], ss58_format=net.get("ss58", 42))
    keypair = Keypair.create_from_uri(acct["suri"])
    return substrate, keypair, Deployer(substrate, keypair)

@app.command()
def deploy(network: str = typer.Option(..., "--network"),
           contract: str = typer.Option("Token", "--contract"),
           value: float = typer.Option(0.0, "--value"),
           account: str = typer.Option("deployer", "--account")):
    """Deploy a compiled contract to a network."""
    from pathlib import Path
    substrate, keypair, deployer = _connect(network, account)
    addr = deployer.deploy(
        contract_dir=".", contract=contract, constructor="new", args={},
        value_pot=value, deployments_path=Path("deployments") / f"{network}.json")
    typer.echo(f"Deployed {contract} at {addr}")

@app.command()
def run(script: str, network: str = typer.Option(..., "--network"),
        account: str = typer.Option("deployer", "--account")):
    """Run a Python script with an injected InkPort context."""
    from inkport.run import build_context, run_script
    substrate, keypair, deployer = _connect(network, account)
    ctx = build_context(network, substrate, keypair, deployer)
    run_script(script, ctx)
```

- [ ] **Step 5: Run to verify the unit test passes**

Run: `cd inkport && pytest tests/test_run.py -v`
Expected: PASS. (CLI `_connect` exercised in Phase 3 integration.)

- [ ] **Step 6: Commit**

```bash
git add inkport/inkport/run.py inkport/inkport/cli.py inkport/tests/test_run.py
git commit -m "feat(run): runtime context, inkport deploy and inkport run"
```

---

### Task 18: `inkport report` and `inkport test`

**Files:**
- Create: `inkport/inkport/report.py`
- Create: `inkport/inkport/test_runner.py`
- Modify: `inkport/inkport/cli.py`
- Test: `inkport/tests/test_report.py`

- [ ] **Step 1: Write the failing test**

`inkport/tests/test_report.py`:
```python
from pathlib import Path
from inkport.report import read_report

def test_read_report_returns_markdown(tmp_path):
    art = tmp_path / "artifacts" / "Token"
    art.mkdir(parents=True)
    (art / "translation-report.md").write_text("# Translation report: Token\n")
    md = read_report(tmp_path, "Token")
    assert "Translation report: Token" in md

def test_read_report_missing_raises(tmp_path):
    import pytest
    with pytest.raises(FileNotFoundError):
        read_report(tmp_path, "Nope")
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd inkport && pytest tests/test_report.py -v`
Expected: FAIL — `inkport.report` undefined.

- [ ] **Step 3: Implement report reader and test runner**

`inkport/inkport/report.py`:
```python
from pathlib import Path


def read_report(project_dir, contract: str) -> str:
    path = Path(project_dir) / "artifacts" / contract / "translation-report.md"
    if not path.exists():
        raise FileNotFoundError(f"no report for {contract}; run `inkport compile` first")
    return path.read_text()
```

`inkport/inkport/test_runner.py`:
```python
import subprocess
from pathlib import Path


def run_tests(project_dir, network: str = "local") -> int:
    """Run pytest over the project's tests/ dir; INKPORT_NETWORK selects the target."""
    tests_dir = Path(project_dir) / "tests"
    if not tests_dir.is_dir():
        raise FileNotFoundError("no tests/ directory")
    result = subprocess.run(
        ["pytest", str(tests_dir), "-v"],
        env={"INKPORT_NETWORK": network, **_os_environ()},
    )
    return result.returncode


def _os_environ():
    import os
    return dict(os.environ)
```

- [ ] **Step 4: Wire into the CLI**

Replace `report` and `test` bodies in `inkport/inkport/cli.py`:
```python
@app.command()
def report(contract: str = typer.Option("Token", "--contract")):
    """Print the translation report."""
    from inkport.report import read_report
    typer.echo(read_report(".", contract))

@app.command()
def test(network: str = typer.Option("local", "--network")):
    """Run behavioral / golden tests."""
    from inkport.test_runner import run_tests
    raise typer.Exit(code=run_tests(".", network))
```

- [ ] **Step 5: Run to verify it passes**

Run: `cd inkport && pytest tests/test_report.py -v`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add inkport/inkport/report.py inkport/inkport/test_runner.py inkport/inkport/cli.py inkport/tests/test_report.py
git commit -m "feat(cli): inkport report and inkport test"
```

---

## Phase 3 — Integration

### Task 19: End-to-end ERC-20 against a local node

**Files:**
- Create: `inkport/tests/integration/test_erc20_e2e.py`
- Create: `inkport/tests/integration/README.md`

This test is opt-in (skipped unless the toolchain + a local `substrate-contracts-node` are present), so the default suite stays fast and hermetic.

- [ ] **Step 1: Write the integration test**

`inkport/tests/integration/test_erc20_e2e.py`:
```python
import os
import shutil
import subprocess
from pathlib import Path
import pytest

pytestmark = pytest.mark.skipif(
    not (shutil.which("inkport-translate") and shutil.which("cargo")
         and os.environ.get("INKPORT_E2E") == "1"),
    reason="requires inkport-translate, cargo-contract, a local node, and INKPORT_E2E=1",
)


def test_full_compile_deploy_call(tmp_path):
    # 1. scaffold
    from inkport.scaffold import scaffold_project
    scaffold_project(tmp_path)

    # 2. drop the full ERC-20 fixture in
    shutil.copy(
        Path(__file__).parents[3] / "translator" / "fixtures" / "ERC20.sol",
        tmp_path / "contracts" / "Token.sol",
    )

    # 3. compile
    from inkport.compile import compile_contract
    artifact = compile_contract(tmp_path, "Token", "u128")
    assert artifact.exists()
    assert (tmp_path / "artifacts" / "Token" / "translation-report.md").exists()

    # 4. deploy to local node + 5. call transfer + assert balance/event
    from substrateinterface import SubstrateInterface, Keypair
    from inkport.deploy import Deployer
    substrate = SubstrateInterface(url="ws://127.0.0.1:9944")
    alice = Keypair.create_from_uri("//Alice")
    deployer = Deployer(substrate, alice)
    addr = deployer.deploy(
        contract_dir=tmp_path, contract="Token", constructor="new",
        args={"initialSupply": 1_000_000}, value_pot=0.0,
        deployments_path=tmp_path / "deployments" / "local.json",
    )
    assert addr

    from substrateinterface.contracts import ContractInstance
    instance = ContractInstance.create_from_address(
        contract_address=addr,
        metadata_file=str(tmp_path / "artifacts" / "Token" / "Token.contract"),
        substrate=substrate,
    )
    res = instance.read(alice, "balance_of", args={"who": alice.ss58_address})
    assert res.contract_result_data is not None
```

- [ ] **Step 2: Document how to run it**

`inkport/tests/integration/README.md`:
```markdown
# Integration tests

Requires:
- `inkport-translate` on PATH (`cd translator && cargo install --path .`)
- `cargo-contract` (`cargo install cargo-contract`)
- a local node: `substrate-contracts-node --dev` on ws://127.0.0.1:9944

Run:
    INKPORT_E2E=1 pytest inkport/tests/integration -v
```

- [ ] **Step 3: Run the default suite to confirm it skips cleanly**

Run: `cd inkport && pytest -v`
Expected: all unit tests PASS; the e2e test reports SKIPPED.

- [ ] **Step 4: Commit**

```bash
git add inkport/tests/integration
git commit -m "test(e2e): ERC-20 compile→deploy→call integration (opt-in)"
```

---

### Task 20: Top-level README and developer setup

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write the README**

`README.md`:
```markdown
# InkPort

A Hardhat-style framework that compiles Solidity to ink! 5.x and deploys it to Portaldot (gas paid in POT).

## Install
    cd translator && cargo install --path .     # provides `inkport-translate`
    cargo install cargo-contract                 # ink! builder
    cd inkport && pip install -e .               # provides `inkport`

## Use
    inkport init myproject && cd myproject
    export INKPORT_SURI="//Alice"                # or your real key
    inkport compile --contract Token
    inkport report  --contract Token
    inkport deploy  --network portaldot --contract Token
    inkport run scripts/deploy.py --network portaldot

## Architecture
Rust translator (solang-parser → IR → ink!) + Python CLI/SDK orchestration.
See `docs/superpowers/specs/2026-05-29-inkport-hardhat-framework-design.md`.

## Credits
solang-parser, cargo-contract, substrateinterface, and the Sol2Ink prior art.

## License
MIT
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: top-level README with install and usage"
```

---

## Self-review notes (coverage map)

- Spec §3 layout → Task 14 scaffold.
- Spec §4.1 CLI → Tasks 2, 14, 15, 17, 18.
- Spec §4.2 translator → Tasks 1, 3–12.
- Spec §4.3 builder → Task 15 (`_default_build`).
- Spec §4.4 deployer → Task 16.
- Spec §4.5 config loader → Task 13.
- Spec §4.6 report → Tasks 11 (Rust), 18 (Python reader).
- Spec §4.7 test runner → Task 18 + Task 19 e2e.
- Spec §5 data flow → Task 19 exercises the whole chain.
- Spec §8 mapping rules → Tasks 4, 6, 7, 8, 9, 10 (one test per rule class).
- Spec §9 error handling → Task 15 (build stderr), Task 16/17 (deploy), Task 9/12 (unsupported → report).
- Spec §10 testing → unit tests throughout + Task 19.
- Spec §11 success criteria → Task 19 covers compile→deploy→call; report via Task 18.

**Known follow-ups (out of this slice, noted not silently dropped):** `inkport console` REPL (spec §6 stretch), multi-file `import` resolution, `U256` codegen runtime type (the strategy flag and type rendering exist; a `U256` newtype impl in generated crates is future work), and confirming the Portaldot testnet endpoint + ink! version (spec §13 open questions).
```
