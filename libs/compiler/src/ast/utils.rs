use foundry_compilers::artifacts::ast::{ContractDefinition, SourceUnit};
use napi::{Env, JsUnknown};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::error::AstError;

pub fn to_js_value<T>(env: &Env, value: &T) -> napi::Result<JsUnknown>
where
  T: Serialize,
{
  env.to_js_value(value)
}

pub fn clone_with_new_ids<T>(value: &T, next_id: &mut i64) -> std::result::Result<T, AstError>
where
  T: Serialize + serde::de::DeserializeOwned,
{
  let mut json = serde_json::to_value(value).map_err(|err| AstError::JsonError(err.to_string()))?;
  walk_renumber(&mut json, next_id);
  sanitize_ast_value(&mut json);
  serde_json::from_value(json).map_err(|err| AstError::JsonError(err.to_string()))
}

pub fn from_js_value<T>(env: &Env, value: JsUnknown) -> napi::Result<T>
where
  T: DeserializeOwned,
{
  env.from_js_value(value)
}

fn walk_max_id(node: &Value, max_id: &mut i64) {
  match node {
    Value::Object(map) => {
      if let Some(Value::Number(num)) = map.get("id") {
        if let Some(id) = num.as_i64() {
          *max_id = (*max_id).max(id);
        }
      }
      map.values().for_each(|child| walk_max_id(child, max_id));
    }
    Value::Array(items) => items.iter().for_each(|child| walk_max_id(child, max_id)),
    _ => {}
  }
}

pub fn max_id(unit: &SourceUnit) -> std::result::Result<i64, AstError> {
  let value = serde_json::to_value(unit).map_err(|err| AstError::JsonError(err.to_string()))?;
  let mut max_id = 0;
  walk_max_id(&value, &mut max_id);
  Ok(max_id)
}

fn walk_renumber(node: &mut Value, next_id: &mut i64) {
  match node {
    Value::Object(map) => {
      if let Some(id_value) = map.get_mut("id") {
        *next_id += 1;
        *id_value = Value::Number((*next_id).into());
      }
      map
        .values_mut()
        .for_each(|child| walk_renumber(child, next_id));
    }
    Value::Array(items) => items
      .iter_mut()
      .for_each(|child| walk_renumber(child, next_id)),
    _ => {}
  }
}

pub fn renumber_contract_definition(
  contract: &mut ContractDefinition,
  start_from: i64,
) -> std::result::Result<(), AstError> {
  let mut value =
    serde_json::to_value(&*contract).map_err(|err| AstError::JsonError(err.to_string()))?;
  let mut next = start_from;
  walk_renumber(&mut value, &mut next);
  sanitize_ast_value(&mut value);
  *contract = serde_json::from_value(value).map_err(|err| AstError::JsonError(err.to_string()))?;
  Ok(())
}

pub fn sanitize_ast_value(value: &mut Value) {
  fn sanitize(node: &mut Value, parent_key: Option<&str>) -> bool {
    match node {
      Value::Object(map) => {
        let keys: Vec<String> = map.keys().cloned().collect();
        for key in keys {
          if let Some(child) = map.get_mut(&key) {
            if !sanitize(child, Some(&key)) {
              map.remove(&key);
            }
          }
        }
        true
      }
      Value::Array(items) => {
        let mut idx = 0;
        while idx < items.len() {
          if sanitize(&mut items[idx], None) {
            idx += 1;
          } else {
            items.remove(idx);
          }
        }
        true
      }
      Value::Null => {
        if parent_key == Some("typeDescriptions") {
          *node = Value::Object(Default::default());
          true
        } else {
          false
        }
      }
      _ => true,
    }
  }

  fn ensure_array(map: &mut serde_json::Map<String, Value>, key: &str) {
    match map.get_mut(key) {
      Some(Value::Array(_)) => {}
      Some(Value::Null) => {
        map.insert(key.to_string(), Value::Array(Vec::new()));
      }
      Some(other) => {
        if !other.is_array() {
          *other = Value::Array(Vec::new());
        }
      }
      None => {
        map.insert(key.to_string(), Value::Array(Vec::new()));
      }
    }
  }

  fn apply_defaults(node: &mut Value) {
    match node {
      Value::Object(map) => {
        if let Some(Value::String(node_type)) = map.get("nodeType") {
          if node_type == "ContractDefinition" {
            ensure_array(map, "baseContracts");
            ensure_array(map, "contractDependencies");
          }
        }
        for child in map.values_mut() {
          apply_defaults(child);
        }
      }
      Value::Array(items) => {
        for item in items {
          apply_defaults(item);
        }
      }
      _ => {}
    }
  }

  sanitize(value, None);
  apply_defaults(value);

  if let Value::Object(map) = value {
    map
      .entry("nodeType")
      .or_insert_with(|| Value::String("SourceUnit".to_string()));
  }
}
