#[cfg(test)]
mod tests {
  use super::super::{parser, stitcher, utils, Shadow};
  use serde_json::Value;

  const TARGET_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MyContract {
    uint256 private secretValue;

    function getSecret() public view returns (uint256) {
        return secretValue;
    }
}
"#;

  const SHADOW_FUNCTION: &str = r#"function exploit() public view returns (uint256) {
        return secretValue * 2;
    }"#;

  const SHADOW_VARIABLE: &str = r#"uint256 public exposedSecret;"#;

  const MULTI_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {
    uint256 public value;
}

contract Second {
    string public name;
}

contract Target {
    uint256 private secret;
}
"#;

  #[test]
  fn test_shadow_creation() {
    let _shadow = Shadow::new(SHADOW_FUNCTION.to_string());
    // Shadow created successfully
  }

  #[test]
  fn test_wrap_shadow_source() {
    let wrapped = parser::wrap_shadow_source(SHADOW_FUNCTION);
    assert!(wrapped.contains("pragma solidity"));
    assert!(wrapped.contains("contract Shadow"));
    assert!(wrapped.contains(SHADOW_FUNCTION));
  }

  #[test]
  fn test_parse_source_ast() {
    let ast =
      parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse contract");

    assert!(ast.is_object());
    assert!(ast.get("nodes").is_some());
  }

  #[test]
  fn test_to_ast_nodes() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());
    let nodes = shadow.to_ast_nodes().expect("Failed to extract nodes");

    assert!(!nodes.is_empty(), "Should have at least one node");

    let first_node: Value = serde_json::from_str(&nodes[0]).expect("Node should be valid JSON");

    assert_eq!(
      first_node.get("nodeType").and_then(|v| v.as_str()),
      Some("FunctionDefinition")
    );
  }

  #[test]
  fn test_find_max_id() {
    let ast = parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse");

    let max_id = utils::find_max_id(&ast);
    assert!(max_id > 0, "Should find IDs in AST");
  }

  #[test]
  fn test_renumber_ids() {
    let mut ast = parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse");

    let original_max = utils::find_max_id(&ast);
    utils::renumber_ids(&mut ast, 1000);
    let new_max = utils::find_max_id(&ast);

    assert!(new_max > original_max + 900, "IDs should be renumbered");
  }

  #[test]
  fn test_is_contract_definition() {
    let ast = parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse");

    let nodes = ast
      .get("nodes")
      .and_then(|v| v.as_array())
      .expect("Should have nodes array");

    let has_contract = nodes.iter().any(utils::is_contract_definition);
    assert!(has_contract, "Should find ContractDefinition node");
  }

  #[test]
  fn test_get_contract_name() {
    let ast = parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse");

    let nodes = ast
      .get("nodes")
      .and_then(|v| v.as_array())
      .expect("Should have nodes array");

    let contract = nodes
      .iter()
      .find(|n| utils::is_contract_definition(n))
      .expect("Should find contract");

    let name = utils::get_contract_name(contract);
    assert_eq!(name, Some("MyContract"));
  }

  #[test]
  fn test_find_target_contract_index_by_name() {
    let ast = parser::parse_source_ast(MULTI_CONTRACT, "Test.sol").expect("Failed to parse");

    let idx = stitcher::find_target_contract_index(&ast, Some("Target"))
      .expect("Should find Target contract");

    let nodes = ast.get("nodes").and_then(|v| v.as_array()).unwrap();
    let contract = &nodes[idx];

    assert_eq!(utils::get_contract_name(contract), Some("Target"));
  }

  #[test]
  fn test_find_target_contract_index_last() {
    let ast = parser::parse_source_ast(MULTI_CONTRACT, "Test.sol").expect("Failed to parse");

    let idx = stitcher::find_target_contract_index(&ast, None).expect("Should find last contract");

    let nodes = ast.get("nodes").and_then(|v| v.as_array()).unwrap();
    let contract = &nodes[idx];

    assert_eq!(utils::get_contract_name(contract), Some("Target"));
  }

  #[test]
  fn test_stitch_into_source() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());

    let result = shadow.stitch_into_source(TARGET_CONTRACT.to_string(), None, None);

    assert!(result.is_ok(), "Stitching should succeed");

    let analyzed_ast: Value =
      serde_json::from_str(&result.unwrap()).expect("Result should be valid JSON");

    assert!(analyzed_ast.is_object());
    assert!(analyzed_ast.get("nodes").is_some());
  }

  #[test]
  fn test_stitch_into_specific_contract() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());

    let result =
      shadow.stitch_into_source(MULTI_CONTRACT.to_string(), None, Some("Target".to_string()));

    assert!(
      result.is_ok(),
      "Stitching into Target should succeed: {:?}",
      result
    );
  }

  #[test]
  fn test_stitch_into_ast() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());

    let target_ast =
      parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse target");
    let target_ast_json = serde_json::to_string(&target_ast).expect("Failed to serialize AST");

    let result = shadow.stitch_into_ast(target_ast_json, None, None);

    assert!(result.is_ok(), "Stitching into AST should succeed");
  }

  #[test]
  fn test_stitch_variable() {
    let shadow = Shadow::new(SHADOW_VARIABLE.to_string());

    let result = shadow.stitch_into_source(TARGET_CONTRACT.to_string(), None, None);

    assert!(result.is_ok(), "Stitching variable should succeed");

    let analyzed_ast: Value =
      serde_json::from_str(&result.unwrap()).expect("Result should be valid JSON");

    assert!(analyzed_ast.is_object());
  }

  #[test]
  fn test_multiple_shadow_nodes() {
    let multi_shadow = format!("{}\n{}", SHADOW_FUNCTION, SHADOW_VARIABLE);
    let shadow = Shadow::new(multi_shadow);

    let nodes = shadow
      .to_ast_nodes()
      .expect("Should extract multiple nodes");

    assert!(nodes.len() >= 2, "Should have at least 2 nodes");
  }

  #[test]
  fn test_parse_source_ast_static() {
    let result = Shadow::parse_source_ast_static(TARGET_CONTRACT.to_string(), None);

    assert!(result.is_ok(), "Static parsing should succeed");

    let ast: Value = serde_json::from_str(&result.unwrap()).expect("Should be valid JSON");

    assert!(ast.is_object());
    assert!(ast.get("nodes").is_some());
  }

  #[test]
  fn test_stitch_preserves_target_structure() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());

    let target_ast =
      parser::parse_source_ast(TARGET_CONTRACT, "Test.sol").expect("Failed to parse target");

    let original_max_id = utils::find_max_id(&target_ast);

    let result = shadow.stitch_into_source(TARGET_CONTRACT.to_string(), None, None);

    assert!(result.is_ok());

    let analyzed_ast: Value = serde_json::from_str(&result.unwrap()).expect("Should be valid JSON");

    let new_max_id = utils::find_max_id(&analyzed_ast);
    assert!(
      new_max_id > original_max_id,
      "Should have new IDs from shadow"
    );
  }

  #[test]
  fn test_error_invalid_contract_name() {
    let shadow = Shadow::new(SHADOW_FUNCTION.to_string());

    let result = shadow.stitch_into_source(
      TARGET_CONTRACT.to_string(),
      None,
      Some("NonExistent".to_string()),
    );

    assert!(result.is_err(), "Should fail for non-existent contract");
  }
}
