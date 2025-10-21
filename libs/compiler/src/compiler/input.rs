use std::collections::BTreeMap;
use std::path::PathBuf;

use foundry_compilers::artifacts::ast::SourceUnit;

use crate::internal::config::CompilerLanguage;

/// Normalised representation of user-provided compiler targets.
#[derive(Debug, Clone)]
pub enum CompilationInput {
  /// Inline source text destined for a virtual in-memory file.
  InlineSource { source: String },
  /// A map of virtual file paths to source text with inferred language.
  SourceMap {
    sources: BTreeMap<String, String>,
    language_override: Option<CompilerLanguage>,
  },
  /// Pre-parsed Solidity AST units keyed by their path.
  AstUnits { units: BTreeMap<String, SourceUnit> },
  /// Concrete filesystem paths that must be read from disk.
  FilePaths {
    paths: Vec<PathBuf>,
    language_override: Option<CompilerLanguage>,
  },
}
