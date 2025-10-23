use std::path::PathBuf;

use foundry_compilers::artifacts::{Settings, SolcInput, SolcLanguage, Source, Sources};
use foundry_compilers::solc::Solc;
use serde_json::{Map, Value};

use crate::internal::errors::{map_err_with_context, Error, Result};

use super::{
  orchestrator::AstOrchestrator,
  parser, stitcher,
  utils::{self},
};

#[derive(Debug)]
enum FunctionSelectorKind {
  Canonical {
    name: String,
    signature: Vec<String>,
  },
  Name(String),
  Fallback,
  Receive,
  Constructor,
}

pub fn inject_edges(
  unit: &mut Value,
  contract_idx: usize,
  selector: &str,
  before_snippets: &[String],
  after_snippets: &[String],
  solc: &Solc,
  settings: &Settings,
) -> Result<()> {
  if before_snippets.is_empty() && after_snippets.is_empty() {
    return Err(Error::new(
      "injectShadowAtEdges requires a `before` and/or `after` snippet.",
    ));
  }

  let mut next_id = utils::max_id(unit);

  let contract = contract_mut_at(unit, contract_idx)?;
  let selector_kind = parse_selector(selector, solc, settings)?;
  let function = resolve_function_mut(contract, &selector_kind)?;

  let body = function
    .get_mut("body")
    .ok_or_else(|| Error::new("Cannot instrument a function without an implementation"))?;

  if body.is_null() {
    return Err(Error::new(
      "Cannot instrument a function without an implementation",
    ));
  }

  ensure_no_inline_assembly(body)?;

  let before_statements = parse_statements(before_snippets, solc, settings)?;
  let after_statements = parse_statements(after_snippets, solc, settings)?;

  if !before_statements.is_empty() || !after_statements.is_empty() {
    let statements = body
      .get_mut("statements")
      .and_then(|value| value.as_array_mut())
      .ok_or_else(|| Error::new("Function body missing statements array"))?;

    if !before_statements.is_empty() {
      let mut clones = clone_statements(&before_statements, &mut next_id);
      for (offset, statement) in clones.drain(..).enumerate() {
        statements.insert(offset, statement);
      }
    }

    if !after_statements.is_empty() {
      inject_after(statements, &after_statements, &mut next_id)?;
      let mut tail = clone_statements(&after_statements, &mut next_id);
      statements.append(&mut tail);
    }
  }

  Ok(())
}

fn parse_selector(
  signature: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<FunctionSelectorKind> {
  let trimmed = signature.trim();
  if trimmed.eq_ignore_ascii_case("fallback") {
    return Ok(FunctionSelectorKind::Fallback);
  }
  if trimmed.eq_ignore_ascii_case("receive") {
    return Ok(FunctionSelectorKind::Receive);
  }
  if trimmed.eq_ignore_ascii_case("constructor") {
    return Ok(FunctionSelectorKind::Constructor);
  }

  if let Some(open) = trimmed.find('(') {
    let close = trimmed
      .rfind(')')
      .ok_or_else(|| Error::new("Function signature must close with ')'."))?;
    if close < open {
      return Err(Error::new("Malformed function signature."));
    }
    let name = trimmed[..open].trim().to_string();
    let params = trimmed[open + 1..close].trim();
    let fragment = format!("function {}({}) external {{}}", name, params);
    let contract = map_err_with_context(
      AstOrchestrator::parse_fragment_contract(&fragment, solc, settings),
      "Failed to parse selector signature",
    )?;
    let function = first_function_definition(&contract)
      .ok_or_else(|| Error::new("Failed to parse function signature"))?;
    let signature = stitcher::function_signature(function)
      .map_err(|err| Error::new(format!("Failed to compute function signature: {}", err)))?;
    return Ok(FunctionSelectorKind::Canonical { name, signature });
  }

  Ok(FunctionSelectorKind::Name(trimmed.to_string()))
}

fn first_function_definition(contract: &Value) -> Option<&Value> {
  contract
    .get("nodes")
    .and_then(|value| value.as_array())
    .and_then(|nodes| {
      nodes.iter().find(|node| {
        node
          .get("nodeType")
          .and_then(|value| value.as_str())
          .map(|node_type| node_type == "FunctionDefinition")
          .unwrap_or(false)
      })
    })
}

fn resolve_function_mut<'a>(
  contract: &'a mut Value,
  selector: &FunctionSelectorKind,
) -> Result<&'a mut Value> {
  let nodes = contract
    .get_mut("nodes")
    .and_then(|value| value.as_array_mut())
    .ok_or_else(|| Error::new("Contract has no members to instrument"))?;

  let mut matches: Vec<usize> = Vec::new();

  for (idx, node) in nodes.iter().enumerate() {
    if node_type(node) != Some("FunctionDefinition") {
      continue;
    }
    match selector {
      FunctionSelectorKind::Fallback => {
        if matches!(function_kind(node), Some("fallback")) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Receive => {
        if matches!(function_kind(node), Some("receive")) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Constructor => {
        if matches!(function_kind(node), Some("constructor")) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Canonical { name, signature } => {
        if node_name(node) == Some(name.as_str()) {
          let current_signature =
            stitcher::function_signature(node).map_err(|err| Error::new(err.to_string()))?;
          if &current_signature == signature {
            matches.push(idx);
          }
        }
      }
      FunctionSelectorKind::Name(name) => {
        if node_name(node) == Some(name.as_str()) {
          matches.push(idx);
        }
      }
    }
  }

  if matches.is_empty() {
    return Err(Error::new(
      "Target function not found for injectShadowAtEdges.",
    ));
  }

  if matches.len() > 1 {
    return Err(Error::new(
      "Function name is ambiguous. Please provide a full function signature.",
    ));
  }

  let idx = matches[0];
  nodes
    .get_mut(idx)
    .ok_or_else(|| Error::new("Invalid function index after resolution"))
}

fn ensure_no_inline_assembly(body: &Value) -> Result<()> {
  let statements = body
    .get("statements")
    .and_then(|value| value.as_array())
    .ok_or_else(|| Error::new("Function body missing statements array"))?;
  for statement in statements {
    ensure_no_inline_assembly_in_statement(statement)?;
  }
  Ok(())
}

fn ensure_no_inline_assembly_in_statement(statement: &Value) -> Result<()> {
  match node_type(statement) {
    Some("InlineAssembly") => Err(Error::new(
      "injectShadowAtEdges does not support functions that contain inline assembly.",
    )),
    Some("Block") | Some("UncheckedBlock") => {
      let statements = statement
        .get("statements")
        .and_then(|value| value.as_array())
        .ok_or_else(|| Error::new("Block missing statements array"))?;
      for child in statements {
        ensure_no_inline_assembly_in_statement(child)?;
      }
      Ok(())
    }
    Some("IfStatement") => {
      if let Some(true_body) = statement.get("trueBody") {
        ensure_no_inline_assembly_in_statement(true_body)?;
      }
      if let Some(false_body) = statement.get("falseBody") {
        ensure_no_inline_assembly_in_statement(false_body)?;
      }
      Ok(())
    }
    Some("WhileStatement") | Some("ForStatement") => {
      if let Some(body) = statement.get("body") {
        ensure_no_inline_assembly_in_statement(body)?;
      }
      Ok(())
    }
    Some("DoWhileStatement") => {
      if let Some(body) = statement.get("body") {
        ensure_no_inline_assembly_in_statement(body)?;
      }
      Ok(())
    }
    Some("TryStatement") => {
      if let Some(clauses) = statement.get("clauses").and_then(|value| value.as_array()) {
        for clause in clauses {
          if let Some(block) = clause.get("block") {
            ensure_no_inline_assembly_in_statement(block)?;
          }
        }
      }
      Ok(())
    }
    _ => Ok(()),
  }
}

fn parse_statements(snippets: &[String], solc: &Solc, settings: &Settings) -> Result<Vec<Value>> {
  if snippets.is_empty() {
    return Ok(Vec::new());
  }

  let joined = snippets
    .iter()
    .map(|snippet| snippet.trim())
    .filter(|snippet| !snippet.is_empty())
    .collect::<Vec<_>>();

  if joined.is_empty() {
    return Ok(Vec::new());
  }

  let mut fragment_lines = Vec::new();
  fragment_lines.push("  function __TevmShadow() internal {".to_string());
  fragment_lines.push(
    joined
      .iter()
      .map(|snippet| format!("    {}", snippet))
      .collect::<Vec<_>>()
      .join("\n"),
  );
  fragment_lines.push("  }".to_string());

  let fragment = fragment_lines.join("\n");

  let contract = parse_fragment_contract(&fragment, solc, settings)?;
  let function = first_function_definition(&contract)
    .ok_or_else(|| Error::new("Failed to parse instrumentation snippets"))?;
  let body = function
    .get("body")
    .ok_or_else(|| Error::new("Instrumentation snippet produced no body statements"))?;
  if body.is_null() {
    return Err(Error::new(
      "Instrumentation snippet produced no body statements.",
    ));
  }
  let statements = body
    .get("statements")
    .and_then(|value| value.as_array())
    .ok_or_else(|| Error::new("Instrumentation snippet missing statements array"))?;
  Ok(statements.to_vec())
}

fn clone_statements(statements: &[Value], next_id: &mut i64) -> Vec<Value> {
  statements
    .iter()
    .map(|statement| utils::clone_with_new_ids(statement, next_id))
    .collect()
}

fn inject_after(statements: &mut Vec<Value>, template: &[Value], next_id: &mut i64) -> Result<()> {
  let mut idx = 0;
  while idx < statements.len() {
    let node_type = node_type(&statements[idx]);
    match node_type {
      Some("Return") => {
        let clones = clone_statements(template, next_id);
        let len = clones.len();
        for (offset, clone) in clones.into_iter().enumerate() {
          statements.insert(idx + offset, clone);
        }
        idx += len + 1;
      }
      Some("Block") | Some("UncheckedBlock") => {
        let block_statements = statements[idx]
          .get_mut("statements")
          .and_then(|value| value.as_array_mut())
          .ok_or_else(|| Error::new("Block missing statements array"))?;
        inject_after(block_statements, template, next_id)?;
        idx += 1;
      }
      Some("IfStatement") => {
        if let Some(true_body) = statements[idx].get_mut("trueBody") {
          inject_into_block_or_statement(true_body, template, next_id)?;
        }
        if let Some(false_body) = statements[idx].get_mut("falseBody") {
          inject_into_block_or_statement(false_body, template, next_id)?;
        }
        idx += 1;
      }
      Some("WhileStatement") | Some("ForStatement") => {
        if let Some(body) = statements[idx].get_mut("body") {
          inject_into_block_or_statement(body, template, next_id)?;
        }
        idx += 1;
      }
      Some("DoWhileStatement") => {
        if let Some(body) = statements[idx].get_mut("body") {
          let block_statements = body
            .get_mut("statements")
            .and_then(|value| value.as_array_mut())
            .ok_or_else(|| Error::new("DoWhile body missing statements array"))?;
          inject_after(block_statements, template, next_id)?;
        }
        idx += 1;
      }
      Some("TryStatement") => {
        if let Some(clauses) = statements[idx]
          .get_mut("clauses")
          .and_then(|value| value.as_array_mut())
        {
          for clause in clauses {
            if let Some(block) = clause.get_mut("block") {
              let block_statements = block
                .get_mut("statements")
                .and_then(|value| value.as_array_mut())
                .ok_or_else(|| Error::new("Try clause block missing statements array"))?;
              inject_after(block_statements, template, next_id)?;
            }
          }
        }
        idx += 1;
      }
      _ => {
        idx += 1;
      }
    }
  }
  Ok(())
}

fn inject_into_block_or_statement(
  node: &mut Value,
  template: &[Value],
  next_id: &mut i64,
) -> Result<()> {
  if node_type(node) == Some("Block") || node_type(node) == Some("UncheckedBlock") {
    let statements = node
      .get_mut("statements")
      .and_then(|value| value.as_array_mut())
      .ok_or_else(|| Error::new("Block missing statements array"))?;
    inject_after(statements, template, next_id)
  } else {
    ensure_block(node, next_id)?;
    let statements = node
      .get_mut("statements")
      .and_then(|value| value.as_array_mut())
      .ok_or_else(|| Error::new("Converted block missing statements array"))?;
    inject_after(statements, template, next_id)
  }
}

fn ensure_block(node: &mut Value, next_id: &mut i64) -> Result<()> {
  if node_type(node) == Some("Block") || node_type(node) == Some("UncheckedBlock") {
    return Ok(());
  }

  let original = std::mem::replace(node, Value::Null);
  let src = original
    .get("src")
    .cloned()
    .unwrap_or_else(|| Value::String("0:0:0".to_string()));

  *next_id += 1;

  let mut block_map = Map::new();
  block_map.insert("nodeType".to_string(), Value::String("Block".to_string()));
  block_map.insert("id".to_string(), Value::Number((*next_id).into()));
  block_map.insert("src".to_string(), src);
  block_map.insert("statements".to_string(), Value::Array(vec![original]));

  *node = Value::Object(block_map);
  Ok(())
}

fn parse_fragment_contract(fragment: &str, solc: &Solc, settings: &Settings) -> Result<Value> {
  let wrapped = parser::wrap_fragment_source(fragment);
  let mut sources = Sources::new();
  sources.insert(PathBuf::from("__AstFragment.sol"), Source::new(&wrapped));

  let mut input = SolcInput::new(SolcLanguage::Solidity, sources, settings.clone());
  input.sanitize(&solc.version);

  let compiler_output: Value = map_err_with_context(
    solc.compile_as(&input),
    "Failed to parse instrumented snippet",
  )?;

  let ast_value = compiler_output
    .get("sources")
    .and_then(|sources| sources.get("__AstFragment.sol"))
    .and_then(|entry| entry.get("ast"))
    .cloned()
    .ok_or_else(|| Error::new("Failed to extract AST"))?;

  let contract =
    parser::extract_fragment_contract(&ast_value).map_err(|err| Error::new(err.to_string()))?;
  Ok(contract)
}

fn contract_mut_at<'a>(unit: &'a mut Value, idx: usize) -> Result<&'a mut Value> {
  let nodes = unit
    .get_mut("nodes")
    .and_then(|value| value.as_array_mut())
    .ok_or_else(|| Error::new("Source unit has no nodes array"))?;
  let Some(node) = nodes.get_mut(idx) else {
    return Err(Error::new("Invalid contract index"));
  };
  if node_type(node) != Some("ContractDefinition") {
    return Err(Error::new("Target index is not a contract definition"));
  }
  Ok(node)
}

fn node_type(value: &Value) -> Option<&str> {
  value.get("nodeType").and_then(|value| value.as_str())
}

fn node_name(value: &Value) -> Option<&str> {
  value.get("name").and_then(|value| value.as_str())
}

fn function_kind(value: &Value) -> Option<&str> {
  value.get("kind").and_then(|value| value.as_str())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ast::orchestrator::AstOrchestrator, internal::solc};
  use serde_json::json;

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn ensure_block_wraps_expression_statements() {
    let mut node = json!({
      "nodeType": "ExpressionStatement",
      "expression": { "nodeType": "Identifier", "name": "foo" }
    });
    let mut next_id = 0;
    ensure_block(&mut node, &mut next_id).expect("wrap into block");

    assert_eq!(node["nodeType"], "Block");
    assert!(node["id"].as_i64().is_some());
    let statements = node["statements"].as_array().expect("statements array");
    assert_eq!(statements.len(), 1);
    assert_eq!(statements[0]["expression"]["name"], "foo");
    assert_eq!(next_id, 1);
  }

  #[test]
  fn inject_after_inserts_template_before_returns() {
    let mut statements = vec![json!({ "nodeType": "Return" })];
    let template = vec![json!({
      "nodeType": "ExpressionStatement",
      "expression": { "nodeType": "Identifier", "name": "probe" }
    })];

    let mut next_id = 0;
    inject_after(&mut statements, &template, &mut next_id).expect("inject");

    assert_eq!(statements.len(), 2);
    assert_eq!(statements[0]["nodeType"], "ExpressionStatement");
    assert_eq!(statements[0]["expression"]["name"], "probe");
    assert_eq!(statements[1]["nodeType"], "Return");
    assert_eq!(next_id, 2);
  }

  #[test]
  fn ensure_no_inline_assembly_detects_assembly_nodes() {
    let block = json!({
      "nodeType": "Block",
      "statements": [
        { "nodeType": "InlineAssembly" }
      ]
    });

    let err = ensure_no_inline_assembly(&block);
    assert!(err.is_err());
  }

  #[test]
  fn parse_selector_parses_canonical_signature() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("default settings");

    let selector =
      parse_selector("tapStored(uint256 value)", &solc, &settings).expect("parse selector");

    match selector {
      FunctionSelectorKind::Canonical { name, signature } => {
        assert_eq!(name, "tapStored");
        assert!(
          signature.iter().any(|entry| entry.contains("uint256")),
          "signature should include uint256 type"
        );
      }
      other => panic!("expected canonical selector, found {:?}", other),
    }
  }
}
