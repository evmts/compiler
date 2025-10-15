#[macro_use]
extern crate napi_derive;

// Public modules
pub mod compile;
pub mod config;
pub mod shadow;
pub mod types;

// Convenience re-exports for ergonomic imports
pub use compile::{SolidityProject, SolidityProjectBuilder};
pub use config::{
  create_current_dapptools_paths, create_current_hardhat_paths, create_dapptools_paths,
  create_hardhat_paths, find_artifacts_dir, find_libs, find_source_dir, sum,
};
pub use shadow::Shadow;
pub use types::{CompileOutput, CompilerError, ContractArtifact, ProjectPaths, SourceLocation};
