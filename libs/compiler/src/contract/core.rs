use std::collections::{BTreeMap, HashMap};

use foundry_compilers::artifacts::{
  bytecode::{
    Bytecode, BytecodeObject, CompactBytecode, CompactDeployedBytecode, DeployedBytecode,
  },
  contract::Contract as FoundryContract,
  ConfigurableContractArtifact, Creation, Ewasm, FunctionDebugData, GasEstimates,
};
use foundry_compilers::Artifact;
use hex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractBytecode {
  bytes: Vec<u8>,
}

impl ContractBytecode {
  pub fn from_bytes<T: Into<Vec<u8>>>(bytes: T) -> Self {
    Self {
      bytes: bytes.into(),
    }
  }

  pub fn from_hex_string(hex_string: &str) -> Result<Self, hex::FromHexError> {
    let trimmed = hex_string.strip_prefix("0x").unwrap_or(hex_string);
    let bytes = hex::decode(trimmed)?;
    Ok(Self::from_bytes(bytes))
  }

  pub fn from_bytecode(bytecode: &Bytecode) -> Option<Self> {
    Self::from_bytecode_object(&bytecode.object)
  }

  pub fn from_compact_bytecode(bytecode: &CompactBytecode) -> Option<Self> {
    Self::from_bytecode_object(&bytecode.object)
  }

  pub fn from_deployed_bytecode(bytecode: &DeployedBytecode) -> Option<Self> {
    bytecode.bytecode.as_ref().and_then(Self::from_bytecode)
  }

  pub fn from_compact_deployed_bytecode(bytecode: &CompactDeployedBytecode) -> Option<Self> {
    bytecode
      .bytecode
      .as_ref()
      .and_then(Self::from_compact_bytecode)
  }

  pub fn from_bytecode_object(object: &BytecodeObject) -> Option<Self> {
    object
      .as_bytes()
      .map(|bytes| Self::from_bytes(bytes.as_ref()))
  }

  pub fn bytes(&self) -> &[u8] {
    &self.bytes
  }

  pub fn into_bytes(self) -> Vec<u8> {
    self.bytes
  }

  pub fn is_empty(&self) -> bool {
    self.bytes.is_empty()
  }

  pub fn len(&self) -> usize {
    self.bytes.len()
  }

  pub fn to_hex(&self) -> String {
    format!("0x{}", hex::encode(&self.bytes))
  }
}

impl AsRef<[u8]> for ContractBytecode {
  fn as_ref(&self) -> &[u8] {
    self.bytes()
  }
}

impl From<Vec<u8>> for ContractBytecode {
  fn from(value: Vec<u8>) -> Self {
    Self { bytes: value }
  }
}

/// Immutable storage slot metadata emitted by Solc.
#[napi(object)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableSlot {
  /// Zero-based byte offset (within the deployed bytecode) where the immutable value begins.
  pub start: u32,
  /// Byte length occupied by the immutable value.
  pub length: u32,
}

#[napi(object, js_name = "FunctionDebugDataEntry")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsFunctionDebugDataEntry {
  /// Program counter offset for the function entry, when emitted.
  #[napi(ts_type = "number | null | undefined")]
  pub entry_point: Option<u32>,
  /// Stable identifier assigned by Solc.
  #[napi(ts_type = "number | null | undefined")]
  pub id: Option<u32>,
  /// Number of stack slots reserved for parameters.
  #[napi(ts_type = "number | null | undefined")]
  pub parameter_slots: Option<u32>,
  /// Number of stack slots reserved for return values.
  #[napi(ts_type = "number | null | undefined")]
  pub return_slots: Option<u32>,
}

impl From<&FunctionDebugData> for JsFunctionDebugDataEntry {
  fn from(data: &FunctionDebugData) -> Self {
    Self {
      entry_point: data.entry_point,
      id: data.id,
      parameter_slots: data.parameter_slots,
      return_slots: data.return_slots,
    }
  }
}

#[napi(object, js_name = "GasEstimatesCreation")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsGasEstimatesCreation {
  /// Estimated gas required to deposit contract code on-chain (stringified decimal).
  pub code_deposit_cost: String,
  /// Estimated execution cost for the deployment transaction (stringified decimal).
  pub execution_cost: String,
  /// Sum of deposit and execution costs (stringified decimal).
  pub total_cost: String,
}

impl From<&Creation> for JsGasEstimatesCreation {
  fn from(creation: &Creation) -> Self {
    Self {
      code_deposit_cost: creation.code_deposit_cost.clone(),
      execution_cost: creation.execution_cost.clone(),
      total_cost: creation.total_cost.clone(),
    }
  }
}

#[napi(object, js_name = "GasEstimates")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsGasEstimates {
  /// Gas estimates related to contract deployment.
  pub creation: JsGasEstimatesCreation,
  /// Gas estimates for external/public functions keyed by signature (stringified decimals).
  pub external: HashMap<String, String>,
  /// Gas estimates for internal functions keyed by signature (stringified decimals).
  pub internal: HashMap<String, String>,
}

impl From<&GasEstimates> for JsGasEstimates {
  fn from(estimates: &GasEstimates) -> Self {
    Self {
      creation: JsGasEstimatesCreation::from(&estimates.creation),
      external: estimates.external.clone().into_iter().collect(),
      internal: estimates.internal.clone().into_iter().collect(),
    }
  }
}

#[napi(object, js_name = "EwasmOutput")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsEwasm {
  /// Optional textual WAST representation emitted by Solc.
  #[napi(ts_type = "string | null | undefined")]
  pub wast: Option<String>,
  /// Base64-encoded WASM module (the `ewasm.wasm` field from Solc's standard JSON output).
  pub wasm: String,
}

impl From<&Ewasm> for JsEwasm {
  fn from(ewasm: &Ewasm) -> Self {
    Self {
      wast: ewasm.wast.clone(),
      wasm: ewasm.wasm.clone(),
    }
  }
}

/// Complete contract snapshot shared between Rust and JavaScript callers. Mirrors the artefact
/// shape exported from Foundry's standard JSON output.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractState {
  /// Name of the contract as reported by the compiler.
  pub name: String,
  /// Deployed address associated with this artifact, if tracked.
  pub address: Option<String>,
  /// ABI description emitted by the compiler (array or object form depending on compiler version).
  pub abi: Option<Value>,
  /// Creation bytecode wrapper when available (`bytecode.object`).
  pub creation_bytecode: Option<ContractBytecode>,
  /// Deployed bytecode wrapper when available (`deployedBytecode.object`).
  pub deployed_bytecode: Option<ContractBytecode>,
  /// Source file path that emitted the contract, relative to the project root when available.
  pub source_path: Option<String>,
  /// Numeric source identifier assigned by solc.
  pub source_id: Option<u32>,
  /// Compiler metadata payload (string or JSON value depending on version).
  pub metadata: Option<Value>,
  /// User documentation section (`userdoc`).
  pub userdoc: Option<Value>,
  /// Developer documentation section (`devdoc`).
  pub devdoc: Option<Value>,
  /// Storage layout information when requested from the compiler.
  pub storage_layout: Option<Value>,
  /// Offsets for immutable variables keyed by label (`immutableReferences`).
  pub immutable_references: Option<BTreeMap<String, Vec<ImmutableSlot>>>,
  /// Map of function signatures to selectors (`methodIdentifiers`).
  pub method_identifiers: Option<BTreeMap<String, String>>,
  /// Function debug metadata keyed by signature.
  pub function_debug_data: Option<BTreeMap<String, FunctionDebugData>>,
  /// Gas estimates for deployment and function execution.
  pub gas_estimates: Option<GasEstimates>,
  /// Assembly listing when requested (`evm.assembly` string).
  pub assembly: Option<String>,
  /// Legacy assembly format when emitted by older compiler modes (`evm.legacyAssembly`).
  pub legacy_assembly: Option<Value>,
  /// Opcode listing when emitted (`evm.bytecode.opcodes`).
  pub opcodes: Option<String>,
  /// Intermediate representation output (IR) when requested (`ir` section from solc).
  pub ir: Option<String>,
  /// Optimised intermediate representation output when requested (`irOptimized`).
  pub ir_optimized: Option<String>,
  /// Ewasm output payload when generated.
  pub ewasm: Option<Ewasm>,
  pub creation_source_map: Option<String>,
}

impl ContractState {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      ..Default::default()
    }
  }
}

/// Internal builder used to assemble [`ContractState`] values.
struct ContractBuilder {
  state: ContractState,
}

impl ContractBuilder {
  fn new(name: impl Into<String>) -> Self {
    Self {
      state: ContractState::new(name),
    }
  }

  fn finish(self) -> ContractState {
    self.state
  }

  fn set_abi(mut self, abi: Option<Value>) -> Self {
    self.state.abi = abi;
    self
  }

  fn set_creation_bytecode(mut self, bytecode: Option<ContractBytecode>) -> Self {
    self.state.creation_bytecode = bytecode;
    self
  }

  fn set_deployed_bytecode(mut self, bytecode: Option<ContractBytecode>) -> Self {
    self.state.deployed_bytecode = bytecode;
    self
  }

  fn set_creation_source_map(mut self, map: Option<String>) -> Self {
    self.state.creation_source_map = map;
    self
  }

  fn set_metadata(mut self, metadata: Option<Value>) -> Self {
    self.state.metadata = metadata;
    self
  }

  fn set_userdoc(mut self, userdoc: Option<Value>) -> Self {
    self.state.userdoc = userdoc;
    self
  }

  fn set_devdoc(mut self, devdoc: Option<Value>) -> Self {
    self.state.devdoc = devdoc;
    self
  }

  fn set_storage_layout(mut self, layout: Option<Value>) -> Self {
    self.state.storage_layout = layout;
    self
  }

  fn set_immutable_references(
    mut self,
    value: Option<BTreeMap<String, Vec<ImmutableSlot>>>,
  ) -> Self {
    self.state.immutable_references = value;
    self
  }

  fn set_method_identifiers(mut self, value: Option<BTreeMap<String, String>>) -> Self {
    self.state.method_identifiers = value;
    self
  }

  fn set_function_debug_data(mut self, value: Option<BTreeMap<String, FunctionDebugData>>) -> Self {
    self.state.function_debug_data = value;
    self
  }

  fn set_gas_estimates(mut self, value: Option<GasEstimates>) -> Self {
    self.state.gas_estimates = value;
    self
  }

  fn set_assembly(mut self, value: Option<String>) -> Self {
    self.state.assembly = value;
    self
  }

  fn set_legacy_assembly(mut self, value: Option<Value>) -> Self {
    self.state.legacy_assembly = value;
    self
  }

  fn set_opcodes(mut self, value: Option<String>) -> Self {
    self.state.opcodes = value;
    self
  }

  fn set_ir(mut self, value: Option<String>) -> Self {
    self.state.ir = value;
    self
  }

  fn set_ir_optimized(mut self, value: Option<String>) -> Self {
    self.state.ir_optimized = value;
    self
  }

  fn set_ewasm(mut self, value: Option<Ewasm>) -> Self {
    self.state.ewasm = value;
    self
  }
  fn set_source_id(mut self, value: Option<u32>) -> Self {
    self.state.source_id = value;
    self
  }
}

pub fn new_state(name: impl Into<String>) -> ContractState {
  ContractState::new(name)
}

pub fn immutable_references_to_js(
  state: &ContractState,
) -> Option<HashMap<String, Vec<ImmutableSlot>>> {
  state.immutable_references.as_ref().map(|map| {
    map
      .iter()
      .map(|(key, slots)| (key.clone(), slots.clone()))
      .collect()
  })
}

pub fn method_identifiers_to_js(state: &ContractState) -> Option<HashMap<String, String>> {
  state.method_identifiers.as_ref().map(|map| {
    map
      .iter()
      .map(|(key, value)| (key.clone(), value.clone()))
      .collect()
  })
}

pub fn function_debug_data_to_js(
  state: &ContractState,
) -> Option<HashMap<String, JsFunctionDebugDataEntry>> {
  state.function_debug_data.as_ref().map(|map| {
    map
      .iter()
      .map(|(name, debug)| (name.clone(), JsFunctionDebugDataEntry::from(debug)))
      .collect()
  })
}

pub fn gas_estimates_to_js(state: &ContractState) -> Option<JsGasEstimates> {
  state.gas_estimates.as_ref().map(JsGasEstimates::from)
}

pub fn ewasm_to_js(state: &ContractState) -> Option<JsEwasm> {
  state.ewasm.as_ref().map(JsEwasm::from)
}

pub fn from_foundry_standard_json(
  name: impl Into<String>,
  contract: &FoundryContract,
) -> ContractState {
  build_from_standard_json(&name.into(), contract)
}

pub fn from_configurable_artifact(
  name: impl Into<String>,
  artifact: &ConfigurableContractArtifact,
) -> ContractState {
  build_from_configurable_artifact(&name.into(), artifact)
}

pub fn from_foundry_project_artifact(
  name: impl Into<String>,
  artifact: &impl Artifact,
) -> ContractState {
  build_from_project_artifact(&name.into(), artifact)
}

fn build_from_project_artifact(name: &str, artifact: &impl Artifact) -> ContractState {
  let mut builder = ContractBuilder::new(name.to_string());
  let bytecode_cow = artifact.get_contract_bytecode();

  if let Some(abi) = serialize_optional(&bytecode_cow.abi) {
    builder = builder.set_abi(Some(abi));
  }

  if let Some(source) = bytecode_cow.bytecode.as_ref() {
    if let Some(bytecode) = ContractBytecode::from_compact_bytecode(source.as_ref()) {
      builder = builder.set_creation_bytecode(Some(bytecode));
    }
    if let Some(map) = source.as_ref().source_map.clone() {
      builder = builder.set_creation_source_map(Some(map));
    }
  }

  if let Some(deployed) = bytecode_cow.deployed_bytecode.as_ref() {
    let immutable_refs = deserialize_immutable_refs(&deployed.as_ref().immutable_references);
    let bytecode = ContractBytecode::from_compact_deployed_bytecode(deployed.as_ref());
    builder = builder
      .set_deployed_bytecode(bytecode)
      .set_immutable_references(optional_map(immutable_refs));
  }

  builder.finish()
}

fn build_from_standard_json(name: &str, contract: &FoundryContract) -> ContractState {
  let mut builder = ContractBuilder::new(name.to_string());

  if let Some(abi) = serialize_optional(&contract.abi) {
    builder = builder.set_abi(Some(abi));
  }

  if let Some(evm) = &contract.evm {
    builder = apply_standard_json_evm(
      builder,
      evm.bytecode.as_ref(),
      evm.deployed_bytecode.as_ref(),
    );

    if !evm.method_identifiers.is_empty() {
      builder = builder.set_method_identifiers(Some(evm.method_identifiers.clone()));
    }

    builder = builder
      .set_assembly(evm.assembly.clone())
      .set_legacy_assembly(evm.legacy_assembly.clone())
      .set_gas_estimates(evm.gas_estimates.clone());
  }

  if let Some(metadata) = contract.metadata.as_ref() {
    builder = builder.set_metadata(serialize(metadata));
  }

  builder
    .set_userdoc(serialize(&contract.userdoc))
    .set_devdoc(serialize(&contract.devdoc))
    .set_storage_layout(serialize(&contract.storage_layout))
    .set_ir(contract.ir.clone())
    .set_ir_optimized(contract.ir_optimized.clone())
    .set_ewasm(contract.ewasm.clone())
    .finish()
}

fn build_from_configurable_artifact(
  name: &str,
  artifact: &ConfigurableContractArtifact,
) -> ContractState {
  let mut builder = ContractBuilder::new(name.to_string());

  if let Some(abi) = serialize_optional(&artifact.abi) {
    builder = builder.set_abi(Some(abi));
  }

  builder = apply_compact_evm_artifacts(
    builder,
    artifact.bytecode.as_ref(),
    artifact.deployed_bytecode.as_ref(),
  );

  if let Some(storage_layout) = artifact.storage_layout.as_ref() {
    builder = builder.set_storage_layout(serialize(storage_layout));
  }

  if let Some(userdoc) = artifact.userdoc.as_ref() {
    builder = builder.set_userdoc(serialize(userdoc));
  }

  if let Some(devdoc) = artifact.devdoc.as_ref() {
    builder = builder.set_devdoc(serialize(devdoc));
  }

  if let Some(metadata) = artifact.metadata.as_ref() {
    builder = builder.set_metadata(serialize(metadata));
  } else if let Some(raw) = artifact.raw_metadata.as_ref() {
    builder = builder.set_metadata(Some(Value::String(raw.clone())));
  }

  builder = builder
    .set_function_debug_data(artifact.function_debug_data.clone())
    .set_gas_estimates(artifact.gas_estimates.clone())
    .set_assembly(artifact.assembly.clone())
    .set_legacy_assembly(artifact.legacy_assembly.clone())
    .set_opcodes(artifact.opcodes.clone())
    .set_method_identifiers(artifact.method_identifiers.clone())
    .set_ir(artifact.ir.clone())
    .set_ir_optimized(artifact.ir_optimized.clone())
    .set_ewasm(artifact.ewasm.clone());

  if let Some(id) = artifact.id {
    builder = builder.set_source_id(Some(id));
  }

  builder.finish()
}

fn apply_standard_json_evm(
  mut builder: ContractBuilder,
  bytecode: Option<&Bytecode>,
  deployed: Option<&DeployedBytecode>,
) -> ContractBuilder {
  if let Some(bytecode) = bytecode {
    builder = builder.set_creation_bytecode(ContractBytecode::from_bytecode(bytecode));
    if let Some(map) = &bytecode.source_map {
      builder = builder.set_creation_source_map(Some(map.clone()));
    }
  }

  if let Some(deployed) = deployed {
    let bytecode = ContractBytecode::from_deployed_bytecode(deployed);
    let immutable_refs = deserialize_immutable_refs(&deployed.immutable_references);
    builder = builder
      .set_deployed_bytecode(bytecode)
      .set_immutable_references(optional_map(immutable_refs));
  }

  builder
}

fn apply_compact_evm_artifacts(
  mut builder: ContractBuilder,
  bytecode: Option<&CompactBytecode>,
  deployed: Option<&CompactDeployedBytecode>,
) -> ContractBuilder {
  if let Some(bytecode) = bytecode {
    builder = builder.set_creation_bytecode(ContractBytecode::from_compact_bytecode(bytecode));
    if let Some(map) = &bytecode.source_map {
      builder = builder.set_creation_source_map(Some(map.clone()));
    }
  }

  if let Some(deployed) = deployed {
    let bytecode = ContractBytecode::from_compact_deployed_bytecode(deployed);
    let immutable_refs = deserialize_immutable_refs(&deployed.immutable_references);
    builder = builder
      .set_deployed_bytecode(bytecode)
      .set_immutable_references(optional_map(immutable_refs));
  }

  builder
}

fn serialize<T: Serialize>(value: &T) -> Option<Value> {
  serde_json::to_value(value).ok()
}

fn serialize_optional<T: Serialize>(value: &Option<T>) -> Option<Value> {
  value.as_ref().and_then(serialize)
}

fn optional_map<K, V>(map: BTreeMap<K, V>) -> Option<BTreeMap<K, V>> {
  if map.is_empty() {
    None
  } else {
    Some(map)
  }
}

pub fn deserialize_immutable_refs<T>(
  source: &BTreeMap<String, Vec<T>>,
) -> BTreeMap<String, Vec<ImmutableSlot>>
where
  T: Serialize,
{
  serde_json::to_value(source)
    .ok()
    .and_then(|value| serde_json::from_value(value).ok())
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn contract_bytecode_from_hex_string_parses() {
    let bytecode = ContractBytecode::from_hex_string("0xDEADBEEF").expect("hex");
    assert_eq!(bytecode.to_hex(), "0xdeadbeef");
    assert_eq!(bytecode.bytes(), &[0xde, 0xad, 0xbe, 0xef]);
  }

  #[test]
  fn contract_bytecode_helpers_work() {
    let bytes = vec![0x60, 0x00, 0x60, 0x01];
    let bytecode = ContractBytecode::from_bytes(bytes.clone());
    assert_eq!(bytecode.bytes(), bytes.as_slice());
    assert_eq!(bytecode.to_hex(), "0x60006001");
  }

  #[test]
  fn contract_state_setters_cover_all_fields() {
    let mut state = ContractState::new("Fixture");
    state.address = Some("0xabc".into());
    state.creation_bytecode = Some(ContractBytecode::from_bytes(vec![0xde, 0xad]));
    state.deployed_bytecode = Some(ContractBytecode::from_bytes(vec![0xca, 0xfe]));
    state.metadata = Some(json!({ "metadata": true }));
    state.userdoc = Some(json!({ "notice": "hi" }));
    state.devdoc = Some(json!({ "details": "yo" }));
    state.storage_layout = Some(json!({ "storage": [] }));
    state.immutable_references = Some(BTreeMap::from([(
      "slot".to_string(),
      vec![ImmutableSlot {
        start: 0,
        length: 32,
      }],
    )]));
    state.function_debug_data = Some(BTreeMap::from([(
      "foo()".to_string(),
      serde_json::from_value(json!({
        "entryPoint": 0,
        "id": 0,
        "parameterSlots": 0,
        "returnSlots": 0
      }))
      .expect("debug"),
    )]));
    state.gas_estimates = Some(
      serde_json::from_value(json!({
        "creation": {
          "codeDepositCost": "1",
          "executionCost": "2",
          "totalCost": "3"
        },
        "external": {},
        "internal": {}
      }))
      .expect("gas"),
    );
    state.assembly = Some("assembly".into());
    state.legacy_assembly = Some(json!({ "legacy": true }));
    state.opcodes = Some("PUSH1 0x00".into());
    state.ir = Some("ir".into());
    state.ir_optimized = Some("optimized".into());
    state.ewasm =
      Some(serde_json::from_value(json!({ "wasm": "0x00", "wast": null })).expect("ewasm"));
    state.creation_source_map = Some("0:0:0".into());
    state.source_path = Some("src/Fixture.sol".into());
    state.source_id = Some(7);

    assert_eq!(state.name, "Fixture");
    assert_eq!(state.address.as_deref(), Some("0xabc"));
    assert_eq!(
      state.metadata.as_ref().and_then(|v| v.get("metadata")),
      Some(&json!(true))
    );
    assert!(state.userdoc.is_some());
    assert!(state.devdoc.is_some());
    assert!(state.storage_layout.is_some());
    assert!(state.immutable_references.is_some());
    assert!(state.function_debug_data.is_some());
    assert!(state.gas_estimates.is_some());
    assert_eq!(state.assembly.as_deref(), Some("assembly"));
    assert!(state.legacy_assembly.is_some());
    assert_eq!(state.opcodes.as_deref(), Some("PUSH1 0x00"));
    assert_eq!(state.ir.as_deref(), Some("ir"));
    assert_eq!(state.ir_optimized.as_deref(), Some("optimized"));
    assert!(state.ewasm.is_some());
    assert_eq!(state.creation_source_map.as_deref(), Some("0:0:0"));
    assert_eq!(state.source_path.as_deref(), Some("src/Fixture.sol"));
    assert_eq!(state.source_id, Some(7));
  }

  #[test]
  fn from_standard_json_extracts_core_fields() {
    let json = r#"{
      "abi": [],
      "metadata": null,
      "userdoc": {},
      "devdoc": {},
      "storageLayout": { "storage": [] },
      "evm": {
        "bytecode": {
          "object": "0x6000",
          "linkReferences": {},
          "sourceMap": "00"
        },
        "deployedBytecode": {
          "bytecode": { "object": "0x6001", "linkReferences": {} },
          "immutableReferences": {
            "": [ { "start": 0, "length": 20 } ]
          }
        },
        "methodIdentifiers": {
          "greet()": "0xabcdef01"
        },
        "gasEstimates": {
          "creation": {
            "codeDepositCost": "0",
            "executionCost": "0",
            "totalCost": "0"
          },
          "external": {},
          "internal": {}
        }
      }
    }"#;

    let contract: FoundryContract = serde_json::from_str(json).expect("contract");
    let state = from_foundry_standard_json("Sample", &contract);
    assert_eq!(state.name, "Sample");
    assert!(state.abi.is_some());
    assert!(state
      .creation_bytecode
      .as_ref()
      .is_some_and(|b| !b.is_empty()));
    assert!(state.method_identifiers.is_some());
    assert!(state.immutable_references.is_some());
    assert_eq!(state.creation_source_map.as_deref(), Some("00"));
    assert!(state.gas_estimates.is_some());
    let refs = state.immutable_references.as_ref().expect("immutable refs");
    let slots = refs.get("").expect("slot list");
    assert_eq!(slots[0].start, 0);
    assert_eq!(slots[0].length, 20);
  }

  #[test]
  fn from_configurable_artifact_preserves_metadata() {
    let json = r#"{
      "abi": [],
      "bytecode": {
        "object": "0x6002",
        "linkReferences": {},
        "sourceMap": "11"
      },
      "deployedBytecode": {
        "bytecode": { "object": "0x6003", "linkReferences": {} },
        "immutableReferences": {
          "": [ { "start": 1, "length": 32 } ]
        }
      },
      "rawMetadata": "{\"compiler\":\"solc\"}",
      "storageLayout": { "storage": [] },
      "userdoc": {"notice": "hi"},
      "devdoc": {"details": "dev"},
      "assembly": "code",
      "methodIdentifiers": { "id()": "0x12345678" },
      "functionDebugData": {
        "id()": {
          "entryPoint": 1,
          "id": 1,
          "parameterSlots": 0,
          "returnSlots": 0
        }
      },
      "gasEstimates": {
        "creation": {
          "codeDepositCost": "1",
          "executionCost": "2",
          "totalCost": "3"
        },
        "external": { "id()": "42" },
        "internal": {}
      },
      "ewasm": { "wasm": "0x00", "wast": null },
      "id": 7
    }"#;

    let artifact: ConfigurableContractArtifact = serde_json::from_str(json).expect("artifact");
    let state = from_configurable_artifact("Library", &artifact);
    assert_eq!(state.name, "Library");
    assert_eq!(state.source_id, Some(7));
    assert!(state.metadata.is_some());
    assert!(state.userdoc.is_some());
    assert!(state.devdoc.is_some());
    assert!(state.method_identifiers.is_some());
    assert!(state.immutable_references.is_some());
    assert_eq!(state.creation_source_map.as_deref(), Some("11"));
    assert!(state.function_debug_data.is_some());
    assert!(state.gas_estimates.is_some());
    assert!(state.ewasm.is_some());
    assert!(state
      .creation_bytecode
      .as_ref()
      .is_some_and(|b| !b.is_empty()));
    let refs = state.immutable_references.as_ref().expect("immutable refs");
    let slots = refs.get("").expect("slot list");
    assert_eq!(slots[0].start, 1);
    assert_eq!(slots[0].length, 32);
  }

  #[test]
  fn from_standard_json_without_optional_fields_leaves_defaults() {
    let json = r#"{
      "abi": [],
      "evm": {
        "bytecode": { "object": "0x" },
        "deployedBytecode": {}
      }
    }"#;

    let contract: FoundryContract = serde_json::from_str(json).expect("contract");
    let state = from_foundry_standard_json("Minimal", &contract);

    assert_eq!(state.name, "Minimal");
    assert!(state.creation_bytecode.is_some());
    assert!(state.metadata.is_none());
    assert!(state.method_identifiers.is_none());
    assert!(state.gas_estimates.is_none());
  }

  #[test]
  fn deserialize_immutable_refs_returns_structured_slots() {
    let source = BTreeMap::from([(
      "slot".to_string(),
      vec![serde_json::json!({ "start": 4, "length": 32 })],
    )]);
    let converted = deserialize_immutable_refs(&source);
    let slots = converted.get("slot").expect("slot entries");
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].start, 4);
    assert_eq!(slots[0].length, 32);
  }
}
