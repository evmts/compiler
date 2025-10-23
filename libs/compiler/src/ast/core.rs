use foundry_compilers::solc::SolcLanguage;
use log::{error, info};
use serde_json::{json, Value};

use super::{instrumenter, orchestrator::AstOrchestrator, stitcher};
use crate::internal::{
  config::{AstConfig, AstConfigOptions, ResolveConflictStrategy},
  errors::{map_err_with_context, Error, Result},
  logging::{ensure_rust_logger, update_level},
  solc,
};

const VIRTUAL_SOURCE_PATH: &str = "__VIRTUAL__.sol";
const LOG_TARGET: &str = "tevm::ast";

#[derive(Clone)]
pub struct State {
  pub config: AstConfig,
  pub ast: Option<Value>,
}

#[derive(Clone)]
pub enum SourceTarget {
  Text(String),
  Ast(Value),
}

#[derive(Clone)]
pub enum FragmentTarget {
  Text(String),
  Ast(Value),
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
      info!(target: LOG_TARGET, "loading AST from pre-built unit");
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
      info!(target: LOG_TARGET, "injecting pre-built AST fragment");
      inject_fragment_ast(state, unit, overrides)?;
    }
  }
  info!(target: LOG_TARGET, "AST fragment injected");
  Ok(())
}

pub fn inject_shadow_at_edges(
  state: &mut State,
  selector: &str,
  before: &[String],
  after: &[String],
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  let contract = contract_override(state, overrides).map(|name| name.to_owned());
  info!(
    target: LOG_TARGET,
    "injecting edge instrumentation (selector={}, contract={:?})",
    selector,
    contract
  );

  let config = resolve_config(state, overrides)?;
  let solc = solc::ensure_installed(&config.solc.version)?;

  let idx = {
    let target_ast = target_ast(state)?;
    find_contract_index(state, target_ast, contract.as_deref())?
  };

  let unit = target_ast_mut(state)?;
  instrumenter::inject_edges(
    unit,
    idx,
    selector,
    before,
    after,
    &solc,
    &config.solc.settings,
  )?;

  info!(
    target: LOG_TARGET,
    "edge instrumentation applied (selector={}, contract={:?})",
    selector,
    contract
  );

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

pub fn source_unit(state: &State) -> Option<&Value> {
  state.ast.as_ref()
}

pub fn source_unit_mut(state: &mut State) -> Option<&mut Value> {
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

fn target_ast_mut(state: &mut State) -> Result<&mut Value> {
  state
    .ast
    .as_mut()
    .ok_or_else(|| Error::new("Ast has no target AST. Call from_source first."))
}

fn target_ast(state: &State) -> Result<&Value> {
  state
    .ast
    .as_ref()
    .ok_or_else(|| Error::new("Ast has no target AST. Call from_source first."))
}

fn find_contract_index(state: &State, ast: &Value, contract_name: Option<&str>) -> Result<usize> {
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
  fragment_contract: Value,
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
  ast: &Value,
  overrides: Option<&AstConfigOptions>,
) -> Result<Vec<usize>> {
  if let Some(name) = contract_override(state, overrides) {
    let idx = stitcher::find_instrumented_contract_index(ast, Some(name))?;
    Ok(vec![idx])
  } else {
    let nodes = ast
      .get("nodes")
      .and_then(|value| value.as_array())
      .ok_or_else(|| Error::new("Target AST does not contain any nodes"))?;
    let indices = nodes
      .iter()
      .enumerate()
      .filter_map(|(idx, node)| {
        node
          .get("nodeType")
          .and_then(|value| value.as_str())
          .filter(|node_type| *node_type == "ContractDefinition")
          .map(|_| idx)
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
  F: FnMut(&mut Value),
{
  let indices = {
    let unit = target_ast(state)?;
    contract_indices(state, unit, overrides)?
  };
  let unit = target_ast_mut(state)?;
  let nodes = unit
    .get_mut("nodes")
    .and_then(|value| value.as_array_mut())
    .ok_or_else(|| Error::new("Target AST does not contain any nodes"))?;

  for idx in indices {
    if let Some(contract) = nodes.get_mut(idx) {
      mutator(contract);
    }
  }
  Ok(())
}

fn expose_variables_internal(
  state: &mut State,
  overrides: Option<&AstConfigOptions>,
) -> Result<()> {
  mutate_contracts(state, overrides, |contract| {
    if let Some(members) = contract
      .get_mut("nodes")
      .and_then(|value| value.as_array_mut())
    {
      for member in members {
        if member
          .get("nodeType")
          .and_then(|value| value.as_str())
          .map(|node_type| node_type == "VariableDeclaration")
          .unwrap_or(false)
        {
          if let Some(object) = member.as_object_mut() {
            match object.get_mut("visibility") {
              Some(value) => {
                if !matches!(value.as_str(), Some("public")) {
                  *value = Value::String("public".to_string());
                }
              }
              None => {
                object.insert(
                  "visibility".to_string(),
                  Value::String("public".to_string()),
                );
              }
            }
          }
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
    if let Some(members) = contract
      .get_mut("nodes")
      .and_then(|value| value.as_array_mut())
    {
      for member in members {
        if member
          .get("nodeType")
          .and_then(|value| value.as_str())
          .map(|node_type| node_type == "FunctionDefinition")
          .unwrap_or(false)
        {
          if let Some(object) = member.as_object_mut() {
            match object.get_mut("visibility") {
              Some(value) => {
                if !matches!(value.as_str(), Some("public")) {
                  *value = Value::String("public".to_string());
                }
              }
              None => {
                object.insert(
                  "visibility".to_string(),
                  Value::String("public".to_string()),
                );
              }
            }
          }
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
  let ast_value = map_err_with_context(
    serde_json::to_value(target),
    "Failed to serialise AST for validation",
  )?;

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

  state.ast = Some(next_ast_value);
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

  let mut settings = config.solc.settings.clone();
  settings.stop_after = None;

  let ast = map_err_with_context(
    AstOrchestrator::parse_source_unit(source, VIRTUAL_SOURCE_PATH, &solc, &settings),
    "Failed to parse target source",
  )?;

  state.ast = Some(ast);
  Ok(())
}

fn load_source_ast(
  state: &mut State,
  target_ast: Value,
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
  fragment_ast: Value,
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
