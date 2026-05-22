use solang_parser::pt::SourceUnitPart;

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
