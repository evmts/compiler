use napi::{Env, JsUnknown, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Number, Value};

pub fn to_js_value<T>(env: &Env, value: &T) -> Result<JsUnknown>
where
  T: Serialize,
{
  env.to_js_value(value)
}

pub fn from_js_value<T>(env: &Env, value: JsUnknown) -> Result<T>
where
  T: DeserializeOwned,
{
  env.from_js_value(value)
}

pub fn find_max_id(node: &Value) -> i64 {
  fn walk(value: &Value, max_id: &mut i64) {
    match value {
      Value::Object(map) => {
        if let Some(Value::Number(num)) = map.get("id") {
          if let Some(id) = num.as_u64() {
            *max_id = (*max_id).max(id as i64);
          }
        }
        for child in map.values() {
          walk(child, max_id);
        }
      }
      Value::Array(items) => {
        for item in items {
          walk(item, max_id);
        }
      }
      _ => {}
    }
  }

  let mut max_id = 0;
  walk(node, &mut max_id);
  max_id
}

pub fn renumber_ids(node: &mut Value, start_from: i64) {
  fn walk(value: &mut Value, next_id: &mut i64) {
    match value {
      Value::Object(map) => {
        if let Some(id_value) = map.get_mut("id") {
          *next_id += 1;
          *id_value = Value::Number(Number::from(*next_id));
        }
        for child in map.values_mut() {
          walk(child, next_id);
        }
      }
      Value::Array(items) => {
        for item in items {
          walk(item, next_id);
        }
      }
      _ => {}
    }
  }

  let mut counter = start_from;
  walk(node, &mut counter);
}

pub fn is_contract_definition(node: &Value) -> bool {
  node
    .get("nodeType")
    .and_then(Value::as_str)
    .map(|kind| kind == "ContractDefinition")
    .unwrap_or(false)
}

pub fn get_contract_name(node: &Value) -> Option<&str> {
  node.get("name").and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn sample_ast() -> Value {
    json!({
      "nodeType": "SourceUnit",
      "nodes": [
        {
          "nodeType": "ContractDefinition",
          "name": "Sample",
          "id": 1,
          "nodes": [
            { "nodeType": "FunctionDefinition", "id": 2 },
            { "nodeType": "VariableDeclaration", "id": 3 }
          ]
        }
      ]
    })
  }

  #[test]
  fn finds_max_id_in_ast() {
    let mut ast = sample_ast();
    assert_eq!(find_max_id(&ast), 3);

    renumber_ids(&mut ast, 10);
    assert!(find_max_id(&ast) > 10);
  }

  #[test]
  fn identifies_contract_definitions() {
    let ast = sample_ast();
    let nodes = ast.get("nodes").and_then(Value::as_array).unwrap();
    assert!(is_contract_definition(&nodes[0]));
  }

  #[test]
  fn extracts_contract_name() {
    let ast = sample_ast();
    let nodes = ast.get("nodes").and_then(Value::as_array).unwrap();
    assert_eq!(get_contract_name(&nodes[0]), Some("Sample"));
  }
}
