use std::path::PathBuf;

use foundry_compilers::artifacts::{Settings, SolcInput, SolcLanguage, Source, Sources};
use foundry_compilers::solc::Solc;

use super::error::AstError;
use serde_json::Value;

// TODO: remove in favor of compile_source with correct settings once we add ast to output
fn parse_source_ast_internal(
  source: &str,
  file_name: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<Value, AstError> {
  let mut sources = Sources::new();
  sources.insert(PathBuf::from(file_name), Source::new(source));

  let mut input = SolcInput::new(SolcLanguage::Solidity, sources, settings.clone());
  input.sanitize(&solc.version);

  let compiler_output: serde_json::Value = solc
    .compile_as::<SolcInput, _>(&input)
    .map_err(|err| AstError::CompilerError(err.to_string()))?;

  if let Some(errors) = compiler_output
    .get("errors")
    .and_then(|value| value.as_array())
  {
    let mut messages = Vec::new();
    for error in errors {
      let severity = error
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
      if severity.eq_ignore_ascii_case("error") {
        let message = error
          .get("formattedMessage")
          .and_then(|value| value.as_str())
          .or_else(|| error.get("message").and_then(|value| value.as_str()))
          .unwrap_or("Compilation error");
        messages.push(message.to_string());
      }
    }
    if !messages.is_empty() {
      return Err(AstError::CompilerError(messages.join("\n")));
    }
  }

  let ast_value = compiler_output
    .get("sources")
    .and_then(|sources| sources.get(file_name))
    .and_then(|entry| entry.get("ast"))
    .ok_or_else(|| AstError::ParseFailed("Failed to extract AST".to_string()))?
    .clone();

  Ok(ast_value)
}

pub fn parse_source_ast(
  source: &str,
  file_name: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<Value, AstError> {
  parse_source_ast_internal(source, file_name, solc, settings)
}

pub fn wrap_fragment_source(source: &str) -> String {
  format!(
    r#"// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract __AstFragment {{
    {}
}}
"#,
    source
  )
}

pub fn parse_fragment_contract(
  fragment_source: &str,
  solc: &Solc,
  settings: &Settings,
) -> Result<Value, AstError> {
  let unit = parse_source_ast_internal(
    &wrap_fragment_source(fragment_source),
    "__AstFragment.sol",
    solc,
    settings,
  )?;
  extract_fragment_contract(&unit)
}

pub fn extract_fragment_contract(unit: &Value) -> Result<Value, AstError> {
  let nodes = unit
    .get("nodes")
    .and_then(|value| value.as_array())
    .ok_or_else(|| AstError::ParseFailed("Source unit does not contain nodes".to_string()))?;

  nodes
    .iter()
    .find(|node| {
      node
        .get("nodeType")
        .and_then(|value| value.as_str())
        .map(|node_type| node_type == "ContractDefinition")
        .unwrap_or(false)
        && node
          .get("name")
          .and_then(|value| value.as_str())
          .map(|name| name == "__AstFragment")
          .unwrap_or(false)
    })
    .cloned()
    .ok_or_else(|| AstError::ParseFailed("Fragment contract '__AstFragment' not found".to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ast::orchestrator::AstOrchestrator, internal::solc};

  const SAMPLE_FRAGMENT: &str = r#"function demo() public pure returns (uint256) { return 1; }"#;
  const SAMPLE_CONTRACT: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Example {
  uint256 public value;
}
"#;

  fn find_default_solc() -> Option<Solc> {
    let version = solc::default_version().ok()?;
    Solc::find_svm_installed_version(&version).ok().flatten()
  }

  #[test]
  fn wraps_fragment_in_shadow_contract() {
    let wrapped = wrap_fragment_source(SAMPLE_FRAGMENT);
    assert!(wrapped.contains("pragma solidity ^0.8.0;"));
    assert!(wrapped.contains("contract __AstFragment"));
    assert!(wrapped.contains(SAMPLE_FRAGMENT));
  }

  #[test]
  fn parses_contract_to_ast_value() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let ast = parse_source_ast(SAMPLE_CONTRACT, "Example.sol", &solc, &settings)
      .expect("should parse contract");
    let nodes = ast
      .get("nodes")
      .and_then(|value| value.as_array())
      .expect("nodes");
    assert!(nodes.iter().any(|node| {
      node
        .get("nodeType")
        .and_then(|value| value.as_str())
        .map(|node_type| node_type == "ContractDefinition")
        .unwrap_or(false)
    }));
  }

  #[test]
  fn parses_fragment_contract() {
    let Some(solc) = find_default_solc() else {
      return;
    };
    let settings = AstOrchestrator::sanitize_settings(None).expect("sanitize default settings");
    let contract =
      parse_fragment_contract(SAMPLE_FRAGMENT, &solc, &settings).expect("parse fragment");
    assert_eq!(
      contract
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default(),
      "__AstFragment"
    );
    let nodes = contract
      .get("nodes")
      .and_then(|value| value.as_array())
      .expect("nodes");
    assert!(nodes.iter().any(|node| {
      node
        .get("nodeType")
        .and_then(|value| value.as_str())
        .map(|node_type| node_type == "FunctionDefinition")
        .unwrap_or(false)
    }));
  }
}
