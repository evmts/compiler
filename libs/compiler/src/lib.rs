#[macro_use]
extern crate napi_derive;

mod compiler;
mod instrument;
mod internal;

// Public modules
pub mod compile;
pub mod config;
pub mod types;

// Convenience re-exports for ergonomic imports
pub use compile::{SolidityProject, SolidityProjectBuilder};
pub use compiler::Compiler;
pub use config::{
  create_current_dapptools_paths, create_current_hardhat_paths, create_dapptools_paths,
  create_hardhat_paths, find_artifacts_dir, find_libs, find_source_dir,
};
pub use instrument::Instrument;
pub use internal::{
  options::{CompilerOptions, InstrumentOptions},
  settings::{
    BytecodeHash, CompilerSettings, DebuggingSettings, EvmVersion, ModelCheckerEngine,
    ModelCheckerInvariant, ModelCheckerSettings, ModelCheckerSolver, ModelCheckerTarget,
    OptimizerDetails, OptimizerSettings, RevertStrings, SettingsMetadata, YulDetails,
  },
};
pub use types::{CompileOutput, CompilerError, ContractArtifact, ProjectPaths, SourceLocation};
