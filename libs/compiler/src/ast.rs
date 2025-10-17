mod error;
pub(crate) mod parser;
mod stitcher;
pub(crate) mod utils;

use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, SourceUnit, SourceUnitPart, Visibility,
};
use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};
use foundry_compilers::solc::SolcLanguage;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown};

use self::utils::{from_js_value, sanitize_ast_value, to_js_value};
use crate::internal::{
  errors::{map_napi_error, napi_error},
  options::{parse_ast_options, AstOptions, SolcConfig},
  solc,
};

const VIRTUAL_SOURCE_PATH: &str = "__VIRTUAL__.sol";

/// High-level helper for manipulating Solidity ASTs prior to recompilation.
#[napi]
#[derive(Clone)]
pub struct Ast {
  config: SolcConfig,
  ast: Option<SourceUnit>,
  options: AstOptions,
}

impl Ast {
  fn contract_override<'a>(&'a self, overrides: Option<&'a AstOptions>) -> Option<&'a str> {
    overrides
      .and_then(|opts| opts.instrumented_contract())
      .or_else(|| self.options.instrumented_contract())
  }

  fn update_options(&mut self, overrides: Option<&AstOptions>) {
    if let Some(opts) = overrides {
      self.options = opts.clone();
    }
  }

  pub(crate) fn sanitize_settings(settings: Option<Settings>) -> Settings {
    let mut settings = settings.unwrap_or_default();
    settings.stop_after = Some("parsing".to_string());
    settings.output_selection = OutputSelection::ast_output_selection();
    settings.evm_version = None;
    settings
  }

  fn resolve_config(&self, overrides: Option<&AstOptions>) -> Result<SolcConfig> {
    let mut config = self.config.merge(overrides)?;
    if config.language != SolcLanguage::Solidity {
      return Err(napi_error(
        "Ast helpers only support solcLanguage \"Solidity\".",
      ));
    }
    config.settings = Self::sanitize_settings(Some(config.settings));
    Ok(config)
  }

  fn target_ast_mut(&mut self) -> Result<&mut SourceUnit> {
    self
      .ast
      .as_mut()
      .ok_or_else(|| napi_error("Ast has no target AST. Call fromSource first."))
  }

  fn target_ast(&self) -> Result<&SourceUnit> {
    self
      .ast
      .as_ref()
      .ok_or_else(|| napi_error("Ast has no target AST. Call fromSource first."))
  }

  fn find_contract_index(&self, ast: &SourceUnit, contract_name: Option<&str>) -> Result<usize> {
    map_napi_error(
      stitcher::find_instrumented_contract_index(ast, contract_name),
      "Failed to locate target contract",
    )
  }

  fn inject_fragment_contract(
    &mut self,
    fragment_contract: ContractDefinition,
    overrides: Option<&AstOptions>,
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
      "Failed to stitch AST nodes",
    )?;

    Ok(())
  }

  fn contract_indices(
    &self,
    ast: &SourceUnit,
    overrides: Option<&AstOptions>,
  ) -> Result<Vec<usize>> {
    if let Some(name) = self.contract_override(overrides) {
      let idx = stitcher::find_instrumented_contract_index(ast, Some(name))?;
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

  fn expose_variables_internal(&mut self, overrides: Option<&AstOptions>) -> Result<()> {
    self.update_options(overrides);
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

  fn expose_functions_internal(&mut self, overrides: Option<&AstOptions>) -> Result<()> {
    self.update_options(overrides);
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

  pub(crate) fn from_source_string(
    &mut self,
    source: &str,
    overrides: Option<&AstOptions>,
  ) -> Result<()> {
    self.update_options(overrides);
    let mut config = self.resolve_config(overrides)?;
    let solc = solc::ensure_installed(&config.version)?;

    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let ast = map_napi_error(
      parser::parse_source_ast(source, VIRTUAL_SOURCE_PATH, &solc, &settings),
      "Failed to parse target source",
    )?;

    config.settings = settings;
    self.config = config;
    self.ast = Some(ast);
    Ok(())
  }

  pub(crate) fn from_source_ast(
    &mut self,
    target_ast: SourceUnit,
    overrides: Option<&AstOptions>,
  ) -> Result<()> {
    self.update_options(overrides);
    let config = self.resolve_config(overrides)?;
    solc::ensure_installed(&config.version)?;

    map_napi_error(
      stitcher::find_instrumented_contract_index(&target_ast, self.contract_override(overrides)),
      "Failed to locate target contract",
    )?;

    self.config = config;
    self.ast = Some(target_ast);
    Ok(())
  }

  pub(crate) fn inject_fragment_string(
    &mut self,
    fragment_source: &str,
    overrides: Option<&AstOptions>,
  ) -> Result<()> {
    let mut config = self.resolve_config(overrides)?;
    let solc = solc::ensure_installed(&config.version)?;
    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let fragment_contract = map_napi_error(
      parser::parse_fragment_contract(fragment_source, &solc, &settings),
      "Failed to parse AST fragment",
    )?;

    config.settings = settings;
    self.config = config;
    self.inject_fragment_contract(fragment_contract, overrides)
  }

  pub(crate) fn inject_fragment_ast(
    &mut self,
    fragment_ast: SourceUnit,
    overrides: Option<&AstOptions>,
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
impl Ast {
  /// Create a new AST helper. Providing `instrumentedContract`
  /// establishes the instrumented contract targeted by subsequent operations.
  #[napi(constructor, ts_args_type = "options?: AstOptions | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_ast_options(&env, options)?;
    let default_settings = Self::sanitize_settings(None);
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(&default_language, &default_settings, parsed.as_ref())?;
    config.settings = Self::sanitize_settings(Some(config.settings));
    if config.language != SolcLanguage::Solidity {
      return Err(napi_error(
        "Ast helpers only support solcLanguage \"Solidity\".",
      ));
    }
    solc::ensure_installed(&config.version)?;

    let ast = Ast {
      config,
      ast: None,
      options: parsed.unwrap_or_default(),
    };
    Ok(ast)
  }

  /// Parse Solidity source into an AST using the configured solc version.
  /// When no `instrumentedContract` is provided, later operations apply to all
  /// contracts in the file.
  #[napi(
    ts_args_type = "target: string | object, options?: AstOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn from_source(
    &mut self,
    env: Env,
    target: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> Result<Ast> {
    let parsed = parse_ast_options(&env, options)?;
    match target {
      Either::A(source) => self.from_source_string(&source, parsed.as_ref())?,
      Either::B(object) => {
        let target_unit: SourceUnit = from_js_value(&env, object.into_unknown())?;
        self.from_source_ast(target_unit, parsed.as_ref())?;
      }
    }
    Ok(self.clone())
  }

  /// Parse an AST fragment from source text or inject a pre-parsed AST fragment
  /// into the targeted contract.
  #[napi(
    ts_args_type = "fragment: string | object, options?: AstOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn inject_shadow(
    &mut self,
    env: Env,
    fragment: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> Result<Ast> {
    let parsed = parse_ast_options(&env, options)?;
    match fragment {
      Either::A(source) => self.inject_fragment_string(&source, parsed.as_ref())?,
      Either::B(object) => {
        let fragment_unit: SourceUnit = from_js_value(&env, object.into_unknown())?;
        self.inject_fragment_ast(fragment_unit, parsed.as_ref())?;
      }
    }
    Ok(self.clone())
  }

  /// Promote private/internal state variables to public visibility. Omitting
  /// `instrumentedContract` applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: AstOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_variables(&mut self, env: Env, options: Option<JsUnknown>) -> Result<Ast> {
    let parsed = parse_ast_options(&env, options)?;
    self.expose_variables_internal(parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Promote private/internal functions to public visibility. Omitting
  /// `instrumentedContract` applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: AstOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_functions(&mut self, env: Env, options: Option<JsUnknown>) -> Result<Ast> {
    let parsed = parse_ast_options(&env, options)?;
    self.expose_functions_internal(parsed.as_ref())?;
    Ok(self.clone())
  }

  /// Get the current intrumented AST.
  #[napi(ts_return_type = "import('./ast-types').SourceUnit")]
  pub fn ast(&self, env: Env) -> Result<JsUnknown> {
    let ast = self
      .ast
      .as_ref()
      .ok_or_else(|| napi_error("Ast has no target unit. Call fromSource first."))?;
    let mut ast_value = map_napi_error(serde_json::to_value(ast), "Failed to serialize AST value")?;
    sanitize_ast_value(&mut ast_value);
    to_js_value(&env, &ast_value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::internal::options::{AstOptions, SolcConfig};
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

    let default_settings = Ast::sanitize_settings(None);
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = Ast::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut instrument = Ast {
      config,
      ast: None,
      options: AstOptions::default(),
    };

    instrument
      .from_source_string(INSTRUMENTED_CONTRACT, None)
      .expect("load source");

    let overrides = AstOptions {
      solc_version: None,
      solc_language: None,
      settings: None,
      instrumented_contract: Some("Target".into()),
    };

    instrument
      .inject_fragment_string(
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
    let default_settings = Ast::sanitize_settings(None);
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = Ast::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut instrument = Ast {
      config,
      ast: None,
      options: AstOptions::default(),
    };

    instrument
      .from_source_string(INSTRUMENTED_CONTRACT, None)
      .expect("load source");
    let overrides = AstOptions {
      solc_version: None,
      solc_language: None,
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
  fn ast_round_trip() {
    let Some(solc) = find_default_solc() else {
      return;
    };

    let default_settings = Ast::sanitize_settings(None);
    let default_language = solc::default_language();
    let mut config = SolcConfig::new(
      &default_language,
      &default_settings,
      Option::<&AstOptions>::None,
    )
    .expect("config");
    config.settings = Ast::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version).expect("ensure solc");

    let mut instrument = Ast {
      config,
      ast: None,
      options: AstOptions::default(),
    };
    instrument
      .from_source_string(INSTRUMENTED_CONTRACT, None)
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
      "expected solc to compile ast without errors, but got errors: {:?}, ast: {:?}",
      output.errors,
      serde_json::to_string_pretty(&ast_value).unwrap_or_default()
    );
  }
}
