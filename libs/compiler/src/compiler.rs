use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::str::FromStr;

use foundry_compilers::{
  artifacts::{
    ast::SourceUnit, remappings::Remapping as FoundryRemapping, CompilerOutput, SolcInput,
    SolcLanguage as FoundrySolcLanguage, Source, Sources,
  },
  buildinfo::BuildInfo,
  solc::{CliSettings, SolcCompiler, SolcSettings, SolcVersionedInput},
  Project, ProjectBuilder, ProjectCompileOutput, ProjectPathsConfig,
};
use foundry_config::{Config as FoundryConfig, SolcReq};
use napi::bindgen_prelude::*;
use napi::{JsObject, JsUnknown};
use serde_json::{json, Value};

use crate::ast::utils::{from_js_value, sanitize_ast_value};
use crate::compile::{from_standard_json, output};
use crate::internal::{
  errors::{map_napi_error, napi_error},
  options::{
    default_compiler_settings, parse_compiler_options, CompilerOptions, SolcConfig, SolcUserOptions,
  },
  solc,
};
use crate::types::CompileOutput;

/// High-level façade for compiling Solidity sources with a pre-selected solc version.
#[napi]
pub struct Compiler {
  default_config: SolcConfig,
  constructor_overrides: Option<CompilerOptions>,
  project: Option<ProjectState>,
}

impl Compiler {
  fn resolve_config(&self, overrides: Option<&CompilerOptions>) -> Result<SolcConfig> {
    let mut config = self.default_config.clone();

    if let Some(constructor) = &self.constructor_overrides {
      config = config.merge(Some(constructor))?;
    }

    if let Some(project) = &self.project {
      if let Some(inferred) = &project.inferred {
        config = config.overlay(inferred);
      }
    }

    config.merge(overrides)
  }

  fn compile_standard_sources(
    &self,
    config: SolcConfig,
    sources: Sources,
    language: FoundrySolcLanguage,
  ) -> Result<CompileOutput> {
    let solc = solc::ensure_installed(&config.version)?;
    let mut input = SolcInput::new(language, sources, config.settings.clone());
    input.sanitize(&solc.version);
    let output: CompilerOutput =
      map_napi_error(solc.compile_as(&input), "Solc compilation failed")?;
    Ok(from_standard_json(output))
  }

  fn compile_ast_sources(
    &self,
    config: SolcConfig,
    ast_sources: BTreeMap<String, SourceUnit>,
  ) -> Result<CompileOutput> {
    let solc = solc::ensure_installed(&config.version)?;
    let settings_value = map_napi_error(
      serde_json::to_value(&config.settings),
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

  fn compile_with_project<F>(
    &self,
    config: SolcConfig,
    compile_fn: F,
    context: &str,
  ) -> Result<CompileOutput>
  where
    F: FnOnce(
      &Project<SolcCompiler>,
    ) -> std::result::Result<
      ProjectCompileOutput<SolcCompiler>,
      foundry_compilers::error::SolcError,
    >,
  {
    let state = self
      .project
      .as_ref()
      .ok_or_else(|| napi_error("Project-aware compilation requires a root-bound compiler"))?;

    solc::ensure_installed(&config.version)?;

    let project = map_napi_error(
      state.build_project(&config),
      "Failed to configure Solidity project",
    )?;
    let output = map_napi_error(compile_fn(&project), context)?;

    Ok(output::into_compile_output(output))
  }
}

#[derive(Clone, Copy)]
enum ProjectLayout {
  Hardhat,
  Foundry,
}

struct ProjectState {
  _layout: ProjectLayout,
  _root: PathBuf,
  paths: ProjectPathsConfig<FoundrySolcLanguage>,
  cached: bool,
  offline: bool,
  no_artifacts: bool,
  solc_jobs: Option<usize>,
  cli_settings: Option<CliSettings>,
  inferred: Option<SolcConfig>,
}

impl ProjectState {
  fn build_project(
    &self,
    config: &SolcConfig,
  ) -> std::result::Result<Project<SolcCompiler>, foundry_compilers::error::SolcError> {
    let mut builder = ProjectBuilder::default().paths(self.paths.clone());

    if !self.cached {
      builder = builder.set_cached(false);
    }
    if self.offline {
      builder = builder.set_offline(true);
    }
    if self.no_artifacts {
      builder = builder.set_no_artifacts(true);
    }
    if let Some(jobs) = self.solc_jobs {
      if jobs == 1 {
        builder = builder.single_solc_jobs();
      } else {
        builder = builder.solc_jobs(jobs);
      }
    }

    let cli_settings = self.cli_settings.clone().unwrap_or_default();
    let settings = SolcSettings {
      settings: config.settings.clone(),
      cli_settings,
    };

    builder.settings(settings).build(SolcCompiler::default())
  }
}

fn build_foundry_state(root: &Path, base_config: &SolcConfig) -> Result<ProjectState> {
  let figment = FoundryConfig::figment_with_root(root);
  let config = map_napi_error(
    FoundryConfig::try_from(figment),
    "Failed to load foundry configuration",
  )?
  .sanitized()
  .canonic();

  let config_paths = config.project_paths();
  let remappings: Vec<FoundryRemapping> = config_paths
    .remappings
    .iter()
    .filter_map(|remapping| FoundryRemapping::from_str(&remapping.to_string()).ok())
    .collect();

  let paths_builder = ProjectPathsConfig::builder()
    .root(config_paths.root.clone())
    .cache(config_paths.cache.clone())
    .artifacts(config_paths.artifacts.clone())
    .build_infos(config_paths.build_infos.clone())
    .sources(config_paths.sources.clone())
    .tests(config_paths.tests.clone())
    .scripts(config_paths.scripts.clone())
    .libs(config_paths.libraries.clone())
    .remappings(remappings);

  let mut paths =
    paths_builder.build_with_root::<FoundrySolcLanguage>(config_paths.root.clone());
  paths.slash_paths();
  let ethers_settings = map_napi_error(
    config.solc_settings(),
    "Failed to derive foundry compiler settings",
  )?;
  let settings_json = map_napi_error(
    serde_json::to_value(&ethers_settings),
    "Failed to serialise foundry compiler settings",
  )?;
  let settings: foundry_compilers::artifacts::Settings = map_napi_error(
    serde_json::from_value(settings_json),
    "Failed to convert foundry compiler settings",
  )?;

  let version = config
    .solc
    .as_ref()
    .and_then(|req| match req {
      SolcReq::Version(version) => Some(version.clone()),
      _ => None,
    })
    .unwrap_or_else(|| base_config.version.clone());

  let inferred = SolcConfig {
    version,
    settings,
    language: FoundrySolcLanguage::Solidity,
  };

  Ok(ProjectState {
    _layout: ProjectLayout::Foundry,
    _root: paths.root.clone(),
    paths,
    cached: config.cache,
    offline: config.offline,
    no_artifacts: false,
    solc_jobs: None,
    cli_settings: Some(CliSettings {
      extra_args: Vec::new(),
      allow_paths: config
        .allow_paths
        .iter()
        .cloned()
        .chain(std::iter::once(config.__root.0.clone()))
        .collect::<BTreeSet<_>>(),
      base_path: Some(config.__root.0.clone()),
      include_paths: config
        .include_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>(),
    }),
    inferred: Some(inferred),
  })
}

fn build_hardhat_state(root: &Path, _base_config: &SolcConfig) -> Result<ProjectState> {
  let mut paths = map_napi_error(
    ProjectPathsConfig::hardhat(root),
    "Failed to create hardhat project paths",
  )?;
  paths.slash_paths();

  let inferred = infer_hardhat_config(&paths);

  Ok(ProjectState {
    _layout: ProjectLayout::Hardhat,
    _root: paths.root.clone(),
    paths,
    cached: true,
    offline: false,
    no_artifacts: false,
    solc_jobs: None,
    cli_settings: inferred.as_ref().map(|(_, cli)| cli.clone()),
    inferred: inferred.map(|(config, _)| config),
  })
}

fn infer_hardhat_config(
  paths: &ProjectPathsConfig<FoundrySolcLanguage>,
) -> Option<(SolcConfig, CliSettings)> {
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

  let build_info: BuildInfo<SolcVersionedInput, CompilerOutput> =
    BuildInfo::read(&path).ok()?;

  let inferred = SolcConfig {
    version: build_info.solc_version.clone(),
    settings: build_info.input.input.settings.clone(),
    language: build_info.input.input.language,
  };

  Some((inferred, build_info.input.cli_settings.clone()))
}

/// Static helpers and configurable entry points exposed to JavaScript.
#[napi]
impl Compiler {
  /// Download and cache the specified solc release via Foundry's SVM backend.
  ///
  /// Returns a Bun-friendly `AsyncTask` that resolves when the toolchain is
  /// ready. If the release is already cached, the task resolves immediately.
  /// Parsing errors and installation failures surface as JavaScript exceptions.
  #[napi]
  pub fn install_solc_version(version: String) -> Result<AsyncTask<solc::InstallSolcTask>> {
    let parsed = solc::parse_version(&version)?;
    Ok(solc::install_async(parsed))
  }

  /// Determine whether a specific solc release is already present in the local SVM cache.
  ///
  /// This helper never triggers downloads; it simply probes the cache, making it
  /// suitable for test suites to fail fast when prerequisites are missing.
  #[napi]
  pub fn is_solc_version_installed(version: String) -> Result<bool> {
    let parsed = solc::parse_version(&version)?;
    solc::is_version_installed(&parsed)
  }

  /// Construct a compiler bound to a solc version and default compiler settings.
  ///
  /// Passing `solcVersion` is optional – when omitted, the default
  /// `DEFAULT_SOLC_VERSION` is enforced. The constructor validates that the
  /// requested version is already present; callers should invoke
  /// `installSolcVersion` ahead of time. Optional `settings` are parsed exactly
  /// once and cached for subsequent compilations.
  #[napi(constructor, ts_args_type = "options?: CompilerOptions | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_compiler_options(&env, options)?;
    let default_settings = default_compiler_settings();
    let default_language = solc::default_language();
    let default_config =
      SolcConfig::new::<CompilerOptions>(&default_language, &default_settings, None)?;

    let constructor_overrides = parsed;
    let effective_config = default_config.merge(constructor_overrides.as_ref())?;

    solc::ensure_installed(&effective_config.version)?;

    Ok(Compiler {
      default_config,
      constructor_overrides,
      project: None,
    })
  }

  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerOptions | undefined"
  )]
  pub fn from_foundry_root(env: Env, root: String, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_compiler_options(&env, options)?;
    let default_settings = default_compiler_settings();
    let default_language = solc::default_language();
    let default_config =
      SolcConfig::new::<CompilerOptions>(&default_language, &default_settings, None)?;
    let constructor_overrides = parsed;
    let effective_config = default_config.merge(constructor_overrides.as_ref())?;

    solc::ensure_installed(&effective_config.version)?;

    let root_path = PathBuf::from(&root);
    let state = build_foundry_state(&root_path, &effective_config)?;

    if let Some(inferred) = &state.inferred {
      solc::ensure_installed(&inferred.version)?;
    }

    Ok(Compiler {
      default_config,
      constructor_overrides,
      project: Some(state),
    })
  }

  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerOptions | undefined"
  )]
  pub fn from_hardhat_root(env: Env, root: String, options: Option<JsUnknown>) -> Result<Self> {
    let parsed = parse_compiler_options(&env, options)?;
    let default_settings = default_compiler_settings();
    let default_language = solc::default_language();
    let default_config =
      SolcConfig::new::<CompilerOptions>(&default_language, &default_settings, None)?;
    let constructor_overrides = parsed;
    let effective_config = default_config.merge(constructor_overrides.as_ref())?;

    solc::ensure_installed(&effective_config.version)?;

    let root_path = PathBuf::from(&root);
    let state = build_hardhat_state(&root_path, &effective_config)?;

    if let Some(inferred) = &state.inferred {
      solc::ensure_installed(&inferred.version)?;
    }

    Ok(Compiler {
      default_config,
      constructor_overrides,
      project: Some(state),
    })
  }

  /// Compile Solidity/Yul source text or a pre-existing AST using the configured solc version.
  ///
  /// - When `target` is a string, the optional `solcLanguage` controls whether it is treated as
  ///   Solidity (default) or Yul.
  /// - Passing an object is interpreted as a Solidity AST and compiled directly.
  /// - `options` allows per-call overrides that merge on top of the constructor defaults.
  ///
  /// The return value mirrors Foundry's standard JSON output and includes ABI,
  /// bytecode, deployed bytecode and any solc diagnostics.
  #[napi(ts_args_type = "target: string | object, options?: CompilerOptions | undefined")]
  pub fn compile_source(
    &self,
    env: Env,
    target: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let mut config = self.resolve_config(parsed.as_ref())?;
    let input = match target {
      Either::A(source) => CompileInput::Source(single_virtual_source(source)),
      Either::B(object) => {
        let ast_unit: SourceUnit = env.from_js_value(object.into_unknown())?;
        CompileInput::Ast(single_virtual_ast(ast_unit))
      }
    };

    match input {
      CompileInput::Source(source) => match config.language {
        FoundrySolcLanguage::Solidity => {
          self.compile_standard_sources(config, source, FoundrySolcLanguage::Solidity)
        }
        FoundrySolcLanguage::Yul => {
          self.compile_standard_sources(config, source, FoundrySolcLanguage::Yul)
        }
        other => {
          let _ = source;
          Err(napi_error(format!(
            "Unsupported solcLanguage \"{other:?}\" for inline sources"
          )))
        }
      },
      CompileInput::Ast(ast_sources) => {
        config.language = FoundrySolcLanguage::Solidity;
        self.compile_ast_sources(config, ast_sources)
      }
    }
  }

  /// Compile multiple sources supplied as a path keyed lookup.
  #[napi(
    ts_args_type = "sources: Record<string, string | object>, options?: CompilerOptions | undefined"
  )]
  pub fn compile_sources(
    &self,
    env: Env,
    sources: JsObject,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;

    let raw_entries: BTreeMap<String, Value> =
      from_js_value(&env, sources.into_unknown()).map_err(|err| napi_error(err.to_string()))?;
    if raw_entries.is_empty() {
      return Err(napi_error("compileSources requires at least one entry."));
    }

    let mut string_entries: BTreeMap<String, String> = BTreeMap::new();
    let mut ast_entries: BTreeMap<String, SourceUnit> = BTreeMap::new();

    for (path, value) in raw_entries {
      match value {
        Value::String(source) => {
          string_entries.insert(path, source);
        }
        Value::Object(_) => {
          let unit: SourceUnit =
            map_napi_error(serde_json::from_value(value), "Failed to parse AST entry")?;
          ast_entries.insert(path, unit);
        }
        _ => {
          return Err(napi_error(
            "compileSources expects each entry to be a Solidity/Yul source string or a Solidity AST object.",
          ));
        }
      }
    }

    if !string_entries.is_empty() && !ast_entries.is_empty() {
      return Err(napi_error(
        "compileSources does not support mixing inline source strings with AST entries in the same call.",
      ));
    }

    if !ast_entries.is_empty() {
      let mut ast_config = config;
      ast_config.language = FoundrySolcLanguage::Solidity;
      return self.compile_ast_sources(ast_config, ast_entries);
    }

    let final_config = config;
    let sources = sources_from_map(string_entries);
    match final_config.language {
      FoundrySolcLanguage::Solidity => {
        self.compile_standard_sources(final_config, sources, FoundrySolcLanguage::Solidity)
      }
      FoundrySolcLanguage::Yul => {
        self.compile_standard_sources(final_config, sources, FoundrySolcLanguage::Yul)
      }
      other => Err(napi_error(format!(
        "Unsupported solcLanguage \"{other:?}\" for compileSources."
      ))),
    }
  }

  /// Compile sources from on-disk files identified by their paths.
  #[napi(ts_args_type = "paths: string[], options?: CompilerOptions | undefined")]
  pub fn compile_files(
    &self,
    env: Env,
    paths: Vec<String>,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    if paths.is_empty() {
      return Err(napi_error("compileFiles requires at least one path."));
    }

    let parsed = parse_compiler_options(&env, options)?;
    let explicit_language = parsed
      .as_ref()
      .and_then(|opts| opts.solc_language())
      .map(FoundrySolcLanguage::from);
    let mut config = self.resolve_config(parsed.as_ref())?;

    let mut string_entries: BTreeMap<String, String> = BTreeMap::new();
    let mut ast_entries: BTreeMap<String, SourceUnit> = BTreeMap::new();
    let mut detected_language: Option<FoundrySolcLanguage> = None;

    for original in paths {
      let content = map_napi_error(fs::read_to_string(&original), "Failed to read source file")?;
      let canonical = fs::canonicalize(&original)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| original.clone());

      let extension = Path::new(&original)
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
          map_napi_error(serde_json::from_str(&content), "Failed to parse JSON input")?;
        if !value.is_object() {
          return Err(napi_error(
            "JSON sources must contain a Solidity AST object.",
          ));
        }
        let unit: SourceUnit =
          map_napi_error(serde_json::from_value(value), "Failed to parse AST entry")?;
        ast_entries.insert(canonical.clone(), unit);
        continue;
      }

      let recognized_source_extension = matches!(extension.as_deref(), Some("sol") | Some("yul"));
      if !recognized_source_extension && maybe_json {
        let value: Value =
          map_napi_error(serde_json::from_str(&content), "Failed to parse JSON input")?;
        if value.is_object() {
          let unit: SourceUnit =
            map_napi_error(serde_json::from_value(value), "Failed to parse AST entry")?;
          ast_entries.insert(canonical.clone(), unit);
          continue;
        }
      }

      string_entries.insert(canonical.clone(), content);

      if explicit_language.is_none() {
        let language = match extension.as_deref() {
          Some("sol") => FoundrySolcLanguage::Solidity,
          Some("yul") => FoundrySolcLanguage::Yul,
          _ => {
            return Err(napi_error(format!(
              "Unable to infer solc language for \"{canonical}\". Provide solcLanguage explicitly.",
            )));
          }
        };

        if let Some(existing) = detected_language {
          if existing != language {
            return Err(napi_error(
              "compileFiles requires all non-AST sources to share the same solc language. Provide solcLanguage explicitly to disambiguate.",
            ));
          }
        } else {
          detected_language = Some(language);
        }
      }

    }

    if !ast_entries.is_empty() {
      if !string_entries.is_empty() {
        return Err(napi_error(
          "compileFiles does not support mixing AST entries with source files. Split the call per input type.",
        ));
      }
      config.language = FoundrySolcLanguage::Solidity;
      return self.compile_ast_sources(config, ast_entries);
    }

    let final_language = explicit_language
      .or(detected_language)
      .unwrap_or(FoundrySolcLanguage::Solidity);

    config.language = final_language;

    let sources = sources_from_map(string_entries);
    self.compile_standard_sources(config, sources, final_language)
  }

  #[napi]
  pub fn compile_project(
    &self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;

    self.compile_with_project(config, |project| project.compile(), "Project compilation failed")
  }

  #[napi]
  pub fn compile_contract(
    &self,
    env: Env,
    contract_name: String,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;
    let name = contract_name.clone();

    self.compile_with_project(
      config,
      move |project| {
        let path = project.find_contract_path(&name)?;
        project.compile_file(path)
      },
      "Contract compilation failed",
    )
  }
}

enum CompileInput {
  Source(Sources),
  Ast(BTreeMap<String, SourceUnit>),
}

const VIRTUAL_SOURCE_PATH: &str = "__VIRTUAL__.sol";

fn single_virtual_source(source: String) -> Sources {
  let path = PathBuf::from(VIRTUAL_SOURCE_PATH);
  let mut sources = Sources::new();
  sources.insert(path, Source::new(source));
  sources
}

fn single_virtual_ast(ast: SourceUnit) -> BTreeMap<String, SourceUnit> {
  let mut sources = BTreeMap::new();
  sources.insert(VIRTUAL_SOURCE_PATH.to_string(), ast);
  sources
}

fn sources_from_map(entries: BTreeMap<String, String>) -> Sources {
  let mut sources = Sources::new();
  for (path, source) in entries {
    sources.insert(PathBuf::from(path), Source::new(source));
  }
  sources
}
