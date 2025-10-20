use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use foundry_compilers::artifacts::ast::SourceUnit;
use foundry_compilers::artifacts::{
  CompilerOutput, SolcInput, SolcLanguage as FoundrySolcLanguage, Source, Sources,
};
use serde_json::{json, Value};

use super::input::CompilationInput;
use super::output::{from_standard_json, CompileOutput};
use super::project_runner::ProjectRunner;
use crate::ast::utils;
use crate::internal::config::{CompilerConfig, CompilerConfigOptions, SolcConfig};
use crate::internal::errors::{map_err_with_context, Error, Result};
use crate::internal::project::{
  create_synthetic_context, FoundryAdapter, HardhatAdapter, ProjectContext, ProjectLayout,
};
use crate::internal::solc;

#[derive(Clone)]
pub struct State {
  pub config: CompilerConfig,
  pub project: Option<ProjectContext>,
}

#[derive(Clone)]
pub enum SourceTarget {
  Text(String),
  Ast(SourceUnit),
}

#[derive(Clone)]
pub enum SourceValue {
  Text(String),
  Ast(SourceUnit),
}

pub fn init(config: CompilerConfig, project: Option<ProjectContext>) -> Result<State> {
  let project = match project {
    Some(context) => Some(context),
    None => ProjectRunner::prepare_synthetic_context(&config)?,
  };
  solc::ensure_installed(&config.solc_version)?;
  Ok(State { config, project })
}

pub fn init_with_context<F>(config: CompilerConfig, context_loader: F) -> Result<State>
where
  F: FnOnce() -> Result<(CompilerConfigOptions, ProjectContext)>,
{
  let (project_overrides, context) = context_loader()?;
  let resolved = config.merged(&project_overrides).map_err(Error::from)?;
  init(resolved, Some(context))
}

pub fn init_from_foundry_root(config: CompilerConfig, root: &Path) -> Result<State> {
  init_with_context(config, || FoundryAdapter::load(root))
}

pub fn init_from_hardhat_root(config: CompilerConfig, root: &Path) -> Result<State> {
  init_with_context(config, || HardhatAdapter::load(root))
}

pub fn init_from_root(config: CompilerConfig, root: &Path) -> Result<State> {
  let context = create_synthetic_context(root)?;
  init(config, Some(context))
}

pub fn resolve_config(
  state: &State,
  overrides: Option<&CompilerConfigOptions>,
) -> Result<CompilerConfig> {
  state.config.merge_options(overrides).map_err(Error::from)
}

pub fn compile_source(
  state: &State,
  config: &CompilerConfig,
  target: SourceTarget,
) -> Result<CompileOutput> {
  let input = match target {
    SourceTarget::Text(source) => CompilationInput::InlineSource { source },
    SourceTarget::Ast(unit) => {
      let mut units = BTreeMap::new();
      units.insert("__VIRTUAL__.sol".to_string(), unit);
      CompilationInput::AstUnits { units }
    }
  };
  compile_as(state, config, input)
}

pub fn compile_sources(
  state: &State,
  config: &CompilerConfig,
  sources: BTreeMap<String, SourceValue>,
) -> Result<CompileOutput> {
  let input = compilation_input_from_values(sources)?;
  compile_as(state, config, input)
}

pub fn compile_files(
  config: &CompilerConfig,
  paths: Vec<PathBuf>,
  language_override: Option<FoundrySolcLanguage>,
) -> Result<CompileOutput> {
  compile_file_paths(config, paths, language_override)
}

pub fn compile_as(
  state: &State,
  config: &CompilerConfig,
  input: CompilationInput,
) -> Result<CompileOutput> {
  if let Some(context) = &state.project {
    let config_cow = if matches!(context.layout, ProjectLayout::Synthetic) {
      let mut clone = config.clone();
      clone.cache_enabled = false;
      Cow::Owned(clone)
    } else {
      Cow::Borrowed(config)
    };
    let runner = ProjectRunner::new(context);
    if let Some(result) = runner.compile(config_cow.as_ref(), &input)? {
      return Ok(result);
    }
  }

  compile_pure(config, input)
}

pub fn compile_project(state: &State, config: &CompilerConfig) -> Result<CompileOutput> {
  let runner = project_runner(state)?;
  runner.compile_project(config)
}

pub fn compile_contract(
  state: &State,
  config: &CompilerConfig,
  contract_name: &str,
) -> Result<CompileOutput> {
  let runner = project_runner(state)?;
  runner.compile_contract(config, contract_name)
}

fn compile_pure(config: &CompilerConfig, input: CompilationInput) -> Result<CompileOutput> {
  match input {
    CompilationInput::InlineSource { source } => compile_inline_source(config, source),
    CompilationInput::SourceMap { sources } => {
      let solc_sources = sources_from_map(sources);
      compile_standard_sources(config, solc_sources, config.solc_language)
    }
    CompilationInput::AstUnits { units } => compile_ast_sources(config, units),
    CompilationInput::FilePaths {
      paths,
      language_override,
    } => compile_file_paths(config, paths, language_override),
  }
}

fn compile_inline_source(config: &CompilerConfig, source: String) -> Result<CompileOutput> {
  let mut sources = Sources::new();
  sources.insert(PathBuf::from("__VIRTUAL__.sol"), Source::new(source));
  compile_standard_sources(config, sources, config.solc_language)
}

fn compile_standard_sources(
  config: &CompilerConfig,
  sources: Sources,
  language: FoundrySolcLanguage,
) -> Result<CompileOutput> {
  let solc_config = SolcConfig {
    version: config.solc_version.clone(),
    settings: config.solc_settings.clone(),
    language,
  };
  let solc = solc::ensure_installed(&solc_config.version)?;
  let mut input = SolcInput::new(language, sources, solc_config.settings.clone());
  input.sanitize(&solc.version);
  let output: CompilerOutput =
    map_err_with_context(solc.compile_as(&input), "Solc compilation failed")?;
  Ok(from_standard_json(output))
}

fn compile_ast_sources(
  config: &CompilerConfig,
  ast_sources: BTreeMap<String, SourceUnit>,
) -> Result<CompileOutput> {
  let solc_config = SolcConfig {
    version: config.solc_version.clone(),
    settings: config.solc_settings.clone(),
    language: FoundrySolcLanguage::Solidity,
  };
  let solc = solc::ensure_installed(&solc_config.version)?;
  let settings_value = map_err_with_context(
    serde_json::to_value(&solc_config.settings),
    "Failed to serialize settings",
  )?;

  let mut sources_value = serde_json::Map::new();
  for (file_name, unit) in ast_sources {
    let mut ast_value =
      map_err_with_context(serde_json::to_value(&unit), "Failed to serialise AST value")?;
    utils::sanitize_ast_value(&mut ast_value);
    sources_value.insert(file_name, json!({ "ast": ast_value }));
  }

  let input = json!({
    "language": "SolidityAST",
    "sources": sources_value,
    "settings": settings_value
  });

  let output: CompilerOutput =
    map_err_with_context(solc.compile_as(&input), "Solc compilation failed")?;
  Ok(from_standard_json(output))
}

fn compile_file_paths(
  config: &CompilerConfig,
  paths: Vec<PathBuf>,
  language_override: Option<FoundrySolcLanguage>,
) -> Result<CompileOutput> {
  if paths.is_empty() {
    return Err(Error::new("compileFiles requires at least one path."));
  }

  let mut string_entries: BTreeMap<String, String> = BTreeMap::new();
  let mut ast_entries: BTreeMap<String, SourceUnit> = BTreeMap::new();
  let mut detected_language: Option<FoundrySolcLanguage> = None;

  for original in paths {
    let content =
      map_err_with_context(fs::read_to_string(&original), "Failed to read source file")?;
    let canonical_path = original.canonicalize().unwrap_or_else(|_| original.clone());
    let canonical_string = canonical_path.to_string_lossy().into_owned();

    if try_parse_ast_from_file(&canonical_string, &content, &mut ast_entries)? {
      continue;
    }

    let inferred = infer_language(&canonical_path, &content, language_override)?;
    if language_override.is_none() {
      if let Some(existing) = detected_language {
        if existing != inferred {
          return Err(Error::new(
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
    return Err(Error::new(
      "compileSources does not support mixing inline source strings with AST entries in the same call.",
    ));
  }

  if !ast_entries.is_empty() {
    let mut updated = config.clone();
    updated.solc_language = FoundrySolcLanguage::Solidity;
    return compile_ast_sources(&updated, ast_entries);
  }

  let final_language = language_override
    .or(detected_language)
    .unwrap_or(config.solc_language);
  let mut updated = config.clone();
  updated.solc_language = final_language;
  let sources = sources_from_map(string_entries);
  compile_standard_sources(&updated, sources, final_language)
}

fn try_parse_ast_from_file(
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
      return Err(Error::new(
        "JSON sources must contain a Solidity AST object.",
      ));
    }
    let value: Value =
      map_err_with_context(serde_json::from_str(content), "Failed to parse JSON input")?;
    if !value.is_object() {
      return Err(Error::new(
        "JSON sources must contain a Solidity AST object.",
      ));
    }
    let unit: SourceUnit =
      map_err_with_context(serde_json::from_value(value), "Failed to parse AST entry")?;
    ast_entries.insert(canonical_path.to_string(), unit);
    return Ok(true);
  }

  if maybe_json {
    let value: Value =
      map_err_with_context(serde_json::from_str(content), "Failed to parse JSON input")?;
    if value.is_object() {
      let unit: SourceUnit =
        map_err_with_context(serde_json::from_value(value), "Failed to parse AST entry")?;
      ast_entries.insert(canonical_path.to_string(), unit);
      return Ok(true);
    }
  }

  Ok(false)
}

fn infer_language(
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
    Some(_) => Err(Error::new(format!(
      "Unable to infer solc language for \"{}\". Provide solcLanguage explicitly.",
      path.display()
    ))),
    None => Ok(FoundrySolcLanguage::Solidity),
  }
}

fn compilation_input_from_values(
  sources: BTreeMap<String, SourceValue>,
) -> Result<CompilationInput> {
  let mut string_entries: BTreeMap<String, String> = BTreeMap::new();
  let mut ast_entries: BTreeMap<String, SourceUnit> = BTreeMap::new();

  for (path, value) in sources {
    match value {
      SourceValue::Text(source) => {
        string_entries.insert(path, source);
      }
      SourceValue::Ast(unit) => {
        ast_entries.insert(path, unit);
      }
    }
  }

  if !string_entries.is_empty() && !ast_entries.is_empty() {
    return Err(Error::new(
      "compileSources does not support mixing inline source strings with AST entries in the same call.",
    ));
  }

  if !ast_entries.is_empty() {
    return Ok(CompilationInput::AstUnits { units: ast_entries });
  }

  Ok(CompilationInput::SourceMap {
    sources: string_entries,
  })
}

fn sources_from_map(entries: BTreeMap<String, String>) -> Sources {
  let mut sources = Sources::new();
  for (path, source) in entries {
    sources.insert(PathBuf::from(path), Source::new(source));
  }
  sources
}

fn project_runner(state: &State) -> Result<ProjectRunner<'_>> {
  let context = state
    .project
    .as_ref()
    .ok_or_else(|| Error::new("This compiler instance is not bound to a project root."))?;
  Ok(ProjectRunner::new(context))
}
