#[cfg(test)]
mod tests {
  use crate::ast::{Ast, FragmentTarget, SourceTarget};
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

  fn find_function<'a>(unit: &'a Value, name: &str) -> Option<&'a Value> {
    let nodes = unit.get("nodes")?.as_array()?;
    for node in nodes {
      if node.get("nodeType")?.as_str()? != "ContractDefinition" {
        continue;
      }
      let contract_nodes = node.get("nodes")?.as_array()?;
      for member in contract_nodes {
        if member.get("nodeType")?.as_str()? == "FunctionDefinition"
          && member.get("name")?.as_str()? == name
        {
          return Some(member);
        }
      }
    }
    None
  }

  #[test]
  fn from_source_parses_contract_definition() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");
    let unit = ast.source_unit().expect("loaded ast");
    let json = unit.clone();
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

    let unit = ast.source_unit().expect("loaded ast");
    let json = unit.clone();
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

    let unit = ast.source_unit().expect("loaded ast");
    let json = unit.clone();
    assert!(json_contains_value(&json, "visibility", "public"));
  }

  #[test]
  fn validate_compiles_without_errors() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");
    ast.validate().expect("validate ast");
  }

  #[test]
  fn inject_shadow_at_edges_instruments_function() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");

    ast
      .inject_shadow_at_edges(
        "read()",
        &["require(true);".to_string()],
        &["require(true);".to_string()],
        None,
      )
      .expect("inject edges");

    let unit = ast.source_unit().expect("loaded ast");
    let function = find_function(unit, "read").expect("read function");
    let body = function
      .get("body")
      .and_then(|value| value.as_object())
      .expect("function body");

    let statements = body
      .get("statements")
      .and_then(|value| value.as_array())
      .expect("statements list");

    let mut expression_statements = 0;
    if let Some(first) = statements.first() {
      assert_eq!(
        first.get("nodeType").and_then(|value| value.as_str()),
        Some("ExpressionStatement"),
        "expected before statements to be prepended"
      );
    }

    for statement in statements {
      if statement.get("nodeType").and_then(|value| value.as_str()) == Some("ExpressionStatement") {
        expression_statements += 1;
      }
    }

    assert!(
      expression_statements >= 2,
      "expected instrumentation statements to be injected"
    );
  }

  #[test]
  fn inject_shadow_at_edges_requires_signature_when_ambiguous() {
    const AMBIGUOUS_CONTRACT: &str = r#"
pragma solidity ^0.8.13;

contract Ambiguous {
  function call(uint256 value) public pure returns (uint256) {
    return value;
  }

  function call(address target) public pure returns (address) {
    return target;
  }
}
"#;

    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(AMBIGUOUS_CONTRACT.into()), None)
      .expect("load source");

    let result = ast.inject_shadow_at_edges("call", &["uint256 __a = 1;".to_string()], &[], None);
    assert!(result.is_err(), "expected ambiguous name to error");
  }

  #[test]
  fn inject_shadow_at_edges_rejects_inline_assembly() {
    const ASSEMBLY_CONTRACT: &str = r#"
pragma solidity ^0.8.13;

contract WithAssembly {
  function useAsm(uint256 value) public pure returns (uint256 result) {
    assembly {
      result := add(value, 1)
    }
  }
}
"#;

    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(ASSEMBLY_CONTRACT.into()), None)
      .expect("load source");

    let result = ast.inject_shadow_at_edges(
      "useAsm(uint256)",
      &["uint256 __before = value;".to_string()],
      &[],
      None,
    );

    assert!(
      result.is_err(),
      "expected inline assembly instrumentation to fail"
    );
  }

  #[test]
  fn inject_shadow_at_edges_errors_on_missing_function() {
    let mut ast = Ast::new(None).expect("create ast");
    ast
      .from_source(SourceTarget::Text(SAMPLE_CONTRACT.into()), None)
      .expect("load source");

    let result =
      ast.inject_shadow_at_edges("missing()", &["uint256 __x = 0;".to_string()], &[], None);

    assert!(result.is_err(), "expected missing function to error");
  }
}
