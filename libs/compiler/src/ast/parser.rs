use std::path::PathBuf;

use foundry_compilers::artifacts::ast::{ContractDefinition, SourceUnit, SourceUnitPart};
use foundry_compilers::artifacts::{Settings, SolcInput, SolcLanguage, Source, Sources};
use foundry_compilers::solc::Solc;

use super::error::AstError;

// TODO: remove in favor of compile_source with correct settings once we add ast to output
pub fn parse_source_ast(
  source: &str,
  file_name: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<SourceUnit, AstError> {
  let mut sources = Sources::new();
  sources.insert(PathBuf::from(file_name), Source::new(source));

  let mut input = SolcInput::new(SolcLanguage::Solidity, sources, settings.clone());
  input.sanitize(&solc.version);

  let compiler_output: serde_json::Value = solc
    .compile_as::<SolcInput, _>(&input)
    .map_err(|err| AstError::CompilerError(err.to_string()))?;

  let ast_value = compiler_output
    .get("sources")
    .and_then(|sources| sources.get(file_name))
    .and_then(|entry| entry.get("ast"))
    .ok_or_else(|| AstError::ParseFailed("Failed to extract AST".to_string()))?
    .clone();

  serde_json::from_value(ast_value).map_err(|err| AstError::JsonError(err.to_string()))
}

pub fn wrap_fragment_source(source: &str) -> String {
  format!(
    r#"// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract __AstFragment {{
    {}
}}
"#,
    source
  )
}

pub fn parse_fragment_contract(
  fragment_source: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<ContractDefinition, AstError> {
  let unit = parse_source_ast(
    &wrap_fragment_source(fragment_source),
    "__AstFragment.sol",
    solc,
    settings,
  )?;
  extract_fragment_contract(&unit).cloned()
}

pub fn extract_fragment_contract(unit: &SourceUnit) -> Result<&ContractDefinition, AstError> {
  unit
    .nodes
    .iter()
    .filter_map(|part| match part {
      SourceUnitPart::ContractDefinition(contract) => Some(contract.as_ref()),
      _ => None,
    })
    .find(|contract| contract.name == "__AstFragment")
    .or_else(|| {
      unit.nodes.iter().rev().find_map(|part| match part {
        SourceUnitPart::ContractDefinition(contract) => Some(contract.as_ref()),
        _ => None,
      })
    })
    .ok_or_else(|| AstError::ParseFailed("Fragment contract not found".to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ast::Ast, internal::solc};
  use foundry_compilers::artifacts::ast::ContractDefinitionPart;

  const SAMPLE_FRAGMENT: &str = r#"function demo() public pure returns (uint256) { return 1; }"#;
  const SAMPLE_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Example {
  uint256 public value;
}
"#;

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn wraps_fragment_in_shadow_contract() {
    let wrapped = wrap_fragment_source(SAMPLE_FRAGMENT);
    assert!(wrapped.contains("pragma solidity ^0.8.0;"));
    assert!(wrapped.contains("contract __AstFragment"));
    assert!(wrapped.contains(SAMPLE_FRAGMENT));
  }

  #[test]
  fn parses_contract_to_ast_value() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Ast::sanitize_settings(None);
    let ast = parse_source_ast(SAMPLE_CONTRACT, "Example.sol", &solc, &settings)
      .expect("should parse contract");
    assert!(ast
      .nodes
      .iter()
      .any(|part| matches!(part, SourceUnitPart::ContractDefinition(_))));
  }

  #[test]
  fn parses_fragment_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Ast::sanitize_settings(None);
    let contract =
      parse_fragment_contract(SAMPLE_FRAGMENT, &solc, &settings).expect("parse fragment");
    assert_eq!(contract.name, "__AstFragment");
    assert!(contract
      .nodes
      .iter()
      .any(|part| matches!(part, ContractDefinitionPart::FunctionDefinition(_))));
  }
}
