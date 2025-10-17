#[macro_use]
extern crate napi_derive;

mod ast;
mod compiler;
mod internal;

// Convenience re-exports for ergonomic imports
pub use ast::Ast;
pub use compiler::{
  CompileOutput, Compiler, CompilerCore, CompilerError, CompilerWithContext, ContractArtifact,
  ContractBytecode, SourceLocation,
};
pub use internal::path::{
  create_current_dapptools_paths, create_current_hardhat_paths, create_dapptools_paths,
  create_hardhat_paths, find_artifacts_dir, find_libs, find_source_dir, ProjectPaths,
};
pub use internal::{
  config::{AstOptions, CompilerConfig},
  settings::{
    BytecodeHash, CompilerSettings, DebuggingSettings, EvmVersion, ModelCheckerEngine,
    ModelCheckerInvariant, ModelCheckerSettings, ModelCheckerSolver, ModelCheckerTarget,
    OptimizerDetails, OptimizerSettings, RevertStrings, SettingsMetadata, YulDetails,
  },
};
