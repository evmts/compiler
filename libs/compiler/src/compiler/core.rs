use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use foundry_compilers::artifacts::{
  ast::SourceUnit, CompilerOutput, SolcInput, SolcLanguage as FoundrySolcLanguage, Source, Sources,
};
use napi::Result;
use serde_json::{json, Value};

use crate::ast::utils;
use crate::compiler::input::CompilationInput;
use crate::compiler::project_runner::ProjectRunner;
use crate::compiler::{from_standard_json, CoreCompileOutput};
use crate::internal::{
  config::{
    CompilerConfig, CompilerConfigBuilder, ConfigOverrides, ResolvedCompilerConfig, SolcConfig,
  },
  errors::{map_napi_error, napi_error},
  project::{FoundryAdapter, HardhatAdapter, ProjectContext, ProjectLayout},
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
    CompilerConfigBuilder::with_base(self.config.clone())
      .apply_options(overrides)?
      .build()
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
    config: ResolvedCompilerConfig,
    context_loader: impl FnOnce() -> Result<(ConfigOverrides, ProjectContext)>,
  ) -> Result<CompilerWithContext> {
    let (project_overrides, context) = context_loader()?;
    let resolved = CompilerConfigBuilder::with_base(config)
      .apply_overrides(project_overrides)?
      .build()?;
    let compiler = CompilerCore::new(resolved.clone(), Some(context.clone()))?;
    Ok(CompilerWithContext {
      compiler,
      context,
      resolved,
    })
  }

  pub fn from_foundry_root(
    config: ResolvedCompilerConfig,
    root: &Path,
  ) -> Result<CompilerWithContext> {
    Self::with_context(config, || FoundryAdapter::load(root))
  }

  pub fn from_hardhat_root(
    config: ResolvedCompilerConfig,
    root: &Path,
  ) -> Result<CompilerWithContext> {
    Self::with_context(config, || HardhatAdapter::load(root))
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
      utils::sanitize_ast_value(&mut ast_value);
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
