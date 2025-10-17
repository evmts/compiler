use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use foundry_compilers::solc::SolcLanguage as FoundrySolcLanguage;
use foundry_compilers::{
  cache::SOLIDITY_FILES_CACHE_FILENAME,
  solc::{CliSettings, SolcCompiler, SolcSettings},
  Project, ProjectBuilder, ProjectPathsConfig,
};
use napi::bindgen_prelude::Result;

use super::config::ResolvedCompilerConfig;
use super::errors::{map_napi_error, napi_error};

#[derive(Clone)]
pub enum ProjectLayout {
  Hardhat,
  Foundry,
  Synthetic,
}

#[derive(Clone)]
pub struct ProjectContext {
  pub layout: ProjectLayout,
  pub root: PathBuf,
  pub paths: ProjectPathsConfig<FoundrySolcLanguage>,
  pub virtual_sources_dir: Option<PathBuf>,
}

pub fn build_project(
  config: &ResolvedCompilerConfig,
  context: &ProjectContext,
) -> Result<Project<SolcCompiler>> {
  let mut paths = context.paths.clone();
  extend_paths_with_config(&mut paths, config);

  let mut builder = ProjectBuilder::default().paths(paths);

  builder = builder.set_cached(config.cache_enabled);
  builder = builder.set_offline(config.offline_mode);
  builder = builder.set_no_artifacts(config.no_artifacts);
  builder = builder.set_build_info(config.build_info_enabled);
  builder = builder.set_slashed_paths(config.slash_paths);
  if let Some(solc_jobs) = config.solc_jobs {
    if solc_jobs == 1 {
      builder = builder.single_solc_jobs();
    } else if solc_jobs > 1 {
      builder = builder.solc_jobs(solc_jobs);
    }
  }
  if !config.ignored_file_paths.is_empty() {
    builder = builder.ignore_paths(config.ignored_file_paths.iter().cloned().collect());
  }
  if !config.ignored_error_codes.is_empty() {
    builder = builder.ignore_error_codes(config.ignored_error_codes.clone());
  }
  builder = builder.set_compiler_severity_filter(config.compiler_severity_filter);

  let cli_settings = CliSettings {
    extra_args: Vec::new(),
    allow_paths: config.allow_paths.clone(),
    base_path: Some(context.root.clone()),
    include_paths: config.include_paths.clone(),
  };

  let solc_settings = SolcSettings {
    settings: config.solc_settings.clone(),
    cli_settings,
  };

  builder = builder.settings(solc_settings);

  map_napi_error(
    builder.build(SolcCompiler::default()),
    "Failed to configure Solidity project",
  )
}

pub fn create_synthetic_context(base_dir: &Path) -> Result<ProjectContext> {
  let root = absolute_path(base_dir);
  let tevm_root = root.join(".tevm");
  let cache_dir = tevm_root.join("cache");
  let artifacts_dir = tevm_root.join("out");
  let build_info_dir = artifacts_dir.join("build-info");
  let virtual_sources_dir = tevm_root.join("virtual-sources");

  for dir in [
    &tevm_root,
    &cache_dir,
    &artifacts_dir,
    &build_info_dir,
    &virtual_sources_dir,
  ] {
    create_dir_if_missing(dir)?;
  }

  let cache_file = cache_dir.join(SOLIDITY_FILES_CACHE_FILENAME);

  let sources_dir = root.clone();
  let tests_dir = root.join("test");
  let scripts_dir = root.join("scripts");

  let paths = ProjectPathsConfig::builder()
    .root(&root)
    .cache(&cache_file)
    .artifacts(&artifacts_dir)
    .build_infos(&build_info_dir)
    .sources(&sources_dir)
    .tests(&tests_dir)
    .scripts(&scripts_dir)
    .no_libs()
    .build_with_root::<FoundrySolcLanguage>(&root);

  Ok(ProjectContext {
    layout: ProjectLayout::Synthetic,
    root,
    paths,
    virtual_sources_dir: Some(virtual_sources_dir),
  })
}

fn extend_paths_with_config(
  paths: &mut ProjectPathsConfig<FoundrySolcLanguage>,
  config: &ResolvedCompilerConfig,
) {
  if !config.library_paths.is_empty() {
    let mut libraries: BTreeSet<PathBuf> = paths.libraries.iter().cloned().collect::<BTreeSet<_>>();
    for lib in &config.library_paths {
      libraries.insert(lib.clone());
    }
    paths.libraries = libraries.into_iter().collect();
  }

  for path in &config.include_paths {
    paths.include_paths.insert(path.clone());
  }

  for path in &config.allow_paths {
    paths.allowed_paths.insert(path.clone());
  }
}

fn absolute_path(path: &Path) -> PathBuf {
  match path.canonicalize() {
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

fn create_dir_if_missing(path: &Path) -> Result<()> {
  if let Err(err) = fs::create_dir_all(path) {
    return Err(napi_error(format!(
      "Failed to create directory {}: {err}",
      path.display()
    )));
  }
  Ok(())
}
