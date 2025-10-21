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

#[derive(Clone)]
pub struct Compiler {
  state: State,
}

impl Compiler {
  pub fn new(options: Option<CompilerConfigOptions>) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init(config, None)?;
    Ok(Self { state })
  }

  pub fn from_foundry_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_foundry_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  pub fn from_hardhat_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_hardhat_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  pub fn from_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfigOptions>,
  ) -> Result<Self> {
    let config = CompilerConfig::from_options(options).map_err(Error::from)?;
    let state = init_from_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  pub fn install_solc_version(version: &str) -> Result<()> {
    let parsed = solc::parse_version(version)?;
    solc::install_version(&parsed)
  }

  pub fn is_solc_version_installed(version: &str) -> Result<bool> {
    let parsed = solc::parse_version(version)?;
    solc::is_version_installed(&parsed)
  }

  pub fn compile_source(
    &self,
    target: SourceTarget,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_source(&self.state, &config, target)
  }

  pub fn compile_sources(
    &self,
    sources: BTreeMap<String, SourceValue>,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_sources(&self.state, &config, sources)
  }

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

  pub fn compile_project(&self, options: Option<CompilerConfigOptions>) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_project(&self.state, &config)
  }

  pub fn compile_contract(
    &self,
    contract_name: &str,
    options: Option<CompilerConfigOptions>,
  ) -> Result<CompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_contract(&self.state, &config, contract_name)
  }

  pub fn config(&self) -> &CompilerConfig {
    &self.state.config
  }

  pub fn config_mut(&mut self) -> &mut CompilerConfig {
    &mut self.state.config
  }

  pub fn project(&self) -> Option<&ProjectContext> {
    self.state.project.as_ref()
  }

  pub fn project_mut(&mut self) -> Option<&mut ProjectContext> {
    self.state.project.as_mut()
  }

  pub fn get_paths(&self) -> Result<ProjectPaths> {
    resolve_project_paths(&self.state)
  }

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
  fn from_compiler(compiler: Compiler) -> Self {
    Self { inner: compiler }
  }
}

#[napi]
impl JsCompiler {
  #[napi]
  pub fn install_solc_version(version: String) -> napi::Result<AsyncTask<solc::InstallSolcTask>> {
    let parsed = to_napi_result(solc::parse_version(&version))?;
    Ok(solc::install_async(parsed))
  }

  #[napi]
  pub fn is_solc_version_installed(version: String) -> napi::Result<bool> {
    let parsed = to_napi_result(solc::parse_version(&version))?;
    to_napi_result(solc::is_version_installed(&parsed))
  }

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
