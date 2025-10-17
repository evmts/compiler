use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use foundry_compilers::artifacts::ast::SourceUnit;
use foundry_compilers::artifacts::SolcLanguage as FoundrySolcLanguage;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown};
use serde_json::Value;

use crate::ast::utils::from_js_value;
use crate::compile::output::{
  CoreCompileOutput, CoreCompilerError, CoreContractArtifact, CoreSourceLocation,
};
use crate::compiler::core::{CompilerCore, CompilerWithContext};
use crate::compiler::input::CompilationInput;
use crate::internal::config::{parse_compiler_config, CompilerConfig, ResolvedCompilerConfig};
use crate::internal::errors::{map_napi_error, napi_error};
use crate::internal::solc;
use crate::types::{
  CompileOutput, CompilerError, ContractArtifact as JsContractArtifact, ContractBytecode,
  SourceLocation as JsSourceLocation,
};

#[napi]
pub struct Compiler {
  core: CompilerCore,
}

impl Compiler {
  fn from_core(core: CompilerCore) -> Self {
    Self { core }
  }

  fn resolve_call_config(
    &self,
    overrides: Option<&CompilerConfig>,
  ) -> napi::Result<ResolvedCompilerConfig> {
    self.core.resolve_config(overrides)
  }

  fn execute_with_input(
    &self,
    config: &ResolvedCompilerConfig,
    input: CompilationInput,
  ) -> napi::Result<CompileOutput> {
    let core_output = self.core.compile_input(config, input)?;
    Ok(map_compile_output(core_output))
  }

  fn parse_sources_object(env: &Env, sources: JsObject) -> napi::Result<CompilationInput> {
    let raw_entries: BTreeMap<String, Value> =
      from_js_value(env, sources.into_unknown()).map_err(|err| napi_error(err.to_string()))?;
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
        Value::Object(map) => {
          let unit: SourceUnit = map_napi_error(
            serde_json::from_value(Value::Object(map)),
            "Failed to parse AST entry",
          )?;
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
      return Ok(CompilationInput::AstUnits { units: ast_entries });
    }

    Ok(CompilationInput::SourceMap {
      sources: string_entries,
    })
  }

  fn map_language_override(overrides: Option<&CompilerConfig>) -> Option<FoundrySolcLanguage> {
    overrides
      .and_then(|opts| opts.solc_language)
      .map(Into::into)
  }
}

#[napi]
impl Compiler {
  #[napi]
  pub fn install_solc_version(version: String) -> napi::Result<AsyncTask<solc::InstallSolcTask>> {
    let parsed = solc::parse_version(&version)?;
    Ok(solc::install_async(parsed))
  }

  #[napi]
  pub fn is_solc_version_installed(version: String) -> napi::Result<bool> {
    let parsed = solc::parse_version(&version)?;
    solc::is_version_installed(&parsed)
  }

  #[napi(constructor, ts_args_type = "options?: CompilerConfig | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> napi::Result<Self> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = ResolvedCompilerConfig::from_options(parsed.as_ref())?;
    let core = CompilerCore::new(config, None)?;
    Ok(Compiler::from_core(core))
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
    let config = ResolvedCompilerConfig::from_options(parsed.as_ref())?;
    let CompilerWithContext { compiler, .. } =
      CompilerCore::from_foundry_root(config, Path::new(&root))?;
    Ok(Compiler::from_core(compiler))
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
    let config = ResolvedCompilerConfig::from_options(parsed.as_ref())?;
    let CompilerWithContext { compiler, .. } =
      CompilerCore::from_hardhat_root(config, Path::new(&root))?;
    Ok(Compiler::from_core(compiler))
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

    let input = match target {
      Either::A(source) => CompilationInput::InlineSource { source },
      Either::B(object) => {
        let unit: SourceUnit = env.from_js_value(object.into_unknown())?;
        let mut units = BTreeMap::new();
        units.insert("__VIRTUAL__.sol".to_string(), unit);
        CompilationInput::AstUnits { units }
      }
    };

    self.execute_with_input(&config, input)
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
    let input = Self::parse_sources_object(&env, sources)?;
    self.execute_with_input(&config, input)
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
    let language_override = Self::map_language_override(parsed.as_ref());
    let path_bufs = paths.into_iter().map(PathBuf::from).collect();
    let input = CompilationInput::FilePaths {
      paths: path_bufs,
      language_override,
    };
    self.execute_with_input(&config, input)
  }

  #[napi(ts_args_type = "options?: CompilerConfig | undefined")]
  pub fn compile_project(
    &self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> napi::Result<CompileOutput> {
    let parsed = parse_compiler_config(&env, options)?;
    let config = self.resolve_call_config(parsed.as_ref())?;
    let output = self.core.compile_project(&config)?;
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
    let output = self.core.compile_contract(&config, &contract_name)?;
    Ok(map_compile_output(output))
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

fn map_contract_artifact(artifact: CoreContractArtifact) -> JsContractArtifact {
  let CoreContractArtifact {
    contract_name,
    abi,
    bytecode,
    deployed_bytecode,
  } = artifact;

  let abi_json = abi.as_ref().and_then(|abi| serde_json::to_string(abi).ok());

  let bytecode = bytecode.map(make_bytecode);
  let deployed_bytecode = deployed_bytecode.map(make_bytecode);

  JsContractArtifact {
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

fn map_source_location(location: CoreSourceLocation) -> JsSourceLocation {
  JsSourceLocation {
    file: location.file,
    start: location.start,
    end: location.end,
  }
}
