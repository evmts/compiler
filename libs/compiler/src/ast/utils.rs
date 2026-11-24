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
  use proptest::prelude::*;
  use serde_json::json;

  fn ast_nodes() -> impl Strategy<Value = Value> {
    (any::<u16>(), any::<bool>())
      .prop_map(|(id, value)| json!({ "nodeType": "Literal", "id": id, "value": value }))
      .prop_recursive(5, 64, 4, |child| {
        (any::<u16>(), prop::collection::vec(child, 1..5)).prop_map(
          |(id, children)| json!({ "nodeType": "Block", "id": id, "statements": children }),
        )
      })
  }

  fn strip_ids(node: &mut Value, ids: &mut Vec<i64>) {
    let object = node.as_object_mut().expect("generated AST object");
    let id = object.remove("id").expect("node ID");
    ids.push(id.as_i64().expect("numeric node ID"));
    if let Some(Value::Array(children)) = object.get_mut("statements") {
      for child in children {
        strip_ids(child, ids);
      }
    }
  }

  proptest! {
    #[test]
    fn renumbering_preserves_tree_and_assigns_contiguous_ids(original in ast_nodes()) {
      let mut original_without_ids = original.clone();
      let mut original_ids = Vec::new();
      strip_ids(&mut original_without_ids, &mut original_ids);
      let highest_id = original_ids.into_iter().max().expect("nonempty tree");
      prop_assert_eq!(max_id(&original), highest_id);

      let mut next_id = highest_id;
      let mut cloned = clone_with_new_ids(&original, &mut next_id);
      let mut cloned_ids = Vec::new();
      strip_ids(&mut cloned, &mut cloned_ids);
      prop_assert_eq!(original_without_ids, cloned);
      prop_assert_eq!(next_id, highest_id + cloned_ids.len() as i64);
      prop_assert_eq!(cloned_ids, (highest_id + 1..=next_id).collect::<Vec<_>>());
    }
  }

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
