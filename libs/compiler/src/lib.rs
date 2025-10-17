#[macro_use]
extern crate napi_derive;

mod ast;
mod compiler;
mod internal;

// Public modules
mod compile;
pub mod config;
pub mod types;

// Convenience re-exports for ergonomic imports
pub use ast::Ast;
pub use compiler::{Compiler, CompilerCore, CompilerWithContext};
pub use config::{
  create_current_dapptools_paths, create_current_hardhat_paths, create_dapptools_paths,
  create_hardhat_paths, find_artifacts_dir, find_libs, find_source_dir,
};
pub use internal::{
  config::{AstOptions, CompilerConfig},
  settings::{
    BytecodeHash, CompilerSettings, DebuggingSettings, EvmVersion, ModelCheckerEngine,
    ModelCheckerInvariant, ModelCheckerSettings, ModelCheckerSolver, ModelCheckerTarget,
    OptimizerDetails, OptimizerSettings, RevertStrings, SettingsMetadata, YulDetails,
  },
};
pub use types::{
  CompileOutput, CompilerError, ContractArtifact, ContractBytecode, ProjectPaths, SourceLocation,
};
