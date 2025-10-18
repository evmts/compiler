use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use foundry_compilers::artifacts::ast::SourceUnit;
use foundry_compilers::artifacts::SolcLanguage as FoundrySolcLanguage;
use hex;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown};
use serde_json::Value;

use crate::ast::utils::from_js_value;
use crate::internal::config::{parse_compiler_config, CompilerConfig, ResolvedCompilerConfig};
use crate::internal::errors::{map_napi_error, napi_error, to_napi_result, Error, Result};
use crate::internal::project::ProjectContext;
use crate::internal::solc;

pub mod core;
mod input;
pub mod output;
mod project_runner;

pub use core::{
  compile_contract, compile_files, compile_project, compile_source, compile_sources, init,
  init_from_foundry_root, init_from_hardhat_root, resolve_config, SourceTarget, SourceValue, State,
};
pub use input::CompilationInput;
use output::{
  CompileOutput, CompilerError, ContractArtifact, ContractBytecode, CoreCompileOutput,
  CoreCompilerError, CoreContractArtifact, CoreSourceLocation, SourceLocation,
};

#[derive(Clone)]
pub struct Compiler {
  state: State,
}

impl Compiler {
  pub fn new(options: Option<CompilerConfig>) -> Result<Self> {
    let config = ResolvedCompilerConfig::from_options(options.as_ref()).map_err(Error::from)?;
    let state = init(config, None)?;
    Ok(Self { state })
  }

  pub fn from_foundry_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfig>,
  ) -> Result<Self> {
    let config = ResolvedCompilerConfig::from_options(options.as_ref()).map_err(Error::from)?;
    let state = init_from_foundry_root(config, root.as_ref())?;
    Ok(Self { state })
  }

  pub fn from_hardhat_root<P: AsRef<Path>>(
    root: P,
    options: Option<CompilerConfig>,
  ) -> Result<Self> {
    let config = ResolvedCompilerConfig::from_options(options.as_ref()).map_err(Error::from)?;
    let state = init_from_hardhat_root(config, root.as_ref())?;
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
    options: Option<CompilerConfig>,
  ) -> Result<CoreCompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_source(&self.state, &config, target)
  }

  pub fn compile_sources(
    &self,
    sources: BTreeMap<String, SourceValue>,
    options: Option<CompilerConfig>,
  ) -> Result<CoreCompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_sources(&self.state, &config, sources)
  }

  pub fn compile_files(
    &self,
    paths: Vec<PathBuf>,
    options: Option<CompilerConfig>,
  ) -> Result<CoreCompileOutput> {
    if paths.is_empty() {
      return Err(Error::new("compileFiles requires at least one path."));
    }
    let config = self.resolve_call_config(options.as_ref())?;
    let language_override = language_override(options.as_ref());
    compile_files(&config, paths, language_override)
  }

  pub fn compile_project(&self, options: Option<CompilerConfig>) -> Result<CoreCompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_project(&self.state, &config)
  }

  pub fn compile_contract(
    &self,
    contract_name: &str,
    options: Option<CompilerConfig>,
  ) -> Result<CoreCompileOutput> {
    let config = self.resolve_call_config(options.as_ref())?;
    compile_contract(&self.state, &config, contract_name)
  }

  pub fn config(&self) -> &ResolvedCompilerConfig {
    &self.state.config
  }

  pub fn config_mut(&mut self) -> &mut ResolvedCompilerConfig {
    &mut self.state.config
  }

  pub fn project(&self) -> Option<&ProjectContext> {
    self.state.project.as_ref()
  }

  pub fn project_mut(&mut self) -> Option<&mut ProjectContext> {
    self.state.project.as_mut()
  }

  pub fn into_state(self) -> State {
    self.state
  }

  fn resolve_call_config(
    &self,
    overrides: Option<&CompilerConfig>,
  ) -> Result<ResolvedCompilerConfig> {
    resolve_config(&self.state, overrides)
  }
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

  #[napi(constructor, ts_args_type = "options?: CompilerConfig | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> napi::Result<Self> {
    let parsed = parse_compiler_config(&env, options)?;
    let compiler = to_napi_result(Compiler::new(parsed.clone()))?;
    Ok(Self::from_compiler(compiler))
  }

  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerConfig | undefined"
  )]
  pub fn from_foundry_root(
    env: Env,
    root: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<Self> {
    let parsed = parse_compiler_config(&env, options)?;
    let compiler = to_napi_result(Compiler::from_foundry_root(
      Path::new(&root),
      parsed.clone(),
    ))?;
    Ok(Self::from_compiler(compiler))
  }

  #[napi(
    factory,
    ts_args_type = "root: string, options?: CompilerConfig | undefined"
  )]
  pub fn from_hardhat_root(
    env: Env,
    root: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<Self> {
    let parsed = parse_compiler_config(&env, options)?;
    let compiler = to_napi_result(Compiler::from_hardhat_root(
      Path::new(&root),
      parsed.clone(),
    ))?;
    Ok(Self::from_compiler(compiler))
  }

  #[napi(ts_args_type = "target: string | object, options?: CompilerConfig | undefined")]
  pub fn compile_source(
    &self,
    env: Env,
    target: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let target = parse_source_target(&env, target)?;
    let output = to_napi_result(compile_source(&self.inner.state, &config, target))?;
    Ok(map_compile_output(output))
  }

  #[napi(
    ts_args_type = "sources: Record<string, string | object>, options?: CompilerConfig | undefined"
  )]
  pub fn compile_sources(
    &self,
    env: Env,
    sources: JsObject,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let map = Self::parse_sources_object(&env, sources)?;
    let output = to_napi_result(compile_sources(&self.inner.state, &config, map))?;
    Ok(map_compile_output(output))
  }

  #[napi(ts_args_type = "paths: string[], options?: CompilerConfig | undefined")]
  pub fn compile_files(
    &self,
    env: Env,
    paths: Vec<String>,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    if paths.is_empty() {
      return Err(napi_error("compileFiles requires at least one path."));
    }
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let language_override = language_override(parsed.as_ref());
    let path_bufs = paths.into_iter().map(PathBuf::from).collect();
    let output = to_napi_result(compile_files(&config, path_bufs, language_override))?;
    Ok(map_compile_output(output))
  }

  #[napi(ts_args_type = "options?: CompilerConfig | undefined")]
  pub fn compile_project(
    &self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let output = to_napi_result(compile_project(&self.inner.state, &config))?;
    Ok(map_compile_output(output))
  }

  #[napi(ts_args_type = "contractName: string, options?: CompilerConfig | undefined")]
  pub fn compile_contract(
    &self,
    env: Env,
    contract_name: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let output = to_napi_result(compile_contract(&self.inner.state, &config, &contract_name))?;
    Ok(map_compile_output(output))
  }
}

impl JsCompiler {
  fn resolve_call_config(
    &self,
    overrides: Option<&CompilerConfig>,
  ) -> napi::Result<ResolvedCompilerConfig> {
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
            "compileSources expects each entry to be a Solidity/Yul source string or a Solidity AST object.",
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

fn map_compile_output(output: CoreCompileOutput) -> CompileOutput {
  let artifacts = output
    .artifacts
    .into_iter()
    .map(map_contract_artifact)
    .collect();
  let errors = output.errors.into_iter().map(map_compiler_error).collect();
  CompileOutput {
    artifacts,
    errors,
    has_compiler_errors: output.has_compiler_errors,
  }
}

fn language_override(overrides: Option<&CompilerConfig>) -> Option<FoundrySolcLanguage> {
  overrides
    .and_then(|opts| opts.solc_language)
    .map(FoundrySolcLanguage::from)
}

fn map_contract_artifact(artifact: CoreContractArtifact) -> ContractArtifact {
  let CoreContractArtifact {
    contract_name,
    abi,
    bytecode,
    deployed_bytecode,
  } = artifact;

  let abi_json = abi.as_ref().and_then(|abi| serde_json::to_string(abi).ok());

  let bytecode = bytecode.map(make_bytecode);
  let deployed_bytecode = deployed_bytecode.map(make_bytecode);

  ContractArtifact {
    contract_name,
    abi,
    abi_json,
    bytecode,
    deployed_bytecode,
  }
}

fn make_bytecode(bytes: Vec<u8>) -> ContractBytecode {
  ContractBytecode {
    hex: Some(format!("0x{}", hex::encode(&bytes))),
    bytes: Some(bytes),
  }
}

fn map_compiler_error(error: CoreCompilerError) -> CompilerError {
  CompilerError {
    message: error.message,
    severity: error.severity,
    source_location: error.source_location.map(map_source_location),
  }
}

fn map_source_location(location: CoreSourceLocation) -> SourceLocation {
  SourceLocation {
    file: location.file,
    start: location.start,
    end: location.end,
  }
}
