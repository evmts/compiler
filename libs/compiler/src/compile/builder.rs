use foundry_compilers::{
  solc::{SolcCompiler, SolcLanguage},
  ProjectBuilder, ProjectPathsConfig,
};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

use super::project::SolidityProject;
use crate::errors::map_napi_error;

#[napi]
pub struct SolidityProjectBuilder {
  builder: ProjectBuilder<SolcCompiler>,
}

#[napi]
impl SolidityProjectBuilder {
  /// Create a new project builder
  #[napi(constructor)]
  pub fn new() -> Self {
    SolidityProjectBuilder {
      builder: ProjectBuilder::default(),
    }
  }

  fn update_builder<F>(&mut self, update: F) -> &Self
  where
    F: FnOnce(ProjectBuilder<SolcCompiler>) -> ProjectBuilder<SolcCompiler>,
  {
    let builder = std::mem::take(&mut self.builder);
    self.builder = update(builder);
    self
  }

  /// Set the project paths using hardhat layout
  #[napi]
  pub fn hardhat_paths(&mut self, root_path: String) -> Result<()> {
    let root = PathBuf::from(root_path);
    let paths: ProjectPathsConfig<SolcLanguage> = map_napi_error(
      ProjectPathsConfig::hardhat(&root),
      "Failed to create hardhat paths",
    )?;

    self.update_builder(|builder| builder.paths(paths));
    Ok(())
  }

  /// Set the project paths using dapptools layout
  #[napi]
  pub fn dapptools_paths(&mut self, root_path: String) -> Result<()> {
    let root = PathBuf::from(root_path);
    let paths: ProjectPathsConfig<SolcLanguage> = map_napi_error(
      ProjectPathsConfig::dapptools(&root),
      "Failed to create dapptools paths",
    )?;

    self.update_builder(|builder| builder.paths(paths));
    Ok(())
  }

  /// Enable ephemeral mode (disable caching)
  #[napi]
  pub fn ephemeral(&mut self) -> &Self {
    self.update_builder(ProjectBuilder::ephemeral)
  }

  /// Set cached mode
  #[napi]
  pub fn set_cached(&mut self, cached: bool) -> &Self {
    self.update_builder(|builder| builder.set_cached(cached))
  }

  /// Enable offline mode
  #[napi]
  pub fn offline(&mut self) -> &Self {
    self.update_builder(ProjectBuilder::offline)
  }

  /// Set offline mode
  #[napi]
  pub fn set_offline(&mut self, offline: bool) -> &Self {
    self.update_builder(|builder| builder.set_offline(offline))
  }

  /// Disable writing artifacts to disk
  #[napi]
  pub fn no_artifacts(&mut self) -> &Self {
    self.update_builder(ProjectBuilder::no_artifacts)
  }

  /// Set whether to write artifacts
  #[napi]
  pub fn set_no_artifacts(&mut self, no_artifacts: bool) -> &Self {
    self.update_builder(|builder| builder.set_no_artifacts(no_artifacts))
  }

  /// Set the number of parallel solc jobs
  #[napi]
  pub fn solc_jobs(&mut self, jobs: u32) -> &Self {
    self.update_builder(|builder| builder.solc_jobs(jobs as usize))
  }

  /// Limit to single solc job
  #[napi]
  pub fn single_solc_jobs(&mut self) -> &Self {
    self.update_builder(ProjectBuilder::single_solc_jobs)
  }

  /// Build the project
  #[napi]
  pub fn build(&mut self) -> Result<SolidityProject> {
    let builder = std::mem::take(&mut self.builder);
    let project = map_napi_error(
      builder.build(SolcCompiler::default()),
      "Failed to build project",
    )?;

    Ok(SolidityProject { project })
  }
}
