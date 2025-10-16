mod error;
pub(crate) mod parser;
mod stitcher;
pub(crate) mod utils;

pub use error::ShadowError;

use foundry_compilers::solc::Solc;
use napi::bindgen_prelude::*;
use napi::{Env, JsUnknown};
use serde_json::Value;

use self::utils::{from_js_value, to_js_value};
use crate::internal::{
  errors::map_napi_error,
  options::{parse_shadow_options, ShadowOptions, SolcConfig},
  solc,
};
use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};

/// Convenience API for deriving AST fragments and stitching them into existing Solidity code.
#[napi]
pub struct Shadow {
  source: String,
  config: SolcConfig,
}

impl Shadow {
  pub(crate) fn sanitize_settings(settings: Option<Settings>) -> Settings {
    let mut settings = settings.unwrap_or_default();
    settings.stop_after = Some("parsing".to_string());
    settings.output_selection = OutputSelection::ast_output_selection();
    settings.evm_version = None;
    settings
  }

  pub(crate) fn from_config(source: String, mut config: SolcConfig) -> Result<Self> {
    config.settings = Self::sanitize_settings(Some(config.settings));
    solc::ensure_installed(&config.version)?;
    Ok(Shadow { source, config })
  }

  fn resolve_config(&self, overrides: Option<&ShadowOptions>) -> Result<SolcConfig> {
    let mut config = self.config.merge(overrides)?;
    config.settings = Self::sanitize_settings(Some(config.settings));
    Ok(config)
  }

  fn parse_target_ast(
    &self,
    solc: &Solc,
    settings: &foundry_compilers::artifacts::Settings,
    source: &str,
    file_name: &str,
  ) -> Result<Value> {
    map_napi_error(
      parser::parse_source_ast(source, file_name, solc, settings),
      "Failed to parse target source",
    )
  }

  fn parse_shadow_ast(
    &self,
    solc: &Solc,
    settings: &foundry_compilers::artifacts::Settings,
  ) -> Result<Value> {
    let wrapped = parser::wrap_shadow_source(&self.source);
    map_napi_error(
      parser::parse_source_ast(&wrapped, "Shadow.sol", solc, settings),
      "Failed to parse shadow fragment",
    )
  }

  fn stitch_into_ast_internal(
    &self,
    solc: &Solc,
    settings: &foundry_compilers::artifacts::Settings,
    target_ast: &mut Value,
    target_contract_name: Option<&str>,
  ) -> Result<Value> {
    let shadow_ast = self.parse_shadow_ast(solc, settings)?;
    let max_target_id = utils::find_max_id(target_ast);

    let contract_idx = map_napi_error(
      stitcher::find_target_contract_index(target_ast, target_contract_name),
      "Failed to locate target contract",
    )?;

    map_napi_error(
      stitcher::stitch_shadow_nodes_into_contract(
        target_ast,
        contract_idx,
        &shadow_ast,
        max_target_id,
      ),
      "Failed to stitch shadow nodes",
    )?;

    Ok(target_ast.clone())
  }
}

/// Static helpers and per-instance operations exposed to JavaScript.
#[napi]
impl Shadow {
  /// Create a shadow instance with a Solidity fragment that will be injected later.
  ///
  /// Optional `options` let callers pin the solc version used for parsing. Any
  /// provided solver settings are sanitised so the parser always runs with
  /// `stopAfter = "parsing"` and AST-only output.
  #[napi(
    constructor,
    ts_args_type = "source: string, options?: ShadowOptions | undefined"
  )]
  pub fn new(env: Env, source: String, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_shadow_options(&env, options)?;
    let default_settings = Self::sanitize_settings(None);
    let config = SolcConfig::new(&default_settings, parsed.as_ref())?;
    Shadow::from_config(source, config)
  }

  /// Parse + stitch the shadow fragment into Solidity source text.
  ///
  /// - `targetSource` is the Solidity code whose AST will be expanded.
  /// - `sourceName` controls diagnostic file names (defaults to `Contract.sol`).
  /// - `targetContractName` selects a specific contract; when omitted the last
  ///   contract in the file is used.
  /// - `options` offer per-call overrides for the solc version/settings.
  ///
  /// Returns a fully analysed AST (`SourceUnit`) as a plain JS object following Foundry's typings.
  #[napi(
    ts_args_type = "targetSource: string, sourceName?: string | undefined, targetContractName?: string | undefined, options?: ShadowOptions | undefined",
    ts_return_type = "import('./ast-types').SourceUnit"
  )]
  pub fn stitch_into_source(
    &self,
    env: Env,
    target_source: String,
    source_name: Option<String>,
    target_contract_name: Option<String>,
    options: Option<JsUnknown>,
  ) -> Result<JsUnknown> {
    let parsed = parse_shadow_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;
    let solc = solc::ensure_installed(&config.version)?;
    let file_name = source_name.as_deref().unwrap_or("Contract.sol");

    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let mut target_ast = self.parse_target_ast(&solc, &settings, &target_source, file_name)?;
    let stitched = self.stitch_into_ast_internal(
      &solc,
      &settings,
      &mut target_ast,
      target_contract_name.as_deref(),
    )?;
    to_js_value(&env, &stitched)
  }

  /// Stitch the fragment into an already parsed AST.
  ///
  /// Accepts any Foundry-style AST object (for example, one produced by
  /// `Shadow.stitchIntoSource` or captured from fixtures). Returns a fresh AST
  /// value with the injected nodes while leaving the input object untouched.
  #[napi(
    ts_args_type = "targetAst: import('./ast-types').SourceUnit, targetContractName?: string | undefined, sourceName?: string | undefined, options?: ShadowOptions | undefined",
    ts_return_type = "import('./ast-types').SourceUnit"
  )]
  pub fn stitch_into_ast(
    &self,
    env: Env,
    target_ast: JsUnknown,
    target_contract_name: Option<String>,
    _source_name: Option<String>,
    options: Option<JsUnknown>,
  ) -> Result<JsUnknown> {
    let parsed = parse_shadow_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;
    let solc = solc::ensure_installed(&config.version)?;

    let settings = Self::sanitize_settings(Some(config.settings.clone()));

    let mut target_ast_value: Value = from_js_value(&env, target_ast)?;
    let stitched = self.stitch_into_ast_internal(
      &solc,
      &settings,
      &mut target_ast_value,
      target_contract_name.as_deref(),
    )?;

    to_js_value(&env, &stitched)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::internal::options::{ShadowOptions, SolcConfig};

  const TARGET_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Target {
  uint256 private value;
}
"#;

  const SHADOW_FUNC: &str = "function added() public view returns (uint256) { return value; }";

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    foundry_compilers::solc::Solc::find_svm_installed_version(&version)
      .ok()
      .flatten()
  }

  #[test]
  fn stitches_shadow_into_target_ast() {
    let Some(solc) = find_default_solc() else {
      return;
    };

    let default_settings = Shadow::sanitize_settings(None);
    let config =
      SolcConfig::new(&default_settings, Option::<&ShadowOptions>::None).expect("config");
    let shadow = Shadow::from_config(SHADOW_FUNC.to_string(), config).expect("shadow");
    let mut target_ast =
      parser::parse_source_ast(TARGET_CONTRACT, "Target.sol", &solc, &default_settings)
        .expect("parse target");
    let stitched = shadow
      .stitch_into_ast_internal(&solc, &default_settings, &mut target_ast, Some("Target"))
      .expect("stitch");

    let contract = stitched
      .get("nodes")
      .and_then(|n| n.as_array())
      .and_then(|nodes| nodes.last())
      .expect("contract node");
    let contains_added_fn = contract
      .get("nodes")
      .and_then(|n| n.as_array())
      .map(|nodes| {
        nodes.iter().any(|node| {
          node
            .get("name")
            .and_then(|n| n.as_str())
            .map(|name| name == "added")
            .unwrap_or(false)
        })
      })
      .unwrap_or(false);

    assert!(
      contains_added_fn,
      "stitched AST should contain added function"
    );
  }
}
