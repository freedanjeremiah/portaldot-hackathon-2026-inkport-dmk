#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    U128,
    I128,
    U256,
    AccountId,
    String,
    Bytes,
    Mapping(Box<Type>, Box<Type>),
    /// Dynamic array `T[]`.
    Array(Box<Type>),
    /// A user-defined struct, referenced by name. Field layout is resolved in
    /// the seal0 backend from the parsed struct definitions.
    Struct(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mutability { View, Mutating, Payable }

#[derive(Debug, Clone)]
pub struct Field { pub name: String, pub ty: Type, pub public: bool }

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
    /// Return types in declaration order. Empty = no return.
    pub returns: Vec<Type>,
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
