use foundry_compilers::{
  solc::{SolcCompiler, SolcLanguage},
  ProjectBuilder, ProjectPathsConfig,
};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

use super::project::SolidityProject;

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

  /// Set the project paths using hardhat layout
  #[napi]
  pub fn hardhat_paths(&mut self, root_path: String) -> Result<()> {
    let root = PathBuf::from(root_path);
    let paths: ProjectPathsConfig<SolcLanguage> =
      ProjectPathsConfig::hardhat(&root).map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to create hardhat paths: {}", e),
        )
      })?;

    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default()).paths(paths);
    Ok(())
  }

  /// Set the project paths using dapptools layout
  #[napi]
  pub fn dapptools_paths(&mut self, root_path: String) -> Result<()> {
    let root = PathBuf::from(root_path);
    let paths: ProjectPathsConfig<SolcLanguage> =
      ProjectPathsConfig::dapptools(&root).map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to create dapptools paths: {}", e),
        )
      })?;

    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default()).paths(paths);
    Ok(())
  }

  /// Enable ephemeral mode (disable caching)
  #[napi]
  pub fn ephemeral(&mut self) -> &Self {
    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default()).ephemeral();
    self
  }

  /// Set cached mode
  #[napi]
  pub fn set_cached(&mut self, cached: bool) -> &Self {
    self.builder =
      std::mem::replace(&mut self.builder, ProjectBuilder::default()).set_cached(cached);
    self
  }

  /// Enable offline mode
  #[napi]
  pub fn offline(&mut self) -> &Self {
    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default()).offline();
    self
  }

  /// Set offline mode
  #[napi]
  pub fn set_offline(&mut self, offline: bool) -> &Self {
    self.builder =
      std::mem::replace(&mut self.builder, ProjectBuilder::default()).set_offline(offline);
    self
  }

  /// Disable writing artifacts to disk
  #[napi]
  pub fn no_artifacts(&mut self) -> &Self {
    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default()).no_artifacts();
    self
  }

  /// Set whether to write artifacts
  #[napi]
  pub fn set_no_artifacts(&mut self, no_artifacts: bool) -> &Self {
    self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
      .set_no_artifacts(no_artifacts);
    self
  }

  /// Set the number of parallel solc jobs
  #[napi]
  pub fn solc_jobs(&mut self, jobs: u32) -> &Self {
    self.builder =
      std::mem::replace(&mut self.builder, ProjectBuilder::default()).solc_jobs(jobs as usize);
    self
  }

  /// Limit to single solc job
  #[napi]
  pub fn single_solc_jobs(&mut self) -> &Self {
    self.builder =
      std::mem::replace(&mut self.builder, ProjectBuilder::default()).single_solc_jobs();
    self
  }

  /// Build the project
  #[napi]
  pub fn build(&mut self) -> Result<SolidityProject> {
    let builder = std::mem::replace(&mut self.builder, ProjectBuilder::default());
    let project = builder.build(SolcCompiler::default()).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to build project: {}", e),
      )
    })?;

    Ok(SolidityProject { project })
  }
}
