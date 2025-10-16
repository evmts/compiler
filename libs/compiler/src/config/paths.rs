use std::path::{Path, PathBuf};

use foundry_compilers::{error::SolcError, solc::SolcLanguage, ProjectPathsConfig};
use napi::bindgen_prelude::*;

use crate::internal::errors::map_napi_error;
use crate::types::ProjectPaths;

fn to_project_paths(config: ProjectPathsConfig<SolcLanguage>) -> ProjectPaths {
  ProjectPaths {
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
  }
}

fn create_paths_with_root<F>(root_path: String, context: &str, factory: F) -> Result<ProjectPaths>
where
  F: FnOnce(&Path) -> std::result::Result<ProjectPathsConfig<SolcLanguage>, SolcError>,
{
  let root = PathBuf::from(root_path);
  let config = map_napi_error(factory(&root), context)?;
  Ok(to_project_paths(config))
}

fn create_paths<F>(context: &str, factory: F) -> Result<ProjectPaths>
where
  F: FnOnce() -> std::result::Result<ProjectPathsConfig<SolcLanguage>, SolcError>,
{
  let config = map_napi_error(factory(), context)?;
  Ok(to_project_paths(config))
}

#[napi]
pub fn create_hardhat_paths(root_path: String) -> Result<ProjectPaths> {
  create_paths_with_root(
    root_path,
    "Failed to create hardhat paths",
    ProjectPathsConfig::<SolcLanguage>::hardhat,
  )
}

#[napi]
pub fn create_dapptools_paths(root_path: String) -> Result<ProjectPaths> {
  create_paths_with_root(
    root_path,
    "Failed to create dapptools paths",
    ProjectPathsConfig::<SolcLanguage>::dapptools,
  )
}

#[napi]
pub fn create_current_hardhat_paths() -> Result<ProjectPaths> {
  create_paths(
    "Failed to create current hardhat paths",
    ProjectPathsConfig::<SolcLanguage>::current_hardhat,
  )
}

#[napi]
pub fn create_current_dapptools_paths() -> Result<ProjectPaths> {
  create_paths(
    "Failed to create current dapptools paths",
    ProjectPathsConfig::<SolcLanguage>::current_dapptools,
  )
}
