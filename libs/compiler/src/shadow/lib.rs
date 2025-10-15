use napi::bindgen_prelude::*;
use napi::Env;
use napi::JsUnknown;
use serde_json::Value;

use super::{error::ShadowError, parser, stitcher, utils};
use utils::{from_js_value, to_js_value};

/// Shadow - Parse and stitch Solidity code fragments into contract ASTs
///
/// Shadow enables parsing incomplete Solidity code (functions, variables, etc.)
/// and stitching them into existing contracts without requiring semantic validity
/// upfront. Demonstrates that Solidity's parser performs pure syntax analysis,
/// allowing AST manipulation before semantic validation.
#[napi]
pub struct Shadow {
  source: String,
}

#[napi]
impl Shadow {
  /// Initialize a new Shadow with a source fragment (function, variable, etc.)
  #[napi(constructor)]
  pub fn new(source: String) -> Self {
    Shadow { source }
  }

  /// Stitch shadow nodes into an existing contract's source code
  /// Convenience wrapper that parses the source first, then stitches ASTs
  /// Returns the fully analyzed AST as a typed object
  ///
  /// Parameters:
  ///   - target_source: The Solidity source code
  ///   - source_name: Optional source file name (defaults to "Contract.sol")
  ///   - target_contract_name: Optional contract name (defaults to last contract)
  #[napi(ts_return_type = "import('./foundry-types').SourceUnit")]
  pub fn stitch_into_source(
    &self,
    env: Env,
    target_source: String,
    source_name: Option<String>,
    target_contract_name: Option<String>,
  ) -> Result<JsUnknown> {
    let file_name = source_name.as_deref().unwrap_or("Contract.sol");

    let mut target_ast = parser::parse_source_ast(&target_source, file_name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    let stitched =
      self.stitch_into_ast_internal(&mut target_ast, target_contract_name.as_deref())?;
    to_js_value(&env, &stitched)
  }

  /// Stitch shadow nodes into an existing contract's AST
  /// Returns fully analyzed AST object
  ///
  /// Parameters:
  ///   - target_ast: The target AST as object
  ///   - target_contract_name: Optional contract name (defaults to last contract)
  ///   - source_name: Optional source file name (defaults to "Contract.sol")
  #[napi(ts_return_type = "import('./foundry-types').SourceUnit")]
  pub fn stitch_into_ast(
    &self,
    env: Env,
    target_ast: JsUnknown,
    target_contract_name: Option<String>,
    _source_name: Option<String>,
  ) -> Result<JsUnknown> {
    let mut target_ast_value: Value = from_js_value(&env, target_ast)?;

    let stitched =
      self.stitch_into_ast_internal(&mut target_ast_value, target_contract_name.as_deref())?;

    to_js_value(&env, &stitched)
  }

  /// Parse Solidity source code and return the strongly typed SourceUnit AST
  /// that foundry-compilers exposes as a plain JavaScript object. The returned
  /// value preserves the original shape and TypeScript bindings are patched via
  /// `foundry-types.ts`.
  // napi-ts-return: import('./foundry-types').SourceUnit
  #[napi(ts_return_type = "import('./foundry-types').SourceUnit")]
  pub fn parse_source_ast(
    env: Env,
    source: String,
    file_name: Option<String>,
  ) -> Result<JsUnknown> {
    let name = file_name.as_deref().unwrap_or("Contract.sol");

    let value = parser::parse_source_ast(&source, name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    to_js_value(&env, &value)
  }

  // Internal helper methods

  fn to_wrapped_ast(&self) -> std::result::Result<Value, ShadowError> {
    let wrapped = parser::wrap_shadow_source(&self.source);
    parser::parse_source_ast(&wrapped, "Shadow.sol")
  }

  fn stitch_into_ast_internal(
    &self,
    target_ast: &mut Value,
    target_contract_name: Option<&str>,
  ) -> Result<Value> {
    let shadow_ast = self
      .to_wrapped_ast()
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    let max_target_id = utils::find_max_id(target_ast);

    let contract_idx = stitcher::find_target_contract_index(target_ast, target_contract_name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    stitcher::stitch_shadow_nodes_into_contract(
      target_ast,
      contract_idx,
      &shadow_ast,
      max_target_id,
    )
    .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    Ok(target_ast.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::shadow::parser;

  const TARGET_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Target {
  uint256 private value;
}
"#;

  const SHADOW_FUNC: &str = "function added() public view returns (uint256) { return value; }";

  #[test]
  fn stitches_shadow_into_target_ast() {
    let shadow = Shadow::new(SHADOW_FUNC.to_string());
    let mut target_ast =
      parser::parse_source_ast(TARGET_CONTRACT, "Target.sol").expect("parse target");
    let stitched = shadow
      .stitch_into_ast_internal(&mut target_ast, Some("Target"))
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

    assert!(contains_added_fn, "stitched AST should contain added function");
  }
}
