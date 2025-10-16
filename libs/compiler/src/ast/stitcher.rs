use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, SourceUnit, SourceUnitPart,
};

use super::{error::AstError, utils};

pub fn find_instrumented_contract_index(
  unit: &SourceUnit,
  contract_name: Option<&str>,
) -> Result<usize, AstError> {
  let mut fallback: Option<usize> = None;

  for (idx, part) in unit.nodes.iter().enumerate() {
    let SourceUnitPart::ContractDefinition(contract) = part else {
      continue;
    };
    let name = &contract.name;
    if let Some(target) = contract_name {
      if name == target {
        return Ok(idx);
      }
    } else {
      fallback = Some(idx);
    }
  }

  contract_name
    .map(|name| {
      Err(AstError::InvalidContractStructure(format!(
        "Contract '{}' not found",
        name
      )))
    })
    .unwrap_or_else(|| {
      fallback.ok_or_else(|| {
        AstError::InvalidContractStructure("No ContractDefinition found".to_string())
      })
    })
}

pub fn stitch_fragment_nodes_into_contract(
  target: &mut SourceUnit,
  contract_idx: usize,
  fragment_contract: &ContractDefinition,
  max_target_id: i64,
) -> Result<(), AstError> {
  let SourceUnitPart::ContractDefinition(target_contract) = target
    .nodes
    .get_mut(contract_idx)
    .ok_or_else(|| AstError::InvalidContractStructure("Invalid contract index".to_string()))?
  else {
    return Err(AstError::InvalidContractStructure(
      "Target index is not a contract".to_string(),
    ));
  };

  let mut fragment = fragment_contract.clone();
  utils::renumber_contract_definition(&mut fragment, max_target_id)?;

  target_contract
    .nodes
    .extend(fragment.nodes.into_iter().map(resolve_contract_part));

  Ok(())
}

fn resolve_contract_part(part: ContractDefinitionPart) -> ContractDefinitionPart {
  part
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::{parser, utils, Ast};
  use crate::internal::solc;
  use foundry_compilers::solc::Solc;

  const MULTI_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {}
contract Second {}
contract Target {}
"#;

  const FRAGMENT: &str = "function hello() public {}";

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn locates_contract_by_name() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Ast::sanitize_settings(None);
    let unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, Some("Target")).unwrap();
    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };
    assert_eq!(contract.name, "Target");
  }

  #[test]
  fn falls_back_to_last_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Ast::sanitize_settings(None);
    let unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, None).unwrap();
    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };
    assert_eq!(contract.name, "Target");
  }

  #[test]
  fn stitches_fragment_into_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = Ast::sanitize_settings(None);
    let mut unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let fragment = parser::parse_fragment_contract(FRAGMENT, &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, Some("Target")).unwrap();
    let max_id = utils::max_id(&unit).unwrap();

    stitch_fragment_nodes_into_contract(&mut unit, idx, &fragment, max_id).unwrap();

    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };

    assert!(contract.nodes.iter().any(|part| matches!(part,
      ContractDefinitionPart::FunctionDefinition(function) if function.name == "hello"
    )));
  }
}
