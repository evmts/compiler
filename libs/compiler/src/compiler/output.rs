use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::path::PathBuf;

use foundry_compilers::artifacts::contract::Contract as FoundryContract;
use foundry_compilers::artifacts::{
  ast::SourceUnit,
  error::{
    Error as FoundryCompilerError, SecondarySourceLocation as FoundrySecondarySourceLocation,
    Severity,
  },
  vyper::VyperCompilationError,
  CompilerOutput, FileToContractsMap, SourceFile,
};
use foundry_compilers::compilers::multi::{MultiCompiler, MultiCompilerError};
use foundry_compilers::compilers::Compiler as FoundryCompiler;
use foundry_compilers::ProjectCompileOutput;
use napi::{Env, JsUnknown};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ast::{utils::sanitize_ast_value, Ast, JsAst, SourceTarget};
use crate::contract;
use crate::contract::{Contract, JsContract, JsContractState};
use crate::internal::config::AstConfigOptions;
use crate::internal::errors::napi_error;

// -----------------------------------------------------------------------------
// Shared error and location types
// -----------------------------------------------------------------------------

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeverityLevel {
  Error,
  Warning,
  Info,
}

#[napi(object)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceLocation {
  pub file: String,
  pub start: i32,
  pub end: i32,
}

#[napi(object)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecondarySourceLocation {
  pub file: Option<String>,
  pub start: Option<i32>,
  pub end: Option<i32>,
  pub message: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VyperSourceLocation {
  pub file: String,
  pub line: Option<i32>,
  pub column: Option<i32>,
}

#[napi(object)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerError {
  pub message: String,
  pub formatted_message: Option<String>,
  pub component: String,
  pub severity: SeverityLevel,
  pub error_type: String,
  pub error_code: Option<i64>,
  pub source_location: Option<SourceLocation>,
  pub secondary_source_locations: Option<Vec<SecondarySourceLocation>>,
  pub vyper_source_location: Option<VyperSourceLocation>,
}

// -----------------------------------------------------------------------------
// Core domain types (Rust-facing)
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct SourceArtifacts {
  pub source_path: Option<String>,
  pub source_id: Option<u32>,
  pub solc_version: Option<Version>,
  pub ast: Option<SourceUnit>,
  pub contracts: BTreeMap<String, Contract>,
}

impl SourceArtifacts {
  fn new(source_path: Option<String>) -> Self {
    Self {
      source_path,
      ..Default::default()
    }
  }

  pub fn to_json(&self) -> SourceArtifactsJson {
    SourceArtifactsJson::from_source_artifacts(self)
  }
}

#[napi(object, js_name = "SourceArtifactsJson")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceArtifactsJson {
  #[napi(ts_type = "string | undefined")]
  pub source_path: Option<String>,
  #[napi(ts_type = "number | undefined")]
  pub source_id: Option<u32>,
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  #[napi(ts_type = "import('./solc-ast').SourceUnit | undefined")]
  pub ast: Option<Value>,
  #[napi(ts_type = "Record<string, ContractState> | undefined")]
  pub contracts: Option<BTreeMap<String, JsContractState>>,
}

impl SourceArtifactsJson {
  fn from_source_artifacts(artifacts: &SourceArtifacts) -> Self {
    let ast = artifacts.ast.as_ref().and_then(|unit| {
      let mut value = serde_json::to_value(unit).ok()?;
      sanitize_ast_value(&mut value);
      Some(value)
    });

    let contracts = if artifacts.contracts.is_empty() {
      None
    } else {
      Some(
        artifacts
          .contracts
          .iter()
          .map(|(name, contract)| {
            let snapshot = contract::contract_state_to_js(contract.state());
            (name.clone(), snapshot)
          })
          .collect(),
      )
    };

    Self {
      source_path: artifacts.source_path.clone(),
      source_id: artifacts.source_id,
      solc_version: artifacts
        .solc_version
        .as_ref()
        .map(|version| version.to_string()),
      ast,
      contracts,
    }
  }
}

#[derive(Clone, Debug)]
pub struct CompileOutput {
  pub raw_artifacts: Value,
  pub artifacts: BTreeMap<String, SourceArtifacts>,
  pub artifact: Option<SourceArtifacts>,
  pub errors: Vec<CompilerError>,
}

impl CompileOutput {
  pub fn has_compiler_errors(&self) -> bool {
    self
      .errors
      .iter()
      .any(|error| error.severity == SeverityLevel::Error)
  }

  pub fn to_json(&self) -> CompileOutputJson {
    CompileOutputJson::from_compile_output(self)
  }
}

#[napi(object, js_name = "CompileOutputJson")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileOutputJson {
  #[napi(ts_type = "SourceArtifactsJson | undefined")]
  pub artifact: Option<SourceArtifactsJson>,
  #[napi(ts_type = "Record<string, SourceArtifactsJson> | undefined")]
  pub artifacts: Option<BTreeMap<String, SourceArtifactsJson>>,
  #[napi(ts_type = "ReadonlyArray<CompilerError> | undefined")]
  pub errors: Option<Vec<CompilerError>>,
  #[napi(ts_type = "Record<string, unknown> | undefined")]
  pub raw_artifacts: Option<Value>,
}

impl CompileOutputJson {
  fn from_compile_output(output: &CompileOutput) -> Self {
    let artifact = output.artifact.as_ref().map(SourceArtifacts::to_json);

    let artifacts = if output.artifacts.is_empty() {
      None
    } else {
      Some(
        output
          .artifacts
          .iter()
          .map(|(path, artifacts)| (path.clone(), artifacts.to_json()))
          .collect(),
      )
    };

    let errors = if output.errors.is_empty() {
      None
    } else {
      Some(output.errors.clone())
    };

    Self {
      artifact,
      artifacts,
      errors,
      raw_artifacts: if output.raw_artifacts.is_null() {
        None
      } else {
        Some(output.raw_artifacts.clone())
      },
    }
  }
}

pub fn into_core_compile_output(output: ProjectCompileOutput<MultiCompiler>) -> CompileOutput {
  let artifacts = collate_project_artifacts(&output);
  let artifact = artifacts
    .values()
    .next()
    .cloned()
    .filter(|_| artifacts.len() == 1);
  CompileOutput {
    raw_artifacts: aggregated_to_value(output.output()),
    errors: output
      .output()
      .errors
      .iter()
      .map(|error: &MultiCompilerError| multi_error_to_core(error))
      .collect(),
    artifact,
    artifacts,
  }
}

pub fn from_standard_json(output: CompilerOutput) -> CompileOutput {
  let raw_artifacts = serde_json::to_value(&output).unwrap_or(Value::Null);
  let errors = output
    .errors
    .iter()
    .map(|error: &FoundryCompilerError| solc_error_to_core(error))
    .collect();
  build_compile_output(&output.contracts, &output.sources, raw_artifacts, errors)
}

fn convert_source_ast(source: &SourceFile) -> Option<SourceUnit> {
  let ast = source.ast.as_ref()?;
  let mut value = serde_json::to_value(ast).ok()?;
  sanitize_ast_value(&mut value);
  serde_json::from_value(value).ok()
}

fn solc_error_to_core(error: &FoundryCompilerError) -> CompilerError {
  let severity = match error.severity {
    Severity::Error => SeverityLevel::Error,
    Severity::Warning => SeverityLevel::Warning,
    Severity::Info => SeverityLevel::Info,
  };
  let secondary = if error.secondary_source_locations.is_empty() {
    None
  } else {
    Some(
      error
        .secondary_source_locations
        .iter()
        .map(to_core_secondary_location)
        .collect(),
    )
  };

  CompilerError {
    message: error.message.clone(),
    formatted_message: error.formatted_message.clone(),
    component: error.component.clone(),
    severity,
    error_type: error.r#type.clone(),
    error_code: error.error_code.map(|code| code as i64),
    source_location: error.source_location.as_ref().map(|loc| SourceLocation {
      file: loc.file.clone(),
      start: loc.start,
      end: loc.end,
    }),
    secondary_source_locations: secondary,
    vyper_source_location: None,
  }
}

pub(crate) fn vyper_error_to_core(error: &VyperCompilationError) -> CompilerError {
  let severity = match error.severity {
    Severity::Error => SeverityLevel::Error,
    Severity::Warning => SeverityLevel::Warning,
    Severity::Info => SeverityLevel::Info,
  };

  let vyper_source_location = error
    .source_location
    .as_ref()
    .and_then(|loc| serde_json::to_value(loc).ok())
    .and_then(convert_vyper_source_location);

  CompilerError {
    message: error.message.clone(),
    formatted_message: error.formatted_message.clone(),
    component: "vyper".to_string(),
    severity,
    error_type: "Vyper".to_string(),
    error_code: None,
    source_location: None,
    secondary_source_locations: None,
    vyper_source_location,
  }
}

fn multi_error_to_core(error: &MultiCompilerError) -> CompilerError {
  match error {
    MultiCompilerError::Solc(error) => solc_error_to_core(error),
    MultiCompilerError::Vyper(error) => vyper_error_to_core(error),
  }
}

pub(crate) fn build_compile_output(
  contracts: &FileToContractsMap<FoundryContract>,
  sources: &BTreeMap<PathBuf, SourceFile>,
  raw_artifacts: Value,
  errors: Vec<CompilerError>,
) -> CompileOutput {
  let mut artifacts: BTreeMap<String, SourceArtifacts> = BTreeMap::new();

  for (path, contract_map) in contracts {
    let key = path.to_string_lossy().to_string();
    let entry = artifacts
      .entry(key.clone())
      .or_insert_with(|| SourceArtifacts::new(Some(key.clone())));

    for (name, foundry_contract) in contract_map {
      let mut core = Contract::from_foundry_standard_json(name.clone(), foundry_contract);
      core.state_mut().source_path = Some(key.clone());
      entry.contracts.insert(name.clone(), core);
    }
  }

  for (path, source) in sources {
    let key = path.to_string_lossy().to_string();
    let entry = artifacts
      .entry(key.clone())
      .or_insert_with(|| SourceArtifacts::new(Some(key.clone())));
    entry.source_id = Some(source.id);
    entry.ast = convert_source_ast(source);
  }

  let artifact = artifacts
    .values()
    .next()
    .cloned()
    .filter(|_| artifacts.len() == 1);

  CompileOutput {
    raw_artifacts,
    artifacts,
    artifact,
    errors,
  }
}

fn to_core_secondary_location(
  location: &FoundrySecondarySourceLocation,
) -> SecondarySourceLocation {
  SecondarySourceLocation {
    file: location.file.clone(),
    start: location.start,
    end: location.end,
    message: location.message.clone(),
  }
}

// TODO: this won't be necessary once merged https://github.com/foundry-rs/compilers/pull/333
fn convert_vyper_source_location(value: Value) -> Option<VyperSourceLocation> {
  let file = value.get("file")?.as_str()?.to_string();
  let line = value
    .get("lineno")
    .and_then(|entry| entry.as_u64())
    .map(clamp_u64_to_i32);
  let column = value
    .get("col_offset")
    .and_then(|entry| entry.as_u64())
    .map(clamp_u64_to_i32);
  Some(VyperSourceLocation { file, line, column })
}

fn clamp_u64_to_i32(value: u64) -> i32 {
  i32::try_from(value).unwrap_or(i32::MAX)
}

fn collate_project_artifacts(
  output: &ProjectCompileOutput<MultiCompiler>,
) -> BTreeMap<String, SourceArtifacts> {
  let mut artifacts: BTreeMap<String, SourceArtifacts> = BTreeMap::new();

  let mut version_lookup: BTreeMap<(String, String), Version> = BTreeMap::new();
  for (path, name, _, version) in output.output().contracts.contracts_with_files_and_version() {
    let key = path.to_string_lossy().to_string();
    version_lookup.insert((key, name.clone()), version.clone());
  }

  for (path, name, artifact) in output.artifacts_with_files() {
    let key = path.to_string_lossy().to_string();
    let entry = artifacts
      .entry(key.clone())
      .or_insert_with(|| SourceArtifacts::new(Some(key.clone())));

    let version = version_lookup.get(&(key.clone(), name.clone())).cloned();
    if entry.solc_version.is_none() {
      entry.solc_version = version.clone();
    }

    let mut contract = Contract::from_configurable_artifact(name.clone(), artifact);
    contract.state_mut().source_path = Some(key.clone());
    if entry.source_id.is_none() {
      entry.source_id = contract.state().source_id;
    }
    entry.contracts.insert(name.clone(), contract);
  }

  for (path, source, version) in output.output().sources.sources_with_version() {
    let key = path.to_string_lossy().to_string();
    let entry = artifacts
      .entry(key.clone())
      .or_insert_with(|| SourceArtifacts::new(Some(key.clone())));
    if entry.solc_version.is_none() {
      entry.solc_version = Some(version.clone());
    }
    if entry.source_id.is_none() {
      entry.source_id = Some(source.id);
    }
    if entry.ast.is_none() {
      entry.ast = convert_source_ast(source);
    }
  }

  artifacts
}

fn aggregated_to_value<C>(aggregated: &foundry_compilers::AggregatedCompilerOutput<C>) -> Value
where
  C: FoundryCompiler,
  C::CompilationError: Serialize,
{
  let mut root = Map::new();
  let mut contracts_map = Map::new();
  for (path, entries) in aggregated.contracts.0.iter() {
    let mut contract_map = Map::new();
    for (name, versions) in entries.iter() {
      if let Some(latest) = versions.last() {
        if let Ok(value) = serde_json::to_value(&latest.contract) {
          contract_map.insert(name.clone(), value);
        }
      }
    }
    contracts_map.insert(
      path.to_string_lossy().to_string(),
      Value::Object(contract_map),
    );
  }
  root.insert("contracts".to_string(), Value::Object(contracts_map));

  let mut sources_map = Map::new();
  for (path, entries) in aggregated.sources.0.iter() {
    if let Some(latest) = entries.last() {
      if let Ok(value) = serde_json::to_value(&latest.source_file) {
        sources_map.insert(path.to_string_lossy().to_string(), value);
      }
    }
  }
  root.insert("sources".to_string(), Value::Object(sources_map));
  root.insert(
    "errors".to_string(),
    serde_json::to_value(&aggregated.errors).unwrap_or(Value::Null),
  );
  Value::Object(root)
}

// -----------------------------------------------------------------------------
// JS-facing compile output wrappers
// -----------------------------------------------------------------------------

#[napi(js_name = "SourceArtifacts")]
#[derive(Clone, Debug)]
pub struct JsSourceArtifacts {
  source_path: Option<String>,
  source_id: Option<u32>,
  solc_version: Option<Version>,
  ast_unit: Option<SourceUnit>,
  json: SourceArtifactsJson,
  contracts: HashMap<String, Contract>,
}

impl JsSourceArtifacts {
  fn from_core(artifacts: SourceArtifacts) -> Self {
    let json = artifacts.to_json();

    let SourceArtifacts {
      source_path,
      source_id,
      solc_version,
      ast,
      contracts,
    } = artifacts;

    Self {
      source_path,
      source_id,
      solc_version,
      ast_unit: ast,
      json,
      contracts: contracts.into_iter().collect(),
    }
  }

  fn ast_config(&self) -> Option<AstConfigOptions> {
    let mut options = AstConfigOptions::default();
    let mut has_override = false;

    if let Some(version) = &self.solc_version {
      options.solc.version = Some(version.clone());
      has_override = true;
    }

    if has_override {
      Some(options)
    } else {
      None
    }
  }
}

#[napi]
impl JsSourceArtifacts {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      source_path: None,
      source_id: None,
      solc_version: None,
      ast_unit: None,
      json: SourceArtifactsJson::default(),
      contracts: HashMap::new(),
    }
  }

  #[napi(getter)]
  pub fn source_path(&self) -> Option<String> {
    self.source_path.clone()
  }

  #[napi(getter)]
  pub fn source_id(&self) -> Option<u32> {
    self.source_id
  }

  #[napi(getter)]
  pub fn solc_version(&self) -> Option<String> {
    self
      .solc_version
      .as_ref()
      .map(|version| version.to_string())
  }

  #[napi(getter, ts_return_type = "Ast | undefined")]
  pub fn ast(&self) -> napi::Result<Option<JsAst>> {
    let unit = match &self.ast_unit {
      Some(unit) => unit.clone(),
      None => return Ok(None),
    };

    let options = self.ast_config();
    let mut ast = Ast::new(options.clone()).map_err(|err| napi_error(err.to_string()))?;
    ast
      .from_source(SourceTarget::Ast(unit), options)
      .map_err(|err| napi_error(err.to_string()))?;

    Ok(Some(JsAst::from_ast(ast)))
  }

  #[napi(getter, ts_return_type = "Record<string, Contract>")]
  pub fn contracts(&self) -> HashMap<String, JsContract> {
    self
      .contracts
      .iter()
      .map(|(name, contract)| (name.clone(), contract::contract_class(contract)))
      .collect()
  }

  #[napi(js_name = "toJson", ts_return_type = "SourceArtifactsJson")]
  pub fn to_json(&self) -> SourceArtifactsJson {
    self.json.clone()
  }
}

#[napi(js_name = "CompileOutput")]
#[derive(Clone, Debug)]
pub struct JsCompileOutput {
  json: CompileOutputJson,
  raw_artifacts: Value,
  artifacts: HashMap<String, JsSourceArtifacts>,
  artifact: Option<JsSourceArtifacts>,
  errors: Vec<CompilerError>,
  has_compiler_errors: bool,
}

impl JsCompileOutput {
  fn from_core(core: CompileOutput) -> Self {
    let has_compiler_errors = core.has_compiler_errors();
    let json = core.to_json();
    let CompileOutput {
      raw_artifacts,
      artifacts,
      artifact,
      errors,
    } = core;

    let artifacts = artifacts
      .into_iter()
      .map(|(path, artifacts)| (path, JsSourceArtifacts::from_core(artifacts)))
      .collect::<HashMap<_, _>>();
    let artifact = artifact.map(JsSourceArtifacts::from_core);

    Self {
      json,
      raw_artifacts,
      artifacts,
      artifact,
      errors,
      has_compiler_errors,
    }
  }
}

#[napi]
impl JsCompileOutput {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      json: CompileOutputJson::default(),
      raw_artifacts: Value::Null,
      artifacts: HashMap::new(),
      artifact: None,
      errors: Vec::new(),
      has_compiler_errors: false,
    }
  }

  #[napi(
    getter,
    js_name = "artifactsJson",
    ts_return_type = "Record<string, unknown>"
  )]
  pub fn raw_artifacts(&self) -> Value {
    self.raw_artifacts.clone()
  }

  #[napi(getter, ts_return_type = "Record<string, SourceArtifacts>")]
  pub fn artifacts(&self) -> HashMap<String, JsSourceArtifacts> {
    self.artifacts.clone()
  }

  #[napi(getter, ts_return_type = "SourceArtifacts | undefined")]
  pub fn artifact(&self) -> Option<JsSourceArtifacts> {
    self.artifact.clone()
  }

  #[napi(getter, ts_return_type = "ReadonlyArray<CompilerError> | undefined")]
  pub fn errors(&self, env: Env) -> napi::Result<JsUnknown> {
    if self.has_compiler_errors() {
      let value = env
        .to_js_value(&self.errors)
        .map_err(|err| napi_error(err.to_string()))?;
      Ok(value)
    } else {
      Ok(env.get_undefined()?.into_unknown())
    }
  }

  #[napi(getter)]
  pub fn diagnostics(&self) -> Vec<CompilerError> {
    self.errors.clone()
  }

  #[napi]
  pub fn has_compiler_errors(&self) -> bool {
    self.has_compiler_errors
  }

  #[napi(js_name = "toJson", ts_return_type = "CompileOutputJson")]
  pub fn to_json(&self) -> CompileOutputJson {
    self.json.clone()
  }
}

pub fn into_js_compile_output(core: CompileOutput) -> JsCompileOutput {
  JsCompileOutput::from_core(core)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use foundry_compilers::artifacts::CompilerOutput as StandardCompilerOutput;
  use foundry_compilers::artifacts::SourceFile;
  use serde_json::json;
  use std::path::PathBuf;

  #[test]
  fn from_standard_json_populates_contracts_map() {
    let json = r#"{
      "contracts": {
        "Test.sol": {
          "Test": {
            "abi": [],
            "evm": {
              "bytecode": { "object": "0x6000" },
              "deployedBytecode": { "bytecode": { "object": "0x6001" }, "immutableReferences": {} }
            }
          }
        }
      },
      "errors": [
        {
          "component": "general",
          "errorCode": "42",
          "formattedMessage": "Error: detail",
          "message": "detail",
          "severity": "error",
          "type": "TypeError",
          "sourceLocation": { "file": "Test.sol", "start": 0, "end": 10 }
        }
      ],
      "sources": {
        "Test.sol": {
          "id": 1
        }
      },
      "version": "0.8.21"
    }"#;

    let output: StandardCompilerOutput = serde_json::from_str(json).expect("compiler output");
    let core = from_standard_json(output);

    assert!(core.has_compiler_errors());
    assert!(core.raw_artifacts["contracts"]["Test.sol"]["Test"].is_object());
    let snapshot = core.to_json();
    let artifacts = snapshot.artifacts.expect("artifacts snapshot");
    let source_snapshot = artifacts.get("Test.sol").expect("source snapshot");
    assert!(snapshot.raw_artifacts.is_some());
    assert!(source_snapshot
      .contracts
      .as_ref()
      .and_then(|contracts| contracts.get("Test"))
      .is_some());
    let snapshot_errors = snapshot.errors.expect("errors snapshot");
    assert!(!snapshot_errors.is_empty());
    let entry = core.artifacts.get("Test.sol").expect("source entry");
    assert!(entry.contracts.contains_key("Test"));
    let error = &core.errors[0];
    assert_eq!(error.severity, SeverityLevel::Error);
    assert_eq!(error.error_code, Some(42));
  }

  #[test]
  fn from_standard_json_captures_ast_when_present() {
    use foundry_compilers::artifacts::ast::Ast;

    let ast: Ast = serde_json::from_value(json!({
      "absolutePath": "Inline.sol",
      "id": 1,
      "exportedSymbols": {},
      "nodeType": "SourceUnit",
      "src": "0:0:0",
      "nodes": [
        {
          "id": 2,
          "nodeType": "ContractDefinition",
          "src": "0:0:0",
          "nodes": [],
          "body": null,
          "contractKind": "contract",
          "fullyImplemented": true,
          "name": "Inline"
        }
      ]
    }))
    .expect("ast");

    let source_file = SourceFile {
      id: 1,
      ast: Some(ast),
    };

    let mut output = CompilerOutput::default();
    output
      .sources
      .insert(PathBuf::from("Inline.sol"), source_file);
    let core = from_standard_json(output);

    let entry = core.artifacts.get("Inline.sol").expect("source entry");
    assert_eq!(entry.source_id, Some(1));
    assert!(core.raw_artifacts["sources"]["Inline.sol"]
      .get("ast")
      .is_some());
    let snapshot = core.to_json();
    let source_snapshot = snapshot
      .artifacts
      .as_ref()
      .expect("artifacts snapshot")
      .get("Inline.sol")
      .expect("inline snapshot");
    assert!(source_snapshot.ast.is_some());
    let raw_snapshot = snapshot.raw_artifacts.expect("raw snapshot");
    assert!(raw_snapshot["sources"]["Inline.sol"].get("ast").is_some());
  }

  #[test]
  fn compiler_error_maps_severity_labels() {
    let json = r#"{
      "contracts": {},
      "errors": [
        {
          "component": "general",
          "formattedMessage": "Warning: detail",
          "message": "detail",
          "severity": "warning",
          "type": "Warning",
          "errorCode": "256"
        }
      ],
      "sources": {},
      "version": "0.8.24"
    }"#;

    let output: StandardCompilerOutput = serde_json::from_str(json).expect("compiler output");
    let core = from_standard_json(output);
    assert_eq!(core.errors.len(), 1);
    let error = &core.errors[0];
    assert_eq!(error.severity, SeverityLevel::Warning);
    assert_eq!(error.error_code, Some(256));
  }

  #[test]
  fn into_js_compile_output_preserves_contracts_and_errors() {
    let mut core = CompileOutput {
      raw_artifacts: json!({ "contracts": {} }),
      artifacts: BTreeMap::new(),
      artifact: None,
      errors: vec![CompilerError {
        message: "detail".into(),
        formatted_message: None,
        component: "general".into(),
        severity: SeverityLevel::Error,
        error_type: "TypeError".into(),
        error_code: Some(1),
        source_location: Some(SourceLocation {
          file: "Test.sol".into(),
          start: 0,
          end: 4,
        }),
        secondary_source_locations: Some(vec![SecondarySourceLocation {
          file: Some("Dep.sol".into()),
          start: Some(2),
          end: Some(6),
          message: Some("secondary".into()),
        }]),
        vyper_source_location: None,
      }],
    };

    let mut artifacts = SourceArtifacts::default();
    let mut contract = Contract::new("Widget");
    contract.with_address(Some("0xabc".into()));
    artifacts.contracts.insert("Widget".into(), contract);
    core.artifacts.insert("Widget.sol".into(), artifacts);

    let js_output = into_js_compile_output(core);
    assert!(js_output
      .artifacts
      .get("Widget.sol")
      .and_then(|entry| entry.contracts.get("Widget"))
      .is_some());
    let snapshot = js_output.to_json();
    assert!(snapshot
      .artifacts
      .as_ref()
      .and_then(|entries| entries.get("Widget.sol"))
      .and_then(|entry| entry.contracts.as_ref())
      .and_then(|contracts| contracts.get("Widget"))
      .is_some());
    assert!(snapshot.raw_artifacts.is_some());
    assert!(js_output.has_compiler_errors());
    assert_eq!(js_output.errors.len(), 1);
    assert_eq!(js_output.errors[0].severity, SeverityLevel::Error);
    assert_eq!(
      js_output.errors[0]
        .source_location
        .as_ref()
        .map(|loc| loc.file.as_str()),
      Some("Test.sol")
    );
  }
}
