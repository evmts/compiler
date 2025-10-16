use foundry_compilers::artifacts::ast::{ContractDefinition, SourceUnit};
use napi::{Env, JsUnknown};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::error::InstrumentError;

pub fn to_js_value<T>(env: &Env, value: &T) -> napi::Result<JsUnknown>
where
  T: Serialize,
{
  env.to_js_value(value)
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

pub fn max_id(unit: &SourceUnit) -> std::result::Result<i64, InstrumentError> {
  let value =
    serde_json::to_value(unit).map_err(|err| InstrumentError::JsonError(err.to_string()))?;
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
) -> std::result::Result<(), InstrumentError> {
  let mut value =
    serde_json::to_value(&*contract).map_err(|err| InstrumentError::JsonError(err.to_string()))?;
  let mut next = start_from;
  walk_renumber(&mut value, &mut next);
  sanitize_ast_value(&mut value);
  *contract =
    serde_json::from_value(value).map_err(|err| InstrumentError::JsonError(err.to_string()))?;
  Ok(())
}

pub fn sanitize_ast_value(value: &mut Value) {
  fn prune(value: &mut Value, parent_key: Option<&str>) -> bool {
    match value {
      Value::Object(map) => {
        let keys: Vec<String> = map.keys().cloned().collect();
        for key in keys {
          if let Some(child) = map.get_mut(&key) {
            prune(child, Some(&key));

            let mut remove_entry = false;
            match child {
              Value::Null => {
                if key == "typeDescriptions" {
                  *child = Value::Object(Default::default());
                } else {
                  remove_entry = true;
                }
              }
              Value::Object(obj) => {
                if key == "typeDescriptions" && obj.is_empty() {
                  // keep as empty object
                }
              }
              Value::Array(items) => {
                items.retain(|item| !item.is_null());
              }
              _ => {}
            }

            if remove_entry {
              map.remove(&key);
            }
          }
        }
        true
      }
      Value::Array(items) => {
        let mut idx = 0;
        while idx < items.len() {
          if prune(&mut items[idx], None) {
            idx += 1;
          } else {
            items.remove(idx);
          }
        }
        true
      }
      Value::Null => {
        if parent_key == Some("typeDescriptions") {
          *value = Value::Object(Default::default());
          true
        } else {
          false
        }
      }
      _ => true,
    }
  }

  prune(value, None);

  if let Value::Object(map) = value {
    map
      .entry("nodeType")
      .or_insert_with(|| Value::String("SourceUnit".to_string()));
  }
}
