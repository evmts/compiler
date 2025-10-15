use foundry_compilers::{solc::SolcLanguage, ProjectPathsConfig};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

use crate::types::ProjectPaths;

#[napi]
pub fn create_hardhat_paths(root_path: String) -> Result<ProjectPaths> {
  let root = PathBuf::from(&root_path);
  let config: ProjectPathsConfig<SolcLanguage> =
    ProjectPathsConfig::hardhat(&root).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create hardhat paths: {}", e),
      )
    })?;

  Ok(ProjectPaths {
    root: config.root.to_string_lossy().to_string(),
    cache: config.cache.to_string_lossy().to_string(),
    artifacts: config.artifacts.to_string_lossy().to_string(),
    sources: config.sources.to_string_lossy().to_string(),
    tests: config.tests.to_string_lossy().to_string(),
    scripts: config.scripts.to_string_lossy().to_string(),
    libraries: config
      .libraries
      .iter()
      .map(|p| p.to_string_lossy().to_string())
      .collect(),
  })
}

#[napi]
pub fn create_dapptools_paths(root_path: String) -> Result<ProjectPaths> {
  let root = PathBuf::from(&root_path);
  let config: ProjectPathsConfig<SolcLanguage> =
    ProjectPathsConfig::dapptools(&root).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create dapptools paths: {}", e),
      )
    })?;

  Ok(ProjectPaths {
    root: config.root.to_string_lossy().to_string(),
    cache: config.cache.to_string_lossy().to_string(),
    artifacts: config.artifacts.to_string_lossy().to_string(),
    sources: config.sources.to_string_lossy().to_string(),
    tests: config.tests.to_string_lossy().to_string(),
    scripts: config.scripts.to_string_lossy().to_string(),
    libraries: config
      .libraries
      .iter()
      .map(|p| p.to_string_lossy().to_string())
      .collect(),
  })
}

#[napi]
pub fn create_current_hardhat_paths() -> Result<ProjectPaths> {
  let config: ProjectPathsConfig<SolcLanguage> =
    ProjectPathsConfig::current_hardhat().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create current hardhat paths: {}", e),
      )
    })?;

  Ok(ProjectPaths {
    root: config.root.to_string_lossy().to_string(),
    cache: config.cache.to_string_lossy().to_string(),
    artifacts: config.artifacts.to_string_lossy().to_string(),
    sources: config.sources.to_string_lossy().to_string(),
    tests: config.tests.to_string_lossy().to_string(),
    scripts: config.scripts.to_string_lossy().to_string(),
    libraries: config
      .libraries
      .iter()
      .map(|p| p.to_string_lossy().to_string())
      .collect(),
  })
}

#[napi]
pub fn create_current_dapptools_paths() -> Result<ProjectPaths> {
  let config: ProjectPathsConfig<SolcLanguage> =
    ProjectPathsConfig::current_dapptools().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create current dapptools paths: {}", e),
      )
    })?;

  Ok(ProjectPaths {
    root: config.root.to_string_lossy().to_string(),
    cache: config.cache.to_string_lossy().to_string(),
    artifacts: config.artifacts.to_string_lossy().to_string(),
    sources: config.sources.to_string_lossy().to_string(),
    tests: config.tests.to_string_lossy().to_string(),
    scripts: config.scripts.to_string_lossy().to_string(),
    libraries: config
      .libraries
      .iter()
      .map(|p| p.to_string_lossy().to_string())
      .collect(),
  })
}
