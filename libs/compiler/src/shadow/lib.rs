use napi::bindgen_prelude::*;
use serde_json::Value;

use super::{error::ShadowError, parser, stitcher, utils};

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

  /// Parse and extract all nodes from the shadow AST as JSON strings
  /// Returns an array of AST node JSON strings
  #[napi]
  pub fn to_ast_nodes(&self) -> Result<Vec<String>> {
    let ast = self
      .to_wrapped_ast()
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    let nodes = ast
      .get("nodes")
      .and_then(|v| v.as_array())
      .ok_or_else(|| Error::new(Status::GenericFailure, "Missing nodes array"))?;

    if nodes.len() <= 1 {
      return Err(Error::new(Status::GenericFailure, "No nodes found"));
    }

    let contract = &nodes[1];
    let contract_nodes = contract
      .get("nodes")
      .and_then(|v| v.as_array())
      .ok_or_else(|| Error::new(Status::GenericFailure, "Contract missing nodes"))?;

    if contract_nodes.is_empty() {
      return Err(Error::new(Status::GenericFailure, "No nodes found"));
    }

    let result: Vec<String> = contract_nodes
      .iter()
      .map(|node| {
        serde_json::to_string(node)
          .map_err(|e| Error::new(Status::GenericFailure, format!("JSON error: {}", e)))
      })
      .collect::<Result<Vec<String>>>()?;

    Ok(result)
  }

  /// Stitch shadow nodes into an existing contract's source code
  /// Convenience wrapper that parses the source first, then stitches ASTs
  /// Returns fully analyzed AST JSON string
  ///
  /// Parameters:
  ///   - target_source: The Solidity source code
  ///   - source_name: Optional source file name (defaults to "Contract.sol")
  ///   - target_contract_name: Optional contract name (defaults to last contract)
  #[napi]
  pub fn stitch_into_source(
    &self,
    target_source: String,
    source_name: Option<String>,
    target_contract_name: Option<String>,
  ) -> Result<String> {
    let file_name = source_name.as_deref().unwrap_or("Contract.sol");

    let target_ast = parser::parse_source_ast(&target_source, file_name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    self.stitch_into_ast_internal(target_ast, target_contract_name.as_deref(), file_name)
  }

  /// Stitch shadow nodes into an existing contract's AST
  /// Returns fully analyzed AST JSON string
  ///
  /// Parameters:
  ///   - target_ast_json: The target AST as JSON string
  ///   - target_contract_name: Optional contract name (defaults to last contract)
  ///   - source_name: Optional source file name (defaults to "Contract.sol")
  #[napi]
  pub fn stitch_into_ast(
    &self,
    target_ast_json: String,
    target_contract_name: Option<String>,
    source_name: Option<String>,
  ) -> Result<String> {
    let target_ast: Value = serde_json::from_str(&target_ast_json)
      .map_err(|e| Error::new(Status::GenericFailure, format!("JSON error: {}", e)))?;

    let file_name = source_name.as_deref().unwrap_or("Contract.sol");

    self.stitch_into_ast_internal(target_ast, target_contract_name.as_deref(), file_name)
  }

  /// Parse Solidity source code to AST JSON
  /// Static utility method for parsing any Solidity source
  #[napi]
  pub fn parse_source_ast_static(source: String, file_name: Option<String>) -> Result<String> {
    let name = file_name.as_deref().unwrap_or("Contract.sol");

    let ast = parser::parse_source_ast(&source, name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    serde_json::to_string(&ast)
      .map_err(|e| Error::new(Status::GenericFailure, format!("JSON error: {}", e)))
  }

  // Internal helper methods

  fn to_wrapped_ast(&self) -> std::result::Result<Value, ShadowError> {
    let wrapped = parser::wrap_shadow_source(&self.source);
    parser::parse_source_ast(&wrapped, "Shadow.sol")
  }

  fn stitch_into_ast_internal(
    &self,
    mut target_ast: Value,
    target_contract_name: Option<&str>,
    _file_name: &str,
  ) -> Result<String> {
    let shadow_ast = self
      .to_wrapped_ast()
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    let max_target_id = utils::find_max_id(&target_ast);

    let contract_idx = stitcher::find_target_contract_index(&target_ast, target_contract_name)
      .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    stitcher::stitch_shadow_nodes_into_contract(
      &mut target_ast,
      contract_idx,
      &shadow_ast,
      max_target_id,
    )
    .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

    // Return the stitched AST directly (parsed AST, no semantic analysis)
    serde_json::to_string(&target_ast)
      .map_err(|e| Error::new(Status::GenericFailure, format!("JSON error: {}", e)))
  }
}
