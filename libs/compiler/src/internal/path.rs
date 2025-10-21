use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use foundry_compilers::ProjectPathsConfig;

/// Canonicalised project directory layout surfaced to JavaScript consumers.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPaths {
  /// Absolute workspace root resolved by the compiler—this is the directory `foundry.toml` or
  /// `hardhat.config` was discovered in (or the synthetic project root for inline sources).
  pub root: String,
  /// Path to the cache directory/file managed by Foundry (typically `cache/solidity-files-cache.json`).
  pub cache: String,
  /// Directory where compiled artifacts are written (e.g. `out/` for Foundry projects).
  pub artifacts: String,
  /// Directory containing Foundry build-info JSON metadata (`out/build-info`).
  pub build_infos: String,
  /// Canonical directory for contract sources (usually `src/`).
  pub sources: String,
  /// Directory containing contract tests (Foundry's `test/` folder).
  pub tests: String,
  /// Directory containing project scripts (`script/` or `scripts/`, depending on the toolchain).
  pub scripts: String,
  /// Additional library directories configured for the project (e.g. `lib/`, `node_modules/`).
  pub libraries: Vec<String>,
  /// Additional include paths forwarded to the compiler. These are canonicalised absolute paths.
  pub include_paths: Vec<String>,
  /// Extra paths permitted by solc's `--allow-paths` flag.
  pub allowed_paths: Vec<String>,
  /// The path to the virtual sources directory used for caching inline inputs (for synthetic
  /// projects this lives under `~/.tevm/virtual-sources`).
  pub virtual_sources: Option<String>,
}

impl ProjectPaths {
  /// Build a JavaScript-facing snapshot from a Foundry project configuration.
  pub fn from_config<L>(config: &ProjectPathsConfig<L>) -> Self {
    ProjectPaths {
      root: config.root.to_string_lossy().to_string(),
      cache: config.cache.to_string_lossy().to_string(),
      artifacts: config.artifacts.to_string_lossy().to_string(),
      build_infos: config.build_infos.to_string_lossy().to_string(),
      sources: config.sources.to_string_lossy().to_string(),
      tests: config.tests.to_string_lossy().to_string(),
      scripts: config.scripts.to_string_lossy().to_string(),
      libraries: config
        .libraries
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect(),
      include_paths: config
        .include_paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect(),
      allowed_paths: config
        .allowed_paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect(),
      virtual_sources: None,
    }
  }

  /// Attach the virtual sources directory used for caching inline inputs.
  pub fn with_virtual_sources(mut self, dir: Option<&Path>) -> Self {
    self.virtual_sources = dir.map(|path| path.to_string_lossy().to_string());
    self
  }
}

impl<L> From<&ProjectPathsConfig<L>> for ProjectPaths {
  fn from(config: &ProjectPathsConfig<L>) -> Self {
    ProjectPaths::from_config(config)
  }
}

/// Canonicalises a path while falling back to an absolute join if canonicalisation fails.
///
/// This ensures missing paths are resolved to the current working directory, so the compiler
/// maintains predictable path resolution even for yet-to-be written files.
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

  #[test]
  fn project_paths_from_config_captures_all_directories() {
    use foundry_compilers::solc::SolcLanguage;
    use foundry_compilers::ProjectPathsConfig;

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let cache_file = root.join("cache").join("cache.json");
    let artifacts = root.join("artifacts");
    let build_infos = artifacts.join("build-info");
    let sources = root.join("src");
    let tests = root.join("test");
    let scripts = root.join("scripts");
    let library = root.join("lib");
    std::fs::create_dir_all(cache_file.parent().unwrap()).expect("cache dir");
    std::fs::create_dir_all(&artifacts).expect("artifacts");
    std::fs::create_dir_all(&build_infos).expect("build infos");
    std::fs::create_dir_all(&sources).expect("sources");
    std::fs::create_dir_all(&tests).expect("tests");
    std::fs::create_dir_all(&scripts).expect("scripts");
    std::fs::create_dir_all(&library).expect("lib");

    let config = ProjectPathsConfig::builder()
      .root(root)
      .cache(&cache_file)
      .artifacts(&artifacts)
      .build_infos(&build_infos)
      .sources(&sources)
      .tests(&tests)
      .scripts(&scripts)
      .libs(vec![library.clone()])
      .include_paths(vec![root.join("includes")])
      .allowed_paths(vec![root.join("allowed")])
      .build_with_root::<SolcLanguage>(root);

    let paths = ProjectPaths::from_config(&config).with_virtual_sources(Some(root));
    assert!(paths.cache.ends_with("cache.json"));

    let canonical = |value: &str| Path::new(value).canonicalize().unwrap();
    assert_eq!(
      canonical(&paths.artifacts),
      artifacts.canonicalize().unwrap()
    );
    assert_eq!(
      canonical(&paths.build_infos),
      build_infos.canonicalize().unwrap()
    );
    assert_eq!(canonical(&paths.sources), sources.canonicalize().unwrap());
    assert_eq!(canonical(&paths.tests), tests.canonicalize().unwrap());
    assert_eq!(canonical(&paths.scripts), scripts.canonicalize().unwrap());
    let library_canonical = library.canonicalize().unwrap();
    assert!(paths
      .libraries
      .iter()
      .map(|entry| canonical(entry))
      .any(|path| path == library_canonical));
    let virtual_sources = canonical(paths.virtual_sources.as_ref().expect("virtual"));
    assert_eq!(virtual_sources, root.canonicalize().unwrap());
    assert!(paths
      .include_paths
      .iter()
      .any(|path| path.ends_with("includes")));
    assert!(paths
      .allowed_paths
      .iter()
      .any(|path| path.ends_with("allowed")));
  }
}
