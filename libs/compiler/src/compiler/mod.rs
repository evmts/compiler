use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use foundry_compilers::artifacts::ast::SourceUnit;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown};
use serde_json::Value;

use crate::ast::utils::from_js_value;
use crate::internal::config::{
  parse_js_compiler_config, CompilerConfig, CompilerConfigOptions, CompilerLanguage,
};
use crate::internal::errors::{map_napi_error, napi_error, to_napi_result, Error, Result};
use crate::internal::path::ProjectPaths;
use crate::internal::project::{default_cache_dir, synthetic_project_paths, ProjectContext};
use crate::internal::solc;
pub use core::{
  compile_contract, compile_files, compile_project, compile_source, compile_sources, init,
  init_from_foundry_root, init_from_hardhat_root, init_from_root, resolve_config, SourceTarget,
  SourceValue, State,
};
pub use input::CompilationInput;
use output::{into_js_compile_output, CompileOutput, JsCompileOutput};

pub mod core;
mod input;
pub mod output;
mod project_runner;

#[cfg(test)]
mod compiler_tests;

/// Stateful compiler façade that merges configuration, discovers nearby Solidity/Vyper projects,
/// and delegates compilation requests to the appropriate Foundry backend. Each instance carries a
/// resolved [`State`] (configuration + optional project context) so repeated compilations are cheap.
#[derive(Clone)]
pub struct Compiler {
  state: State,
}

impl Compiler {
  /// Create a compiler using the provided options merged on top of the defaults. When no project
  /// root is detected we spin up a synthetic workspace rooted in `~/.tevm` so subsequent calls can
  /// cache inline sources and emitted artifacts.
  pub fn new(options: Option<CompilerConfigOptions>) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init(config, None)?;
    Ok(Self { state })
  }

  /// Instantiate a compiler scoped to an existing Foundry project root. The workspace metadata is
  /// loaded from the root (`foundry.toml`) and merged with the supplied options.
  pub fn from_foundry_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_foundry_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  /// Instantiate a compiler scoped to a Hardhat project root. Hardhat configuration is parsed and
  /// normalised before being merged with the provided options.
  pub fn from_hardhat_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_hardhat_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  /// Instantiate a compiler using an arbitrary filesystem root. Best suited for ad-hoc projects that
  /// still expect Foundry's output directory layout (e.g. temporary repositories).
  pub fn from_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  /// Parse the supplied semantic version and ensure the matching `solc` binary is present on disk.
  /// The download is skipped when the version already exists.
  pub fn install_solc_version(version: &str) -> Result<()> {
    let parsed = solc::parse_version(version)?;
    solc::install_version(&parsed)
  }

  /// Return whether the requested `solc` version is already available locally.
  pub fn is_solc_version_installed(version: &str) -> Result<bool> {
    let parsed = solc::parse_version(version)?;
    solc::is_version_installed(&parsed)
  }

  /// Compile a single inline source string or Solidity AST using the compiler's current
  /// configuration merged with any per-call overrides. Returns a `CompileOutput` that mirrors the
  /// TypeScript `CompileOutput<THasErrors, undefined>` shape. Passing an empty string results in a
  /// parse error from solc.
  pub fn compile_source(
    &self,
    target: SourceTarget,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_source(&self.state, &config, target)
  }

  /// Compile an in-memory map of sources or AST units. All entries must share the same language
  /// unless a language override is provided via the options. Keys should match the virtual file
  /// names you expect to show up in diagnostics. Passing an empty map results in a validation error.
  pub fn compile_sources(
    &self,
    sources: BTreeMap<String, SourceValue>,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_sources(&self.state, &config, sources)
  }

  /// Compile concrete files from disk. The language is inferred from file extensions unless
  /// disambiguated through the provided overrides. Paths are canonicalised before being passed to
  /// Foundry, and all non-AST files must share the same language (provide an explicit override when
  /// mixing `sol` and `vy`).
  pub fn compile_files(
    &self,
    paths: Vec<PathBuf>,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    if paths.is_empty() {
      return Err(Error::new("compileFiles requires at least one path."));
    }
    let config = self.resolve_call_config(options.as_ref())?;
    let language_override = language_override(options.as_ref());
    compile_files(&config, paths, language_override)
  }

  /// Compile every contract discovered in the attached project or synthetic workspace. Equivalent to
  /// running `forge build`/`hardhat compile` with the resolved configuration.
  pub fn compile_project(&self, options: Option<CompilerConfigOptions>) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_project(&self.state, &config)
  }

  /// Compile a single contract by name within the attached project or workspace. Contract names are
  /// matched against the canonical `<File>:<Contract>` identifiers that Foundry uses internally.
  pub fn compile_contract(
    &self,
    contract_name: &str,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_contract(&self.state, &config, contract_name)
  }

  /// Access the resolved compiler configuration backing this instance.
  pub fn config(&self) -> &CompilerConfig {
    &self.state.config
  }

  /// Mutate the underlying configuration before the next compilation call.
  pub fn config_mut(&mut self) -> &mut CompilerConfig {
    &mut self.state.config
  }

  /// Inspect the bound project context, if any (Foundry project, Hardhat project, or synthetic).
  pub fn project(&self) -> Option<&ProjectContext> {
    self.state.project.as_ref()
  }

  /// Mutate the bound project context, if any.
  pub fn project_mut(&mut self) -> Option<&mut ProjectContext> {
    self.state.project.as_mut()
  }

  /// Resolve the filesystem layout used for caching and artifact emission (`ProjectPaths`). If no
  /// project is attached a synthetic layout rooted in `~/.tevm` is returned.
  pub fn get_paths(&self) -> Result<ProjectPaths> {
    resolve_project_paths(&self.state)
  }

  /// Consume the compiler and return the internal state for advanced workflows.
  pub fn into_state(self) -> State {
    self.state
  }

  fn resolve_call_config(
    &self,
    overrides: Option<&CompilerConfigOptions>,
  ) -> Result<CompilerConfig> {
    resolve_config(&self.state, overrides)
  }
}

fn resolve_project_paths(state: &State) -> Result<ProjectPaths> {
  if let Some(context) = &state.project {
    return Ok(context.project_paths());
  }

  let base_dir = default_cache_dir();
  synthetic_project_paths(base_dir.as_path())
}

#[napi(js_name = "Compiler")]
#[derive(Clone)]
pub struct JsCompiler {
  inner: Compiler,
}

impl JsCompiler {
  /// Wrap a Rust `Compiler` for consumption through the N-API bindings.
  fn from_compiler(compiler: Compiler) -> Self {
    Self { inner: compiler }
  }
}

#[napi]
impl JsCompiler {
  /// Download and install a `solc` binary that matches the requested semantic
  /// version. The promise resolves once the binary has been persisted locally.
  #[napi]
  pub fn install_solc_version(version: String) -> napi::Result<AsyncTask<solc::InstallSolcTask>> {
    let parsed = to_napi_result(solc::parse_version(&version))?;
    Ok(solc::install_async(parsed))
  }

  /// Check whether a `solc` binary for the provided version is already available.
  #[napi]
  pub fn is_solc_version_installed(version: String) -> napi::Result<bool> {
    let parsed = to_napi_result(solc::parse_version(&version))?;
    to_napi_result(solc::is_version_installed(&parsed))
  }

  /// Create a compiler that automatically discovers nearby project configuration.
  /// Pass `CompilerConfigOptions` to override defaults such as the solc version or
  /// remappings used for inline compilation.
  #[napi(
    constructor,
    ts_args_type = "options?: CompilerConfigOptions | undefined"
  )]
  pub fn new(env: Env, options: Option<JsUnknown>) -> napi::Result<Self> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let config_options = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let compiler = to_napi_result(Compiler::new(config_options))?;
    Ok(Self::from_compiler(compiler))
  }

  /// Construct a compiler that loads configuration from an existing Foundry project.
  /// The returned instance is already bound to the project for subsequent calls.
  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerConfigOptions | undefined"
  )]
  pub fn from_foundry_root(
    env: Env,
    root: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<Self> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let config_options = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let compiler = to_napi_result(Compiler::from_foundry_root(
      Path::new(&root),
      config_options,
    ))?;
    Ok(Self::from_compiler(compiler))
  }

  /// Construct a compiler that understands a Hardhat project layout rooted at `root`.
  /// Hardhat configuration is normalised before being merged with the supplied options.
  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerConfigOptions | undefined"
  )]
  pub fn from_hardhat_root(
    env: Env,
    root: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<Self> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let config_options = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let compiler = to_napi_result(Compiler::from_hardhat_root(
      Path::new(&root),
      config_options,
    ))?;
    Ok(Self::from_compiler(compiler))
  }

  /// Construct a compiler bound to an arbitrary project root that follows the Foundry
  /// directory layout. Useful when working with generated or temporary repositories.
  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerConfigOptions | undefined"
  )]
  pub fn from_root(env: Env, root: String, options: Option<JsUnknown>) -> napi::Result<Self> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let config_options = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let compiler = to_napi_result(Compiler::from_root(Path::new(&root), config_options))?;
    Ok(Self::from_compiler(compiler))
  }

  /// Compile inline Solidity, Yul, or Vyper source text or an in-memory Solidity AST.
  /// Returns a rich `CompileOutput` snapshot describing contracts, sources, and errors.
  #[napi(
    ts_args_type = "target: string | object, options?: CompilerConfigOptions | undefined",
    ts_return_type = "CompileOutput<true, undefined> | CompileOutput<false, undefined>"
  )]
  pub fn compile_source(
    &self,
    env: Env,
    target: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsCompileOutput> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let config = self.resolve_call_config(overrides.as_ref())?;
    let target = parse_source_target(&env, target)?;
    let output = to_napi_result(compile_source(&self.inner.state, &config, target))?;
    Ok(into_js_compile_output(output))
  }

  /// Compile a keyed map of sources or AST entries. Entries must share a language
  /// unless overridden via the provided compiler options.
  #[napi(
    ts_generic_types = "TSources extends Record<string, string | object> = Record<string, string | object>",
    ts_args_type = "sources: TSources, options?: CompilerConfigOptions | undefined",
    ts_return_type = "CompileOutput<true, Extract<keyof TSources, string>[]> | CompileOutput<false, Extract<keyof TSources, string>[]>"
  )]
  pub fn compile_sources(
    &self,
    env: Env,
    sources: JsObject,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsCompileOutput> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let config = self.resolve_call_config(overrides.as_ref())?;
    let map = Self::parse_sources_object(&env, sources)?;
    let output = to_napi_result(compile_sources(&self.inner.state, &config, map))?;
    Ok(into_js_compile_output(output))
  }

  /// Compile concrete files on disk. Language is inferred from extensions unless the
  /// overrides provide an explicit compiler language.
  #[napi(
    ts_generic_types = "TFilePaths extends readonly string[] = readonly string[]",
    ts_args_type = "paths: TFilePaths, options?: CompilerConfigOptions | undefined",
    ts_return_type = "CompileOutput<true, TFilePaths> | CompileOutput<false, TFilePaths>"
  )]
  pub fn compile_files(
    &self,
    env: Env,
    paths: Vec<String>,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsCompileOutput> {
    if paths.is_empty() {
      return Err(napi_error("compileFiles requires at least one path."));
    }
    let parsed = parse_js_compiler_config(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let config = self.resolve_call_config(overrides.as_ref())?;
    let language_override = language_override(overrides.as_ref());
    let path_bufs = paths.into_iter().map(PathBuf::from).collect();
    let output = to_napi_result(compile_files(&config, path_bufs, language_override))?;
    Ok(into_js_compile_output(output))
  }

  /// Compile the project associated with this compiler instance, returning a snapshot
  /// covering every source file that emitted artifacts.
  #[napi(
    ts_args_type = "options?: CompilerConfigOptions | undefined",
    ts_return_type = "CompileOutput<true, string[]> | CompileOutput<false, string[]>"
  )]
  pub fn compile_project(
    &self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsCompileOutput> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let config = self.resolve_call_config(overrides.as_ref())?;
    let output = to_napi_result(compile_project(&self.inner.state, &config))?;
    Ok(into_js_compile_output(output))
  }

  /// Compile a single contract from the attached project by its canonical name.
  #[napi(
    ts_args_type = "contractName: string, options?: CompilerConfigOptions | undefined",
    ts_return_type = "CompileOutput<true, undefined> | CompileOutput<false, undefined>"
  )]
  pub fn compile_contract(
    &self,
    env: Env,
    contract_name: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsCompileOutput> {
    let parsed = parse_js_compiler_config(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| CompilerConfigOptions::try_from(opts))
      .transpose()?;
    let config = self.resolve_call_config(overrides.as_ref())?;
    let output = to_napi_result(compile_contract(&self.inner.state, &config, &contract_name))?;
    Ok(into_js_compile_output(output))
  }

  /// Return the canonicalised project paths used for artifacts, cache directories,
  /// and virtual source storage.
  #[napi]
  pub fn get_paths(&self) -> napi::Result<ProjectPaths> {
    to_napi_result(self.inner.get_paths())
  }
}

impl JsCompiler {
  fn resolve_call_config(
    &self,
    overrides: Option<&CompilerConfigOptions>,
  ) -> napi::Result<CompilerConfig> {
    to_napi_result(resolve_config(&self.inner.state, overrides))
  }

  fn parse_sources_object(
    env: &Env,
    sources: JsObject,
  ) -> napi::Result<BTreeMap<String, SourceValue>> {
    let raw_entries: BTreeMap<String, Value> =
      from_js_value(env, sources.into_unknown()).map_err(|err| napi_error(err.to_string()))?;
    if raw_entries.is_empty() {
      return Err(napi_error("compileSources requires at least one entry."));
    }

    let mut result: BTreeMap<String, SourceValue> = BTreeMap::new();

    for (path, value) in raw_entries {
      match value {
        Value::String(source) => {
          result.insert(path, SourceValue::Text(source));
        }
        Value::Object(map) => {
          let unit: SourceUnit = map_napi_error(
            serde_json::from_value(Value::Object(map)),
            "Failed to parse AST entry",
          )?;
          result.insert(path, SourceValue::Ast(unit));
        }
        _ => {
          return Err(napi_error(
            "compileSources expects each entry to be a Solidity, Yul, or Vyper source string, or a Solidity AST object.",
          ));
        }
      }
    }

    Ok(result)
  }
}

fn parse_source_target(env: &Env, target: Either<String, JsObject>) -> napi::Result<SourceTarget> {
  match target {
    Either::A(source) => Ok(SourceTarget::Text(source)),
    Either::B(object) => {
      let unit: SourceUnit = from_js_value(env, object.into_unknown())?;
      Ok(SourceTarget::Ast(unit))
    }
  }
}

fn language_override(overrides: Option<&CompilerConfigOptions>) -> Option<CompilerLanguage> {
  overrides.and_then(|opts| {
    opts
      .compiler
      .or_else(|| opts.solc.language.map(CompilerLanguage::from))
  })
}
