use foundry_compilers::{
  artifacts::{output_selection::OutputSelection, Settings, SolcLanguage, Source, Sources},
  compilers::solc::Solc,
};
use semver::Version;
use serde_json::Value;
use std::path::PathBuf;

use super::error::ShadowError;

/// Parse Solidity source code and return AST JSON
/// Uses stopAfter: "parsing" to get syntax-only AST
pub fn parse_source_ast(source: &str, file_name: &str) -> Result<Value, ShadowError> {
  let version = Version::parse("0.8.24")
    .map_err(|e| ShadowError::CompilerError(format!("Invalid version: {}", e)))?;

  let solc = Solc::find_svm_installed_version(&version)
    .map_err(|e| ShadowError::CompilerError(format!("Failed to find solc: {}", e)))?
    .ok_or_else(|| ShadowError::CompilerError("Solc 0.8.24 not found".to_string()))?;

  let mut sources = Sources::new();
  sources.insert(PathBuf::from(file_name), Source::new(source));

  let mut settings = Settings::default();
  settings.stop_after = Some("parsing".to_string());
  settings.output_selection = OutputSelection::ast_output_selection();
  settings.evm_version = None;

  let parse_input = foundry_compilers::artifacts::SolcInput {
    language: SolcLanguage::Solidity,
    sources,
    settings,
  };

  let parse_output: Value = solc.compile_as(&parse_input)?;

  let ast = parse_output
    .get("sources")
    .and_then(|s| s.get(file_name))
    .and_then(|s| s.get("ast"))
    .ok_or_else(|| ShadowError::ParseFailed("Failed to extract AST".to_string()))?;

  Ok(ast.clone())
}

/// Wrap shadow source in minimal contract boilerplate
pub fn wrap_shadow_source(source: &str) -> String {
  format!(
    r#"// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract Shadow {{
    {}
}}
"#,
    source
  )
}
