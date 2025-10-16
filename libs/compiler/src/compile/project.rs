use foundry_compilers::{
  solc::{SolcCompiler, SolcLanguage},
  Project, ProjectBuilder, ProjectCompileOutput, ProjectPathsConfig,
};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

use super::output;
use crate::internal::errors::map_napi_error;
use crate::types::CompileOutput;

#[napi]
pub struct SolidityProject {
  pub(crate) project: Project<SolcCompiler>,
}

#[napi]
impl SolidityProject {
  /// Create a new project from a root path using Hardhat layout
  #[napi(factory)]
  pub fn from_hardhat_root(root_path: String) -> Result<Self> {
    let root = PathBuf::from(&root_path);
    let paths: ProjectPathsConfig<SolcLanguage> = map_napi_error(
      ProjectPathsConfig::hardhat(&root),
      "Failed to create hardhat paths",
    )?;

    let project = map_napi_error(
      ProjectBuilder::default()
        .paths(paths)
        .build(SolcCompiler::default()),
      "Failed to build project",
    )?;

    Ok(SolidityProject { project })
  }

  /// Create a new project from a root path using Dapptools layout
  #[napi(factory)]
  pub fn from_dapptools_root(root_path: String) -> Result<Self> {
    let root = PathBuf::from(&root_path);
    let paths: ProjectPathsConfig<SolcLanguage> = map_napi_error(
      ProjectPathsConfig::dapptools(&root),
      "Failed to create dapptools paths",
    )?;

    let project = map_napi_error(
      ProjectBuilder::default()
        .paths(paths)
        .build(SolcCompiler::default()),
      "Failed to build project",
    )?;

    Ok(SolidityProject { project })
  }

  /// Compile all contracts in the project
  #[napi]
  pub fn compile(&self) -> Result<CompileOutput> {
    self.compile_with(Project::compile, "Compilation failed")
  }

  /// Compile a single file
  #[napi]
  pub fn compile_file(&self, file_path: String) -> Result<CompileOutput> {
    let path = PathBuf::from(file_path);
    self.compile_with(|project| project.compile_file(&path), "Compilation failed")
  }

  /// Compile multiple files
  #[napi]
  pub fn compile_files(&self, file_paths: Vec<String>) -> Result<CompileOutput> {
    let paths: Vec<PathBuf> = file_paths.iter().map(PathBuf::from).collect();
    self.compile_with(|project| project.compile_files(paths), "Compilation failed")
  }

  /// Find the path of a contract by its name
  #[napi]
  pub fn find_contract_path(&self, contract_name: String) -> Result<String> {
    let path = map_napi_error(
      self.project.find_contract_path(&contract_name),
      "Failed to find contract",
    )?;

    Ok(path.to_string_lossy().to_string())
  }

  /// Get all source files in the project
  #[napi]
  pub fn get_sources(&self) -> Result<Vec<String>> {
    let sources = map_napi_error(self.project.sources(), "Failed to get sources")?;

    Ok(
      sources
        .keys()
        .map(|p| p.to_string_lossy().to_string())
        .collect(),
    )
  }

  fn compile_with<F>(&self, compile_fn: F, context: &str) -> Result<CompileOutput>
  where
    F: FnOnce(
      &Project<SolcCompiler>,
    ) -> std::result::Result<
      ProjectCompileOutput<SolcCompiler>,
      foundry_compilers::error::SolcError,
    >,
  {
    let output = map_napi_error(compile_fn(&self.project), context)?;
    Ok(output::into_compile_output(output))
  }
}
