use std::path::PathBuf;

use foundry_compilers::ProjectPathsConfig;

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

fn path_to_string(path: PathBuf) -> String {
  path.to_string_lossy().to_string()
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
