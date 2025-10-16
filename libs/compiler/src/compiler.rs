use std::path::PathBuf;

use foundry_compilers::artifacts::{
  ast::SourceUnit, CompilerOutput, Settings, SolcInput, SolcLanguage, Source, Sources,
};
use foundry_compilers::solc::Solc;
use napi::bindgen_prelude::*;
use serde_json::json;

use crate::ast::utils::sanitize_ast_value;
use crate::compile::from_standard_json;
use crate::internal::{
  errors::map_napi_error,
  options::{default_compiler_settings, parse_compiler_options, CompilerOptions, SolcConfig},
  solc,
};
use crate::types::CompileOutput;
use napi::JsUnknown;

const DEFAULT_VIRTUAL_SOURCE: &str = "Instrumented.sol";

/// High-level façade for compiling Solidity sources with a pre-selected solc version.
#[napi]
pub struct Compiler {
  config: SolcConfig,
}

impl Compiler {
  fn resolve_config(&self, overrides: Option<&CompilerOptions>) -> Result<SolcConfig> {
    self.config.merge(overrides)
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
    let config = SolcConfig::new(&default_settings, parsed.as_ref())?;

    solc::ensure_installed(&config.version)?;

    Ok(Compiler { config })
  }

  /// Compile an in-memory Solidity source file using the configured solc version.
  ///
  /// - `source` is the Solidity text to compile.
  /// - `options` allows per-call overrides that merge on top of the constructor defaults.
  ///
  /// The return value mirrors Foundry's standard JSON output and includes ABI,
  /// bytecode, deployed bytecode and any solc diagnostics.
  #[napi(ts_args_type = "source: string, options?: CompilerOptions | undefined")]
  pub fn compile_source(
    &self,
    env: Env,
    source: String,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;
    let solc = solc::ensure_installed(&config.version)?;
    let sources = build_sources(source);
    let input = build_input(&solc, config.settings.clone(), sources);

    let output: CompilerOutput =
      map_napi_error(solc.compile_as(&input), "Solc compilation failed")?;
    Ok(from_standard_json(output))
  }

  /// Compile a Solidity AST (`SourceUnit`) using the configured solc version.
  ///
  /// - `options` allows per-call overrides that merge on top of the constructor defaults.
  #[napi(
    ts_args_type = "ast: import('./ast-types').SourceUnit, options?: CompilerOptions | undefined"
  )]
  pub fn compile_ast(
    &self,
    env: Env,
    ast: JsUnknown,
    options: Option<JsUnknown>,
  ) -> Result<CompileOutput> {
    let parsed = parse_compiler_options(&env, options)?;
    let config = self.resolve_config(parsed.as_ref())?;
    let solc = solc::ensure_installed(&config.version)?;

    let ast_unit: SourceUnit = env.from_js_value(ast)?;
    let mut ast_value = map_napi_error(
      serde_json::to_value(&ast_unit),
      "Failed to serialise AST value",
    )?;
    sanitize_ast_value(&mut ast_value);
    let file_name = DEFAULT_VIRTUAL_SOURCE.to_string();

    let settings_value = map_napi_error(
      serde_json::to_value(&config.settings),
      "Failed to serialize settings",
    )?;

    let input = json!({
      "language": "SolidityAST",
      "sources": {
        file_name.clone(): {
          "ast": ast_value
        }
      },
      "settings": settings_value
    });

    let output: CompilerOutput =
      map_napi_error(solc.compile_as(&input), "Solc compilation failed")?;
    Ok(from_standard_json(output))
  }
}

fn build_sources(source: String) -> Sources {
  let path = PathBuf::from(DEFAULT_VIRTUAL_SOURCE);
  let mut sources = Sources::new();
  sources.insert(path, Source::new(source));
  sources
}

fn build_input(solc: &Solc, settings: Settings, sources: Sources) -> SolcInput {
  let mut input = SolcInput::new(SolcLanguage::Solidity, sources, settings);
  input.sanitize(&solc.version);
  input
}
