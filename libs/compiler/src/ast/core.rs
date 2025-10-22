use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, SourceUnit, SourceUnitPart, Visibility,
};
use foundry_compilers::solc::SolcLanguage;

use super::{orchestrator::AstOrchestrator, stitcher, utils};
use crate::internal::config::{AstConfig, AstConfigOptions, ResolveConflictStrategy};
use crate::internal::errors::{map_err_with_context, Error, Result};
use crate::internal::logging::{ensure_rust_logger, update_level};
use crate::internal::solc;
use log::{error, info};
use serde_json::{json, Value};

const VIRTUAL_SOURCE_PATH: &str = "__VIRTUAL__.sol";
const LOG_TARGET: &str = "tevm::ast";

#[derive(Clone)]
pub struct State {
  pub config: AstConfig,
  pub ast: Option<SourceUnit>,
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

pub fn init(options: Option<AstConfigOptions>) -> Result<State> {
  let default_settings = AstOrchestrator::sanitize_settings(None).map_err(Error::from)?;
  let default_language = solc::default_language();
  let mut config = AstConfig::from_options(&default_language, &default_settings, options.as_ref())
    .map_err(Error::from)?;
  ensure_rust_logger(config.logging_level)?;
  info!(target: LOG_TARGET, "initialising AST state with language {:?}", default_language);
  config.solc.settings =
    AstOrchestrator::sanitize_settings(Some(config.solc.settings.clone())).map_err(Error::from)?;
  if config.solc.language != SolcLanguage::Solidity {
    error!(target: LOG_TARGET, "Ast helpers only support solcLanguage \"Solidity\"");
    return Err(Error::new(
      "Ast helpers only support solcLanguage \"Solidity\".",
    ));
  }
  solc::ensure_installed(&config.solc.version)?;
  info!(
    target: LOG_TARGET,
    "AST ready (instrumented_contract={:?})",
    config.instrumented_contract()
  );

  Ok(State { config, ast: None })
}

pub fn from_source(
  state: &mut State,
  target: SourceTarget,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  match target {
    SourceTarget::Text(source) => {
      info!(
        target: LOG_TARGET,
        "loading AST from source text (len={})",
        source.len()
      );
      load_source_text(state, &source, overrides)?;
    }
    SourceTarget::Ast(unit) => {
      let node_count = unit.nodes.len();
      info!(
        target: LOG_TARGET,
        "loading AST from pre-built unit (nodes={})",
        node_count
      );
      load_source_ast(state, unit, overrides)?;
    }
  }
  info!(target: LOG_TARGET, "AST source loaded");
  Ok(())
}

pub fn inject_shadow(
  state: &mut State,
  fragment: FragmentTarget,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  match fragment {
    FragmentTarget::Text(source) => {
      info!(
        target: LOG_TARGET,
        "injecting AST fragment from shadow source (len={})",
        source.len()
      );
      inject_fragment_string(state, &source, overrides)?;
    }
    FragmentTarget::Ast(unit) => {
      let node_count = unit.nodes.len();
      info!(
        target: LOG_TARGET,
        "injecting pre-built AST fragment (nodes={})",
        node_count
      );
      inject_fragment_ast(state, unit, overrides)?;
    }
  }
  info!(target: LOG_TARGET, "AST fragment injected");
  Ok(())
}

pub fn expose_internal_variables(
  state: &mut State,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let contract = contract_override(state, overrides).unwrap_or("<all>");
  info!(
    target: LOG_TARGET,
    "exposing internal variables (contract={})",
    contract
  );
  expose_variables_internal(state, overrides)?;
  info!(target: LOG_TARGET, "internal variables exposed");
  Ok(())
}

pub fn expose_internal_functions(
  state: &mut State,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let contract = contract_override(state, overrides).unwrap_or("<all>");
  info!(
    target: LOG_TARGET,
    "exposing internal functions (contract={})",
    contract
  );
  expose_functions_internal(state, overrides)?;
  info!(target: LOG_TARGET, "internal functions exposed");
  Ok(())
}

pub fn source_unit(state: &State) -> Option<&SourceUnit> {
  state.ast.as_ref()
}

pub fn source_unit_mut(state: &mut State) -> Option<&mut SourceUnit> {
  state.ast.as_mut()
}

fn contract_override<'a>(
  state: &'a State,
  overrides: Option<&'a AstConfigOptions>,
) -> Option<&'a str> {
  overrides
    .and_then(|opts| opts.instrumented_contract())
    .or_else(|| state.config.instrumented_contract())
}

fn resolve_config(state: &State, overrides: Option<&AstConfigOptions>) -> Result<AstConfig> {
  let mut config = state.config.merge_options(overrides).map_err(Error::from)?;
  if config.solc.language != SolcLanguage::Solidity {
    return Err(Error::new(
      "Ast helpers only support solcLanguage \"Solidity\".",
    ));
  }
  config.solc.settings = map_err_with_context(
    AstOrchestrator::sanitize_settings(Some(config.solc.settings.clone())),
    "Failed to sanitize compiler settings",
  )?;
  update_level(config.logging_level);
  info!(
    target: LOG_TARGET,
    "resolved AST config (solc={}, instrumented_contract={:?})",
    config.solc.version,
    config.instrumented_contract()
  );
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
  overrides: Option<&AstConfigOptions>,
  strategy: ResolveConflictStrategy,
) -> Result<()> {
  let contract_name = contract_override(state, overrides).map(|name| name.to_owned());
  let contract_idx = {
    let target_ast = target_ast(state)?;
    find_contract_index(state, target_ast, contract_name.as_deref())?
  };

  let target_ast = target_ast_mut(state)?;
  map_err_with_context(
    AstOrchestrator::stitch_fragment_into_contract(
      target_ast,
      contract_idx,
      &fragment_contract,
      strategy,
    ),
    "Failed to stitch AST nodes",
  )
}

fn contract_indices(
  state: &State,
  ast: &SourceUnit,
  overrides: Option<&AstConfigOptions>,
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
  overrides: Option<&AstConfigOptions>,
  mut mutator: F,
) -> Result<()>
where
  F: FnMut(&mut ContractDefinition),
{
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

fn expose_variables_internal(
  state: &mut State,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
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

fn expose_functions_internal(
  state: &mut State,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
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

pub fn validate(state: &mut State, overrides: Option<&AstConfigOptions>) -> Result<()> {
  info!(
    target: LOG_TARGET,
    "validating AST (current_contract={:?})",
    state.config.instrumented_contract()
  );
  let config = resolve_config(state, overrides)?;
  let mut compile_config = config.solc.clone();
  compile_config.settings.stop_after = None;

  let target = target_ast(state)?;
  let mut ast_value = map_err_with_context(
    serde_json::to_value(target),
    "Failed to serialise AST for validation",
  )?;
  utils::sanitize_ast_value(&mut ast_value);

  let settings_value = map_err_with_context(
    serde_json::to_value(&compile_config.settings),
    "Failed to serialise compiler settings",
  )?;

  let input = json!({
    "language": "SolidityAST",
    "sources": {
      VIRTUAL_SOURCE_PATH: { "ast": ast_value }
    },
    "settings": settings_value
  });

  let solc = solc::ensure_installed(&compile_config.version)?;
  let output: Value = map_err_with_context(solc.compile_as(&input), "Solc validation failed")?;

  if let Some(errors) = output.get("errors").and_then(|value| value.as_array()) {
    let mut messages = Vec::new();
    for error in errors {
      let severity = error
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
      if severity.eq_ignore_ascii_case("error") {
        let message = error
          .get("formattedMessage")
          .and_then(|value| value.as_str())
          .or_else(|| error.get("message").and_then(|value| value.as_str()))
          .unwrap_or("Compilation error");
        messages.push(message.to_string());
      }
    }
    if !messages.is_empty() {
      error!(
        target: LOG_TARGET,
        "AST validation failed with {} error(s)",
        messages.len()
      );
      return Err(Error::new(format!(
        "AST validation failed:\n{}",
        messages.join("\n")
      )));
    }
  }

  let next_ast_value = output
    .get("sources")
    .and_then(|sources| sources.get(VIRTUAL_SOURCE_PATH))
    .and_then(|entry| entry.get("ast"))
    .cloned()
    .ok_or_else(|| Error::new("Validation succeeded but AST output was missing"))?;

  let next_ast = map_err_with_context(
    serde_json::from_value::<SourceUnit>(next_ast_value),
    "Failed to deserialise validated AST",
  )?;

  state.ast = Some(next_ast);
  info!(target: LOG_TARGET, "AST validation succeeded");
  Ok(())
}

fn load_source_text(
  state: &mut State,
  source: &str,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  let solc = solc::ensure_installed(&config.solc.version)?;

  let ast = map_err_with_context(
    AstOrchestrator::parse_source_unit(source, VIRTUAL_SOURCE_PATH, &solc, &config.solc.settings),
    "Failed to parse target source",
  )?;

  state.ast = Some(ast);
  Ok(())
}

fn load_source_ast(
  state: &mut State,
  target_ast: SourceUnit,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  solc::ensure_installed(&config.solc.version)?;

  map_err_with_context(
    stitcher::find_instrumented_contract_index(&target_ast, contract_override(state, overrides)),
    "Failed to locate target contract",
  )?;

  state.ast = Some(target_ast);
  Ok(())
}

fn inject_fragment_string(
  state: &mut State,
  fragment_source: &str,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  let solc = solc::ensure_installed(&config.solc.version)?;

  let strategy = config.resolve_conflict_strategy;
  let fragment_contract = map_err_with_context(
    AstOrchestrator::parse_fragment_contract(fragment_source, &solc, &config.solc.settings),
    "Failed to parse AST fragment",
  )?;

  inject_fragment_contract(state, fragment_contract, overrides, strategy)
}

fn inject_fragment_ast(
  state: &mut State,
  fragment_ast: SourceUnit,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let config = resolve_config(state, overrides)?;
  solc::ensure_installed(&config.solc.version)?;

  let strategy = config.resolve_conflict_strategy;
  let fragment_contract = map_err_with_context(
    AstOrchestrator::extract_fragment_contract(&fragment_ast),
    "Failed to locate fragment contract",
  )?;

  inject_fragment_contract(state, fragment_contract, overrides, strategy)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::utils;
  use crate::internal::config::{AstConfigOptions, CompilerLanguage, SolcConfig};
  use crate::internal::settings::{CompilerSettingsOptions, OptimizerSettingsOptions};
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
      CompilerLanguage::from(default_language),
      &default_settings,
      Option::<&AstConfigOptions>::None,
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

    let overrides = AstConfigOptions {
      solc: crate::SolcConfigOptions::default(),
      instrumented_contract: Some("Target".into()),
      logging_level: None,
      resolve_conflict_strategy: None,
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
      CompilerLanguage::from(default_language),
      &default_settings,
      Option::<&AstConfigOptions>::None,
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
    let overrides = AstConfigOptions {
      solc: crate::SolcConfigOptions::default(),
      instrumented_contract: Some("Target".into()),
      logging_level: None,
      resolve_conflict_strategy: None,
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
  fn overrides_do_not_persist_across_calls() {
    if find_default_solc().is_none() {
      return;
    }

    let mut state = init(None).expect("init ast");
    let initial_config = state.config.clone();

    let mut overrides = AstConfigOptions::default();
    overrides.instrumented_contract = Some("Target".to_string());
    overrides.solc.settings = Some({
      let mut settings = CompilerSettingsOptions::default();
      settings.optimizer = Some(OptimizerSettingsOptions {
        enabled: Some(true),
        runs: Some(200),
        ..Default::default()
      });
      settings
    });

    let initial_settings_json =
      serde_json::to_value(&state.config.solc.settings).expect("serialize initial settings");

    from_source(
      &mut state,
      SourceTarget::Text(INSTRUMENTED_CONTRACT.into()),
      Some(&overrides),
    )
    .expect("load source with override");

    assert_eq!(
      state.config.instrumented_contract(),
      initial_config.instrumented_contract()
    );

    assert_eq!(
      serde_json::to_value(&state.config.solc.settings).expect("serialize settings"),
      initial_settings_json,
      "expected base compiler settings to remain unchanged after from_source override"
    );

    expose_internal_variables(&mut state, Some(&overrides))
      .expect("apply override without persisting");

    assert_eq!(
      state.config.instrumented_contract(),
      initial_config.instrumented_contract()
    );

    assert_eq!(
      serde_json::to_value(&state.config.solc.settings).expect("serialize settings"),
      initial_settings_json,
      "expected base compiler settings to remain unchanged after expose override"
    );

    validate(&mut state, Some(&overrides)).expect("validate with override");

    assert_eq!(
      serde_json::to_value(&state.config.solc.settings).expect("serialize settings"),
      initial_settings_json,
      "expected base compiler settings to remain unchanged after validate override"
    );
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
      CompilerLanguage::from(default_language),
      &default_settings,
      Option::<&AstConfigOptions>::None,
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

    let settings_value =
      serde_json::to_value(&state.config.solc.settings).expect("serialize settings");

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
