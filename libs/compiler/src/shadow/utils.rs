use serde_json::Value;

/// Find the maximum ID in an AST to avoid ID collisions when stitching
pub fn find_max_id(value: &Value) -> i64 {
  match value {
    Value::Object(map) => {
      let mut max_id = 0i64;

      if let Some(Value::Number(id)) = map.get("id") {
        if let Some(id_val) = id.as_i64() {
          max_id = max_id.max(id_val);
        }
      }

      for val in map.values() {
        let child_max = find_max_id(val);
        max_id = max_id.max(child_max);
      }

      max_id
    }
    Value::Array(arr) => arr.iter().map(find_max_id).max().unwrap_or(0),
    _ => 0,
  }
}

/// Renumber all IDs in an AST by adding an offset
pub fn renumber_ids(value: &mut Value, offset: i64) {
  match value {
    Value::Object(map) => {
      if let Some(Value::Number(id)) = map.get("id") {
        if let Some(id_val) = id.as_i64() {
          map.insert("id".to_string(), Value::Number((id_val + offset).into()));
        }
      }

      for val in map.values_mut() {
        renumber_ids(val, offset);
      }
    }
    Value::Array(arr) => {
      for val in arr.iter_mut() {
        renumber_ids(val, offset);
      }
    }
    _ => {}
  }
}

/// Check if a JSON node is a ContractDefinition
pub fn is_contract_definition(node: &Value) -> bool {
  node
    .get("nodeType")
    .and_then(|v| v.as_str())
    .map(|s| s == "ContractDefinition")
    .unwrap_or(false)
}

/// Get the name of a contract node
pub fn get_contract_name(node: &Value) -> Option<&str> {
  node.get("name").and_then(|v| v.as_str())
}
