use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use foundry_compilers::artifacts::{
  error::Severity, output_selection::OutputSelection, remappings::Remapping, Settings,
};
use foundry_compilers::solc::SolcLanguage as FoundrySolcLanguage;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown, NapiRaw, ValueType};
use semver::Version;

use super::errors::{map_napi_error, napi_error};
use super::settings::CompilerSettings;

/// Normalised compiler configuration used by the Rust compiler facade.
#[derive(Clone, Debug)]
pub struct ResolvedCompilerConfig {
  /// Active solc version.
  pub solc_version: Version,
  /// Active solc language (Solidity or Yul).
  pub solc_language: FoundrySolcLanguage,
  /// Fully resolved standard-json settings.
  pub solc_settings: Settings,
  /// Controls whether Foundry caching is enabled.
  pub cache_enabled: bool,
  /// Optional project root used for caching and relative paths.
  pub base_dir: Option<PathBuf>,
  /// Prevent network access (svm downloads).
  pub offline_mode: bool,
  /// Skip writing artifacts to disk.
  pub no_artifacts: bool,
  /// Emit build-info files alongside artifacts.
  pub build_info_enabled: bool,
  /// Slash backslashes in generated paths (useful on Windows).
  pub slash_paths: bool,
  /// Explicit solc job count; `None` lets Foundry auto-detect.
  pub solc_jobs: Option<usize>,
  /// Enable sparse output selections for faster compiles.
  pub sparse_output: bool,
  /// Allowed import roots passed to solc (`--allow-paths`).
  pub allow_paths: BTreeSet<PathBuf>,
  /// Extra include paths for solc (`--include-path`).
  pub include_paths: BTreeSet<PathBuf>,
  /// Additional library directories.
  pub library_paths: Vec<PathBuf>,
  /// Resolved remappings.
  pub remappings: Vec<Remapping>,
  /// Paths suppressed from diagnostics.
  pub ignored_file_paths: BTreeSet<PathBuf>,
  /// Compiler diagnostic codes to ignore.
  pub ignored_error_codes: Vec<u64>,
  /// Minimum severity treated as an error.
  pub compiler_severity_filter: Severity,
}

impl Default for ResolvedCompilerConfig {
  fn default() -> Self {
    ResolvedCompilerConfig {
      solc_version: crate::internal::solc::default_version()
        .unwrap_or_else(|_| Version::new(0, 8, 30)),
      solc_language: crate::internal::solc::default_language(),
      solc_settings: default_compiler_settings(),
      cache_enabled: true,
      base_dir: None,
      offline_mode: false,
      no_artifacts: false,
      build_info_enabled: false,
      slash_paths: true,
      solc_jobs: None,
      sparse_output: false,
      allow_paths: BTreeSet::new(),
      include_paths: BTreeSet::new(),
      library_paths: Vec::new(),
      remappings: Vec::new(),
      ignored_file_paths: BTreeSet::new(),
      ignored_error_codes: Vec::new(),
      compiler_severity_filter: Severity::Error,
    }
  }
}

impl ResolvedCompilerConfig {
  /// Merge a set of overrides into the current configuration.
  pub fn merged(&self, overrides: &ConfigOverrides) -> Result<Self> {
    let mut next = self.clone();

    if let Some(version) = &overrides.solc_version {
      next.solc_version = version.clone();
    }
    if let Some(language) = overrides.solc_language {
      next.solc_language = language;
    }
    if let Some(settings) = overrides.resolved_settings.as_ref() {
      next.solc_settings = settings.clone();
    } else if let Some(settings) = overrides.solc_settings.as_ref() {
      next.solc_settings = resolve_settings(&next.solc_settings, Some(settings))?;
    }
    if let Some(cache) = overrides.cache_enabled {
      next.cache_enabled = cache;
    }
    if let Some(base_dir) = overrides.base_dir.as_ref() {
      next.base_dir = Some(base_dir.clone());
    }
    if let Some(offline) = overrides.offline_mode {
      next.offline_mode = offline;
    }
    if let Some(no_artifacts) = overrides.no_artifacts {
      next.no_artifacts = no_artifacts;
    }
    if let Some(build_info) = overrides.build_info_enabled {
      next.build_info_enabled = build_info;
    }
    if let Some(slash_paths) = overrides.slash_paths {
      next.slash_paths = slash_paths;
    }
    if let Some(solc_jobs) = overrides.solc_jobs.clone() {
      next.solc_jobs = solc_jobs;
    }
    if let Some(sparse_output) = overrides.sparse_output {
      next.sparse_output = sparse_output;
    }
    if let Some(allow_paths) = overrides.allow_paths.as_ref() {
      next.allow_paths = allow_paths.clone();
    }
    if let Some(include_paths) = overrides.include_paths.as_ref() {
      next.include_paths = include_paths.clone();
    }
    if let Some(libraries) = overrides.library_paths.as_ref() {
      next.library_paths = libraries.clone();
    }
    if let Some(remappings) = overrides.remappings.as_ref() {
      next.remappings = remappings.clone();
    }
    if let Some(ignored_paths) = overrides.ignored_file_paths.as_ref() {
      next.ignored_file_paths = ignored_paths.clone();
    }
    if let Some(ignored_codes) = overrides.ignored_error_codes.as_ref() {
      next.ignored_error_codes = ignored_codes.clone();
    }
    if let Some(severity) = overrides.compiler_severity_filter {
      next.compiler_severity_filter = severity;
    }

    Ok(next)
  }

  /// Merge optional JavaScript options on top of the config.
  pub fn merge_options(&self, options: Option<&CompilerConfig>) -> Result<Self> {
    let Some(options) = options else {
      return Ok(self.clone());
    };
    let overrides = ConfigOverrides::from_options(options)?;
    self.merged(&overrides)
  }

  /// Build a config purely from options (defaults + constructor overrides).
  pub fn from_options(options: Option<&CompilerConfig>) -> Result<Self> {
    let merged = ResolvedCompilerConfig::default().merge_options(options)?;
    let mut sanitized = merged.clone();
    sanitized.solc_settings = sanitize_settings(&merged.solc_settings)?;
    Ok(sanitized)
  }
}

/// Builder-style struct used to compose `CompilerConfig`.
#[derive(Default)]
pub struct ConfigOverrides {
  pub solc_version: Option<Version>,
  pub solc_language: Option<FoundrySolcLanguage>,
  pub solc_settings: Option<CompilerSettings>,
  pub resolved_settings: Option<Settings>,
  pub cache_enabled: Option<bool>,
  pub base_dir: Option<PathBuf>,
  pub offline_mode: Option<bool>,
  pub no_artifacts: Option<bool>,
  pub build_info_enabled: Option<bool>,
  pub slash_paths: Option<bool>,
  pub solc_jobs: Option<Option<usize>>,
  pub sparse_output: Option<bool>,
  pub allow_paths: Option<BTreeSet<PathBuf>>,
  pub include_paths: Option<BTreeSet<PathBuf>>,
  pub library_paths: Option<Vec<PathBuf>>,
  pub remappings: Option<Vec<Remapping>>,
  pub ignored_file_paths: Option<BTreeSet<PathBuf>>,
  pub ignored_error_codes: Option<Vec<u64>>,
  pub compiler_severity_filter: Option<Severity>,
}

impl ConfigOverrides {
  pub fn from_options(options: &CompilerConfig) -> Result<Self> {
    let mut overrides = ConfigOverrides::default();

    if let Some(version) = options.solc_version.as_ref() {
      overrides.solc_version = Some(parse_version(version)?);
    }

    overrides.solc_language = options.solc_language.map(Into::into);

    if let Some(settings) = options.solc_settings.as_ref().or(options.settings.as_ref()) {
      overrides.solc_settings = Some(settings.clone());
    }

    overrides.cache_enabled = options.cache_enabled;
    overrides.base_dir = options
      .base_dir
      .as_ref()
      .map(|dir| canonicalize_path(Path::new(dir)));
    overrides.offline_mode = options.offline_mode;
    overrides.no_artifacts = options.no_artifacts;
    overrides.build_info_enabled = options.build_info_enabled;
    overrides.slash_paths = options.slash_paths;
    overrides.solc_jobs = options
      .solc_jobs
      .map(|jobs| if jobs == 0 { None } else { Some(jobs as usize) });
    overrides.sparse_output = options.sparse_output;
    overrides.allow_paths = options.allow_paths.as_ref().map(|paths| to_path_set(paths));
    overrides.include_paths = options
      .include_paths
      .as_ref()
      .map(|paths| to_path_set(paths));
    overrides.library_paths = options
      .library_paths
      .as_ref()
      .map(|paths| to_path_vec(paths));
    overrides.remappings = map_remappings(options.remappings.as_ref())?;
    overrides.ignored_file_paths = options
      .ignored_paths
      .as_ref()
      .map(|paths| to_path_set(paths));
    overrides.ignored_error_codes = options
      .ignored_error_codes
      .as_ref()
      .map(|codes| codes.iter().map(|code| *code as u64).collect());

    if let Some(severity) = options.compiler_severity.as_ref() {
      overrides.compiler_severity_filter = Some(parse_severity(severity)?);
    }

    Ok(overrides)
  }
}

/// JavaScript-facing compiler configuration shared by all entry points.
#[napi(object)]
#[derive(Clone, Default)]
pub struct CompilerConfig {
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  #[napi(ts_type = "import('./index').SolcLanguage | undefined")]
  pub solc_language: Option<SolcLanguage>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub solc_settings: Option<CompilerSettings>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub settings: Option<CompilerSettings>,
  #[napi(ts_type = "boolean | undefined")]
  pub cache_enabled: Option<bool>,
  #[napi(ts_type = "string | undefined")]
  pub base_dir: Option<String>,
  #[napi(ts_type = "boolean | undefined")]
  pub offline_mode: Option<bool>,
  #[napi(ts_type = "boolean | undefined")]
  pub no_artifacts: Option<bool>,
  #[napi(ts_type = "boolean | undefined")]
  pub build_info_enabled: Option<bool>,
  #[napi(ts_type = "boolean | undefined")]
  pub slash_paths: Option<bool>,
  #[napi(ts_type = "number | undefined")]
  pub solc_jobs: Option<u32>,
  #[napi(ts_type = "boolean | undefined")]
  pub sparse_output: Option<bool>,
  #[napi(ts_type = "string[] | undefined")]
  pub allow_paths: Option<Vec<String>>,
  #[napi(ts_type = "string[] | undefined")]
  pub include_paths: Option<Vec<String>>,
  #[napi(ts_type = "string[] | undefined")]
  pub library_paths: Option<Vec<String>>,
  #[napi(ts_type = "string[] | undefined")]
  pub remappings: Option<Vec<String>>,
  #[napi(ts_type = "number[] | undefined")]
  pub ignored_error_codes: Option<Vec<i64>>,
  #[napi(ts_type = "string[] | undefined")]
  pub ignored_paths: Option<Vec<String>>,
  #[napi(ts_type = "string | undefined")]
  pub compiler_severity: Option<String>,
}

/// JavaScript-facing AST options.
#[napi(object)]
#[derive(Clone, Default)]
pub struct AstOptions {
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  #[napi(ts_type = "import('./index').SolcLanguage | undefined")]
  pub solc_language: Option<SolcLanguage>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub settings: Option<CompilerSettings>,
  #[napi(ts_type = "string | undefined")]
  pub instrumented_contract: Option<String>,
}

/// Supported solc languages.
#[napi(string_enum)]
#[derive(Debug, Eq, PartialEq)]
pub enum SolcLanguage {
  Solidity,
  Yul,
}

impl From<SolcLanguage> for FoundrySolcLanguage {
  fn from(language: SolcLanguage) -> Self {
    match language {
      SolcLanguage::Solidity => FoundrySolcLanguage::Solidity,
      SolcLanguage::Yul => FoundrySolcLanguage::Yul,
    }
  }
}

pub(crate) trait SolcUserOptions {
  fn solc_version(&self) -> Option<&str>;
  fn solc_language(&self) -> Option<SolcLanguage>;
  fn settings(&self) -> Option<&CompilerSettings>;
}

impl SolcUserOptions for CompilerConfig {
  fn solc_version(&self) -> Option<&str> {
    self.solc_version.as_deref()
  }

  fn solc_language(&self) -> Option<SolcLanguage> {
    self.solc_language
  }

  fn settings(&self) -> Option<&CompilerSettings> {
    self.solc_settings.as_ref().or(self.settings.as_ref())
  }
}

impl SolcUserOptions for AstOptions {
  fn solc_version(&self) -> Option<&str> {
    self.solc_version.as_deref()
  }

  fn solc_language(&self) -> Option<SolcLanguage> {
    self.solc_language
  }

  fn settings(&self) -> Option<&CompilerSettings> {
    self.settings.as_ref()
  }
}

impl AstOptions {
  pub fn instrumented_contract(&self) -> Option<&str> {
    self.instrumented_contract.as_deref()
  }
}

#[derive(Clone)]
pub struct SolcConfig {
  pub version: Version,
  pub settings: Settings,
  pub language: FoundrySolcLanguage,
}

impl SolcConfig {
  pub fn new<O: SolcUserOptions>(
    default_language: &FoundrySolcLanguage,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let default_version = crate::internal::solc::default_version()?;
    Self::with_defaults(
      default_language,
      &default_version,
      default_settings,
      overrides,
    )
  }

  pub fn with_defaults<O: SolcUserOptions>(
    default_language: &FoundrySolcLanguage,
    default_version: &Version,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .map(crate::internal::solc::parse_version)
      .transpose()?
      .unwrap_or_else(|| default_version.clone());

    let language = overrides
      .and_then(|opts| opts.solc_language())
      .map(FoundrySolcLanguage::from)
      .unwrap_or_else(|| default_language.clone());

    let settings = resolve_settings(default_settings, overrides.and_then(|opts| opts.settings()))?;

    Ok(SolcConfig {
      version,
      settings,
      language,
    })
  }

  pub fn merge<O: SolcUserOptions>(&self, overrides: Option<&O>) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .map(crate::internal::solc::parse_version)
      .transpose()?
      .unwrap_or_else(|| self.version.clone());

    let language = overrides
      .and_then(|opts| opts.solc_language())
      .map(FoundrySolcLanguage::from)
      .unwrap_or_else(|| self.language.clone());

    let settings = resolve_settings(&self.settings, overrides.and_then(|opts| opts.settings()))?;

    Ok(SolcConfig {
      version,
      settings,
      language,
    })
  }
}

/// Parse `CompilerConfig` from a JS value (undefined/null → `None`).
pub(crate) fn parse_compiler_config(
  env: &Env,
  value: Option<JsUnknown>,
) -> Result<Option<CompilerConfig>> {
  parse_options(value)?
    .map(|unknown| unsafe { CompilerConfig::from_napi_value(env.raw(), unknown.raw()) })
    .transpose()
}

/// Parse `AstOptions` from a JS value (undefined/null → `None`).
pub(crate) fn parse_ast_options(env: &Env, value: Option<JsUnknown>) -> Result<Option<AstOptions>> {
  parse_options(value)?
    .map(|unknown| unsafe { AstOptions::from_napi_value(env.raw(), unknown.raw()) })
    .transpose()
}

fn parse_options(value: Option<JsUnknown>) -> Result<Option<JsUnknown>> {
  let Some(value) = value else {
    return Ok(None);
  };

  match value.get_type()? {
    ValueType::Undefined | ValueType::Null => Ok(None),
    ValueType::Object => {
      let object: JsObject = value.coerce_to_object()?;
      validate_object_field(&object, "solcSettings")?;
      validate_object_field(&object, "settings")?;
      Ok(Some(object.into_unknown()))
    }
    _ => Err(napi_error(
      "Compiler options must be an object or undefined/null.",
    )),
  }
}

fn default_compiler_settings() -> Settings {
  Settings::default()
}

fn validate_object_field(object: &JsObject, field: &str) -> Result<()> {
  if !object.has_named_property(field)? {
    return Ok(());
  }

  let value: JsUnknown = object.get_named_property(field)?;
  match value.get_type()? {
    ValueType::Undefined | ValueType::Null | ValueType::Object => Ok(()),
    _ => Err(napi_error(format!(
      "{field} override must be provided as an object."
    ))),
  }
}

fn resolve_settings(base: &Settings, overrides: Option<&CompilerSettings>) -> Result<Settings> {
  match overrides {
    Some(settings) => {
      let mut merged = settings.clone().overlay(base)?;
      if output_selection_is_effectively_empty(&merged.output_selection) {
        merged.output_selection = Settings::default().output_selection;
      }
      Ok(merged)
    }
    None => Ok(base.clone()),
  }
}

fn sanitize_settings(settings: &Settings) -> Result<Settings> {
  let mut merged = settings.clone();
  if output_selection_is_effectively_empty(&merged.output_selection) {
    merged.output_selection = Settings::default().output_selection;
  }
  Ok(merged)
}

fn canonicalize_path(path: &Path) -> PathBuf {
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

fn to_path_set(paths: &[String]) -> BTreeSet<PathBuf> {
  paths
    .iter()
    .map(|p| canonicalize_path(Path::new(p)))
    .collect()
}

fn output_selection_is_effectively_empty(selection: &OutputSelection) -> bool {
  let map = selection.as_ref();
  if map.is_empty() {
    return true;
  }

  map.values().all(|contracts| {
    contracts
      .values()
      .all(|outputs| outputs.iter().all(|output| output.trim().is_empty()))
  })
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use super::*;

  #[test]
  fn empty_output_selection_is_sanitized() {
    let base = Settings::default();
    let mut overrides = CompilerSettings::default();
    overrides.output_selection = Some(BTreeMap::from([(
      "*".to_string(),
      BTreeMap::from([("*".to_string(), Vec::new()), (String::new(), Vec::new())]),
    )]));

    let merged = resolve_settings(&base, Some(&overrides)).expect("settings");
    assert!(
      !output_selection_is_effectively_empty(&merged.output_selection),
      "merged selection should fallback to defaults"
    );
  }
}

fn to_path_vec(paths: &Vec<String>) -> Vec<PathBuf> {
  paths
    .iter()
    .map(|p| canonicalize_path(Path::new(p)))
    .collect()
}

fn map_remappings(remappings: Option<&Vec<String>>) -> Result<Option<Vec<Remapping>>> {
  remappings
    .map(|values| {
      values
        .iter()
        .map(|value| {
          Remapping::from_str(value)
            .map_err(|err| napi_error(format!("Invalid remapping \"{value}\": {err}")))
        })
        .collect::<Result<Vec<_>>>()
    })
    .transpose()
}

fn parse_version(value: &str) -> Result<Version> {
  map_napi_error(
    Version::parse(value.trim().trim_start_matches('v')),
    "Failed to parse solc version",
  )
}

fn parse_severity(value: &str) -> Result<Severity> {
  match value.to_ascii_lowercase().as_str() {
    "error" => Ok(Severity::Error),
    "warning" => Ok(Severity::Warning),
    "info" | "information" => Ok(Severity::Info),
    other => Err(napi_error(format!(
      "Unsupported compiler severity filter \"{other}\""
    ))),
  }
}
