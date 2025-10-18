#[cfg(test)]
mod tests {
  use crate::ast::{Ast, FragmentTarget, SourceTarget, SourceUnit};
  use serde_json::Value;

  const SAMPLE_CONTRACT: &str = r#"
pragma solidity ^0.8.13;

contract Sample {
  uint256 internal stored;

  function read() internal view returns (uint256) {
    return stored;
  }
}
"#;

  const SHADOW_FRAGMENT: &str =
    r#"function expose() external view returns (uint256) { return stored; }"#;

  fn to_json(unit: &SourceUnit) -> Value {
    serde_json::to_value(unit).expect("serialize source unit")
  }

  fn contains_contract(unit: &Value, name: &str) -> bool {
    unit["nodes"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|node| node.as_object())
      .any(|node| node.get("name").and_then(Value::as_str) == Some(name))
  }

  fn json_contains_value(value: &Value, key: &str, expected: &str) -> bool {
    match value {
      Value::Object(map) => {
        if map
          .get(key)
          .and_then(Value::as_str)
          .map(|value| value == expected)
          .unwrap_or(false)
        {
          return true;
        }
        map
          .values()
          .any(|child| json_contains_value(child, key, expected))
      }
      Value::Array(items) => items
        .iter()
        .any(|child| json_contains_value(child, key, expected)),
      _ => false,
    }
  }

  #[test]
  fn from_source_parses_contract_definition() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");
    let unit = ast.ast().expect("loaded ast");
    let json = to_json(unit);
    assert!(contains_contract(&json, "Sample"));
  }

  #[test]
  fn inject_shadow_adds_fragment_functions() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");
    ast
      .inject_shadow(FragmentTarget::Text(SHADOW_FRAGMENT.into()), None)
      .expect("inject fragment");

    let unit = ast.ast().expect("loaded ast");
    let json = to_json(unit);
    assert!(json_contains_value(&json, "name", "expose"));
  }

  #[test]
  fn expose_internal_members_updates_visibility() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");

    ast
      .expose_internal_variables(None)
      .expect("expose variables");
    ast
      .expose_internal_functions(None)
      .expect("expose functions");

    let unit = ast.ast().expect("loaded ast");
    let json = to_json(unit);
    assert!(json_contains_value(&json, "visibility", "public"));
  }

  #[test]
  fn validate_compiles_without_errors() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");
    ast.validate(None).expect("validate ast");
  }
}
