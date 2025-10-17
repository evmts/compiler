use std::collections::BTreeMap;
use std::path::PathBuf;

use foundry_compilers::artifacts::ast::SourceUnit;
use foundry_compilers::artifacts::SolcLanguage as FoundrySolcLanguage;

/// Normalised representation of user-provided compiler targets.
#[derive(Debug, Clone)]
pub enum CompilationInput {
  /// Inline source text destined for a virtual in-memory file.
  InlineSource { source: String },
  /// A map of virtual file paths to source text.
  SourceMap { sources: BTreeMap<String, String> },
  /// Pre-parsed Solidity AST units keyed by their path.
  AstUnits { units: BTreeMap<String, SourceUnit> },
  /// Concrete filesystem paths that must be read from disk.
  FilePaths {
    paths: Vec<PathBuf>,
    language_override: Option<FoundrySolcLanguage>,
  },
}
