use foundry_compilers::artifacts::ast::{ContractDefinition, SourceUnit};
use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};
use foundry_compilers::solc::Solc;

use super::{error::AstError, parser, stitcher, utils};
use crate::internal::settings;

pub(crate) struct AstOrchestrator;

impl AstOrchestrator {
  pub fn sanitize_settings(settings: Option<Settings>) -> Result<Settings, AstError> {
    let base = settings.unwrap_or_default();
    let sanitized =
      settings::sanitize_settings(&base).map_err(|err| AstError::ConfigError(err.to_string()))?;
    let mut sanitized = sanitized;
    sanitized.stop_after = Some("parsing".to_string());
    sanitized.output_selection = OutputSelection::ast_output_selection();
    sanitized.evm_version = None;
    Ok(sanitized)
  }

  pub fn parse_source_unit(
    source: &str,
    file_name: &str,
    solc: &Solc,
    settings: &Settings,
  ) -> Result<SourceUnit, AstError> {
    parser::parse_source_ast(source, file_name, solc, settings)
  }

  pub fn parse_fragment_contract(
    fragment_source: &str,
    solc: &Solc,
    settings: &Settings,
  ) -> Result<ContractDefinition, AstError> {
    parser::parse_fragment_contract(fragment_source, solc, settings)
  }

  pub fn extract_fragment_contract(unit: &SourceUnit) -> Result<ContractDefinition, AstError> {
    parser::extract_fragment_contract(unit).map(|contract| contract.clone())
  }

  pub fn stitch_fragment_into_contract(
    target: &mut SourceUnit,
    contract_idx: usize,
    fragment_contract: &ContractDefinition,
  ) -> Result<(), AstError> {
    let max_target_id = utils::max_id(target)?;
    stitcher::stitch_fragment_nodes_into_contract(
      target,
      contract_idx,
      fragment_contract,
      max_target_id,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ast::stitcher, internal::solc};
  use foundry_compilers::{
    artifacts::ast::{ContractDefinitionPart, SourceUnitPart},
    solc::Solc,
  };

  const MULTI_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {}
contract Target {
  uint256 internal value;
}
"#;

  const FRAGMENT: &str = "function expose() internal returns (uint256) { return value; }";

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn sanitize_settings_applies_ast_defaults() {
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize settings");
    assert_eq!(settings.stop_after.as_deref(), Some("parsing"));
    assert!(
      !settings.output_selection.as_ref().is_empty(),
      "expected output selection to include AST entries"
    );
    assert!(settings.evm_version.is_none());
  }

  #[test]
  fn stitches_fragment_through_service() {
    let Some(solc) = find_default_solc() else {
      return;
    };

    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let mut unit =
      AstOrchestrator::parse_source_unit(MULTI_CONTRACT, "Target.sol", &solc, &settings)
        .expect("parse source unit");
    let fragment =
      AstOrchestrator::parse_fragment_contract(FRAGMENT, &solc, &settings).expect("parse fragment");

    let idx = stitcher::find_instrumented_contract_index(&unit, Some("Target"))
      .expect("find target contract");

    AstOrchestrator::stitch_fragment_into_contract(&mut unit, idx, &fragment)
      .expect("stitch fragment");

    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("expected contract definition");
    };

    assert!(contract.nodes.iter().any(|part| matches!(part,
      ContractDefinitionPart::FunctionDefinition(function) if function.name == "expose"
    )));
  }
}
