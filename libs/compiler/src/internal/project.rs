use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use foundry_compilers::artifacts::{
  error::Severity, remappings::Remapping, CompilerOutput, Settings,
};
use foundry_compilers::buildinfo::BuildInfo;
use foundry_compilers::solc::{SolcLanguage as FoundrySolcLanguage, SolcVersionedInput};
use foundry_compilers::{
  cache::SOLIDITY_FILES_CACHE_FILENAME,
  solc::{CliSettings, SolcCompiler, SolcSettings},
  Project, ProjectBuilder, ProjectPathsConfig,
};
use foundry_config::{Config as FoundryConfig, SolcReq};

use crate::internal::config::{CompilerConfig, CompilerConfigOptions, SolcConfig};
use crate::internal::errors::{map_err_with_context, Error, Result};
use crate::internal::path::{canonicalize_path, canonicalize_with_base};
use crate::internal::settings::CompilerSettingsOptions;

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

impl ProjectContext {
  pub fn normalise_paths(
    &self,
    config: &CompilerConfig,
    inputs: &[PathBuf],
  ) -> Result<Vec<PathBuf>> {
    let mut resolved = Vec::with_capacity(inputs.len());
    for entry in inputs {
      let candidate = if entry.is_absolute() {
        entry.clone()
      } else if let Some(base) = config.base_dir.as_ref() {
        base.join(entry)
      } else {
        self.root.join(entry)
      };

      let canonical = match candidate.canonicalize() {
        Ok(path) => path,
        Err(_) => candidate.clone(),
      };

      if !canonical.exists() {
        return Err(Error::new(format!(
          "Source file {} does not exist",
          canonical.display()
        )));
      }

      resolved.push(canonical);
    }
    Ok(resolved)
  }

  pub fn virtual_source_path(&self, hash: &str, extension: &str) -> Result<PathBuf> {
    let dir = self
      .virtual_sources_dir
      .as_ref()
      .ok_or_else(|| Error::new("Cannot cache inline sources without a baseDir"))?;

    if let Err(err) = std::fs::create_dir_all(dir) {
      return Err(Error::new(format!(
        "Failed to prepare virtual sources directory {}: {err}",
        dir.display()
      )));
    }

    Ok(dir.join(format!("virtual-{hash}.{extension}")))
  }
}

pub fn build_project(
  config: &CompilerConfig,
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

  map_err_with_context(
    builder.build(SolcCompiler::default()),
    "Failed to configure Solidity project",
  )
}

pub fn create_synthetic_context(base_dir: &Path) -> Result<ProjectContext> {
  let root = canonicalize_path(base_dir);
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
  config: &CompilerConfig,
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

fn create_dir_if_missing(path: &Path) -> Result<()> {
  if let Err(err) = fs::create_dir_all(path) {
    return Err(Error::new(format!(
      "Failed to create directory {}: {err}",
      path.display()
    )));
  }
  Ok(())
}

pub struct FoundryAdapter;

impl FoundryAdapter {
  pub fn load(root: &Path) -> Result<(CompilerConfigOptions, ProjectContext)> {
    let figment = FoundryConfig::figment_with_root(root);
    let config = map_err_with_context(
      FoundryConfig::try_from(figment),
      "Failed to load foundry configuration",
    )?
    .sanitized()
    .canonic();

    let mut overrides = CompilerConfigOptions::default();
    let base_dir = config.__root.0.clone();
    overrides.base_dir = Some(base_dir.clone());
    overrides.cache_enabled = Some(config.cache);
    overrides.offline_mode = Some(config.offline);
    overrides.no_artifacts = Some(false);
    overrides.build_info_enabled = Some(config.build_info);
    overrides.sparse_output = Some(config.sparse_mode);

    if let Some(SolcReq::Version(version)) = &config.solc {
      overrides.solc.version = Some(version.clone());
    }

    let ethers_settings = map_err_with_context(
      config.solc_settings(),
      "Failed to derive foundry compiler settings",
    )?;
    let settings_json = map_err_with_context(
      serde_json::to_value(&ethers_settings),
      "Failed to serialise foundry compiler settings",
    )?;
    let settings: Settings = map_err_with_context(
      serde_json::from_value(settings_json),
      "Failed to convert foundry compiler settings",
    )?;
    overrides.solc.resolved_settings = Some(settings);

    overrides.allow_paths = Some(
      config
        .allow_paths
        .iter()
        .map(|p| canonicalize_with_base(&base_dir, p))
        .collect::<BTreeSet<_>>(),
    );
    if let Some(allow) = overrides.allow_paths.as_mut() {
      allow.insert(base_dir.clone());
    }
    overrides.include_paths = Some(
      config
        .include_paths
        .iter()
        .map(|p| canonicalize_with_base(&base_dir, p))
        .collect::<BTreeSet<_>>(),
    );
    overrides.library_paths = Some(
      config
        .libs
        .iter()
        .map(|p| canonicalize_with_base(&base_dir, p))
        .collect::<Vec<_>>(),
    );
    overrides.remappings = Some(
      config
        .remappings
        .iter()
        .filter_map(|remapping| Remapping::from_str(&remapping.to_string()).ok())
        .collect(),
    );
    overrides.ignored_error_codes = Some(
      config
        .ignored_error_codes
        .iter()
        .map(|code| (*code).into())
        .collect(),
    );
    if config.deny_warnings {
      overrides.compiler_severity_filter = Some(Severity::Warning);
    }

    let config_paths = config.project_paths();
    let mut paths = ProjectPathsConfig::builder()
      .root(config_paths.root.clone())
      .cache(config_paths.cache.clone())
      .artifacts(config_paths.artifacts.clone())
      .build_infos(config_paths.build_infos.clone())
      .sources(config_paths.sources.clone())
      .tests(config_paths.tests.clone())
      .scripts(config_paths.scripts.clone())
      .libs(config_paths.libraries.clone())
      .remappings(
        config_paths
          .remappings
          .iter()
          .filter_map(|remapping| Remapping::from_str(&remapping.to_string()).ok())
          .collect::<Vec<_>>(),
      )
      .build_with_root::<FoundrySolcLanguage>(&config_paths.root);
    paths.slash_paths();
    let context = ProjectContext {
      layout: ProjectLayout::Foundry,
      root: base_dir,
      paths,
      virtual_sources_dir: None,
    };

    Ok((overrides, context))
  }
}

pub struct HardhatAdapter;

impl HardhatAdapter {
  pub fn load(root: &Path) -> Result<(CompilerConfigOptions, ProjectContext)> {
    let mut paths = map_err_with_context(
      ProjectPathsConfig::hardhat(root),
      "Failed to create hardhat project paths",
    )?;
    paths.slash_paths();

    let mut overrides = CompilerConfigOptions::default();
    overrides.base_dir = Some(paths.root.clone());
    overrides.cache_enabled = Some(true);
    overrides.build_info_enabled = Some(true);
    overrides.no_artifacts = Some(false);

    if let Some((solc_config, cli_settings)) = infer_hardhat_build_info(&paths) {
      overrides.solc.version = Some(solc_config.version);
      let settings_json = map_err_with_context(
        serde_json::to_value(&solc_config.settings),
        "Failed to serialise hardhat compiler settings",
      )?;
      let solc_settings: CompilerSettingsOptions = map_err_with_context(
        serde_json::from_value(settings_json),
        "Failed to convert hardhat compiler settings",
      )?;
      overrides.solc.settings = Some(solc_settings);
      overrides.allow_paths = Some(
        cli_settings
          .allow_paths
          .into_iter()
          .map(|p| canonicalize_with_base(&paths.root, &p))
          .collect::<BTreeSet<_>>(),
      );
      if let Some(allow) = overrides.allow_paths.as_mut() {
        allow.insert(paths.root.clone());
      }
      overrides.include_paths = Some(
        cli_settings
          .include_paths
          .into_iter()
          .map(|p| canonicalize_with_base(&paths.root, &p))
          .collect::<BTreeSet<_>>(),
      );
    }

    overrides.library_paths = Some(
      paths
        .libraries
        .iter()
        .map(|p| canonicalize_with_base(&paths.root, p))
        .collect::<Vec<_>>(),
    );

    let context = ProjectContext {
      layout: ProjectLayout::Hardhat,
      root: paths.root.clone(),
      paths,
      virtual_sources_dir: None,
    };

    Ok((overrides, context))
  }
}

fn infer_hardhat_build_info(
  paths: &ProjectPathsConfig<FoundrySolcLanguage>,
) -> Option<(SolcConfig, CliSettingsData)> {
  let entries = fs::read_dir(&paths.build_infos).ok()?;
  let mut latest: Option<(SystemTime, PathBuf)> = None;

  for entry in entries.flatten() {
    let file_type = entry.file_type().ok()?;
    if !file_type.is_file() {
      continue;
    }

    if entry
      .path()
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext != "json")
      .unwrap_or(true)
    {
      continue;
    }

    let modified = entry
      .metadata()
      .and_then(|meta| meta.modified())
      .unwrap_or(SystemTime::UNIX_EPOCH);

    match &mut latest {
      Some((time, path)) => {
        if modified > *time {
          *time = modified;
          *path = entry.path();
        }
      }
      None => latest = Some((modified, entry.path())),
    }
  }

  let (_, path) = latest?;
  let build_info: BuildInfo<SolcVersionedInput, CompilerOutput> = BuildInfo::read(&path).ok()?;

  let compiler_config = SolcConfig {
    version: build_info.solc_version.clone(),
    settings: build_info.input.input.settings.clone(),
    language: build_info.input.input.language,
  };

  let cli_settings = CliSettingsData {
    allow_paths: build_info
      .input
      .cli_settings
      .allow_paths
      .iter()
      .cloned()
      .collect(),
    include_paths: build_info
      .input
      .cli_settings
      .include_paths
      .iter()
      .cloned()
      .collect(),
  };

  Some((compiler_config, cli_settings))
}

struct CliSettingsData {
  allow_paths: BTreeSet<PathBuf>,
  include_paths: BTreeSet<PathBuf>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;
  use tempfile::tempdir;

  #[test]
  fn normalise_paths_resolves_relative_entries() {
    let temp = tempdir().expect("tempdir");
    let context = create_synthetic_context(temp.path()).expect("context");
    let target = context.root.join("Example.sol");
    std::fs::write(&target, "// test").expect("write file");

    let config = CompilerConfig::default();
    let resolved = context
      .normalise_paths(&config, &[PathBuf::from("Example.sol")])
      .expect("normalised paths");
    assert_eq!(resolved, vec![target.canonicalize().unwrap()]);
  }

  #[test]
  fn virtual_source_path_prepares_directory() {
    let temp = tempdir().expect("tempdir");
    let context = create_synthetic_context(temp.path()).expect("context");
    let path = context
      .virtual_source_path("hash", "sol")
      .expect("virtual path");
    assert!(path.ends_with("virtual-hash.sol"));
    assert!(path.parent().unwrap().exists());
  }
}
