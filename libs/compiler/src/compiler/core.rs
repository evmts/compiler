use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use foundry_compilers::artifacts::{
  ast::SourceUnit, CompilerOutput, Settings, SolcInput, SolcLanguage as FoundrySolcLanguage,
  Source, Sources,
};
use foundry_compilers::buildinfo::BuildInfo;
use foundry_compilers::solc::SolcVersionedInput;
use foundry_compilers::{
  artifacts::{error::Severity, remappings::Remapping},
  ProjectPathsConfig,
};
use foundry_config::{Config as FoundryConfig, SolcReq};
use napi::Result;
use serde_json::{json, Value};

use crate::ast::utils::sanitize_ast_value;
use crate::compile::output::{from_standard_json, CoreCompileOutput};
use crate::compiler::input::CompilationInput;
use crate::compiler::project_runner::ProjectRunner;
use crate::internal::{
  config::{CompilerConfig, ConfigOverrides, ResolvedCompilerConfig, SolcConfig},
  errors::{map_napi_error, napi_error},
  project::{ProjectContext, ProjectLayout},
  settings::CompilerSettings,
  solc,
};

pub struct CompilerCore {
  config: ResolvedCompilerConfig,
  project: Option<ProjectContext>,
}

pub struct CompilerWithContext {
  pub compiler: CompilerCore,
  pub context: ProjectContext,
  pub resolved: ResolvedCompilerConfig,
}

impl CompilerCore {
  pub fn new(
    mut config: ResolvedCompilerConfig,
    mut project: Option<ProjectContext>,
  ) -> Result<Self> {
    if project.is_none() {
      project = ProjectRunner::prepare_synthetic_context(&mut config)?;
    }
    solc::ensure_installed(&config.solc_version)?;
    Ok(Self { config, project })
  }

  pub fn base_config(&self) -> &ResolvedCompilerConfig {
    &self.config
  }

  pub fn has_project(&self) -> bool {
    self.project.is_some()
  }

  pub fn project(&self) -> Option<&ProjectContext> {
    self.project.as_ref()
  }

  pub fn resolve_config(
    &self,
    overrides: Option<&CompilerConfig>,
  ) -> Result<ResolvedCompilerConfig> {
    self.config.merge_options(overrides)
  }

  pub fn compile_input(
    &self,
    config: &ResolvedCompilerConfig,
    input: CompilationInput,
  ) -> Result<CoreCompileOutput> {
    if let Some(context) = &self.project {
      let config_cow = if matches!(context.layout, ProjectLayout::Synthetic) {
        let mut clone = config.clone();
        clone.cache_enabled = false;
        Cow::Owned(clone)
      } else {
        Cow::Borrowed(config)
      };
      let runner = ProjectRunner::new(context);
      if let Some(result) = runner.compile_input(config_cow.as_ref(), &input)? {
        return Ok(result);
      }
    }

    self.compile_pure(config, input)
  }

  pub fn compile_project(&self, config: &ResolvedCompilerConfig) -> Result<CoreCompileOutput> {
    let runner = self.project_runner()?;
    runner.compile_project(config)
  }

  pub fn compile_contract(
    &self,
    config: &ResolvedCompilerConfig,
    contract_name: &str,
  ) -> Result<CoreCompileOutput> {
    let runner = self.project_runner()?;
    runner.compile_contract(config, contract_name)
  }

  pub fn with_context(
    mut config: ResolvedCompilerConfig,
    context_loader: impl FnOnce() -> Result<(ConfigOverrides, ProjectContext)>,
  ) -> Result<CompilerWithContext> {
    let (project_overrides, context) = context_loader()?;
    config = config.merged(&project_overrides)?;
    let compiler = CompilerCore::new(config.clone(), Some(context.clone()))?;
    Ok(CompilerWithContext {
      compiler,
      context,
      resolved: config,
    })
  }

  pub fn from_foundry_root(
    config: ResolvedCompilerConfig,
    root: &Path,
  ) -> Result<CompilerWithContext> {
    Self::with_context(config, || load_foundry_project(root))
  }

  pub fn from_hardhat_root(
    config: ResolvedCompilerConfig,
    root: &Path,
  ) -> Result<CompilerWithContext> {
    Self::with_context(config, || load_hardhat_project(root))
  }

  fn project_runner(&self) -> Result<ProjectRunner<'_>> {
    let context = self
      .project
      .as_ref()
      .ok_or_else(|| napi_error("This compiler instance is not bound to a project root."))?;
    Ok(ProjectRunner::new(context))
  }

  fn compile_pure(
    &self,
    config: &ResolvedCompilerConfig,
    input: CompilationInput,
  ) -> Result<CoreCompileOutput> {
    match input {
      CompilationInput::InlineSource { source } => self.compile_inline_source(config, source),
      CompilationInput::SourceMap { sources } => {
        let solc_sources = sources_from_map(sources);
        self.compile_standard_sources(config, solc_sources, config.solc_language)
      }
      CompilationInput::AstUnits { units } => self.compile_ast_sources(config, units),
      CompilationInput::FilePaths {
        paths,
        language_override,
      } => self.compile_file_paths(config, paths, language_override),
    }
  }

  fn compile_inline_source(
    &self,
    config: &ResolvedCompilerConfig,
    source: String,
  ) -> Result<CoreCompileOutput> {
    let mut sources = Sources::new();
    sources.insert(PathBuf::from("__VIRTUAL__.sol"), Source::new(source));
    self.compile_standard_sources(config, sources, config.solc_language)
  }

  fn compile_standard_sources(
    &self,
    config: &ResolvedCompilerConfig,
    sources: Sources,
    language: FoundrySolcLanguage,
  ) -> Result<CoreCompileOutput> {
    let solc_config = SolcConfig {
      version: config.solc_version.clone(),
      settings: config.solc_settings.clone(),
      language,
    };
    let solc = solc::ensure_installed(&solc_config.version)?;
    let mut input = SolcInput::new(language, sources, solc_config.settings.clone());
    input.sanitize(&solc.version);
    let output: CompilerOutput =
      map_napi_error(solc.compile_as(&input), "Solc compilation failed")?;
    Ok(from_standard_json(output))
  }

  fn compile_ast_sources(
    &self,
    config: &ResolvedCompilerConfig,
    ast_sources: BTreeMap<String, SourceUnit>,
  ) -> Result<CoreCompileOutput> {
    let solc_config = SolcConfig {
      version: config.solc_version.clone(),
      settings: config.solc_settings.clone(),
      language: FoundrySolcLanguage::Solidity,
    };
    let solc = solc::ensure_installed(&solc_config.version)?;
    let settings_value = map_napi_error(
      serde_json::to_value(&solc_config.settings),
      "Failed to serialize settings",
    )?;

    let mut sources_value = serde_json::Map::new();
    for (file_name, unit) in ast_sources {
      let mut ast_value =
        map_napi_error(serde_json::to_value(&unit), "Failed to serialise AST value")?;
      sanitize_ast_value(&mut ast_value);
      sources_value.insert(file_name, json!({ "ast": ast_value }));
    }

    let input = json!({
      "language": "SolidityAST",
      "sources": sources_value,
      "settings": settings_value
    });

    let output: CompilerOutput =
      map_napi_error(solc.compile_as(&input), "Solc compilation failed")?;
    Ok(from_standard_json(output))
  }

  fn compile_file_paths(
    &self,
    config: &ResolvedCompilerConfig,
    paths: Vec<PathBuf>,
    language_override: Option<FoundrySolcLanguage>,
  ) -> Result<CoreCompileOutput> {
    if paths.is_empty() {
      return Err(napi_error("compileFiles requires at least one path."));
    }

    let mut string_entries: BTreeMap<String, String> = BTreeMap::new();
    let mut ast_entries: BTreeMap<String, SourceUnit> = BTreeMap::new();
    let mut detected_language: Option<FoundrySolcLanguage> = None;

    for original in paths {
      let content = map_napi_error(fs::read_to_string(&original), "Failed to read source file")?;
      let canonical_path = original.canonicalize().unwrap_or_else(|_| original.clone());
      let canonical_string = canonical_path.to_string_lossy().into_owned();

      if self.try_parse_ast_from_file(&canonical_string, &content, &mut ast_entries)? {
        continue;
      }

      let inferred = self.infer_language(&canonical_path, &content, language_override)?;
      if language_override.is_none() {
        if let Some(existing) = detected_language {
          if existing != inferred {
            return Err(napi_error(
              "compileFiles requires all non-AST sources to share the same solc language. Provide solcLanguage explicitly to disambiguate.",
            ));
          }
        } else {
          detected_language = Some(inferred);
        }
      }

      string_entries.insert(canonical_string, content);
    }

    if !string_entries.is_empty() && !ast_entries.is_empty() {
      return Err(napi_error(
        "compileFiles does not support mixing AST entries with source files. Split the call per input type.",
      ));
    }

    if !ast_entries.is_empty() {
      let mut updated = config.clone();
      updated.solc_language = FoundrySolcLanguage::Solidity;
      return self.compile_ast_sources(&updated, ast_entries);
    }

    let final_language = language_override
      .or(detected_language)
      .unwrap_or(config.solc_language);
    let mut updated = config.clone();
    updated.solc_language = final_language;
    let sources = sources_from_map(string_entries);
    self.compile_standard_sources(&updated, sources, final_language)
  }

  fn try_parse_ast_from_file(
    &self,
    canonical_path: &str,
    content: &str,
    ast_entries: &mut BTreeMap<String, SourceUnit>,
  ) -> Result<bool> {
    let extension = Path::new(canonical_path)
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext.to_ascii_lowercase());
    let trimmed = content.trim_start();
    let maybe_json = trimmed.starts_with('{');

    if matches!(extension.as_deref(), Some("json")) {
      if !maybe_json {
        return Err(napi_error(
          "JSON sources must contain a Solidity AST object.",
        ));
      }
      let value: Value =
        map_napi_error(serde_json::from_str(content), "Failed to parse JSON input")?;
      if !value.is_object() {
        return Err(napi_error(
          "JSON sources must contain a Solidity AST object.",
        ));
      }
      let unit: SourceUnit =
        map_napi_error(serde_json::from_value(value), "Failed to parse AST entry")?;
      ast_entries.insert(canonical_path.to_string(), unit);
      return Ok(true);
    }

    if maybe_json {
      let value: Value =
        map_napi_error(serde_json::from_str(content), "Failed to parse JSON input")?;
      if value.is_object() {
        let unit: SourceUnit =
          map_napi_error(serde_json::from_value(value), "Failed to parse AST entry")?;
        ast_entries.insert(canonical_path.to_string(), unit);
        return Ok(true);
      }
    }

    Ok(false)
  }

  fn infer_language(
    &self,
    path: &Path,
    _content: &str,
    override_language: Option<FoundrySolcLanguage>,
  ) -> Result<FoundrySolcLanguage> {
    if let Some(language) = override_language {
      return Ok(language);
    }

    let extension = path.extension().and_then(|ext| ext.to_str());
    match extension.map(|ext| ext.to_ascii_lowercase()) {
      Some(ext) if ext == "yul" => Ok(FoundrySolcLanguage::Yul),
      Some(ext) if ext == "sol" || ext.is_empty() => Ok(FoundrySolcLanguage::Solidity),
      Some(_) => Err(napi_error(format!(
        "Unable to infer solc language for \"{}\". Provide solcLanguage explicitly.",
        path.display()
      ))),
      None => Ok(FoundrySolcLanguage::Solidity),
    }
  }
}

fn sources_from_map(entries: BTreeMap<String, String>) -> Sources {
  let mut sources = Sources::new();
  for (path, source) in entries {
    sources.insert(PathBuf::from(path), Source::new(source));
  }
  sources
}

fn load_foundry_project(root: &Path) -> Result<(ConfigOverrides, ProjectContext)> {
  let figment = FoundryConfig::figment_with_root(root);
  let config = map_napi_error(
    FoundryConfig::try_from(figment),
    "Failed to load foundry configuration",
  )?
  .sanitized()
  .canonic();

  let mut overrides = ConfigOverrides::default();
  let base_dir = config.__root.0.clone();
  overrides.base_dir = Some(base_dir.clone());
  overrides.cache_enabled = Some(config.cache);
  overrides.offline_mode = Some(config.offline);
  overrides.no_artifacts = Some(false);
  overrides.build_info_enabled = Some(config.build_info);
  overrides.sparse_output = Some(config.sparse_mode);

  if let Some(SolcReq::Version(version)) = &config.solc {
    overrides.solc_version = Some(version.clone());
  }

  let ethers_settings = map_napi_error(
    config.solc_settings(),
    "Failed to derive foundry compiler settings",
  )?;
  let settings_json = map_napi_error(
    serde_json::to_value(&ethers_settings),
    "Failed to serialise foundry compiler settings",
  )?;
  let settings: Settings = map_napi_error(
    serde_json::from_value(settings_json),
    "Failed to convert foundry compiler settings",
  )?;
  overrides.resolved_settings = Some(settings);

  overrides.allow_paths = Some(
    config
      .allow_paths
      .iter()
      .map(|p| canonicalize_with_root(&base_dir, p))
      .collect::<BTreeSet<_>>(),
  );
  if let Some(allow) = overrides.allow_paths.as_mut() {
    allow.insert(base_dir.clone());
  }
  overrides.include_paths = Some(
    config
      .include_paths
      .iter()
      .map(|p| canonicalize_with_root(&base_dir, p))
      .collect::<BTreeSet<_>>(),
  );
  overrides.library_paths = Some(
    config
      .libs
      .iter()
      .map(|p| canonicalize_with_root(&base_dir, p))
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

fn load_hardhat_project(root: &Path) -> Result<(ConfigOverrides, ProjectContext)> {
  let mut paths = map_napi_error(
    ProjectPathsConfig::hardhat(root),
    "Failed to create hardhat project paths",
  )?;
  paths.slash_paths();

  let mut overrides = ConfigOverrides::default();
  overrides.base_dir = Some(paths.root.clone());
  overrides.cache_enabled = Some(true);
  overrides.build_info_enabled = Some(true);
  overrides.no_artifacts = Some(false);

  if let Some((solc_config, cli_settings)) = infer_hardhat_build_info(&paths) {
    overrides.solc_version = Some(solc_config.version);
    let settings_json = map_napi_error(
      serde_json::to_value(&solc_config.settings),
      "Failed to serialise hardhat compiler settings",
    )?;
    let compiler_settings: CompilerSettings = map_napi_error(
      serde_json::from_value(settings_json),
      "Failed to convert hardhat compiler settings",
    )?;
    overrides.solc_settings = Some(compiler_settings);
    overrides.allow_paths = Some(
      cli_settings
        .allow_paths
        .into_iter()
        .map(|p| canonicalize_with_root(&paths.root, &p))
        .collect::<BTreeSet<_>>(),
    );
    if let Some(allow) = overrides.allow_paths.as_mut() {
      allow.insert(paths.root.clone());
    }
    overrides.include_paths = Some(
      cli_settings
        .include_paths
        .into_iter()
        .map(|p| canonicalize_with_root(&paths.root, &p))
        .collect::<BTreeSet<_>>(),
    );
  }

  overrides.library_paths = Some(
    paths
      .libraries
      .iter()
      .map(|p| canonicalize_with_root(&paths.root, p))
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

fn canonicalize_with_root(root: &Path, path: &Path) -> PathBuf {
  let joined = if path.is_absolute() {
    path.to_path_buf()
  } else {
    root.join(path)
  };
  joined.canonicalize().unwrap_or(joined)
}
