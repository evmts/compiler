use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, SourceUnit, SourceUnitPart, Visibility,
};
use foundry_compilers::solc::SolcLanguage;

use super::{orchestrator::AstOrchestrator, stitcher};
use crate::internal::config::{AstOptions, SolcConfig};
use crate::internal::errors::{map_err_with_context, Error, Result};
use crate::internal::solc;

const VIRTUAL_SOURCE_PATH: &str = "__VIRTUAL__.sol";

#[derive(Clone)]
pub struct State {
  pub config: SolcConfig,
  pub ast: Option<SourceUnit>,
  pub options: AstOptions,
}

#[derive(Clone)]
pub enum SourceTarget {
  Text(String),
  Ast(SourceUnit),
}

#[derive(Clone)]
pub enum FragmentTarget {
  Text(String),
  Ast(SourceUnit),
}

pub fn init(options: Option<AstOptions>) -> Result<State> {
  let default_settings = AstOrchestrator::sanitize_settings(None).map_err(Error::from)?;
  let default_language = solc::default_language();
  let mut config =
    SolcConfig::new(&default_language, &default_settings, options.as_ref()).map_err(Error::from)?;
  config.settings =
    AstOrchestrator::sanitize_settings(Some(config.settings.clone())).map_err(Error::from)?;
  if config.language != SolcLanguage::Solidity {
    return Err(Error::new(
      "Ast helpers only support solcLanguage \"Solidity\".",
    ));
  }
  solc::ensure_installed(&config.version)?;

  Ok(State {
    config,
    ast: None,
    options: options.unwrap_or_default(),
  })
}

pub fn from_source(
  state: &mut State,
  target: SourceTarget,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  match target {
    SourceTarget::Text(source) => load_source_text(state, &source, overrides)?,
    SourceTarget::Ast(unit) => load_source_ast(state, unit, overrides)?,
  }
  Ok(())
}

pub fn inject_shadow(
  state: &mut State,
  fragment: FragmentTarget,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  match fragment {
    FragmentTarget::Text(source) => inject_fragment_string(state, &source, overrides)?,
    FragmentTarget::Ast(unit) => inject_fragment_ast(state, unit, overrides)?,
  }
  Ok(())
}

pub fn expose_internal_variables(state: &mut State, overrides: Option<&AstOptions>) -> Result<()> {
  expose_variables_internal(state, overrides)
}

pub fn expose_internal_functions(state: &mut State, overrides: Option<&AstOptions>) -> Result<()> {
  expose_functions_internal(state, overrides)
}

pub fn source_unit(state: &State) -> Option<&SourceUnit> {
  state.ast.as_ref()
}

pub fn source_unit_mut(state: &mut State) -> Option<&mut SourceUnit> {
  state.ast.as_mut()
}

fn contract_override<'a>(state: &'a State, overrides: Option<&'a AstOptions>) -> Option<&'a str> {
  overrides
    .and_then(|opts| opts.instrumented_contract())
    .or_else(|| state.options.instrumented_contract())
}

fn update_options(state: &mut State, overrides: Option<&AstOptions>) {
  if let Some(opts) = overrides {
    state.options = opts.clone();
  }
}

fn resolve_config(state: &State, overrides: Option<&AstOptions>) -> Result<SolcConfig> {
  let mut config = state.config.merge(overrides).map_err(Error::from)?;
  if config.language != SolcLanguage::Solidity {
    return Err(Error::new(
      "Ast helpers only support solcLanguage \"Solidity\".",
    ));
  }
  config.settings = map_err_with_context(
    AstOrchestrator::sanitize_settings(Some(config.settings.clone())),
    "Failed to sanitize compiler settings",
  )?;
  Ok(config)
}

fn target_ast_mut(state: &mut State) -> Result<&mut SourceUnit> {
  state
    .ast
    .as_mut()
    .ok_or_else(|| Error::new("Ast has no target AST. Call from_source first."))
}

fn target_ast(state: &State) -> Result<&SourceUnit> {
  state
    .ast
    .as_ref()
    .ok_or_else(|| Error::new("Ast has no target AST. Call from_source first."))
}

fn find_contract_index(
  state: &State,
  ast: &SourceUnit,
  contract_name: Option<&str>,
) -> Result<usize> {
  map_err_with_context(
    stitcher::find_instrumented_contract_index(
      ast,
      contract_name.or_else(|| contract_override(state, None)),
    ),
    "Failed to locate target contract",
  )
}

fn inject_fragment_contract(
  state: &mut State,
  fragment_contract: ContractDefinition,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  let contract_name = contract_override(state, overrides).map(|name| name.to_owned());
  let contract_idx = {
    let target_ast = target_ast(state)?;
    find_contract_index(state, target_ast, contract_name.as_deref())?
  };

  let target_ast = target_ast_mut(state)?;
  map_err_with_context(
    AstOrchestrator::stitch_fragment_into_contract(target_ast, contract_idx, &fragment_contract),
    "Failed to stitch AST nodes",
  )
}

fn contract_indices(
  state: &State,
  ast: &SourceUnit,
  overrides: Option<&AstOptions>,
) -> Result<Vec<usize>> {
  if let Some(name) = contract_override(state, overrides) {
    let idx = stitcher::find_instrumented_contract_index(ast, Some(name))?;
    Ok(vec![idx])
  } else {
    let indices = ast
      .nodes
      .iter()
      .enumerate()
      .filter_map(|(idx, part)| {
        matches!(part, SourceUnitPart::ContractDefinition(_)).then_some(idx)
      })
      .collect::<Vec<_>>();

    if indices.is_empty() {
      Err(Error::new(
        "Target AST does not contain any contract definitions",
      ))
    } else {
      Ok(indices)
    }
  }
}

fn mutate_contracts<F>(
  state: &mut State,
  overrides: Option<&AstOptions>,
  mut mutator: F,
) -> Result<()>
where
  F: FnMut(&mut ContractDefinition),
{
  update_options(state, overrides);
  let indices = {
    let unit = target_ast(state)?;
    contract_indices(state, unit, overrides)?
  };
  let unit = target_ast_mut(state)?;
  for idx in indices {
    let SourceUnitPart::ContractDefinition(contract) = unit
      .nodes
      .get_mut(idx)
      .ok_or_else(|| Error::new("Invalid contract index"))?
    else {
      continue;
    };
    mutator(contract);
  }
  Ok(())
}

fn expose_variables_internal(state: &mut State, overrides: Option<&AstOptions>) -> Result<()> {
  mutate_contracts(state, overrides, |contract| {
    for member in &mut contract.nodes {
      if let ContractDefinitionPart::VariableDeclaration(variable) = member {
        if matches!(
          variable.visibility,
          Visibility::Private | Visibility::Internal
        ) {
          variable.visibility = Visibility::Public;
        }
      }
    }
  })
}

fn expose_functions_internal(state: &mut State, overrides: Option<&AstOptions>) -> Result<()> {
  mutate_contracts(state, overrides, |contract| {
    for member in &mut contract.nodes {
      if let ContractDefinitionPart::FunctionDefinition(function) = member {
        if matches!(
          function.visibility,
          Visibility::Private | Visibility::Internal
        ) {
          function.visibility = Visibility::Public;
        }
      }
    }
  })
}

fn load_source_text(state: &mut State, source: &str, overrides: Option<&AstOptions>) -> Result<()> {
  update_options(state, overrides);
  let config = resolve_config(state, overrides)?;
  let solc = solc::ensure_installed(&config.version)?;

  let ast = map_err_with_context(
    AstOrchestrator::parse_source_unit(source, VIRTUAL_SOURCE_PATH, &solc, &config.settings),
    "Failed to parse target source",
  )?;

  state.config = config;
  state.ast = Some(ast);
  Ok(())
}

fn load_source_ast(
  state: &mut State,
  target_ast: SourceUnit,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  update_options(state, overrides);
  let config = resolve_config(state, overrides)?;
  solc::ensure_installed(&config.version)?;

  map_err_with_context(
    stitcher::find_instrumented_contract_index(&target_ast, contract_override(state, overrides)),
    "Failed to locate target contract",
  )?;

  state.config = config;
  state.ast = Some(target_ast);
  Ok(())
}

fn inject_fragment_string(
  state: &mut State,
  fragment_source: &str,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  let solc = solc::ensure_installed(&config.version)?;

  let fragment_contract = map_err_with_context(
    AstOrchestrator::parse_fragment_contract(fragment_source, &solc, &config.settings),
    "Failed to parse AST fragment",
  )?;

  state.config = config;
  inject_fragment_contract(state, fragment_contract, overrides)
}

fn inject_fragment_ast(
  state: &mut State,
  fragment_ast: SourceUnit,
  overrides: Option<&AstOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  solc::ensure_installed(&config.version)?;
  state.config = config;

  let fragment_contract = map_err_with_context(
    AstOrchestrator::extract_fragment_contract(&fragment_ast),
    "Failed to locate fragment contract",
  )?;

  inject_fragment_contract(state, fragment_contract, overrides)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::utils;
  use crate::internal::config::{AstOptions, SolcConfig};
  use crate::internal::solc;
  use foundry_compilers::artifacts::CompilerOutput;
  use foundry_compilers::solc::Solc;
  use serde_json::{json, Value};

  const INSTRUMENTED_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Target {
  uint256 private value;
  function callMe() internal view returns (uint256) {
    return value;
  }
}
"#;

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn parses_and_injects_fragment() {
    if find_default_solc().is_none() {
      return;
    }

    let default_settings =
      AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = AstOrchestrator::sanitize_settings(Some(config.settings.clone()))
      .expect("sanitize config settings");
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut state = init(None).expect("init ast");

    from_source(
      &mut state,
      SourceTarget::Text(INSTRUMENTED_CONTRACT.into()),
      None,
    )
    .expect("load source");

    let overrides = AstOptions {
      solc_version: None,
      solc_language: None,
      solc_settings: None,
      instrumented_contract: Some("Target".into()),
    };

    inject_shadow(
      &mut state,
      FragmentTarget::Text(
        "function extra() public view returns (uint256) { return value; }".into(),
      ),
      Some(&overrides),
    )
    .expect("inject fragment");

    let ast = source_unit(&state).expect("ast");
    let contract = ast
      .nodes
      .iter()
      .filter_map(|part| match part {
        SourceUnitPart::ContractDefinition(contract) => Some(contract.as_ref()),
        _ => None,
      })
      .last()
      .expect("contract node");

    assert!(contract.nodes.iter().any(|part| matches!(part,
      ContractDefinitionPart::FunctionDefinition(function) if function.name == "extra"
    )));

    fn collect_ids(value: &Value, out: &mut Vec<i64>) {
      match value {
        Value::Object(map) => {
          if let Some(Value::Number(id)) = map.get("id") {
            if let Some(id) = id.as_i64() {
              out.push(id);
            }
          }
          map.values().for_each(|child| collect_ids(child, out));
        }
        Value::Array(items) => items.iter().for_each(|child| collect_ids(child, out)),
        _ => {}
      }
    }

    let mut ids = Vec::new();
    collect_ids(&serde_json::to_value(ast).expect("serialize ast"), &mut ids);
    let unique = ids
      .iter()
      .copied()
      .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), unique.len());
  }

  #[test]
  fn exposes_internal_members() {
    if find_default_solc().is_none() {
      return;
    }
    let default_settings =
      AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = AstOrchestrator::sanitize_settings(Some(config.settings.clone()))
      .expect("sanitize config settings");
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut state = init(None).expect("init ast");

    from_source(
      &mut state,
      SourceTarget::Text(INSTRUMENTED_CONTRACT.into()),
      None,
    )
    .expect("load source");
    let overrides = AstOptions {
      solc_version: None,
      solc_language: None,
      solc_settings: None,
      instrumented_contract: Some("Target".into()),
    };
    expose_internal_variables(&mut state, Some(&overrides)).expect("expose vars");
    expose_internal_functions(&mut state, Some(&overrides)).expect("expose funcs");

    let ast = source_unit(&state).expect("ast");
    let contract = ast
      .nodes
      .iter()
      .filter_map(|part| match part {
        SourceUnitPart::ContractDefinition(contract) => Some(contract.as_ref()),
        _ => None,
      })
      .last()
      .expect("contract node");

    let variable_visibility = contract.nodes.iter().find_map(|part| match part {
      ContractDefinitionPart::VariableDeclaration(variable) => Some(variable.visibility.clone()),
      _ => None,
    });

    assert_eq!(variable_visibility, Some(Visibility::Public));

    let function_visibility = contract.nodes.iter().find_map(|part| match part {
      ContractDefinitionPart::FunctionDefinition(function) => Some(function.visibility.clone()),
      _ => None,
    });

    assert_eq!(function_visibility, Some(Visibility::Public));
  }

  #[test]
  fn ast_round_trip() {
    let Some(solc) = find_default_solc() else {
      return;
    };

    let default_settings =
      AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = AstOrchestrator::sanitize_settings(Some(config.settings.clone()))
      .expect("sanitize config settings");
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut state = init(None).expect("init ast");
    from_source(
      &mut state,
      SourceTarget::Text(INSTRUMENTED_CONTRACT.into()),
      None,
    )
    .expect("load source");
    expose_internal_variables(&mut state, None).expect("expose vars");
    expose_internal_functions(&mut state, None).expect("expose funcs");

    let ast = source_unit(&state).expect("ast");
    let mut ast_value = serde_json::to_value(ast).expect("serialize ast");
    utils::sanitize_ast_value(&mut ast_value);

    let settings_value = serde_json::to_value(&state.config.settings).expect("serialize settings");

    let input = json!({
      "language": "SolidityAST",
      "sources": {
        VIRTUAL_SOURCE_PATH: {
          "ast": ast_value
        }
      },
      "settings": settings_value
    });

    let output: CompilerOutput = solc
      .compile_as(&input)
      .expect("round-trip compilation attempt");

    assert!(
      output.errors.is_empty(),
      "expected solc to compile ast without errors, but got errors: {:?}",
      output.errors
    );
  }
}
