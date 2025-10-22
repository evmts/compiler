use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::str::FromStr;

use foundry_compilers::artifacts::vyper::{VyperOptimizationMode, VyperSettings};
use foundry_compilers::artifacts::{
  error::Severity, output_selection::OutputSelection, remappings::Remapping, Settings,
};
use foundry_compilers::solc::SolcLanguage as FoundrySolcLanguage;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown, NapiRaw, ValueType};
use semver::Version;

use crate::internal::errors::{map_napi_error, napi_error};
use crate::internal::logging::LoggingLevel;
use crate::internal::path::{to_path_set, to_path_vec};
use crate::internal::settings::{
  merge_settings, sanitize_settings, CompilerSettingsOptions, JsCompilerSettingsOptions,
  VyperSettingsOptions,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilerLanguage {
  Solidity,
  Yul,
  Vyper,
}

impl From<FoundrySolcLanguage> for CompilerLanguage {
  fn from(language: FoundrySolcLanguage) -> Self {
    match language {
      FoundrySolcLanguage::Solidity => CompilerLanguage::Solidity,
      FoundrySolcLanguage::Yul => CompilerLanguage::Yul,
      _ => CompilerLanguage::Solidity,
    }
  }
}

impl From<CompilerLanguage> for FoundrySolcLanguage {
  fn from(language: CompilerLanguage) -> Self {
    match language {
      CompilerLanguage::Solidity => FoundrySolcLanguage::Solidity,
      CompilerLanguage::Yul => FoundrySolcLanguage::Yul,
      CompilerLanguage::Vyper => {
        panic!("CompilerLanguage::Vyper cannot be converted to Solc language")
      }
    }
  }
}

impl CompilerLanguage {
  pub fn is_solc_language(&self) -> bool {
    matches!(self, CompilerLanguage::Solidity | CompilerLanguage::Yul)
  }

  pub fn to_solc_language(self) -> Option<FoundrySolcLanguage> {
    match self {
      CompilerLanguage::Solidity => Some(FoundrySolcLanguage::Solidity),
      CompilerLanguage::Yul => Some(FoundrySolcLanguage::Yul),
      CompilerLanguage::Vyper => None,
    }
  }
}

fn solc_language_from(language: CompilerLanguage) -> Result<FoundrySolcLanguage> {
  match language {
    CompilerLanguage::Solidity => Ok(FoundrySolcLanguage::Solidity),
    CompilerLanguage::Yul => Ok(FoundrySolcLanguage::Yul),
    CompilerLanguage::Vyper => Err(napi_error(
      "Vyper compiler language is not supported by the Solc toolchain.",
    )),
  }
}

/// Fully resolved Vyper configuration mirrored from user options and the library defaults.
#[derive(Clone, Debug)]
pub struct VyperCompilerSettings {
  /// Absolute path to the Vyper executable. When `None`, we fall back to resolving `vyper`
  /// from the current `PATH` (see `vyper::default_path`).
  pub path: Option<PathBuf>,
  /// Optimisation mode forwarded to the Vyper CLI. The default mirrors Vyper's chosen strategy
  /// (typically `gas`).
  pub optimize: Option<VyperOptimizationMode>,
  /// Target EVM version used when generating bytecode. Inherits Vyper's default target when
  /// unspecified.
  pub evm_version: Option<crate::internal::settings::EvmVersion>,
  /// Controls whether bytecode metadata (hashes, compiler info) is embedded in the output.
  /// Leaving this unset honours Vyper's defaults.
  pub bytecode_metadata: Option<bool>,
  /// Additional directories searched when resolving Vyper `import` statements. These are merged
  /// with the project's library/include paths.
  pub search_paths: Option<Vec<PathBuf>>,
  /// Fine-grained output selection that complements the Foundry defaults. When omitted we use
  /// `OutputSelection::default_output_selection()` which exposes the common ABI/code/metadata bundle.
  pub output_selection: Option<OutputSelection>,
  /// Enables Vyper's experimental code generation backend. We keep it disabled unless explicitly
  /// requested because upstream still treats it as unstable.
  pub experimental_codegen: Option<bool>,
}

impl VyperCompilerSettings {
  pub fn to_settings_options(&self) -> VyperSettingsOptions {
    VyperSettingsOptions {
      optimize: self.optimize,
      evm_version: self.evm_version.clone(),
      bytecode_metadata: self.bytecode_metadata,
      output_selection: self.output_selection.clone().map(|selection| selection.0),
      search_paths: self.search_paths.clone(),
      experimental_codegen: self.experimental_codegen,
    }
  }

  pub fn to_vyper_settings(
    &self,
    search_paths: Option<Vec<PathBuf>>,
  ) -> napi::Result<VyperSettings> {
    let mut options = self.to_settings_options();
    if let Some(paths) = search_paths {
      options.search_paths = Some(paths);
    }
    options.overlay(&VyperSettings::default())
  }
}

impl Default for VyperCompilerSettings {
  fn default() -> Self {
    Self {
      path: None,
      optimize: None,
      evm_version: None,
      bytecode_metadata: None,
      search_paths: None,
      output_selection: Some(OutputSelection::default_output_selection()),
      experimental_codegen: None,
    }
  }
}

/// Finalised compiler configuration consumed by the Rust compiler façade and forwarded to Foundry's
/// multi-compiler. Every field here is canonicalised and ready to be reused across calls.
#[derive(Clone, Debug)]
pub struct CompilerConfig {
  /// Active language front-end. Defaults to `CompilerLanguage::Solidity` unless overridden via
  /// options or project metadata.
  pub language: CompilerLanguage,
  /// Semver-qualified solc release resolved after applying user overrides. Defaults to `0.8.30`.
  pub solc_version: Version,
  /// Sanitised solc `Settings` struct emitted to the underlying compiler.
  pub solc_settings: Settings,
  /// Resolved Vyper settings used when `language == CompilerLanguage::Vyper`.
  pub vyper_settings: VyperCompilerSettings,
  /// Controls whether synthetic projects cache inline sources on disk (`~/.tevm/virtual-sources`).
  pub cache_enabled: bool,
  /// Forces offline compilation when `true`. When `false`, Foundry may fetch remappings on demand.
  pub offline_mode: bool,
  /// Skips emitting artifact files entirely when `true`.
  pub no_artifacts: bool,
  /// Emits Foundry build-info JSON files alongside the compiled artifacts when `true`.
  pub build_info_enabled: bool,
  /// Normalises emitted paths to forward slashes so results remain cross-platform stable.
  pub slash_paths: bool,
  /// Explicit solc job count override. `None` signals that Foundry should choose automatically.
  pub solc_jobs: Option<usize>,
  /// Emits a reduced artifact payload when `true`, mirroring Foundry's `sparse` output mode.
  pub sparse_output: bool,
  /// Canonicalised paths forwarded to solc's `--allow-paths` flag.
  pub allow_paths: BTreeSet<PathBuf>,
  /// Canonicalised directories appended to the compiler's include path.
  pub include_paths: BTreeSet<PathBuf>,
  /// Additional library directories searched when resolving imports.
  pub library_paths: Vec<PathBuf>,
  /// Resolved Solidity remappings applied prior to compilation (`prefix=path`).
  pub remappings: Vec<Remapping>,
  /// Canonicalised files or directories the compiler must ignore.
  pub ignored_file_paths: BTreeSet<PathBuf>,
  /// Numeric compiler error codes to suppress.
  pub ignored_error_codes: Vec<u64>,
  /// Lowest diagnostic severity surfaced to consumers. Defaults to `Severity::Error` (errors only).
  pub compiler_severity_filter: Severity,
  /// Global logging level applied to compiler operations.
  pub logging_level: LoggingLevel,
}

impl Default for CompilerConfig {
  fn default() -> Self {
    CompilerConfig {
      language: CompilerLanguage::Solidity,
      solc_version: crate::internal::solc::default_version()
        .unwrap_or_else(|_| Version::new(0, 8, 30)),
      solc_settings: Settings::default(),
      vyper_settings: VyperCompilerSettings::default(),
      cache_enabled: true,
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
      logging_level: LoggingLevel::default(),
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
  /// Specific solc version to use instead of the default.
  pub version: Option<Version>,
  /// Front-end language to run (Solidity or Yul). Defaults to the caller's context.
  pub language: Option<FoundrySolcLanguage>,
  /// Partial solc settings that will be merged with Foundry defaults.
  pub settings: Option<CompilerSettingsOptions>,
  /// Pre-resolved solc settings that replace the defaults entirely when provided.
  pub resolved_settings: Option<Settings>,
}

/// Vyper-specific overrides captured from user input.
#[derive(Clone, Debug, Default)]
pub struct VyperConfigOptions {
  /// Optional path to a Vyper binary. Defaults to discovering `vyper` on `PATH`.
  pub path: Option<PathBuf>,
  /// Requested Vyper optimisation mode.
  pub optimize: Option<VyperOptimizationMode>,
  /// Requested EVM version for Vyper output.
  pub evm_version: Option<crate::internal::settings::EvmVersion>,
  /// Controls whether Vyper embeds bytecode metadata.
  pub bytecode_metadata: Option<bool>,
  /// Additional directories searched for Vyper imports.
  pub search_paths: Option<Vec<PathBuf>>,
  /// Fine-grained Vyper output selection overrides.
  pub output_selection: Option<OutputSelection>,
  /// Enables Vyper's experimental code generation backend.
  pub experimental_codegen: Option<bool>,
}

/// Strongly-typed Rust overrides that can be merged into a [`CompilerConfig`].
#[derive(Clone, Debug, Default)]
pub struct CompilerConfigOptions {
  /// Desired compiler language. When `None`, the current compiler configuration (or project
  /// metadata) decides whether we compile Solidity, Yul, or Vyper.
  pub compiler: Option<CompilerLanguage>,
  /// Solc-specific overrides such as version, optimizer configuration, and output selection.
  pub solc: SolcConfigOptions,
  /// Vyper-specific overrides applied whenever the active language front-end is `Vyper`.
  pub vyper: VyperConfigOptions,
  /// Overrides the cache flag. Set to `false` to avoid writing virtual sources to
  /// `~/.tevm/virtual-sources` during inline compilations.
  pub cache_enabled: Option<bool>,
  /// Forces offline compilation. Useful when you need deterministic builds without network access.
  pub offline_mode: Option<bool>,
  /// Disables artifact emission when `Some(true)` to keep compilation side-effect free.
  pub no_artifacts: Option<bool>,
  /// Enables build-info generation when `Some(true)`, mirroring Foundry's `--build-info` flag.
  pub build_info_enabled: Option<bool>,
  /// Forces slashed path output when provided. `Some(false)` preserves platform-specific path
  /// separators in emitted artifacts.
  pub slash_paths: Option<bool>,
  /// Explicit solc job count override. Use `Some(Some(n))` to pin the concurrency level or
  /// `Some(None)` to reset back to auto-detection.
  pub solc_jobs: Option<Option<usize>>,
  /// Requests sparse artifact output (lighter JSON artifacts) when set to `true`.
  pub sparse_output: Option<bool>,
  /// Additional paths forwarded to solc's `--allow-paths`. Entries are canonicalised before use.
  pub allow_paths: Option<BTreeSet<PathBuf>>,
  /// Additional include directories merged with the existing configuration.
  pub include_paths: Option<BTreeSet<PathBuf>>,
  /// Library directories appended to the search path that Foundry hands to solc.
  pub library_paths: Option<Vec<PathBuf>>,
  /// Remappings merged into the configuration. Use the `prefix=path` format expected by solc.
  pub remappings: Option<Vec<Remapping>>,
  /// Files or directories added to the ignore list; each entry is canonicalised.
  pub ignored_file_paths: Option<BTreeSet<PathBuf>>,
  /// Extra compiler error codes ignored when present (e.g. `codegen::1264`).
  pub ignored_error_codes: Option<Vec<u64>>,
  /// Overrides the severity filter applied to compiler diagnostics. Accepts `Severity::Error`,
  /// `Severity::Warning`, or `Severity::Info`.
  pub compiler_severity_filter: Option<Severity>,
  /// Overrides the compiler logging level. Defaults to [`LoggingLevel::Info`].
  pub logging_level: Option<LoggingLevel>,
}

/// Overrides for the AST helper configuration.
#[derive(Clone, Debug, Default)]
pub struct AstConfigOptions {
  /// Solc options applied during AST parsing and validation.
  pub solc: SolcConfigOptions,
  /// Contract name targeted by helper operations. Applies to all contracts when `None`.
  pub instrumented_contract: Option<String>,
  /// Overrides the logging level applied to AST operations.
  pub logging_level: Option<LoggingLevel>,
  /// Controls how AST stitching resolves contract member conflicts.
  pub resolve_conflict_strategy: Option<ResolveConflictStrategy>,
}

impl AstConfigOptions {
  pub fn instrumented_contract(&self) -> Option<&str> {
    self.instrumented_contract.as_deref()
  }
}

/// Resolved configuration driving AST operations.
#[derive(Clone, Debug)]
pub struct AstConfig {
  /// Sanitised solc configuration applied to AST compilation.
  pub solc: SolcConfig,
  /// Contract name targeted by helper operations, when provided.
  pub instrumented_contract: Option<String>,
  /// Logging level used for AST workflows.
  pub logging_level: LoggingLevel,
  /// Conflict resolution strategy applied when stitching fragments.
  pub resolve_conflict_strategy: ResolveConflictStrategy,
}

impl AstConfig {
  pub fn from_options(
    default_language: &FoundrySolcLanguage,
    default_settings: &Settings,
    options: Option<&AstConfigOptions>,
  ) -> Result<Self> {
    let solc = SolcConfig::new(
      CompilerLanguage::from(*default_language),
      default_settings,
      options,
    )?;
    let logging_level = options
      .and_then(|opts| opts.logging_level)
      .unwrap_or_default();
    let resolve_conflict_strategy = options
      .and_then(|opts| opts.resolve_conflict_strategy)
      .unwrap_or_default();
    Ok(AstConfig {
      solc,
      instrumented_contract: options.and_then(|opts| opts.instrumented_contract.clone()),
      logging_level,
      resolve_conflict_strategy,
    })
  }

  pub fn merged(&self, overrides: &AstConfigOptions) -> Result<Self> {
    let solc = self.solc.merge(Some(overrides))?;
    let instrumented_contract = overrides
      .instrumented_contract
      .clone()
      .or_else(|| self.instrumented_contract.clone());
    let logging_level = overrides.logging_level.unwrap_or(self.logging_level);
    let resolve_conflict_strategy = overrides
      .resolve_conflict_strategy
      .unwrap_or(self.resolve_conflict_strategy);
    Ok(AstConfig {
      solc,
      instrumented_contract,
      logging_level,
      resolve_conflict_strategy,
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

    if let Some(language) = options.language {
      overrides.compiler = Some(language.into());
    }

    if let Some(settings) = options.solc_settings.as_ref() {
      overrides.solc.settings = Some(CompilerSettingsOptions::try_from(settings)?);
    }

    overrides.cache_enabled = options.cache_enabled;
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

    if let Some(vyper) = options.vyper_settings.as_ref() {
      overrides.vyper = VyperConfigOptions::try_from(vyper)?;
    }

    if let Some(level) = options.logging_level {
      overrides.logging_level = Some(level.into());
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

impl TryFrom<&JsVyperCompilerConfig> for VyperConfigOptions {
  type Error = napi::Error;

  fn try_from(options: &JsVyperCompilerConfig) -> Result<Self> {
    let mut typed = VyperConfigOptions::default();

    if let Some(path) = options.path.as_ref() {
      typed.path = Some(PathBuf::from(path));
    }
    if let Some(mode) = options.optimize {
      typed.optimize = Some(mode.into());
    }
    typed.evm_version = options.evm_version.clone();
    typed.bytecode_metadata = options.bytecode_metadata;
    if let Some(paths) = options.search_paths.as_ref() {
      typed.search_paths = Some(to_path_vec(paths.as_slice()));
    }
    if let Some(selection) = options.output_selection.as_ref() {
      let value = map_napi_error(
        serde_json::to_value(selection),
        "Failed to serialise Vyper output selection",
      )?;
      typed.output_selection = Some(map_napi_error(
        serde_json::from_value(value),
        "Failed to parse Vyper output selection",
      )?);
    }
    typed.experimental_codegen = options.experimental_codegen;

    Ok(typed)
  }
}

impl TryFrom<JsVyperCompilerConfig> for VyperConfigOptions {
  type Error = napi::Error;

  fn try_from(options: JsVyperCompilerConfig) -> Result<Self> {
    VyperConfigOptions::try_from(&options)
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
    typed.logging_level = options.logging_level.map(Into::into);
    typed.resolve_conflict_strategy = options
      .resolve_conflict_strategy
      .map(ResolveConflictStrategy::from);

    Ok(typed)
  }
}

impl TryFrom<JsAstConfigOptions> for AstConfigOptions {
  type Error = napi::Error;

  fn try_from(options: JsAstConfigOptions) -> Result<Self> {
    AstConfigOptions::try_from(&options)
  }
}

/// Compiler configuration merged on top of the defaults for each call.
#[napi(object, js_name = "CompilerConfigOptions")]
#[derive(Clone, Default)]
pub struct JsCompilerConfigOptions {
  /// Semantic version of `solc` to run (e.g. `"0.8.30"`). Defaults to the bundled `0.8.30`
  /// release when omitted.
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  /// Override the compiler front-end (`Solidity`, `Yul`, or `Vyper`). Falls back to
  /// `CompilerLanguage::Solidity` unless project metadata specifies otherwise.
  #[napi(ts_type = "CompilerLanguage | undefined")]
  pub language: Option<JsCompilerLanguage>,
  /// Partial `solc` settings merged on top of Foundry's defaults. Useful for tweaking optimiser
  /// runs, metadata output, or per-path remappings without rebuilding the Rust crate.
  #[napi(ts_type = "CompilerSettings | undefined")]
  pub solc_settings: Option<JsCompilerSettingsOptions>,
  /// Nested Vyper-specific configuration. Falls back to environment defaults when omitted.
  #[napi(ts_type = "VyperCompilerConfig | undefined")]
  pub vyper_settings: Option<JsVyperCompilerConfig>,
  /// Enables the synthetic workspace cache used for inline sources. When `true` (default) we cache
  /// sources under `~/.tevm/virtual-sources`; `false` keeps everything in-memory for ephemeral runs.
  #[napi(ts_type = "boolean | undefined")]
  pub cache_enabled: Option<bool>,
  /// Prevents network access during compilation. Defaults to `false` so Foundry can download
  /// missing remappings when necessary.
  #[napi(ts_type = "boolean | undefined")]
  pub offline_mode: Option<bool>,
  /// Skips writing artifacts to disk when `true`. Defaults to `false`.
  #[napi(ts_type = "boolean | undefined")]
  pub no_artifacts: Option<bool>,
  /// Emits Foundry build-info files when `true`. Defaults to `false`.
  #[napi(ts_type = "boolean | undefined")]
  pub build_info_enabled: Option<bool>,
  /// Normalises emitted paths to use forward slashes. Defaults to `true` for cross-platform
  /// stability.
  #[napi(ts_type = "boolean | undefined")]
  pub slash_paths: Option<bool>,
  /// Explicit solc job count. Defaults to letting Foundry auto-detect; set this when you want a
  /// deterministic concurrency level inside CI.
  #[napi(ts_type = "number | undefined")]
  pub solc_jobs: Option<u32>,
  /// Emits minimal artifact output when `true` (Foundry's sparse output mode). Defaults to `false`.
  #[napi(ts_type = "boolean | undefined")]
  pub sparse_output: Option<bool>,
  /// Additional filesystem roots forwarded to solc's `--allow-paths`. Provide absolute paths or
  /// paths relative to the invoking process. Defaults to an empty list.
  #[napi(ts_type = "string[] | undefined")]
  pub allow_paths: Option<Vec<String>>,
  /// Extra include directories resolved before compilation. Entries are canonicalised relative to
  /// the compiler's working directory before use.
  #[napi(ts_type = "string[] | undefined")]
  pub include_paths: Option<Vec<String>>,
  /// Library directories appended to the search path. Particularly useful when mirroring Hardhat's
  /// `libraries` behaviour.
  #[napi(ts_type = "string[] | undefined")]
  pub library_paths: Option<Vec<String>>,
  /// Solidity remappings applied to the compilation (`prefix=path` format). Empty by default.
  #[napi(ts_type = "string[] | undefined")]
  pub remappings: Option<Vec<String>>,
  /// Error codes that should be ignored (e.g. `1878`). Defaults to an empty list.
  #[napi(ts_type = "number[] | undefined")]
  pub ignored_error_codes: Option<Vec<i64>>,
  /// Files or directories excluded from compilation. Paths are canonicalised before being
  /// forwarded to Foundry.
  #[napi(ts_type = "string[] | undefined")]
  pub ignored_paths: Option<Vec<String>>,
  /// Lowest severity level to surface (`"error"`, `"warning"`, or `"info"`). Defaults to
  /// `"Error"` which hides warnings.
  #[napi(ts_type = "string | undefined")]
  pub compiler_severity: Option<String>,
  /// Controls the logging verbosity (`"silent"`, `"error"`, `"warn"`, `"info"`). Defaults to
  /// `"info"`.
  #[napi(ts_type = "LoggingLevel | undefined")]
  pub logging_level: Option<JsLoggingLevel>,
}

/// Selects which frontend pipeline the compiler should use.
#[napi(string_enum, js_name = "CompilerLanguage")]
#[derive(Debug, Eq, PartialEq)]
pub enum JsCompilerLanguage {
  Solidity,
  Yul,
  Vyper,
}

impl From<JsCompilerLanguage> for CompilerLanguage {
  fn from(language: JsCompilerLanguage) -> Self {
    match language {
      JsCompilerLanguage::Solidity => CompilerLanguage::Solidity,
      JsCompilerLanguage::Yul => CompilerLanguage::Yul,
      JsCompilerLanguage::Vyper => CompilerLanguage::Vyper,
    }
  }
}

/// Logging levels surfaced to JavaScript callers.
#[napi(string_enum, js_name = "LoggingLevel")]
#[derive(Debug, Eq, PartialEq)]
pub enum JsLoggingLevel {
  Silent,
  Error,
  Warn,
  Info,
}

impl From<JsLoggingLevel> for LoggingLevel {
  fn from(level: JsLoggingLevel) -> Self {
    match level {
      JsLoggingLevel::Silent => LoggingLevel::Silent,
      JsLoggingLevel::Error => LoggingLevel::Error,
      JsLoggingLevel::Warn => LoggingLevel::Warn,
      JsLoggingLevel::Info => LoggingLevel::Info,
    }
  }
}

/// Optimisation goals exposed by the Vyper compiler.
#[napi(string_enum, js_name = "VyperOptimizationMode")]
#[derive(Debug, Eq, PartialEq)]
pub enum JsVyperOptimizationMode {
  Gas,
  Codesize,
  None,
}

impl From<JsVyperOptimizationMode> for VyperOptimizationMode {
  fn from(mode: JsVyperOptimizationMode) -> Self {
    match mode {
      JsVyperOptimizationMode::Gas => VyperOptimizationMode::Gas,
      JsVyperOptimizationMode::Codesize => VyperOptimizationMode::Codesize,
      JsVyperOptimizationMode::None => VyperOptimizationMode::None,
    }
  }
}

/// Vyper-specific configuration surfaced alongside the general compiler settings.
#[napi(object, js_name = "VyperCompilerConfig")]
#[derive(Clone, Default)]
pub struct JsVyperCompilerConfig {
  /// Absolute path to a `vyper` executable. Defaults to resolving `vyper` from the `PATH`.
  #[napi(ts_type = "string | undefined")]
  pub path: Option<String>,
  /// Optimisation strategy forwarded to the Vyper compiler. Uses Vyper defaults when unset.
  #[napi(ts_type = "VyperOptimizationMode | undefined")]
  pub optimize: Option<JsVyperOptimizationMode>,
  /// Target EVM version. Defaults to Vyper's bundled target when omitted.
  #[napi(ts_type = "EvmVersion | undefined")]
  pub evm_version: Option<crate::internal::settings::EvmVersion>,
  /// Whether to embed bytecode metadata. Falls back to Vyper defaults when unspecified.
  #[napi(ts_type = "boolean | undefined")]
  pub bytecode_metadata: Option<bool>,
  /// Additional import search paths for Vyper. Relative entries are resolved from the project root.
  #[napi(ts_type = "string[] | undefined")]
  pub search_paths: Option<Vec<String>>,
  /// Fine-grained Vyper output selection map. Defaults to Foundry's aggregate selection (`abi`,
  /// `evm.bytecode`, `metadata`).
  #[napi(ts_type = "import('./solc-settings').OutputSelection | undefined")]
  pub output_selection: Option<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
  /// Enables experimental Vyper codegen features. Disabled by default.
  #[napi(ts_type = "boolean | undefined")]
  pub experimental_codegen: Option<bool>,
}

/// AST helper configuration allowing overrides for the solc version and target contract.
#[napi(object, js_name = "AstConfigOptions")]
#[derive(Clone, Default)]
pub struct JsAstConfigOptions {
  /// Semantic version of `solc` used for AST parsing. Defaults to `0.8.30`.
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  /// Solc language mode. Only `Solidity` is supported and used by default.
  pub solc_language: Option<SolcLanguage>,
  /// Partial solc settings merged with the AST orchestrator defaults.
  #[napi(ts_type = "CompilerSettings | undefined")]
  pub solc_settings: Option<JsCompilerSettingsOptions>,
  /// Contract name to target when mutating the AST. Applies to every contract when omitted.
  #[napi(ts_type = "string | undefined")]
  pub instrumented_contract: Option<String>,
  /// Logging verbosity applied while manipulating the AST.
  #[napi(ts_type = "LoggingLevel | undefined")]
  pub logging_level: Option<JsLoggingLevel>,
  /// Conflict resolution strategy applied while stitching fragments.
  /// Default strategy is `ResolveConflictStrategy::Safe`, which will fail to compile if
  /// conflicting members are found (usually conflicting name). `ResolveConflictStrategy::Replace`
  /// will overwrite the existing members when conflicting.
  #[napi(ts_type = "ResolveConflictStrategy | undefined")]
  pub resolve_conflict_strategy: Option<JsResolveConflictStrategy>,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResolveConflictStrategy {
  #[default]
  Safe,
  Replace,
}

#[napi(string_enum, js_name = "ResolveConflictStrategy")]
#[derive(Debug, Eq, PartialEq)]
pub enum JsResolveConflictStrategy {
  Safe,
  Replace,
}

impl From<JsResolveConflictStrategy> for ResolveConflictStrategy {
  fn from(strategy: JsResolveConflictStrategy) -> Self {
    match strategy {
      JsResolveConflictStrategy::Safe => ResolveConflictStrategy::Safe,
      JsResolveConflictStrategy::Replace => ResolveConflictStrategy::Replace,
    }
  }
}

impl From<ResolveConflictStrategy> for JsResolveConflictStrategy {
  fn from(strategy: ResolveConflictStrategy) -> Self {
    match strategy {
      ResolveConflictStrategy::Safe => JsResolveConflictStrategy::Safe,
      ResolveConflictStrategy::Replace => JsResolveConflictStrategy::Replace,
    }
  }
}

pub(crate) trait SolcUserOptions {
  fn solc_version(&self) -> Option<&Version>;
  fn compiler_language(&self) -> Option<CompilerLanguage>;
  fn compiler_settings(&self) -> Option<&CompilerSettingsOptions>;
  fn resolved_settings(&self) -> Option<&Settings>;
}

impl SolcUserOptions for SolcConfigOptions {
  fn solc_version(&self) -> Option<&Version> {
    self.version.as_ref()
  }

  fn compiler_language(&self) -> Option<CompilerLanguage> {
    self.language.map(CompilerLanguage::from)
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

  fn compiler_language(&self) -> Option<CompilerLanguage> {
    self.compiler
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

  fn compiler_language(&self) -> Option<CompilerLanguage> {
    self.solc.language.map(CompilerLanguage::from)
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
    default_language: CompilerLanguage,
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
    default_language: CompilerLanguage,
    default_version: &Version,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .cloned()
      .unwrap_or_else(|| default_version.clone());

    let language = overrides
      .and_then(|opts| opts.compiler_language())
      .unwrap_or(default_language);
    let solc_language = solc_language_from(language)?;

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
      language: solc_language,
    })
  }

  pub(crate) fn merge<O: SolcUserOptions>(&self, overrides: Option<&O>) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .cloned()
      .unwrap_or_else(|| self.version.clone());

    let language = overrides
      .and_then(|opts| opts.compiler_language())
      .unwrap_or_else(|| CompilerLanguage::from(self.language));
    let solc_language = solc_language_from(language)?;

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
      language: solc_language,
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
      validate_object_field(&object, "vyper")?;
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
      compiler,
      mut solc,
      mut vyper,
      cache_enabled,
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
      logging_level,
    } = overrides;

    if let Some(language) = compiler {
      self.config.language = language;
    } else if let Some(language) = solc.language.take() {
      self.config.language = CompilerLanguage::from(language);
    }
    if let Some(version) = solc.version.take() {
      self.config.solc_version = version;
    }
    if let Some(settings) = solc.resolved_settings.take() {
      self.config.solc_settings = sanitize_settings(&settings)?;
    } else if let Some(settings) = solc.settings.take() {
      self.config.solc_settings = merge_settings(&self.config.solc_settings, Some(&settings))?;
    }
    if let Some(path) = vyper.path.take() {
      self.config.vyper_settings.path = Some(path);
    }
    if let Some(optimize) = vyper.optimize.take() {
      self.config.vyper_settings.optimize = Some(optimize);
    }
    if let Some(evm_version) = vyper.evm_version.take() {
      self.config.vyper_settings.evm_version = Some(evm_version);
    }
    if let Some(bytecode_metadata) = vyper.bytecode_metadata.take() {
      self.config.vyper_settings.bytecode_metadata = Some(bytecode_metadata);
    }
    if let Some(search_paths) = vyper.search_paths.take() {
      self.config.vyper_settings.search_paths = Some(search_paths);
    }
    if let Some(selection) = vyper.output_selection.take() {
      self.config.vyper_settings.output_selection = Some(selection);
    }
    if let Some(experimental) = vyper.experimental_codegen.take() {
      self.config.vyper_settings.experimental_codegen = Some(experimental);
    }
    if let Some(cache) = cache_enabled {
      self.config.cache_enabled = cache;
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
    if let Some(level) = logging_level {
      self.config.logging_level = level;
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
  use super::*;
  use crate::internal::settings::EvmVersion as SettingsEvmVersion;
  use serde_json::json;
  use std::collections::BTreeMap;
  use std::path::PathBuf;

  #[test]
  fn compiler_settings_options_accepts_camel_case_fields() {
    let value = json!({
      "stopAfter": "parsing",
      "viaIr": true,
      "outputSelection": {"*": {"*": ["abi"]}},
      "evmVersion": "prague",
      "modelChecker": {"contracts": {"*": ["*"]}}
    });

    let settings: CompilerSettingsOptions = serde_json::from_value(value).expect("settings");
    assert_eq!(settings.stop_after.as_deref(), Some("parsing"));
    assert_eq!(settings.via_ir, Some(true));
    assert!(settings
      .output_selection
      .as_ref()
      .and_then(|map| map.get("*").and_then(|entry| entry
        .get("*")
        .map(|values| values.contains(&"abi".to_string()))))
      .unwrap_or(false));
    match settings.evm_version {
      Some(SettingsEvmVersion::Prague) => {}
      other => panic!("unexpected evm version: {:?}", other),
    }
    assert!(settings.model_checker.is_some());
  }

  #[test]
  fn compiler_config_from_options_merges_library_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let base_path = temp.path().join("lib");
    std::fs::create_dir_all(&base_path).expect("lib dir");

    let mut options = CompilerConfigOptions::default();
    options.library_paths = Some(vec![base_path.clone(), base_path.clone()]);

    let config = CompilerConfig::from_options(Some(options)).expect("config");
    assert_eq!(config.library_paths, vec![base_path.clone(), base_path]);
  }

  #[test]
  fn builder_respects_compiler_language_override() {
    let mut options = CompilerConfigOptions::default();
    options.compiler = Some(CompilerLanguage::Yul);
    let config = CompilerConfigBuilder::from_defaults()
      .apply_compiler_options(options)
      .expect("apply options")
      .build()
      .expect("build");
    assert_eq!(config.language, CompilerLanguage::Yul);
  }

  #[test]
  fn builder_falls_back_to_solc_language_override() {
    let mut options = CompilerConfigOptions::default();
    options.solc.language = Some(FoundrySolcLanguage::Yul);
    let config = CompilerConfigBuilder::from_defaults()
      .apply_compiler_options(options)
      .expect("apply options")
      .build()
      .expect("build");
    assert_eq!(config.language, CompilerLanguage::Yul);
  }

  #[test]
  fn js_compiler_options_accept_vyper_language() {
    let mut options = JsCompilerConfigOptions::default();
    options.language = Some(JsCompilerLanguage::Vyper);
    let parsed = CompilerConfigOptions::try_from(&options).expect("options");
    assert!(matches!(parsed.compiler, Some(CompilerLanguage::Vyper)));
  }

  #[test]
  fn js_vyper_config_parses_fields() {
    let mut options = JsVyperCompilerConfig::default();
    options.path = Some("/tmp/vyper-bin".to_string());
    options.optimize = Some(JsVyperOptimizationMode::Gas);
    options.search_paths = Some(vec!["./lib1".to_string(), "./lib2".to_string()]);
    let parsed = VyperConfigOptions::try_from(&options).expect("vyper options");
    assert_eq!(parsed.path, Some(PathBuf::from("/tmp/vyper-bin")));
    assert_eq!(parsed.optimize, Some(VyperOptimizationMode::Gas));
    let parsed_paths = parsed.search_paths.expect("search paths");
    assert_eq!(parsed_paths.len(), 2);
    assert!(parsed_paths[0].ends_with("lib1") || parsed_paths[1].ends_with("lib1"));
    assert!(parsed_paths[0].ends_with("lib2") || parsed_paths[1].ends_with("lib2"));
  }

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
    assert_eq!(built.language, baseline.language);
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
