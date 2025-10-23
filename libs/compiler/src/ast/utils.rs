use napi::{Env, JsUnknown};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

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
      for child in map.values() {
        walk_max_id(child, max_id);
      }
    }
    Value::Array(items) => {
      for child in items {
        walk_max_id(child, max_id);
      }
    }
    _ => {}
  }
}

fn walk_renumber(node: &mut Value, next_id: &mut i64) {
  match node {
    Value::Object(map) => {
      if map.get("nodeType").is_some() {
        *next_id += 1;
        map.insert("id".to_string(), Value::Number((*next_id).into()));
      }
      for child in map.values_mut() {
        walk_renumber(child, next_id);
      }
    }
    Value::Array(items) => {
      for child in items {
        walk_renumber(child, next_id);
      }
    }
    _ => {}
  }
}

pub fn max_id(value: &Value) -> i64 {
  let mut max_id = 0;
  walk_max_id(value, &mut max_id);
  max_id
}

pub fn clone_with_new_ids(value: &Value, next_id: &mut i64) -> Value {
  let mut clone = value.clone();
  walk_renumber(&mut clone, next_id);
  clone
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn max_id_finds_highest_nested_identifier() {
    let value = json!({
      "nodeType": "SourceUnit",
      "id": 1,
      "nodes": [
        { "nodeType": "ContractDefinition", "id": 7 },
        {
          "nodeType": "PragmaDirective",
          "nodes": [
            { "nodeType": "Literal", "id": 3 }
          ]
        }
      ]
    });

    assert_eq!(max_id(&value), 7);
  }

  #[test]
  fn clone_with_new_ids_preserves_original_and_generates_unique_ids() {
    let original = json!({
      "nodeType": "FunctionDefinition",
      "id": 12,
      "body": {
        "nodeType": "Block",
        "id": 13,
        "statements": []
      }
    });

    let mut next_id = 20;
    let cloned = clone_with_new_ids(&original, &mut next_id);

    assert_eq!(next_id, 22);
    assert_eq!(original["id"], 12);
    assert_eq!(original["body"]["id"], 13);
    assert_eq!(cloned["id"], 21);
    assert_eq!(cloned["body"]["id"], 22);
  }

  #[test]
  fn clone_with_new_ids_assigns_ids_when_missing() {
    let original = json!({
      "nodeType": "Block",
      "statements": [
        { "nodeType": "ExpressionStatement" }
      ]
    });

    let mut next_id = 0;
    let cloned = clone_with_new_ids(&original, &mut next_id);

    assert!(cloned["id"].as_i64().is_some());
    let statements = cloned["statements"].as_array().unwrap();
    assert!(statements[0]["id"].as_i64().is_some());
    assert_eq!(next_id, 2);
  }
}
