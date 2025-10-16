use foundry_compilers::artifacts::{Settings, SolcInput, SolcLanguage, Source, Sources};
use foundry_compilers::solc::Solc;
use serde_json::Value;
use std::path::PathBuf;

use super::error::ShadowError;

/// Parse Solidity source code and return AST JSON
/// Uses stopAfter: "parsing" to get syntax-only AST
pub fn parse_source_ast(
  source: &str,
  file_name: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<Value, ShadowError> {
  let mut sources = Sources::new();
  sources.insert(PathBuf::from(file_name), Source::new(source));

  let mut input = SolcInput::new(SolcLanguage::Solidity, sources, settings.clone());
  input.sanitize(&solc.version);

  let parse_output: Value = solc
    .compile_as(&input)
    .map_err(|e| ShadowError::CompilerError(e.to_string()))?;

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
  use crate::{internal::solc, shadow::Shadow};
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

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn parses_contract_to_ast_value() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Shadow::sanitize_settings(None);
    let ast: Value = parse_source_ast(SAMPLE_CONTRACT, "Example.sol", &solc, &settings)
      .expect("should parse contract");
    assert!(ast.get("nodes").is_some(), "AST should contain nodes array");
    let nodes = ast
      .get("nodes")
      .and_then(|n| n.as_array())
      .expect("nodes should be array");
    assert!(!nodes.is_empty());
  }

  #[test]
  fn produces_typed_source_unit() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Shadow::sanitize_settings(None);
    let ast =
      parse_source_ast(SAMPLE_CONTRACT, "Example.sol", &solc, &settings).expect("parse contract");
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
