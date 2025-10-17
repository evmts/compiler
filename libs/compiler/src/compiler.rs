use std::collections::BTreeMap;
use std::path::PathBuf;

use foundry_compilers::artifacts::{
  ast::SourceUnit, CompilerOutput, SolcInput, SolcLanguage as FoundrySolcLanguage, Source, Sources,
};
use napi::bindgen_prelude::*;
use napi::{JsObject, JsUnknown};
use serde_json::{json, Value};

use crate::ast::utils::{from_js_value, sanitize_ast_value};
use crate::compile::from_standard_json;
use crate::internal::{
  errors::{map_napi_error, napi_error},
  options::{default_compiler_settings, parse_compiler_options, CompilerOptions, SolcConfig},
  solc,
};
use crate::types::CompileOutput;

/// High-level façade for compiling Solidity sources with a pre-selected solc version.
#[napi]
pub struct Compiler {
  config: SolcConfig,
}

impl Compiler {
  fn resolve_config(&self, overrides: Option<&CompilerOptions>) -> Result<SolcConfig> {
    self.config.merge(overrides)
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
    let config = SolcConfig::new(&default_language, &default_settings, parsed.as_ref())?;

    solc::ensure_installed(&config.version)?;

    Ok(Compiler { config })
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
    let config = self.resolve_config(parsed.as_ref())?;
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
      CompileInput::Ast(ast_sources) => self.compile_ast_sources(config, ast_sources),
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
      return self.compile_ast_sources(config, ast_entries);
    }

    let solc_sources = sources_from_map(string_entries);
    match config.language {
      FoundrySolcLanguage::Solidity => {
        self.compile_standard_sources(config, solc_sources, FoundrySolcLanguage::Solidity)
      }
      FoundrySolcLanguage::Yul => {
        self.compile_standard_sources(config, solc_sources, FoundrySolcLanguage::Yul)
      }
      other => Err(napi_error(format!(
        "Unsupported solcLanguage \"{other:?}\" for compileSources."
      ))),
    }
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
