use serde_json::{json, Value};
use std::collections::HashMap;

use crate::internal::config::ResolveConflictStrategy;

use super::{error::AstError, utils};

const CONTRACT_DEFINITION: &str = "ContractDefinition";

pub fn find_instrumented_contract_index(
  unit: &Value,
  contract_name: Option<&str>,
) -> Result<usize, AstError> {
  let nodes = unit
    .get("nodes")
    .and_then(|value| value.as_array())
    .ok_or_else(|| AstError::InvalidContractStructure("Source unit has no nodes array".into()))?;

  let mut fallback: Option<usize> = None;

  for (idx, node) in nodes.iter().enumerate() {
    if node_type(node) != Some(CONTRACT_DEFINITION) {
      continue;
    }
    if let Some(target) = contract_name {
      let name = node_name(node).unwrap_or_default();
      if name == target {
        return Ok(idx);
      }
    } else {
      fallback = Some(idx);
    }
  }

  contract_name
    .map(|name| {
      Err(AstError::InvalidContractStructure(format!(
        "Contract '{}' not found",
        name
      )))
    })
    .unwrap_or_else(|| {
      fallback.ok_or_else(|| {
        AstError::InvalidContractStructure("No ContractDefinition found".to_string())
      })
    })
}

pub fn stitch_fragment_nodes_into_contract(
  target: &mut Value,
  contract_idx: usize,
  fragment_contract: &Value,
  max_target_id: i64,
  strategy: ResolveConflictStrategy,
) -> Result<(), AstError> {
  let target_contract = contract_mut_at(target, contract_idx)?;
  let fragment_nodes = fragment_contract
    .get("nodes")
    .and_then(|value| value.as_array())
    .ok_or_else(|| {
      AstError::InvalidContractStructure("Fragment contract missing nodes array".to_string())
    })?;

  let target_nodes = target_contract
    .get_mut("nodes")
    .and_then(|value| value.as_array_mut())
    .ok_or_else(|| {
      AstError::InvalidContractStructure("Target contract missing nodes array".to_string())
    })?;

  let mut next_id = max_target_id;

  match strategy {
    ResolveConflictStrategy::Safe => {
      for part in fragment_nodes {
        let cloned = utils::clone_with_new_ids(part, &mut next_id);
        target_nodes.push(cloned);
      }
      Ok(())
    }
    ResolveConflictStrategy::Replace => stitch_replace(target_nodes, fragment_nodes, &mut next_id),
  }
}

fn stitch_replace(
  target_nodes: &mut Vec<Value>,
  fragment_nodes: &[Value],
  next_id: &mut i64,
) -> Result<(), AstError> {
  let mut target_index_by_key: HashMap<ConflictKey, (usize, Vec<i64>)> = HashMap::new();

  for (idx, node) in target_nodes.iter().enumerate() {
    if let Some(key) = contract_part_key(node)? {
      let mut ids = Vec::new();
      collect_ids(node, &mut ids);
      target_index_by_key.insert(key, (idx, ids));
    }
  }

  let mut replacements: Vec<(usize, Vec<i64>, Value)> = Vec::new();
  let mut append_nodes: Vec<Value> = Vec::new();

  for node in fragment_nodes {
    let candidate = if let Some(key) = contract_part_key(node)? {
      if let Some((idx, ids)) = target_index_by_key.remove(&key) {
        replacements.push((idx, ids, node.clone()));
        continue;
      }
      node.clone()
    } else {
      node.clone()
    };
    append_nodes.push(candidate);
  }

  replacements.sort_by_key(|(idx, _, _)| *idx);

  for (idx, snapshot, mut replacement) in replacements {
    apply_id_snapshot(&mut replacement, &snapshot, next_id);
    if let Some(slot) = target_nodes.get_mut(idx) {
      *slot = replacement;
    } else {
      return Err(AstError::InvalidContractStructure(
        "Replacement index out of bounds".to_string(),
      ));
    }
  }

  for node in append_nodes {
    let cloned = utils::clone_with_new_ids(&node, next_id);
    target_nodes.push(cloned);
  }

  Ok(())
}

fn node_type(value: &Value) -> Option<&str> {
  value.get("nodeType").and_then(|value| value.as_str())
}

fn node_name(value: &Value) -> Option<&str> {
  value.get("name").and_then(|value| value.as_str())
}

fn contract_mut_at<'a>(unit: &'a mut Value, idx: usize) -> Result<&'a mut Value, AstError> {
  let nodes = unit
    .get_mut("nodes")
    .and_then(|value| value.as_array_mut())
    .ok_or_else(|| AstError::InvalidContractStructure("Source unit has no nodes array".into()))?;

  let Some(node) = nodes.get_mut(idx) else {
    return Err(AstError::InvalidContractStructure(
      "Invalid contract index".to_string(),
    ));
  };

  if node_type(node) != Some(CONTRACT_DEFINITION) {
    return Err(AstError::InvalidContractStructure(
      "Target index is not a contract definition".to_string(),
    ));
  }

  Ok(node)
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ConflictKey {
  Function {
    name: String,
    signature: Vec<String>,
    kind: String,
  },
  Variable(String),
  Event(String),
  Error(String),
  Modifier(String),
  Struct(String),
  Enum(String),
  UserDefinedValueType(String),
}

fn contract_part_key(node: &Value) -> Result<Option<ConflictKey>, AstError> {
  match node_type(node) {
    Some("FunctionDefinition") => {
      let name = node_name(node).unwrap_or_default().to_string();
      let signature = function_signature(node)?;
      let kind = function_kind_tag(node);
      Ok(Some(ConflictKey::Function {
        name,
        signature,
        kind,
      }))
    }
    Some("VariableDeclaration") => {
      Ok(node_name(node).map(|name| ConflictKey::Variable(name.to_string())))
    }
    Some("EventDefinition") => Ok(node_name(node).map(|name| ConflictKey::Event(name.to_string()))),
    Some("ErrorDefinition") => Ok(node_name(node).map(|name| ConflictKey::Error(name.to_string()))),
    Some("ModifierDefinition") => {
      Ok(node_name(node).map(|name| ConflictKey::Modifier(name.to_string())))
    }
    Some("StructDefinition") => {
      Ok(node_name(node).map(|name| ConflictKey::Struct(name.to_string())))
    }
    Some("EnumDefinition") => Ok(node_name(node).map(|name| ConflictKey::Enum(name.to_string()))),
    Some("UserDefinedValueTypeDefinition") => {
      Ok(node_name(node).map(|name| ConflictKey::UserDefinedValueType(name.to_string())))
    }
    Some("UsingForDirective") => Ok(None),
    _ => Ok(None),
  }
}

pub(crate) fn function_signature(function: &Value) -> Result<Vec<String>, AstError> {
  let parameters = function
    .get("parameters")
    .and_then(|value| value.get("parameters"))
    .and_then(|value| value.as_array())
    .ok_or_else(|| {
      AstError::InvalidContractStructure(
        "FunctionDefinition parameters list is missing".to_string(),
      )
    })?;

  parameters
    .iter()
    .enumerate()
    .map(|(idx, param)| parameter_type_key(param, idx))
    .collect()
}

fn function_kind_tag(function: &Value) -> String {
  function
    .get("kind")
    .and_then(|value| value.as_str())
    .map(|kind| kind.to_string())
    .unwrap_or_else(|| "function".to_string())
}

fn parameter_type_key(param: &Value, idx: usize) -> Result<String, AstError> {
  if let Some(identifier) = param
    .get("typeDescriptions")
    .and_then(|value| value.get("typeIdentifier"))
    .and_then(|value| value.as_str())
  {
    return Ok(identifier.to_string());
  }
  if let Some(type_string) = param
    .get("typeDescriptions")
    .and_then(|value| value.get("typeString"))
    .and_then(|value| value.as_str())
  {
    return Ok(type_string.to_string());
  }
  if let Some(type_name) = param.get("typeName") {
    return serialise_without_ids(type_name);
  }
  Ok(format!("__anon_parameter_{}", idx))
}

fn serialise_without_ids(node: &Value) -> Result<String, AstError> {
  let mut clone = node.clone();
  drop_ids(&mut clone);
  serde_json::to_string(&clone).map_err(|err| AstError::JsonError(err.to_string()))
}

fn drop_ids(node: &mut Value) {
  match node {
    Value::Object(map) => {
      map.remove("id");
      for child in map.values_mut() {
        drop_ids(child);
      }
    }
    Value::Array(items) => {
      for item in items {
        drop_ids(item);
      }
    }
    _ => {}
  }
}

fn collect_ids(node: &Value, ids: &mut Vec<i64>) {
  match node {
    Value::Object(map) => {
      if let Some(Value::Number(num)) = map.get("id") {
        if let Some(id) = num.as_i64() {
          ids.push(id);
        }
      }
      for child in map.values() {
        collect_ids(child, ids);
      }
    }
    Value::Array(items) => {
      for item in items {
        collect_ids(item, ids);
      }
    }
    _ => {}
  }
}

fn apply_id_snapshot(node: &mut Value, snapshot: &[i64], next_id: &mut i64) {
  let mut cursor = 0usize;
  assign_ids_with_snapshot(node, snapshot, &mut cursor, next_id);
}

fn assign_ids_with_snapshot(
  node: &mut Value,
  snapshot: &[i64],
  cursor: &mut usize,
  next_id: &mut i64,
) {
  match node {
    Value::Object(map) => {
      if map.get("nodeType").is_some() {
        let replacement = if *cursor < snapshot.len() {
          let id = snapshot[*cursor];
          *cursor += 1;
          id
        } else {
          *next_id += 1;
          *next_id
        };
        map.insert("id".to_string(), json!(replacement));
      }
      for child in map.values_mut() {
        assign_ids_with_snapshot(child, snapshot, cursor, next_id);
      }
    }
    Value::Array(items) => {
      for item in items {
        assign_ids_with_snapshot(item, snapshot, cursor, next_id);
      }
    }
    _ => {}
  }
}
