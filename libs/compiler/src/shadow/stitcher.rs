use serde_json::Value;

use super::{error::ShadowError, utils};

/// Find the index of the target contract in the AST nodes array
/// If contract_name is None, returns the last ContractDefinition
pub fn find_target_contract_index(
  root: &Value,
  contract_name: Option<&str>,
) -> Result<usize, ShadowError> {
  let nodes = root
    .get("nodes")
    .and_then(|v| v.as_array())
    .ok_or_else(|| ShadowError::InvalidContractStructure("Missing nodes array".to_string()))?;

  if let Some(name) = contract_name {
    for (i, node) in nodes.iter().enumerate() {
      if !utils::is_contract_definition(node) {
        continue;
      }
      if let Some(node_name) = utils::get_contract_name(node) {
        if node_name == name {
          return Ok(i);
        }
      }
    }
    Err(ShadowError::InvalidContractStructure(format!(
      "Contract '{}' not found",
      name
    )))
  } else {
    let mut last_idx: Option<usize> = None;
    for (i, node) in nodes.iter().enumerate() {
      if utils::is_contract_definition(node) {
        last_idx = Some(i);
      }
    }
    last_idx.ok_or_else(|| {
      ShadowError::InvalidContractStructure("No ContractDefinition found".to_string())
    })
  }
}

/// Stitch shadow nodes into target contract
/// Modifies target_root in place
pub fn stitch_shadow_nodes_into_contract(
  target_root: &mut Value,
  contract_idx: usize,
  shadow_ast: &Value,
  max_target_id: i64,
) -> Result<(), ShadowError> {
  let shadow_nodes = shadow_ast
    .get("nodes")
    .and_then(|v| v.as_array())
    .ok_or_else(|| ShadowError::InvalidContractStructure("Shadow AST missing nodes".to_string()))?;

  if shadow_nodes.len() <= 1 {
    return Err(ShadowError::NoNodesFound);
  }

  let mut shadow_contract = shadow_nodes[1].clone();
  utils::renumber_ids(&mut shadow_contract, max_target_id);

  let shadow_contract_nodes = shadow_contract
    .get("nodes")
    .and_then(|v| v.as_array())
    .ok_or_else(|| {
      ShadowError::InvalidContractStructure("Shadow contract missing nodes".to_string())
    })?
    .clone();

  let target_nodes = target_root
    .get_mut("nodes")
    .and_then(|v| v.as_array_mut())
    .ok_or_else(|| ShadowError::InvalidContractStructure("Target AST missing nodes".to_string()))?;

  let target_contract = target_nodes
    .get_mut(contract_idx)
    .ok_or_else(|| ShadowError::InvalidContractStructure("Invalid contract index".to_string()))?;

  let target_contract_nodes = target_contract
    .get_mut("nodes")
    .and_then(|v| v.as_array_mut())
    .ok_or_else(|| {
      ShadowError::InvalidContractStructure("Target contract missing nodes".to_string())
    })?;

  for node in shadow_contract_nodes {
    target_contract_nodes.push(node);
  }

  Ok(())
}
