use foundry_compilers::artifacts::ast::{
  Block, BlockOrStatement, ContractDefinition, ContractDefinitionPart, FunctionDefinition,
  FunctionKind, SourceUnit, SourceUnitPart, Statement, TryCatchClause,
};
use foundry_compilers::artifacts::{Settings, SolcInput, SolcLanguage, Source, Sources};
use foundry_compilers::solc::Solc;

use crate::internal::errors::{map_err_with_context, Error, Result};
use serde_json::Value;
use std::path::PathBuf;

use super::{orchestrator::AstOrchestrator, parser, stitcher, utils};

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
  unit: &mut foundry_compilers::artifacts::ast::SourceUnit,
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

  let mut next_id = utils::max_id(unit)?;

  let SourceUnitPart::ContractDefinition(contract) = unit
    .nodes
    .get_mut(contract_idx)
    .ok_or_else(|| Error::new("Invalid contract index"))?
  else {
    return Err(Error::new("Target index is not a contract definition"));
  };
  let selector_kind = parse_selector(selector, solc, settings)?;

  let function = resolve_function_mut(contract, &selector_kind)?;

  let body = function
    .body
    .as_mut()
    .ok_or_else(|| Error::new("Cannot instrument a function without an implementation"))?;

  ensure_no_inline_assembly(body)?;

  let before_statements = parse_statements(before_snippets, solc, settings)?;
  let after_statements = parse_statements(after_snippets, solc, settings)?;

  if !before_statements.is_empty() {
    let mut prefix = clone_statements(&before_statements, &mut next_id)?;
    let mut combined = Vec::with_capacity(prefix.len() + body.statements.len());
    combined.append(&mut prefix);
    combined.extend(body.statements.clone());
    body.statements = combined;
  }

  if !after_statements.is_empty() {
    inject_after(&mut body.statements, &after_statements, &mut next_id)?;

    let mut tail = clone_statements(&after_statements, &mut next_id)?;
    body.statements.extend(tail.drain(..));
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
    let fragment_function = contract
      .nodes
      .iter()
      .find_map(|part| {
        if let ContractDefinitionPart::FunctionDefinition(def) = part {
          Some(def)
        } else {
          None
        }
      })
      .ok_or_else(|| Error::new("Failed to parse function signature"))?;
    let signature = stitcher::function_signature(fragment_function).map_err(Error::from)?;
    return Ok(FunctionSelectorKind::Canonical { name, signature });
  }

  Ok(FunctionSelectorKind::Name(trimmed.to_string()))
}

fn resolve_function_mut<'a>(
  contract: &'a mut ContractDefinition,
  selector: &FunctionSelectorKind,
) -> Result<&'a mut FunctionDefinition> {
  let mut matches: Vec<usize> = Vec::new();

  for (idx, part) in contract.nodes.iter().enumerate() {
    let ContractDefinitionPart::FunctionDefinition(function) = part else {
      continue;
    };
    match selector {
      FunctionSelectorKind::Fallback => {
        if matches!(function.kind(), FunctionKind::Fallback) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Receive => {
        if matches!(function.kind(), FunctionKind::Receive) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Constructor => {
        if matches!(function.kind(), FunctionKind::Constructor) {
          matches.push(idx);
        }
      }
      FunctionSelectorKind::Canonical { name, signature } => {
        if function.name == *name {
          let current_signature = stitcher::function_signature(function).map_err(Error::from)?;
          if &current_signature == signature {
            matches.push(idx);
          }
        }
      }
      FunctionSelectorKind::Name(name) => {
        if &function.name == name {
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
  let ContractDefinitionPart::FunctionDefinition(function) = contract
    .nodes
    .get_mut(idx)
    .ok_or_else(|| Error::new("Invalid function index after resolution"))?
  else {
    return Err(Error::new("Resolved index is not a function definition"));
  };
  Ok(function)
}

fn ensure_no_inline_assembly(body: &Block) -> Result<()> {
  for statement in &body.statements {
    ensure_no_inline_assembly_in_statement(statement)?;
  }
  Ok(())
}

fn ensure_no_inline_assembly_in_statement(statement: &Statement) -> Result<()> {
  match statement {
    Statement::InlineAssembly(_) => Err(Error::new(
      "injectShadowAtEdges does not support functions containing inline assembly.",
    )),
    Statement::Block(block) => {
      for stmt in &block.statements {
        ensure_no_inline_assembly_in_statement(stmt)?;
      }
      Ok(())
    }
    Statement::IfStatement(if_stmt) => {
      ensure_no_inline_assembly_in_block_or_statement(&if_stmt.true_body)?;
      if let Some(false_body) = &if_stmt.false_body {
        ensure_no_inline_assembly_in_block_or_statement(false_body)?;
      }
      Ok(())
    }
    Statement::WhileStatement(while_stmt) => {
      ensure_no_inline_assembly_in_block_or_statement(&while_stmt.body)
    }
    Statement::DoWhileStatement(do_stmt) => ensure_no_inline_assembly(&do_stmt.body),
    Statement::ForStatement(for_stmt) => {
      ensure_no_inline_assembly_in_block_or_statement(&for_stmt.body)
    }
    Statement::TryStatement(try_stmt) => {
      for clause in &try_stmt.clauses {
        ensure_no_inline_assembly(&clause.block)?;
      }
      Ok(())
    }
    Statement::UncheckedBlock(unchecked) => {
      for stmt in &unchecked.statements {
        ensure_no_inline_assembly_in_statement(stmt)?;
      }
      Ok(())
    }
    _ => Ok(()),
  }
}

fn ensure_no_inline_assembly_in_block_or_statement(node: &BlockOrStatement) -> Result<()> {
  match node {
    BlockOrStatement::Block(block) => ensure_no_inline_assembly(block),
    BlockOrStatement::Statement(statement) => ensure_no_inline_assembly_in_statement(statement),
  }
}

fn parse_statements(
  snippets: &[String],
  solc: &Solc,
  settings: &Settings,
) -> Result<Vec<Statement>> {
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
  let function = contract
    .nodes
    .iter()
    .find_map(|part| {
      if let ContractDefinitionPart::FunctionDefinition(func) = part {
        Some(func)
      } else {
        None
      }
    })
    .ok_or_else(|| Error::new("Failed to parse instrumentation snippets"))?;
  let Some(block) = &function.body else {
    return Err(Error::new(
      "Instrumentation snippet produced no body statements.",
    ));
  };
  Ok(block.statements.clone())
}

fn clone_statements(statements: &[Statement], next_id: &mut i64) -> Result<Vec<Statement>> {
  let mut clones = Vec::with_capacity(statements.len());
  for statement in statements {
    clones.push(utils::clone_with_new_ids(statement, next_id)?);
  }
  Ok(clones)
}

fn parse_fragment_contract(
  fragment: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<ContractDefinition> {
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

  let mut ast_value = ast_value;
  ensure_kind_fields(&mut ast_value);
  utils::sanitize_ast_value(&mut ast_value);

  let unit: SourceUnit = serde_json::from_value(ast_value)
    .map_err(|err| Error::new(format!("Failed to parse fragment AST: {}", err)))?;

  unit
    .nodes
    .iter()
    .find_map(|part| {
      if let SourceUnitPart::ContractDefinition(contract) = part {
        Some((**contract).clone())
      } else {
        None
      }
    })
    .ok_or_else(|| Error::new("Fragment contract not found"))
}

fn ensure_kind_fields(node: &mut Value) {
  match node {
    Value::Object(map) => {
      if let Some(node_type) = map
        .get("nodeType")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
      {
        if node_type == "FunctionDefinition" && !map.contains_key("kind") {
          map.insert("kind".to_string(), Value::String("function".to_string()));
        }
        if node_type == "FunctionCall" && !map.contains_key("kind") {
          map.insert(
            "kind".to_string(),
            Value::String("functionCall".to_string()),
          );
        }
      }
      for child in map.values_mut() {
        ensure_kind_fields(child);
      }
    }
    Value::Array(items) => {
      for item in items {
        ensure_kind_fields(item);
      }
    }
    _ => {}
  }
}

fn inject_after(
  statements: &mut Vec<Statement>,
  template: &[Statement],
  next_id: &mut i64,
) -> Result<()> {
  let mut idx = 0;
  while idx < statements.len() {
    match &mut statements[idx] {
      Statement::Return(_) => {
        let clones = clone_statements(template, next_id)?;
        let len = clones.len();
        statements.splice(idx..idx, clones);
        idx += len + 1;
      }
      Statement::Block(block) => {
        inject_after(&mut block.statements, template, next_id)?;
        idx += 1;
      }
      Statement::IfStatement(if_stmt) => {
        inject_into_block_or_statement(&mut if_stmt.true_body, template, next_id)?;
        if let Some(false_body) = if_stmt.false_body.as_mut() {
          inject_into_block_or_statement(false_body, template, next_id)?;
        }
        idx += 1;
      }
      Statement::WhileStatement(while_stmt) => {
        inject_into_block_or_statement(&mut while_stmt.body, template, next_id)?;
        idx += 1;
      }
      Statement::DoWhileStatement(do_stmt) => {
        inject_after(&mut do_stmt.body.statements, template, next_id)?;
        idx += 1;
      }
      Statement::ForStatement(for_stmt) => {
        inject_into_block_or_statement(&mut for_stmt.body, template, next_id)?;
        idx += 1;
      }
      Statement::TryStatement(try_stmt) => {
        for TryCatchClause { block, .. } in &mut try_stmt.clauses {
          inject_after(&mut block.statements, template, next_id)?;
        }
        idx += 1;
      }
      Statement::UncheckedBlock(unchecked) => {
        inject_after(&mut unchecked.statements, template, next_id)?;
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
  target: &mut BlockOrStatement,
  template: &[Statement],
  next_id: &mut i64,
) -> Result<()> {
  match target {
    BlockOrStatement::Block(block) => inject_after(&mut block.statements, template, next_id),
    BlockOrStatement::Statement(statement) => {
      inject_after_in_statement(statement, template, next_id)
    }
  }
}

fn inject_after_in_statement(
  statement: &mut Statement,
  template: &[Statement],
  next_id: &mut i64,
) -> Result<()> {
  match statement {
    Statement::Block(block) => inject_after(&mut block.statements, template, next_id),
    Statement::IfStatement(if_stmt) => {
      inject_into_block_or_statement(&mut if_stmt.true_body, template, next_id)?;
      if let Some(false_body) = if_stmt.false_body.as_mut() {
        inject_into_block_or_statement(false_body, template, next_id)?;
      }
      Ok(())
    }
    Statement::WhileStatement(while_stmt) => {
      inject_into_block_or_statement(&mut while_stmt.body, template, next_id)
    }
    Statement::DoWhileStatement(do_stmt) => {
      inject_after(&mut do_stmt.body.statements, template, next_id)
    }
    Statement::ForStatement(for_stmt) => {
      inject_into_block_or_statement(&mut for_stmt.body, template, next_id)
    }
    Statement::TryStatement(try_stmt) => {
      for TryCatchClause { block, .. } in &mut try_stmt.clauses {
        inject_after(&mut block.statements, template, next_id)?;
      }
      Ok(())
    }
    Statement::UncheckedBlock(unchecked) => {
      inject_after(&mut unchecked.statements, template, next_id)
    }
    _ => Ok(()),
  }
}
