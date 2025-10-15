use foundry_compilers::ProjectPathsConfig;
use std::path::PathBuf;

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

#[napi]
pub fn find_artifacts_dir(root_path: String) -> String {
  let root = PathBuf::from(root_path);
  let artifacts_dir = ProjectPathsConfig::find_artifacts_dir(&root);
  artifacts_dir.to_string_lossy().to_string()
}

#[napi]
pub fn find_source_dir(root_path: String) -> String {
  let root = PathBuf::from(root_path);
  let source_dir = ProjectPathsConfig::find_source_dir(&root);
  source_dir.to_string_lossy().to_string()
}

#[napi]
pub fn find_libs(root_path: String) -> Vec<String> {
  let root = PathBuf::from(root_path);
  let libs = ProjectPathsConfig::find_libs(&root);
  libs
    .iter()
    .map(|p| p.to_string_lossy().to_string())
    .collect()
}
