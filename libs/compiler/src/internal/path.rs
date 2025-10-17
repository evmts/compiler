use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use foundry_compilers::{error::SolcError, solc::SolcLanguage, ProjectPathsConfig};
use napi::bindgen_prelude::*;

use crate::internal::errors::map_napi_error;

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ProjectPaths {
  pub root: String,
  pub cache: String,
  pub artifacts: String,
  pub sources: String,
  pub tests: String,
  pub scripts: String,
  pub libraries: Vec<String>,
}

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
      .map(|path| path.to_string_lossy().to_string())
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

fn path_to_string(path: PathBuf) -> String {
  path.to_string_lossy().to_string()
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

#[napi]
pub fn find_artifacts_dir(root_path: String) -> String {
  let root = PathBuf::from(root_path);
  let artifacts_dir = ProjectPathsConfig::find_artifacts_dir(&root);
  path_to_string(artifacts_dir)
}

#[napi]
pub fn find_source_dir(root_path: String) -> String {
  let root = PathBuf::from(root_path);
  let source_dir = ProjectPathsConfig::find_source_dir(&root);
  path_to_string(source_dir)
}

#[napi]
pub fn find_libs(root_path: String) -> Vec<String> {
  let root = PathBuf::from(root_path);
  let libs = ProjectPathsConfig::find_libs(&root);
  libs.into_iter().map(path_to_string).collect()
}

/// Canonicalises a path while falling back to an absolute join if canonicalisation fails.
///
/// This mirrors the previous behaviour where missing paths defaulted to the current working
/// directory, ensuring the compiler maintains predictable path resolution even for yet-to-be
/// written files.
pub fn canonicalize_path(path: &Path) -> PathBuf {
  match std::fs::canonicalize(path) {
    Ok(canonical) => canonical,
    Err(_) => {
      if path.is_absolute() {
        path.to_path_buf()
      } else {
        std::env::current_dir()
          .unwrap_or_else(|_| PathBuf::from("."))
          .join(path)
      }
    }
  }
}

/// Canonicalises `path` relative to `base`, returning the best-effort absolute path.
pub fn canonicalize_with_base(base: &Path, path: &Path) -> PathBuf {
  if path.is_absolute() {
    return canonicalize_path(path);
  }
  canonicalize_path(&base.join(path))
}

/// Converts a collection of string paths into a canonicalised set.
pub fn to_path_set(paths: &[String]) -> BTreeSet<PathBuf> {
  paths
    .iter()
    .map(|value| canonicalize_path(Path::new(value)))
    .collect()
}

/// Converts a collection of string paths into a canonicalised vector.
pub fn to_path_vec(paths: &[String]) -> Vec<PathBuf> {
  paths
    .iter()
    .map(|value| canonicalize_path(Path::new(value)))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn canonicalize_relative_paths_with_base() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base = temp.path();
    let nested = base.join("nested");
    std::fs::create_dir_all(&nested).expect("create nested");

    let resolved = canonicalize_with_base(base, Path::new("nested"));
    assert_eq!(resolved, canonicalize_path(&nested));
  }

  #[test]
  fn to_path_set_deduplicates() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base = temp.path();
    let file = base.join("file.sol");
    std::fs::write(&file, "").expect("write file");

    let entries = vec![
      file.to_string_lossy().to_string(),
      file.to_string_lossy().to_string(),
    ];
    let set = to_path_set(&entries);
    assert_eq!(set.len(), 1);
    assert_eq!(set.iter().next().unwrap(), &canonicalize_path(&file));
  }
}
