use foundry_compilers::{
  artifacts::{output_selection::OutputSelection, Settings, SolcLanguage, Source, Sources},
  compilers::solc::Solc,
};
use semver::Version;
use serde_json::Value;
use std::path::PathBuf;

use super::error::ShadowError;

/// Parse Solidity source code and return AST JSON
/// Uses stopAfter: "parsing" to get syntax-only AST
pub fn parse_source_ast(source: &str, file_name: &str) -> Result<Value, ShadowError> {
  let version = Version::parse("0.8.30")
    .map_err(|e| ShadowError::CompilerError(format!("Invalid version: {}", e)))?;

  let solc = Solc::find_svm_installed_version(&version)
    .map_err(|e| ShadowError::CompilerError(format!("Failed to find solc: {}", e)))?
    .ok_or_else(|| ShadowError::CompilerError("Solc 0.8.30 not found".to_string()))?;

  let mut sources = Sources::new();
  sources.insert(PathBuf::from(file_name), Source::new(source));

  let mut settings = Settings::default();
  settings.stop_after = Some("parsing".to_string());
  settings.output_selection = OutputSelection::ast_output_selection();
  settings.evm_version = None;

  let parse_input = foundry_compilers::artifacts::SolcInput {
    language: SolcLanguage::Solidity,
    sources,
    settings,
  };

  let parse_output: Value = solc.compile_as(&parse_input)?;

  let ast = parse_output
    .get("sources")
    .and_then(|s| s.get(file_name))
    .and_then(|s| s.get("ast"))
    .ok_or_else(|| ShadowError::ParseFailed("Failed to extract AST".to_string()))?;

  Ok(ast.clone())
}

/// Wrap shadow source in minimal contract boilerplate
pub fn wrap_shadow_source(source: &str) -> String {
  format!(
    r#"// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract Shadow {{
    {}
}}
"#,
    source
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::Value;

  const SAMPLE_FRAGMENT: &str = r#"function demo() public pure returns (uint256) { return 1; }"#;
  const SAMPLE_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Example {
  uint256 public value;
}
"#;

  #[test]
  fn wraps_fragment_in_shadow_contract() {
    let wrapped = wrap_shadow_source(SAMPLE_FRAGMENT);
    assert!(wrapped.contains("pragma solidity ^0.8.0;"));
    assert!(wrapped.contains("contract Shadow"));
    assert!(wrapped.contains(SAMPLE_FRAGMENT));
  }

  #[test]
  fn parses_contract_to_ast_value() {
    let ast: Value =
      parse_source_ast(SAMPLE_CONTRACT, "Example.sol").expect("should parse contract");
    assert!(ast.get("nodes").is_some(), "AST should contain nodes array");
    let nodes = ast
      .get("nodes")
      .and_then(|n| n.as_array())
      .expect("nodes should be array");
    assert!(!nodes.is_empty());
  }

  #[test]
  fn produces_typed_source_unit() {
    let ast = parse_source_ast(SAMPLE_CONTRACT, "Example.sol").expect("parse contract");
    let unit: foundry_compilers::artifacts::ast::SourceUnit =
      serde_json::from_value(ast).expect("deserialize SourceUnit");
    assert!(
      unit.nodes.iter().any(|n| matches!(
        n,
        foundry_compilers::artifacts::ast::SourceUnitPart::ContractDefinition(_)
      )),
      "typed AST should contain contract definition"
    );
  }
}
