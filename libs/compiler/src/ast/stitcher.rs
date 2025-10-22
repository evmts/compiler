use foundry_compilers::artifacts::ast::{
  ContractDefinition, ContractDefinitionPart, FunctionDefinition, SourceUnit, SourceUnitPart,
  VariableDeclaration,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::internal::config::ResolveConflictStrategy;

use super::{error::AstError, utils};

pub fn find_instrumented_contract_index(
  unit: &SourceUnit,
  contract_name: Option<&str>,
) -> Result<usize, AstError> {
  let mut fallback: Option<usize> = None;

  for (idx, part) in unit.nodes.iter().enumerate() {
    let SourceUnitPart::ContractDefinition(contract) = part else {
      continue;
    };
    let name = &contract.name;
    if let Some(target) = contract_name {
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
  target: &mut SourceUnit,
  contract_idx: usize,
  fragment_contract: &ContractDefinition,
  max_target_id: i64,
  strategy: ResolveConflictStrategy,
) -> Result<(), AstError> {
  let SourceUnitPart::ContractDefinition(target_contract) = target
    .nodes
    .get_mut(contract_idx)
    .ok_or_else(|| AstError::InvalidContractStructure("Invalid contract index".to_string()))?
  else {
    return Err(AstError::InvalidContractStructure(
      "Target index is not a contract".to_string(),
    ));
  };

  match strategy {
    ResolveConflictStrategy::Safe => {
      let mut fragment = fragment_contract.clone();
      utils::renumber_contract_definition(&mut fragment, max_target_id)?;
      target_contract
        .nodes
        .extend(fragment.nodes.into_iter().map(resolve_contract_part));
      Ok(())
    }
    ResolveConflictStrategy::Replace => {
      let mut next_id = max_target_id;
      let mut target_index_by_key: HashMap<ConflictKey, (usize, Vec<i64>)> = HashMap::new();
      for (idx, part) in target_contract.nodes.iter().enumerate() {
        if let Some(key) = contract_part_key(part)? {
          let ids = collect_ids(part)?;
          target_index_by_key.insert(key, (idx, ids));
        }
      }

      let mut replacements: Vec<(usize, Vec<i64>, ContractDefinitionPart)> = Vec::new();
      let mut append_nodes: Vec<ContractDefinitionPart> = Vec::new();

      for part in fragment_contract
        .nodes
        .iter()
        .cloned()
        .map(resolve_contract_part)
      {
        if let Some(key) = contract_part_key(&part)? {
          if let Some((idx, ids)) = target_index_by_key.remove(&key) {
            replacements.push((idx, ids, part));
            continue;
          }
        }
        append_nodes.push(part);
      }

      for (idx, ids, mut part) in replacements {
        renumber_part_with_snapshot(&mut part, &ids, &mut next_id)?;
        let slot = target_contract.nodes.get_mut(idx).ok_or_else(|| {
          AstError::InvalidContractStructure("Replacement index out of bounds".to_string())
        })?;
        *slot = part;
      }

      for mut part in append_nodes {
        renumber_part_with_snapshot(&mut part, &[], &mut next_id)?;
        target_contract.nodes.push(part);
      }

      Ok(())
    }
  }
}

fn resolve_contract_part(part: ContractDefinitionPart) -> ContractDefinitionPart {
  part
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

fn contract_part_key(part: &ContractDefinitionPart) -> Result<Option<ConflictKey>, AstError> {
  match part {
    ContractDefinitionPart::FunctionDefinition(function) => Ok(Some(ConflictKey::Function {
      name: function.name.clone(),
      signature: function_signature(function)?,
      kind: function_kind_tag(function),
    })),
    ContractDefinitionPart::VariableDeclaration(variable) => {
      Ok(Some(ConflictKey::Variable(variable.name.clone())))
    }
    ContractDefinitionPart::EventDefinition(event) => {
      Ok(Some(ConflictKey::Event(event.name.clone())))
    }
    ContractDefinitionPart::ErrorDefinition(error) => {
      Ok(Some(ConflictKey::Error(error.name.clone())))
    }
    ContractDefinitionPart::ModifierDefinition(modifier) => {
      Ok(Some(ConflictKey::Modifier(modifier.name.clone())))
    }
    ContractDefinitionPart::StructDefinition(struct_definition) => {
      Ok(Some(ConflictKey::Struct(struct_definition.name.clone())))
    }
    ContractDefinitionPart::EnumDefinition(enum_definition) => {
      Ok(Some(ConflictKey::Enum(enum_definition.name.clone())))
    }
    ContractDefinitionPart::UserDefinedValueTypeDefinition(value_type) => Ok(Some(
      ConflictKey::UserDefinedValueType(value_type.name.clone()),
    )),
    ContractDefinitionPart::UsingForDirective(_) => Ok(None),
  }
}

pub(crate) fn function_signature(function: &FunctionDefinition) -> Result<Vec<String>, AstError> {
  function
    .parameters
    .parameters
    .iter()
    .enumerate()
    .map(|(idx, param)| parameter_type_key(param, idx))
    .collect()
}

fn function_kind_tag(function: &FunctionDefinition) -> String {
  match function.kind() {
    foundry_compilers::artifacts::ast::FunctionKind::Constructor => "constructor",
    foundry_compilers::artifacts::ast::FunctionKind::Function => "function",
    foundry_compilers::artifacts::ast::FunctionKind::Fallback => "fallback",
    foundry_compilers::artifacts::ast::FunctionKind::Receive => "receive",
    foundry_compilers::artifacts::ast::FunctionKind::FreeFunction => "free",
  }
  .to_string()
}

fn parameter_type_key(param: &VariableDeclaration, idx: usize) -> Result<String, AstError> {
  if let Some(identifier) = &param.type_descriptions.type_identifier {
    return Ok(identifier.clone());
  }
  if let Some(type_string) = &param.type_descriptions.type_string {
    return Ok(type_string.clone());
  }
  if let Some(type_name) = &param.type_name {
    return serialise_without_ids(type_name);
  }
  Ok(format!("__unknown_{}", idx))
}

fn serialise_without_ids<T: Serialize>(value: &T) -> Result<String, AstError> {
  let mut json = serde_json::to_value(value).map_err(|err| AstError::JsonError(err.to_string()))?;
  strip_ids(&mut json);
  serde_json::to_string(&json).map_err(|err| AstError::JsonError(err.to_string()))
}

fn strip_ids(node: &mut Value) {
  match node {
    Value::Object(map) => {
      map.remove("id");
      map.remove("src");
      for child in map.values_mut() {
        strip_ids(child);
      }
    }
    Value::Array(items) => {
      for item in items {
        strip_ids(item);
      }
    }
    _ => {}
  }
}

fn collect_ids(part: &ContractDefinitionPart) -> Result<Vec<i64>, AstError> {
  let json = serde_json::to_value(part).map_err(|err| AstError::JsonError(err.to_string()))?;
  let mut ids = Vec::new();
  collect_ids_from_value(&json, &mut ids);
  Ok(ids)
}

fn collect_ids_from_value(node: &Value, ids: &mut Vec<i64>) {
  match node {
    Value::Object(map) => {
      if let Some(Value::Number(num)) = map.get("id") {
        if let Some(id) = num.as_i64() {
          ids.push(id);
        }
      }
      for child in map.values() {
        collect_ids_from_value(child, ids);
      }
    }
    Value::Array(items) => {
      for item in items {
        collect_ids_from_value(item, ids);
      }
    }
    _ => {}
  }
}

fn renumber_part_with_snapshot(
  part: &mut ContractDefinitionPart,
  snapshot: &[i64],
  next_id: &mut i64,
) -> Result<(), AstError> {
  let mut json =
    serde_json::to_value(&*part).map_err(|err| AstError::JsonError(err.to_string()))?;
  let mut snapshot_iter = snapshot.iter();
  assign_ids_with_snapshot(&mut json, &mut snapshot_iter, next_id);
  utils::sanitize_ast_value(&mut json);
  *part = serde_json::from_value(json).map_err(|err| AstError::JsonError(err.to_string()))?;
  Ok(())
}

fn assign_ids_with_snapshot(
  node: &mut Value,
  snapshot: &mut std::slice::Iter<'_, i64>,
  next_id: &mut i64,
) {
  match node {
    Value::Object(map) => {
      if let Some(id_value) = map.get_mut("id") {
        if let Some(source_id) = snapshot.next() {
          *next_id = (*next_id).max(*source_id);
          *id_value = Value::Number(serde_json::Number::from(*source_id));
        } else {
          *next_id += 1;
          let assigned = *next_id;
          *id_value = Value::Number(serde_json::Number::from(assigned));
        }
      }
      for child in map.values_mut() {
        assign_ids_with_snapshot(child, snapshot, next_id);
      }
    }
    Value::Array(items) => {
      for item in items {
        assign_ids_with_snapshot(item, snapshot, next_id);
      }
    }
    _ => {}
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::{orchestrator::AstOrchestrator, parser, utils};
  use crate::internal::{config::ResolveConflictStrategy, solc};
  use foundry_compilers::solc::Solc;

  const MULTI_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {}
contract Second {}
contract Target {}
"#;

  const FRAGMENT: &str = "function hello() public {}";
  const TARGET_WITH_FUNCTION: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Target {
  function hello() public pure returns (uint256) {
    return 1;
  }
}
"#;

  const REPLACEMENT_FRAGMENT: &str = r#"
function hello() public pure returns (uint256) {
  return 2;
}
uint256 public replacementCounter;
"#;

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn locates_contract_by_name() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, Some("Target")).unwrap();
    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };
    assert_eq!(contract.name, "Target");
  }

  #[test]
  fn falls_back_to_last_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, None).unwrap();
    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };
    assert_eq!(contract.name, "Target");
  }

  #[test]
  fn stitches_fragment_into_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let mut unit = parser::parse_source_ast(MULTI_CONTRACT, "Multi.sol", &solc, &settings).unwrap();
    let fragment = parser::parse_fragment_contract(FRAGMENT, &solc, &settings).unwrap();
    let idx = find_instrumented_contract_index(&unit, Some("Target")).unwrap();
    let max_id = utils::max_id(&unit).unwrap();

    stitch_fragment_nodes_into_contract(
      &mut unit,
      idx,
      &fragment,
      max_id,
      ResolveConflictStrategy::Safe,
    )
    .unwrap();

    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };

    assert!(contract.nodes.iter().any(|part| matches!(part,
      ContractDefinitionPart::FunctionDefinition(function) if function.name == "hello"
    )));
  }

  #[test]
  fn safe_strategy_retains_conflicting_members() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let mut unit = parser::parse_source_ast(TARGET_WITH_FUNCTION, "Target.sol", &solc, &settings)
      .expect("parse target source");
    let fragment =
      parser::parse_fragment_contract(REPLACEMENT_FRAGMENT, &solc, &settings).expect("fragment");
    let idx = find_instrumented_contract_index(&unit, Some("Target")).expect("target index");
    let max_id = utils::max_id(&unit).expect("max target id");

    stitch_fragment_nodes_into_contract(
      &mut unit,
      idx,
      &fragment,
      max_id,
      ResolveConflictStrategy::Safe,
    )
    .expect("stitch safe");

    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };
    let hello_functions = contract
      .nodes
      .iter()
      .filter(|part| {
        matches!(part, ContractDefinitionPart::FunctionDefinition(function) if function.name == "hello")
      })
      .count();
    assert_eq!(hello_functions, 2);
  }

  #[test]
  fn replace_strategy_overwrites_conflicting_function_and_appends_new_members() {
    use foundry_compilers::artifacts::ast::{Expression, Statement};

    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let mut unit = parser::parse_source_ast(TARGET_WITH_FUNCTION, "Target.sol", &solc, &settings)
      .expect("parse target");
    let idx = find_instrumented_contract_index(&unit, Some("Target")).expect("target index");
    let max_id = utils::max_id(&unit).expect("max target id");

    let original_function_id = {
      let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
        panic!("Expected contract definition");
      };
      contract
        .nodes
        .iter()
        .find_map(|part| match part {
          ContractDefinitionPart::FunctionDefinition(function) if function.name == "hello" => {
            Some(function.id)
          }
          _ => None,
        })
        .expect("original hello function")
    };

    let fragment =
      parser::parse_fragment_contract(REPLACEMENT_FRAGMENT, &solc, &settings).expect("fragment");

    stitch_fragment_nodes_into_contract(
      &mut unit,
      idx,
      &fragment,
      max_id,
      ResolveConflictStrategy::Replace,
    )
    .expect("stitch replace");

    let SourceUnitPart::ContractDefinition(contract) = &unit.nodes[idx] else {
      panic!("Expected contract definition");
    };

    let replaced_function = contract
      .nodes
      .iter()
      .find_map(|part| match part {
        ContractDefinitionPart::FunctionDefinition(function) if function.name == "hello" => {
          Some(function)
        }
        _ => None,
      })
      .expect("replaced function present");

    assert_eq!(replaced_function.id, original_function_id);

    let body = replaced_function
      .body
      .as_ref()
      .expect("function body present");
    let Statement::Return(ret) = body.statements.first().expect("return statement present") else {
      panic!("expected return statement");
    };
    let Expression::Literal(literal) = ret.expression.as_ref().expect("return expression present")
    else {
      panic!("expected literal expression");
    };
    assert_eq!(literal.value.as_deref(), Some("2"));

    let appended_variable = contract
      .nodes
      .iter()
      .find_map(|part| match part {
        ContractDefinitionPart::VariableDeclaration(variable)
          if variable.name == "replacementCounter" =>
        {
          Some(variable)
        }
        _ => None,
      })
      .expect("appended variable present");
    assert!((appended_variable.id as i64) > max_id);
  }
}
