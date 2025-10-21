mod core;

use crate::ast::utils::from_js_value;
use crate::internal::errors::napi_error;
use core::{
  ewasm_to_js, from_configurable_artifact, from_foundry_project_artifact,
  from_foundry_standard_json, function_debug_data_to_js, gas_estimates_to_js,
  immutable_references_to_js, method_identifiers_to_js, new_state,
};
use foundry_compilers::artifacts::ConfigurableContractArtifact;
use foundry_compilers::Artifact;
use napi::bindgen_prelude::*;
use napi::{JsUnknown, ValueType};
use serde_json::Value;
use std::collections::HashMap;

pub use core::{
  ContractBytecode, ContractState, ImmutableSlot, JsEwasm, JsFunctionDebugDataEntry, JsGasEstimates,
};

// -----------------------------------------------------------------------------
// Rust-facing contract wrapper
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Contract {
  state: ContractState,
}

impl Contract {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      state: new_state(name),
    }
  }

  pub fn from_foundry_standard_json(
    name: impl Into<String>,
    contract: &foundry_compilers::artifacts::contract::Contract,
  ) -> Self {
    Self {
      state: from_foundry_standard_json(name, contract),
    }
  }

  pub fn from_configurable_artifact(
    name: impl Into<String>,
    artifact: &ConfigurableContractArtifact,
  ) -> Self {
    Self {
      state: from_configurable_artifact(name, artifact),
    }
  }

  pub fn from_foundry_project_artifact(name: impl Into<String>, artifact: &impl Artifact) -> Self {
    Self {
      state: from_foundry_project_artifact(name, artifact),
    }
  }

  pub fn state(&self) -> &ContractState {
    &self.state
  }

  pub fn state_mut(&mut self) -> &mut ContractState {
    &mut self.state
  }

  pub fn into_state(self) -> ContractState {
    self.state
  }

  pub fn name(&self) -> &str {
    &self.state.name
  }

  pub fn address(&self) -> Option<&str> {
    self.state.address.as_deref()
  }

  pub fn creation_bytecode(&self) -> Option<&ContractBytecode> {
    self.state.creation_bytecode.as_ref()
  }

  pub fn deployed_bytecode(&self) -> Option<&ContractBytecode> {
    self.state.deployed_bytecode.as_ref()
  }

  pub fn with_address(&mut self, address: Option<String>) {
    self.state.address = address;
  }

  pub fn with_creation_bytecode(&mut self, bytecode: Option<ContractBytecode>) {
    self.state.creation_bytecode = bytecode;
  }

  pub fn with_deployed_bytecode(&mut self, bytecode: Option<ContractBytecode>) {
    self.state.deployed_bytecode = bytecode;
  }
}

impl From<Contract> for ContractState {
  fn from(contract: Contract) -> Self {
    contract.state
  }
}

impl From<ContractState> for Contract {
  fn from(state: ContractState) -> Self {
    Self { state }
  }
}

// -----------------------------------------------------------------------------
// JavaScript-facing snapshots
// -----------------------------------------------------------------------------

#[napi(object, js_name = "ContractBytecode")]
#[derive(Clone, Debug)]
pub struct JsContractBytecode {
  #[napi(ts_type = "`0x${string}` | null | undefined")]
  pub hex: Option<String>,
  #[napi(ts_type = "Uint8Array | null | undefined")]
  pub bytes: Option<Vec<u8>>,
}

impl From<&ContractBytecode> for JsContractBytecode {
  fn from(bytecode: &ContractBytecode) -> Self {
    Self {
      hex: Some(bytecode.to_hex()),
      bytes: Some(bytecode.bytes().to_vec()),
    }
  }
}

#[napi(object, js_name = "ContractState")]
#[derive(Clone, Debug)]
pub struct JsContractState {
  pub name: String,
  #[napi(ts_type = "`0x${string}` | null | undefined")]
  pub address: Option<String>,
  #[napi(ts_type = "unknown | null | undefined")]
  pub abi: Option<Value>,
  pub source_path: Option<String>,
  pub source_id: Option<u32>,
  #[napi(ts_type = "ContractBytecode | null | undefined")]
  pub creation_bytecode: Option<JsContractBytecode>,
  #[napi(ts_type = "ContractBytecode | null | undefined")]
  pub deployed_bytecode: Option<JsContractBytecode>,
  #[napi(ts_type = "string | Record<string, unknown> | null | undefined")]
  pub metadata: Option<Value>,
  #[napi(ts_type = "Record<string, unknown> | null | undefined")]
  pub userdoc: Option<Value>,
  #[napi(ts_type = "Record<string, unknown> | null | undefined")]
  pub devdoc: Option<Value>,
  #[napi(ts_type = "import('./solc-storage-layout').StorageLayout | null | undefined")]
  pub storage_layout: Option<Value>,
  #[napi(ts_type = "Record<string, { start: number; length: number }[]> | null | undefined")]
  pub immutable_references: Option<HashMap<String, Vec<ImmutableSlot>>>,
  #[napi(ts_type = "Record<string, `0x${string}`> | null | undefined")]
  pub method_identifiers: Option<HashMap<String, String>>,
  #[napi(ts_type = "Record<string, FunctionDebugDataEntry> | null | undefined")]
  pub function_debug_data: Option<HashMap<String, JsFunctionDebugDataEntry>>,
  #[napi(ts_type = "GasEstimates | null | undefined")]
  pub gas_estimates: Option<JsGasEstimates>,
  pub assembly: Option<String>,
  #[napi(ts_type = "Record<string, unknown> | null | undefined")]
  pub legacy_assembly: Option<Value>,
  pub opcodes: Option<String>,
  pub ir: Option<String>,
  pub ir_optimized: Option<String>,
  #[napi(ts_type = "EwasmOutput | null | undefined")]
  pub ewasm: Option<JsEwasm>,
  #[napi(ts_type = "string | null | undefined")]
  pub creation_source_map: Option<String>,
}

// -----------------------------------------------------------------------------
// Conversions between Rust and JS representations
// -----------------------------------------------------------------------------

pub fn contract_class(contract: &Contract) -> JsContract {
  JsContract::from_contract(contract.clone())
}

pub fn contract_state_to_js(state: &ContractState) -> JsContractState {
  JsContractState {
    name: state.name.clone(),
    address: state.address.clone(),
    abi: state.abi.clone(),
    source_path: state.source_path.clone(),
    source_id: state.source_id,
    creation_bytecode: state
      .creation_bytecode
      .as_ref()
      .map(JsContractBytecode::from),
    deployed_bytecode: state
      .deployed_bytecode
      .as_ref()
      .map(JsContractBytecode::from),
    metadata: state.metadata.clone(),
    userdoc: state.userdoc.clone(),
    devdoc: state.devdoc.clone(),
    storage_layout: state.storage_layout.clone(),
    immutable_references: immutable_references_to_js(state),
    method_identifiers: method_identifiers_to_js(state),
    function_debug_data: function_debug_data_to_js(state),
    gas_estimates: gas_estimates_to_js(state),
    assembly: state.assembly.clone(),
    legacy_assembly: state.legacy_assembly.clone(),
    opcodes: state.opcodes.clone(),
    ir: state.ir.clone(),
    ir_optimized: state.ir_optimized.clone(),
    ewasm: ewasm_to_js(state),
    creation_source_map: state.creation_source_map.clone(),
  }
}

// -----------------------------------------------------------------------------
// JSON helpers
// -----------------------------------------------------------------------------

fn contract_state_from_json_value(value: &Value) -> napi::Result<ContractState> {
  let obj = value
    .as_object()
    .ok_or_else(|| napi_error("Contract state must be an object".to_string()))?;

  let name = obj
    .get("name")
    .and_then(Value::as_str)
    .ok_or_else(|| napi_error("Contract state requires a name".to_string()))?
    .to_string();

  let mut state = ContractState::new(name);
  state.address = obj.get("address").and_then(value_to_string);
  state.abi = clone_non_null(obj.get("abi"));
  state.source_path = obj.get("sourcePath").and_then(value_to_string);
  state.source_id = obj
    .get("sourceId")
    .and_then(Value::as_u64)
    .map(|value| value as u32);
  state.creation_bytecode = json_to_bytecode(obj.get("creationBytecode"))?;
  state.deployed_bytecode = json_to_bytecode(obj.get("deployedBytecode"))?;
  state.metadata = clone_non_null(obj.get("metadata"));
  state.userdoc = clone_non_null(obj.get("userdoc"));
  state.devdoc = clone_non_null(obj.get("devdoc"));
  state.storage_layout = clone_non_null(obj.get("storageLayout"));
  state.immutable_references = obj
    .get("immutableReferences")
    .and_then(|value| serde_json::from_value(value.clone()).ok());
  state.method_identifiers = obj
    .get("methodIdentifiers")
    .and_then(|value| serde_json::from_value(value.clone()).ok());
  state.function_debug_data = obj
    .get("functionDebugData")
    .and_then(|value| serde_json::from_value(value.clone()).ok());
  state.gas_estimates = obj
    .get("gasEstimates")
    .and_then(|value| serde_json::from_value(value.clone()).ok());
  state.assembly = obj.get("assembly").and_then(value_to_string);
  state.legacy_assembly = clone_non_null(obj.get("legacyAssembly"));
  state.opcodes = obj.get("opcodes").and_then(value_to_string);
  state.ir = obj.get("ir").and_then(value_to_string);
  state.ir_optimized = obj.get("irOptimized").and_then(value_to_string);
  state.ewasm = obj
    .get("ewasm")
    .and_then(|value| serde_json::from_value(value.clone()).ok());
  state.creation_source_map = obj.get("creationSourceMap").and_then(value_to_string);

  Ok(state)
}

fn value_to_string(value: &Value) -> Option<String> {
  value.as_str().map(|s| s.to_string())
}

fn clone_non_null(value: Option<&Value>) -> Option<Value> {
  value.and_then(|val| {
    if val.is_null() {
      None
    } else {
      Some(val.clone())
    }
  })
}

fn json_to_bytecode(value: Option<&Value>) -> napi::Result<Option<ContractBytecode>> {
  let Some(value) = value else {
    return Ok(None);
  };

  if value.is_null() {
    return Ok(None);
  }

  if let Some(string) = value.as_str() {
    return hex_to_bytecode(string);
  }

  let obj = value
    .as_object()
    .ok_or_else(|| napi_error("Bytecode must be a string or object".to_string()))?;

  if let Some(hex) = obj.get("hex").and_then(Value::as_str) {
    return hex_to_bytecode(hex);
  }

  if let Some(bytes) = obj.get("bytes") {
    if let Some(array) = bytes.as_array() {
      let mut buffer = Vec::with_capacity(array.len());
      for entry in array {
        let value = entry
          .as_u64()
          .ok_or_else(|| napi_error("Bytecode bytes must be numbers".to_string()))?;
        if value > 0xff {
          return Err(napi_error("Bytecode bytes must be < 256".to_string()));
        }
        buffer.push(value as u8);
      }
      return Ok(Some(ContractBytecode::from_bytes(buffer)));
    }
  }

  if obj.get("type").and_then(Value::as_str) == Some("Buffer") {
    if let Some(data) = obj.get("data").and_then(Value::as_array) {
      let mut buffer = Vec::with_capacity(data.len());
      for entry in data {
        let value = entry
          .as_u64()
          .ok_or_else(|| napi_error("Buffer data must be numbers".to_string()))?;
        if value > 0xff {
          return Err(napi_error("Buffer data must be < 256".to_string()));
        }
        buffer.push(value as u8);
      }
      return Ok(Some(ContractBytecode::from_bytes(buffer)));
    }
  }

  Err(napi_error("Invalid bytecode format".to_string()))
}

fn hex_to_bytecode(hex: &str) -> napi::Result<Option<ContractBytecode>> {
  let trimmed = hex.strip_prefix("0x").unwrap_or(hex);
  let bytes = hex::decode(trimmed)
    .map_err(|err| napi_error(format!("Invalid hex-encoded bytecode: {err}")))?;
  Ok(Some(ContractBytecode::from_bytes(bytes)))
}

// -----------------------------------------------------------------------------
// JsContract wrapper (exposed via N-API)
// -----------------------------------------------------------------------------

#[napi(js_name = "Contract")]
#[derive(Clone)]
pub struct JsContract {
  inner: Contract,
}

impl JsContract {
  pub(crate) fn from_contract(contract: Contract) -> Self {
    Self { inner: contract }
  }

  pub(crate) fn from_state(state: ContractState) -> Self {
    Self::from_contract(Contract::from(state))
  }

  fn into_json(&self) -> JsContractState {
    contract_state_to_js(self.inner.state())
  }
}

#[napi]
impl JsContract {
  #[napi(constructor, ts_args_type = "state: ContractState")]
  pub fn new(env: Env, state: JsUnknown) -> napi::Result<Self> {
    let value: Value = from_js_value(&env, state)?;
    let state = contract_state_from_json_value(&value)?;
    Ok(Self::from_state(state))
  }

  #[napi(factory, ts_args_type = "name: string, contract: object | string")]
  pub fn from_solc_contract_output(
    env: Env,
    name: String,
    contract: JsUnknown,
  ) -> napi::Result<Self> {
    let value = match contract.get_type()? {
      ValueType::String => {
        let text = contract.coerce_to_string()?.into_utf8()?.into_owned()?;
        serde_json::from_str::<Value>(&text)
          .map_err(|err| napi_error(format!("Failed to parse contract JSON: {err}")))?
      }
      _ => from_js_value(&env, contract)?,
    };

    let parsed: foundry_compilers::artifacts::contract::Contract = serde_json::from_value(value)
      .map_err(|err| napi_error(format!("Failed to parse contract JSON: {err}")))?;

    Ok(Self::from_contract(Contract::from_foundry_standard_json(
      name, &parsed,
    )))
  }

  #[napi(getter)]
  pub fn name(&self) -> String {
    self.inner.name().to_string()
  }

  #[napi(getter, ts_return_type = "`0x${string}` | null | undefined")]
  pub fn address(&self) -> Option<String> {
    self.inner.address().map(|value| value.to_string())
  }

  #[napi(getter)]
  pub fn creation_bytecode(&self) -> Option<JsContractBytecode> {
    self.inner.creation_bytecode().map(JsContractBytecode::from)
  }

  #[napi(getter)]
  pub fn deployed_bytecode(&self) -> Option<JsContractBytecode> {
    self.inner.deployed_bytecode().map(JsContractBytecode::from)
  }

  #[napi(getter, ts_return_type = "ContractState['abi']")]
  pub fn abi(&self) -> Option<Value> {
    self.inner.state().abi.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['metadata']")]
  pub fn metadata(&self) -> Option<Value> {
    self.inner.state().metadata.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['userdoc']")]
  pub fn userdoc(&self) -> Option<Value> {
    self.inner.state().userdoc.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['devdoc']")]
  pub fn devdoc(&self) -> Option<Value> {
    self.inner.state().devdoc.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['storageLayout']")]
  pub fn storage_layout(&self) -> Option<Value> {
    self.inner.state().storage_layout.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['immutableReferences']")]
  pub fn immutable_references(&self) -> Option<HashMap<String, Vec<ImmutableSlot>>> {
    immutable_references_to_js(self.inner.state())
  }

  #[napi(getter, ts_return_type = "ContractState['methodIdentifiers']")]
  pub fn method_identifiers(&self) -> Option<HashMap<String, String>> {
    method_identifiers_to_js(self.inner.state())
  }

  #[napi(getter, ts_return_type = "ContractState['functionDebugData']")]
  pub fn function_debug_data(&self) -> Option<HashMap<String, JsFunctionDebugDataEntry>> {
    function_debug_data_to_js(self.inner.state())
  }

  #[napi(getter, ts_return_type = "ContractState['gasEstimates']")]
  pub fn gas_estimates(&self) -> Option<JsGasEstimates> {
    gas_estimates_to_js(self.inner.state())
  }

  #[napi(getter, ts_return_type = "string | null")]
  pub fn assembly(&self) -> Option<String> {
    self.inner.state().assembly.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['legacyAssembly']")]
  pub fn legacy_assembly(&self) -> Option<Value> {
    self.inner.state().legacy_assembly.clone()
  }

  #[napi(getter, ts_return_type = "string | null")]
  pub fn opcodes(&self) -> Option<String> {
    self.inner.state().opcodes.clone()
  }

  #[napi(getter, ts_return_type = "string | null")]
  pub fn ir(&self) -> Option<String> {
    self.inner.state().ir.clone()
  }

  #[napi(getter, ts_return_type = "string | null")]
  pub fn ir_optimized(&self) -> Option<String> {
    self.inner.state().ir_optimized.clone()
  }

  #[napi(getter, ts_return_type = "ContractState['ewasm']")]
  pub fn ewasm(&self) -> Option<JsEwasm> {
    ewasm_to_js(self.inner.state())
  }

  #[napi(getter, ts_return_type = "string | null")]
  pub fn creation_source_map(&self) -> Option<String> {
    self.inner.state().creation_source_map.clone()
  }

  #[napi]
  pub fn with_address(&mut self, address: Option<String>) -> napi::Result<Self> {
    self.inner.with_address(address);
    Ok(self.clone())
  }

  #[napi]
  pub fn with_creation_bytecode(&mut self, bytecode: Option<Buffer>) -> napi::Result<Self> {
    self
      .inner
      .with_creation_bytecode(bytecode.map(|buffer| ContractBytecode::from_bytes(buffer.to_vec())));
    Ok(self.clone())
  }

  #[napi]
  pub fn with_deployed_bytecode(&mut self, bytecode: Option<Buffer>) -> napi::Result<Self> {
    self
      .inner
      .with_deployed_bytecode(bytecode.map(|buffer| ContractBytecode::from_bytes(buffer.to_vec())));
    Ok(self.clone())
  }

  #[napi]
  pub fn to_json(&self) -> JsContractState {
    self.into_json()
  }
}
