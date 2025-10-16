mod error;
pub(crate) mod parser;
mod stitcher;
pub(crate) mod utils;

use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, SourceUnit, SourceUnitPart, Visibility,
};
use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};
use napi::bindgen_prelude::*;
use napi::{Env, JsUnknown};

use self::utils::{from_js_value, sanitize_ast_value, to_js_value};
use crate::internal::{
  errors::{map_napi_error, napi_error},
  options::{parse_instrument_options, InstrumentOptions, SolcConfig},
  solc,
};

const DEFAULT_VIRTUAL_SOURCE: &str = "Instrumented.sol";

/// High-level helper for manipulating Solidity ASTs prior to recompilation.
#[napi]
#[derive(Clone)]
pub struct Instrument {
  config: SolcConfig,
  ast: Option<SourceUnit>,
  source_name: Option<String>,
  default_contract: Option<String>,
}

impl Instrument {
  fn contract_override<'a>(&'a self, overrides: Option<&'a InstrumentOptions>) -> Option<&'a str> {
    overrides
      .and_then(|opts| opts.instrumented_contract())
      .or_else(|| self.default_contract.as_deref())
  }

  fn update_default_contract(&mut self, overrides: Option<&InstrumentOptions>) {
    if let Some(opts) = overrides {
      self.default_contract = opts.instrumented_contract.clone();
    }
  }

  pub(crate) fn sanitize_settings(settings: Option<Settings>) -> Settings {
    let mut settings = settings.unwrap_or_default();
    settings.stop_after = Some("parsing".to_string());
    settings.output_selection = OutputSelection::ast_output_selection();
    settings.evm_version = None;
    settings
  }

  fn resolve_config(&self, overrides: Option<&InstrumentOptions>) -> Result<SolcConfig> {
    let mut config = self.config.merge(overrides)?;
    config.settings = Self::sanitize_settings(Some(config.settings));
    Ok(config)
  }

  fn target_ast_mut(&mut self) -> Result<&mut SourceUnit> {
    self
      .ast
      .as_mut()
      .ok_or_else(|| napi_error("Instrument has no target AST. Call fromSource or fromAst first."))
  }

  fn target_ast(&self) -> Result<&SourceUnit> {
    self
      .ast
      .as_ref()
      .ok_or_else(|| napi_error("Instrument has no target AST. Call fromSource or fromAst first."))
  }

  fn find_contract_index(&self, ast: &SourceUnit, contract_name: Option<&str>) -> Result<usize> {
    map_napi_error(
      stitcher::find_target_contract_index(ast, contract_name),
      "Failed to locate target contract",
    )
  }

  fn inject_fragment_contract(
    &mut self,
    fragment_contract: ContractDefinition,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<()> {
    let contract_name = self
      .contract_override(overrides)
      .map(|name| name.to_owned());
    let contract_idx = {
      let target_ast = self.target_ast()?;
      self.find_contract_index(target_ast, contract_name.as_deref())?
    };

    let target_ast = self.target_ast_mut()?;
    let max_target_id = map_napi_error(utils::max_id(target_ast), "Failed to inspect AST ids")?;

    map_napi_error(
      stitcher::stitch_fragment_nodes_into_contract(
        target_ast,
        contract_idx,
        &fragment_contract,
        max_target_id,
      ),
      "Failed to stitch instrumentation nodes",
    )?;

    Ok(())
  }

  fn contract_indices(
    &self,
    ast: &SourceUnit,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<Vec<usize>> {
    if let Some(name) = self.contract_override(overrides) {
      let idx = stitcher::find_target_contract_index(ast, Some(name))?;
      Ok(vec![idx])
    } else {
      let indices = ast
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, part)| {
          if matches!(part, SourceUnitPart::ContractDefinition(_)) {
            Some(idx)
          } else {
            None
          }
        })
        .collect::<Vec<_>>();

      if indices.is_empty() {
        Err(napi_error(
          "Target AST does not contain any contract definitions",
        ))
      } else {
        Ok(indices)
      }
    }
  }

  fn expose_variables_internal(&mut self, overrides: Option<&InstrumentOptions>) -> Result<()> {
    self.update_default_contract(overrides);
    let target_ast_ptr = self.target_ast_mut()? as *mut SourceUnit;
    // safety: pointer valid during this scope
    let target_ast = unsafe { &mut *target_ast_ptr };
    let indices = self.contract_indices(target_ast, overrides)?;

    for idx in indices {
      let SourceUnitPart::ContractDefinition(contract) = target_ast
        .nodes
        .get_mut(idx)
        .ok_or_else(|| napi_error("Invalid contract index"))?
      else {
        continue;
      };

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
    }

    Ok(())
  }

  fn expose_functions_internal(&mut self, overrides: Option<&InstrumentOptions>) -> Result<()> {
    self.update_default_contract(overrides);
    let target_ast_ptr = self.target_ast_mut()? as *mut SourceUnit;
    let target_ast = unsafe { &mut *target_ast_ptr };
    let indices = self.contract_indices(target_ast, overrides)?;

    for idx in indices {
      let SourceUnitPart::ContractDefinition(contract) = target_ast
        .nodes
        .get_mut(idx)
        .ok_or_else(|| napi_error("Invalid contract index"))?
      else {
        continue;
      };

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
    }

    Ok(())
  }

  pub(crate) fn from_compiler_config(
    base: &SolcConfig,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<Self> {
    let base_settings = Self::sanitize_settings(Some(base.settings.clone()));
    let mut config = SolcConfig::with_defaults(&base.version, &base_settings, overrides)?;
    config.settings = Self::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version)?;

    let mut instrument = Instrument {
      config,
      ast: None,
      source_name: None,
      default_contract: None,
    };
    instrument.update_default_contract(overrides);
    Ok(instrument)
  }

  pub(crate) fn load_source(
    &mut self,
    source: &str,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<()> {
    self.update_default_contract(overrides);
    let mut config = self.resolve_config(overrides)?;
    let solc = solc::ensure_installed(&config.version)?;

    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let ast = map_napi_error(
      parser::parse_source_ast(source, DEFAULT_VIRTUAL_SOURCE, &solc, &settings),
      "Failed to parse target source",
    )?;

    config.settings = settings;
    self.config = config;
    self.ast = Some(ast);
    self.source_name = Some(DEFAULT_VIRTUAL_SOURCE.to_string());
    Ok(())
  }

  pub(crate) fn load_ast(
    &mut self,
    env: &Env,
    target_ast: JsUnknown,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<()> {
    self.update_default_contract(overrides);
    let config = self.resolve_config(overrides)?;
    solc::ensure_installed(&config.version)?;

    let ast_unit: SourceUnit = from_js_value(env, target_ast)?;

    map_napi_error(
      stitcher::find_target_contract_index(&ast_unit, self.contract_override(overrides)),
      "Failed to locate target contract",
    )?;

    self.config = config;
    self.ast = Some(ast_unit);
    self.source_name = None;
    Ok(())
  }

  pub(crate) fn inject_fragment_from_source(
    &mut self,
    fragment_source: &str,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<()> {
    let mut config = self.resolve_config(overrides)?;
    let solc = solc::ensure_installed(&config.version)?;
    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let fragment_contract = map_napi_error(
      parser::parse_fragment_contract(fragment_source, &solc, &settings),
      "Failed to parse instrumentation fragment",
    )?;

    config.settings = settings;
    self.config = config;
    self.inject_fragment_contract(fragment_contract, overrides)
  }

  pub(crate) fn inject_fragment_from_ast_value(
    &mut self,
    fragment_ast: SourceUnit,
    overrides: Option<&InstrumentOptions>,
  ) -> Result<()> {
    let config = self.resolve_config(overrides)?;
    solc::ensure_installed(&config.version)?;
    self.config = config;

    let fragment_contract = map_napi_error(
      parser::extract_fragment_contract(&fragment_ast).map(|contract| contract.clone()),
      "Failed to locate fragment contract",
    )?;

    self.inject_fragment_contract(fragment_contract, overrides)
  }
}

/// JavaScript-facing API surface.
#[napi]
impl Instrument {
  /// Create a new instrumentation helper. Providing `instrumentedContract`
  /// establishes the default contract targeted by subsequent operations.
  #[napi(constructor, ts_args_type = "options?: InstrumentOptions | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_instrument_options(&env, options)?;
    let default_settings = Self::sanitize_settings(None);
    let mut config = SolcConfig::new(&default_settings, parsed.as_ref())?;
    config.settings = Self::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version)?;

    let mut instrument = Instrument {
      config,
      ast: None,
      source_name: None,
      default_contract: None,
    };
    instrument.update_default_contract(parsed.as_ref());
    Ok(instrument)
  }

  /// Parse Solidity source into an AST using the configured solc version.
  /// When no `instrumentedContract` is provided, later operations apply to all
  /// contracts in the file.
  #[napi(
    ts_args_type = "targetSource: string, options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn from_source(
    &mut self,
    env: Env,
    target_source: String,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    self.load_source(&target_source, parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Load a pre-existing `SourceUnit` for instrumentation.
  #[napi(
    ts_args_type = "targetAst: import('./ast-types').SourceUnit, options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn from_ast(
    &mut self,
    env: Env,
    target_ast: JsUnknown,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    self.load_ast(&env, target_ast, parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Parse an instrumentation fragment from source text and inject it into the
  /// targeted contract.
  #[napi(
    ts_args_type = "fragmentSource: string, options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn inject_shadow_source(
    &mut self,
    env: Env,
    fragment_source: String,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    self.inject_fragment_from_source(&fragment_source, parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Inject a pre-parsed instrumentation fragment into the targeted contract.
  #[napi(
    ts_args_type = "fragmentAst: import('./ast-types').SourceUnit, options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn inject_shadow_ast(
    &mut self,
    env: Env,
    fragment_ast: JsUnknown,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    let fragment_unit: SourceUnit = from_js_value(&env, fragment_ast)?;
    self.inject_fragment_from_ast_value(fragment_unit, parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Promote private/internal state variables to public visibility. Omitting
  /// `instrumentedContract` applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_variables(
    &mut self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    self.expose_variables_internal(parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Promote private/internal functions to public visibility. Omitting
  /// `instrumentedContract` applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: InstrumentOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_functions(
    &mut self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> Result<Instrument> {
    let parsed = parse_instrument_options(&env, options)?;
    self.expose_functions_internal(parsed.as_ref())?;
    Ok(self.clone())
  }

  #[napi(ts_return_type = "import('./ast-types').SourceUnit")]
  pub fn ast(&self, env: Env) -> Result<JsUnknown> {
    let ast = self.ast.as_ref().ok_or_else(|| {
      napi_error("Instrument has no target AST. Call fromSource or fromAst first.")
    })?;
    let mut ast_value = map_napi_error(serde_json::to_value(ast), "Failed to serialize AST value")?;
    sanitize_ast_value(&mut ast_value);
    to_js_value(&env, &ast_value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::internal::options::{InstrumentOptions, SolcConfig};
  use foundry_compilers::artifacts::CompilerOutput;
  use foundry_compilers::solc::Solc;
  use serde_json::{json, Value};

  const TARGET_CONTRACT: &str = r#"
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

    let default_settings = Instrument::sanitize_settings(None);
    let mut config =
      SolcConfig::new(&default_settings, Option::<&InstrumentOptions>::None).expect("config");
    config.settings = Instrument::sanitize_settings(Some(config.settings));

    let mut instrument = Instrument::from_compiler_config(&config, None).expect("instrument");

    instrument
      .load_source(TARGET_CONTRACT, None)
      .expect("load source");

    let overrides = InstrumentOptions {
      solc_version: None,
      settings: None,
      instrumented_contract: Some("Target".into()),
    };

    instrument
      .inject_fragment_from_source(
        "function extra() public view returns (uint256) { return value; }",
        Some(&overrides),
      )
      .expect("inject fragment");

    let ast = instrument.ast.as_ref().expect("ast");
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

    // Ensure uniqueness of ids
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
    let default_settings = Instrument::sanitize_settings(None);
    let mut config =
      SolcConfig::new(&default_settings, Option::<&InstrumentOptions>::None).expect("config");
    config.settings = Instrument::sanitize_settings(Some(config.settings));

    let mut instrument = Instrument {
      config,
      ast: None,
      source_name: None,
      default_contract: None,
    };

    instrument
      .load_source(TARGET_CONTRACT, None)
      .expect("load source");
    let overrides = InstrumentOptions {
      solc_version: None,
      settings: None,
      instrumented_contract: Some("Target".into()),
    };
    instrument
      .expose_variables_internal(Some(&overrides))
      .expect("expose vars");
    instrument
      .expose_functions_internal(Some(&overrides))
      .expect("expose funcs");

    let ast = instrument.ast.as_ref().expect("ast");
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
  fn instrumented_ast_round_trip() {
    let Some(solc) = find_default_solc() else {
      return;
    };

    let default_settings = Instrument::sanitize_settings(None);
    let mut config =
      SolcConfig::new(&default_settings, Option::<&InstrumentOptions>::None).expect("config");
    config.settings = Instrument::sanitize_settings(Some(config.settings));

    let mut instrument = Instrument::from_compiler_config(&config, None).expect("instrument");
    instrument
      .load_source(TARGET_CONTRACT, None)
      .expect("load source");
    instrument
      .expose_variables_internal(None)
      .expect("expose vars");
    instrument
      .expose_functions_internal(None)
      .expect("expose funcs");

    let ast = instrument.ast.as_ref().expect("ast");
    let mut ast_value = serde_json::to_value(ast).expect("serialize ast");
    sanitize_ast_value(&mut ast_value);

    let settings_value =
      serde_json::to_value(&instrument.config.settings).expect("serialize settings");

    let input = json!({
      "language": "SolidityAST",
      "sources": {
        DEFAULT_VIRTUAL_SOURCE: {
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
      "expected solc to compile instrumented ast without errors, but got errors: {:?}, ast: {:?}",
      output.errors,
      serde_json::to_string_pretty(&ast_value).unwrap_or_default()
    );
  }
}
