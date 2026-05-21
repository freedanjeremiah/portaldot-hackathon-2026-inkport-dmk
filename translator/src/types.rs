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
