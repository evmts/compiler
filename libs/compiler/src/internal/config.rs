use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use foundry_compilers::artifacts::{error::Severity, remappings::Remapping, Settings};
use foundry_compilers::solc::SolcLanguage as FoundrySolcLanguage;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown, NapiRaw, ValueType};
use semver::Version;

use crate::internal::errors::{map_napi_error, napi_error};
use crate::internal::path::{canonicalize_path, to_path_set, to_path_vec};
use crate::internal::settings::{
  merge_settings, sanitize_settings, CompilerSettingsOptions, JsCompilerSettingsOptions,
};

/// Finalised compiler configuration consumed by the Rust compiler facade and
/// passed downstream to Foundry.
#[derive(Clone, Debug)]
pub struct CompilerConfig {
  pub solc_version: Version,
  pub solc_language: FoundrySolcLanguage,
  pub solc_settings: Settings,
  pub cache_enabled: bool,
  pub base_dir: Option<PathBuf>,
  pub offline_mode: bool,
  pub no_artifacts: bool,
  pub build_info_enabled: bool,
  pub slash_paths: bool,
  pub solc_jobs: Option<usize>,
  pub sparse_output: bool,
  pub allow_paths: BTreeSet<PathBuf>,
  pub include_paths: BTreeSet<PathBuf>,
  pub library_paths: Vec<PathBuf>,
  pub remappings: Vec<Remapping>,
  pub ignored_file_paths: BTreeSet<PathBuf>,
  pub ignored_error_codes: Vec<u64>,
  pub compiler_severity_filter: Severity,
}

impl Default for CompilerConfig {
  fn default() -> Self {
    CompilerConfig {
      solc_version: crate::internal::solc::default_version()
        .unwrap_or_else(|_| Version::new(0, 8, 30)),
      solc_language: crate::internal::solc::default_language(),
      solc_settings: Settings::default(),
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

impl CompilerConfig {
  pub fn merged(&self, overrides: &CompilerConfigOptions) -> Result<Self> {
    CompilerConfigBuilder::with_base(self.clone())
      .apply_compiler_options(overrides.clone())?
      .build()
  }

  pub fn merge_options(&self, options: Option<&CompilerConfigOptions>) -> Result<Self> {
    let mut builder = CompilerConfigBuilder::with_base(self.clone());
    if let Some(overrides) = options {
      builder = builder.apply_compiler_options(overrides.clone())?;
    }
    builder.build()
  }

  pub fn from_options(options: Option<CompilerConfigOptions>) -> Result<Self> {
    let mut builder = CompilerConfigBuilder::from_defaults();
    if let Some(overrides) = options {
      builder = builder.apply_compiler_options(overrides)?;
    }
    builder.build()
  }
}

/// Optional overrides for constructing a [`SolcConfig`].
#[derive(Clone, Debug, Default)]
pub struct SolcConfigOptions {
  pub version: Option<Version>,
  pub language: Option<FoundrySolcLanguage>,
  pub settings: Option<CompilerSettingsOptions>,
  pub resolved_settings: Option<Settings>,
}

/// Strongly-typed Rust overrides that can be merged into a [`CompilerConfig`].
#[derive(Clone, Debug, Default)]
pub struct CompilerConfigOptions {
  pub solc: SolcConfigOptions,
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

#[derive(Clone, Debug, Default)]
pub struct AstConfigOptions {
  pub solc: SolcConfigOptions,
  pub instrumented_contract: Option<String>,
}

impl AstConfigOptions {
  pub fn instrumented_contract(&self) -> Option<&str> {
    self.instrumented_contract.as_deref()
  }
}

#[derive(Clone, Debug)]
pub struct AstConfig {
  pub solc: SolcConfig,
  pub instrumented_contract: Option<String>,
}

impl AstConfig {
  pub fn from_options(
    default_language: &FoundrySolcLanguage,
    default_settings: &Settings,
    options: Option<&AstConfigOptions>,
  ) -> Result<Self> {
    let solc = SolcConfig::new(default_language, default_settings, options)?;
    Ok(AstConfig {
      solc,
      instrumented_contract: options.and_then(|opts| opts.instrumented_contract.clone()),
    })
  }

  pub fn merged(&self, overrides: &AstConfigOptions) -> Result<Self> {
    let solc = self.solc.merge(Some(overrides))?;
    let instrumented_contract = overrides
      .instrumented_contract
      .clone()
      .or_else(|| self.instrumented_contract.clone());
    Ok(AstConfig {
      solc,
      instrumented_contract,
    })
  }

  pub fn merge_options(&self, overrides: Option<&AstConfigOptions>) -> Result<Self> {
    match overrides {
      Some(overrides) => self.merged(overrides),
      None => Ok(self.clone()),
    }
  }

  pub fn instrumented_contract(&self) -> Option<&str> {
    self.instrumented_contract.as_deref()
  }
}

impl TryFrom<&CompilerConfigOptions> for CompilerConfigOptions {
  type Error = napi::Error;

  fn try_from(value: &CompilerConfigOptions) -> Result<Self> {
    Ok(value.clone())
  }
}

impl TryFrom<&JsCompilerConfigOptions> for CompilerConfigOptions {
  type Error = napi::Error;

  fn try_from(options: &JsCompilerConfigOptions) -> Result<Self> {
    let mut overrides = CompilerConfigOptions::default();

    if let Some(version) = options.solc_version.as_ref() {
      overrides.solc.version = Some(parse_version(version)?);
    }

    overrides.solc.language = options.solc_language.map(Into::into);

    if let Some(settings) = options.solc_settings.as_ref() {
      overrides.solc.settings = Some(CompilerSettingsOptions::try_from(settings)?);
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
    overrides.allow_paths = options
      .allow_paths
      .as_ref()
      .map(|paths| to_path_set(paths.as_slice()));
    overrides.include_paths = options
      .include_paths
      .as_ref()
      .map(|paths| to_path_set(paths.as_slice()));
    overrides.library_paths = options
      .library_paths
      .as_ref()
      .map(|paths| to_path_vec(paths.as_slice()));
    overrides.remappings = map_remappings(options.remappings.as_ref())?;
    overrides.ignored_file_paths = options
      .ignored_paths
      .as_ref()
      .map(|paths| to_path_set(paths.as_slice()));
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

impl TryFrom<JsCompilerConfigOptions> for CompilerConfigOptions {
  type Error = napi::Error;

  fn try_from(options: JsCompilerConfigOptions) -> Result<Self> {
    CompilerConfigOptions::try_from(&options)
  }
}

impl TryFrom<&AstConfigOptions> for AstConfigOptions {
  type Error = napi::Error;

  fn try_from(value: &AstConfigOptions) -> Result<Self> {
    Ok(value.clone())
  }
}

impl TryFrom<&JsAstConfigOptions> for AstConfigOptions {
  type Error = napi::Error;

  fn try_from(options: &JsAstConfigOptions) -> Result<Self> {
    let mut typed = AstConfigOptions::default();

    if let Some(version) = options.solc_version.as_ref() {
      typed.solc.version = Some(parse_version(version)?);
    }

    typed.solc.language = options.solc_language.map(FoundrySolcLanguage::from);
    if let Some(settings) = options.solc_settings.as_ref() {
      typed.solc.settings = Some(CompilerSettingsOptions::try_from(settings)?);
    }
    typed.instrumented_contract = options.instrumented_contract.clone();

    Ok(typed)
  }
}

impl TryFrom<JsAstConfigOptions> for AstConfigOptions {
  type Error = napi::Error;

  fn try_from(options: JsAstConfigOptions) -> Result<Self> {
    AstConfigOptions::try_from(&options)
  }
}

/// JavaScript-facing configuration captured through N-API bindings.
#[napi(object, js_name = "CompilerConfigOptions")]
#[derive(Clone, Default)]
pub struct JsCompilerConfigOptions {
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  pub solc_language: Option<SolcLanguage>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub solc_settings: Option<JsCompilerSettingsOptions>,
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

#[napi(object, js_name = "AstConfigOptions")]
#[derive(Clone, Default)]
pub struct JsAstConfigOptions {
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  pub solc_language: Option<SolcLanguage>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub solc_settings: Option<JsCompilerSettingsOptions>,
  #[napi(ts_type = "string | undefined")]
  pub instrumented_contract: Option<String>,
}

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
  fn solc_version(&self) -> Option<&Version>;
  fn solc_language(&self) -> Option<FoundrySolcLanguage>;
  fn compiler_settings(&self) -> Option<&CompilerSettingsOptions>;
  fn resolved_settings(&self) -> Option<&Settings>;
}

impl SolcUserOptions for SolcConfigOptions {
  fn solc_version(&self) -> Option<&Version> {
    self.version.as_ref()
  }

  fn solc_language(&self) -> Option<FoundrySolcLanguage> {
    self.language
  }

  fn compiler_settings(&self) -> Option<&CompilerSettingsOptions> {
    self.settings.as_ref()
  }

  fn resolved_settings(&self) -> Option<&Settings> {
    self.resolved_settings.as_ref()
  }
}

impl SolcUserOptions for CompilerConfigOptions {
  fn solc_version(&self) -> Option<&Version> {
    self.solc.version.as_ref()
  }

  fn solc_language(&self) -> Option<FoundrySolcLanguage> {
    self.solc.language
  }

  fn compiler_settings(&self) -> Option<&CompilerSettingsOptions> {
    self.solc.settings.as_ref()
  }

  fn resolved_settings(&self) -> Option<&Settings> {
    self.solc.resolved_settings.as_ref()
  }
}

impl SolcUserOptions for AstConfigOptions {
  fn solc_version(&self) -> Option<&Version> {
    self.solc.version.as_ref()
  }

  fn solc_language(&self) -> Option<FoundrySolcLanguage> {
    self.solc.language
  }

  fn compiler_settings(&self) -> Option<&CompilerSettingsOptions> {
    self.solc.settings.as_ref()
  }

  fn resolved_settings(&self) -> Option<&Settings> {
    self.solc.resolved_settings.as_ref()
  }
}

#[derive(Clone, Debug)]
pub struct SolcConfig {
  pub version: Version,
  pub settings: Settings,
  pub language: FoundrySolcLanguage,
}

impl SolcConfig {
  pub(crate) fn new<O: SolcUserOptions>(
    default_language: &FoundrySolcLanguage,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let default_version = crate::internal::solc::default_version().map_err(napi::Error::from)?;
    Self::with_defaults(
      default_language,
      &default_version,
      default_settings,
      overrides,
    )
  }

  pub(crate) fn with_defaults<O: SolcUserOptions>(
    default_language: &FoundrySolcLanguage,
    default_version: &Version,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .cloned()
      .unwrap_or_else(|| default_version.clone());

    let language = overrides
      .and_then(|opts| opts.solc_language())
      .unwrap_or_else(|| default_language.clone());

    let settings = if let Some(resolved) = overrides.and_then(|opts| opts.resolved_settings()) {
      sanitize_settings(resolved)?
    } else {
      merge_settings(
        default_settings,
        overrides.and_then(|opts| opts.compiler_settings()),
      )?
    };

    Ok(SolcConfig {
      version,
      settings,
      language,
    })
  }

  pub(crate) fn merge<O: SolcUserOptions>(&self, overrides: Option<&O>) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .cloned()
      .unwrap_or_else(|| self.version.clone());

    let language = overrides
      .and_then(|opts| opts.solc_language())
      .unwrap_or_else(|| self.language.clone());

    let settings = if let Some(resolved) = overrides.and_then(|opts| opts.resolved_settings()) {
      sanitize_settings(resolved)?
    } else {
      merge_settings(
        &self.settings,
        overrides.and_then(|opts| opts.compiler_settings()),
      )?
    };

    Ok(SolcConfig {
      version,
      settings,
      language,
    })
  }
}

pub(crate) fn parse_js_compiler_config(
  env: &Env,
  value: Option<JsUnknown>,
) -> Result<Option<JsCompilerConfigOptions>> {
  parse_options(value)?
    .map(|unknown| unsafe { JsCompilerConfigOptions::from_napi_value(env.raw(), unknown.raw()) })
    .transpose()
}

pub(crate) fn parse_js_ast_options(
  env: &Env,
  value: Option<JsUnknown>,
) -> Result<Option<JsAstConfigOptions>> {
  match parse_options(value)? {
    Some(unknown) => {
      let object = unsafe { JsObject::from_napi_value(env.raw(), unknown.raw()) }?;
      validate_object_field(&object, "settings")?;
      unsafe { JsAstConfigOptions::from_napi_value(env.raw(), unknown.raw()) }.map(Some)
    }
    None => Ok(None),
  }
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
      Ok(Some(object.into_unknown()))
    }
    _ => Err(napi_error(
      "Compiler options must be an object or undefined/null.",
    )),
  }
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

#[derive(Default)]
pub(crate) struct CompilerConfigBuilder {
  config: CompilerConfig,
}

impl CompilerConfigBuilder {
  pub fn from_defaults() -> Self {
    Self {
      config: CompilerConfig::default(),
    }
  }

  pub fn with_base(base: CompilerConfig) -> Self {
    Self { config: base }
  }

  pub fn apply_compiler_options(mut self, overrides: CompilerConfigOptions) -> Result<Self> {
    let CompilerConfigOptions {
      mut solc,
      cache_enabled,
      base_dir,
      offline_mode,
      no_artifacts,
      build_info_enabled,
      slash_paths,
      solc_jobs,
      sparse_output,
      allow_paths,
      include_paths,
      library_paths,
      remappings,
      ignored_file_paths,
      ignored_error_codes,
      compiler_severity_filter,
    } = overrides;

    if let Some(version) = solc.version.take() {
      self.config.solc_version = version;
    }
    if let Some(language) = solc.language.take() {
      self.config.solc_language = language;
    }
    if let Some(settings) = solc.resolved_settings.take() {
      self.config.solc_settings = sanitize_settings(&settings)?;
    } else if let Some(settings) = solc.settings.take() {
      self.config.solc_settings = merge_settings(&self.config.solc_settings, Some(&settings))?;
    }
    if let Some(cache) = cache_enabled {
      self.config.cache_enabled = cache;
    }
    if let Some(base_dir) = base_dir {
      self.config.base_dir = Some(base_dir);
    }
    if let Some(offline) = offline_mode {
      self.config.offline_mode = offline;
    }
    if let Some(no_artifacts) = no_artifacts {
      self.config.no_artifacts = no_artifacts;
    }
    if let Some(build_info) = build_info_enabled {
      self.config.build_info_enabled = build_info;
    }
    if let Some(slash_paths) = slash_paths {
      self.config.slash_paths = slash_paths;
    }
    if let Some(solc_jobs) = solc_jobs {
      self.config.solc_jobs = solc_jobs;
    }
    if let Some(sparse_output) = sparse_output {
      self.config.sparse_output = sparse_output;
    }
    if let Some(allow_paths) = allow_paths {
      self.config.allow_paths = allow_paths;
    }
    if let Some(include_paths) = include_paths {
      self.config.include_paths = include_paths;
    }
    if let Some(libraries) = library_paths {
      self.config.library_paths = libraries;
    }
    if let Some(remappings) = remappings {
      self.config.remappings = remappings;
    }
    if let Some(ignored_paths) = ignored_file_paths {
      self.config.ignored_file_paths = ignored_paths;
    }
    if let Some(ignored_codes) = ignored_error_codes {
      self.config.ignored_error_codes = ignored_codes;
    }
    if let Some(severity) = compiler_severity_filter {
      self.config.compiler_severity_filter = severity;
    }

    Ok(self)
  }

  pub fn build(mut self) -> Result<CompilerConfig> {
    self.config.solc_settings = sanitize_settings(&self.config.solc_settings)?;
    Ok(self.config)
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use super::*;

  #[test]
  fn empty_output_selection_is_sanitized() {
    let base = Settings::default();
    let mut overrides = CompilerSettingsOptions::default();
    overrides.output_selection = Some(BTreeMap::from([(
      "*".to_string(),
      BTreeMap::from([("*".to_string(), Vec::new()), (String::new(), Vec::new())]),
    )]));

    let merged = merge_settings(&base, Some(&overrides)).expect("settings");
    assert!(
      !crate::internal::settings::output_selection_is_effectively_empty(&merged.output_selection),
      "merged selection should fallback to defaults"
    );
  }

  #[test]
  fn builder_defaults_without_options() {
    let baseline = CompilerConfig::default();
    let built = CompilerConfigBuilder::from_defaults()
      .build()
      .expect("builder without options");
    assert_eq!(built.solc_version, baseline.solc_version);
    assert_eq!(built.solc_language, baseline.solc_language);
  }

  #[test]
  fn invalid_severity_string_is_rejected() {
    let mut options = JsCompilerConfigOptions::default();
    options.compiler_severity = Some("not-a-level".to_string());
    let error = CompilerConfigOptions::try_from(&options).expect_err("should fail");
    assert!(error
      .to_string()
      .contains("Unsupported compiler severity filter"));
  }
}
