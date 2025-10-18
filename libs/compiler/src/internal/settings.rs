use std::collections::BTreeMap;

use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};
use napi::bindgen_prelude::Result;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json;

use crate::internal::errors::map_napi_error;

/// Rust-facing optional overrides that can be merged into Foundry `Settings`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompilerSettingsOptions {
  #[serde(
    rename = "stopAfter",
    alias = "stop_after",
    skip_serializing_if = "Option::is_none"
  )]
  pub stop_after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub remappings: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optimizer: Option<OptimizerSettingsOptions>,
  #[serde(
    rename = "modelChecker",
    alias = "model_checker",
    skip_serializing_if = "Option::is_none"
  )]
  pub model_checker: Option<ModelCheckerSettingsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<SettingsMetadataOptions>,
  #[serde(
    rename = "outputSelection",
    alias = "output_selection",
    skip_serializing_if = "Option::is_none"
  )]
  pub output_selection: Option<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
  #[serde(
    rename = "evmVersion",
    alias = "evm_version",
    skip_serializing_if = "Option::is_none"
  )]
  pub evm_version: Option<EvmVersion>,
  #[serde(
    rename = "viaIR",
    alias = "viaIr",
    skip_serializing_if = "Option::is_none"
  )]
  pub via_ir: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebuggingSettingsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub libraries: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

impl CompilerSettingsOptions {
  pub(crate) fn overlay(self, base: &Settings) -> Result<Settings> {
    let mut base_value = map_napi_error(
      serde_json::to_value(base),
      "Failed to serialise base compiler settings",
    )?;
    let overrides = map_napi_error(
      serde_json::to_value(self),
      "Failed to serialise compiler settings",
    )?;

    merge_settings_json(&mut base_value, overrides);

    map_napi_error(
      serde_json::from_value(base_value),
      "Failed to parse compiler settings",
    )
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerSettingsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub enabled: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub runs: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<OptimizerDetailsOptions>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerDetailsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub peephole: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub inliner: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jumpdest_remover: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub order_literals: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deduplicate: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cse: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub constant_optimizer: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub yul: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub yul_details: Option<YulDetailsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub simple_counter_for_loop_unchecked_increment: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YulDetailsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stack_allocation: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optimizer_steps: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggingSettingsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub revert_strings: Option<RevertStrings>,
  #[serde(
    default,
    skip_serializing_if = "Vec::is_empty",
    deserialize_with = "deserialize_null_default"
  )]
  pub debug_info: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsMetadataOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub use_literal_content: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bytecode_hash: Option<BytecodeHash>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cbor_metadata: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCheckerSettingsOptions {
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  pub contracts: BTreeMap<String, Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub engine: Option<ModelCheckerEngine>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub targets: Option<Vec<ModelCheckerTarget>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub invariants: Option<Vec<ModelCheckerInvariant>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unproved: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub div_mod_with_slacks: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub solvers: Option<Vec<ModelCheckerSolver>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unsupported: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_proved_safe: Option<bool>,
}

/// JavaScript-facing wrappers mirroring the option structs.
#[napi(object, js_name = "CompilerSettings")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsCompilerSettingsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "'parsing' | undefined")]
  pub stop_after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "`${string}=${string}`[] | undefined")]
  pub remappings: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').OptimizerSettings | undefined")]
  pub optimizer: Option<JsOptimizerSettingsOptions>,
  #[serde(rename = "modelChecker", skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').ModelCheckerSettings | undefined")]
  pub model_checker: Option<JsModelCheckerSettingsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').SettingsMetadata | undefined")]
  pub metadata: Option<JsSettingsMetadataOptions>,
  #[serde(rename = "outputSelection", skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./solc-types').OutputSelection | undefined")]
  pub output_selection: Option<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
  #[serde(rename = "evmVersion", skip_serializing_if = "Option::is_none")]
  pub evm_version: Option<EvmVersion>,
  #[serde(rename = "viaIR", skip_serializing_if = "Option::is_none")]
  pub via_ir: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').DebuggingSettings | undefined")]
  pub debug: Option<JsDebuggingSettingsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "Record<string, Record<string, string>> | undefined")]
  pub libraries: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

#[napi(object, js_name = "OptimizerSettings")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsOptimizerSettingsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub enabled: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub runs: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').OptimizerDetails | undefined")]
  pub details: Option<JsOptimizerDetailsOptions>,
}

#[napi(object, js_name = "OptimizerDetails")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsOptimizerDetailsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub peephole: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub inliner: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jumpdest_remover: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub order_literals: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deduplicate: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cse: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub constant_optimizer: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub yul: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./index').YulDetails | undefined")]
  pub yul_details: Option<JsYulDetailsOptions>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub simple_counter_for_loop_unchecked_increment: Option<bool>,
}

#[napi(object, js_name = "YulDetails")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsYulDetailsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stack_allocation: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optimizer_steps: Option<String>,
}

#[napi(object, js_name = "DebuggingSettings")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsDebuggingSettingsOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub revert_strings: Option<RevertStrings>,
  #[serde(
    default,
    skip_serializing_if = "Vec::is_empty",
    deserialize_with = "deserialize_null_default"
  )]
  pub debug_info: Vec<String>,
}

#[napi(object, js_name = "SettingsMetadata")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsSettingsMetadataOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub use_literal_content: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bytecode_hash: Option<BytecodeHash>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cbor_metadata: Option<bool>,
}

#[napi(object, js_name = "ModelCheckerSettings")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsModelCheckerSettingsOptions {
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  #[napi(ts_type = "Record<string, string[]> | undefined")]
  pub contracts: BTreeMap<String, Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub engine: Option<ModelCheckerEngine>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub targets: Option<Vec<ModelCheckerTarget>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub invariants: Option<Vec<ModelCheckerInvariant>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unproved: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub div_mod_with_slacks: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub solvers: Option<Vec<ModelCheckerSolver>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unsupported: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_proved_safe: Option<bool>,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
  D: Deserializer<'de>,
  T: Default + Deserialize<'de>,
{
  Option::<T>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

pub(crate) fn merge_settings_json(base: &mut serde_json::Value, overrides: serde_json::Value) {
  match (base, overrides) {
    (serde_json::Value::Object(base_map), serde_json::Value::Object(overrides_map)) => {
      for (key, value) in overrides_map {
        match base_map.get_mut(&key) {
          Some(existing) => merge_settings_json(existing, value),
          None => {
            base_map.insert(key, value);
          }
        }
      }
    }
    (target, value) => {
      *target = value;
    }
  }
}

pub fn merge_settings(
  base: &Settings,
  overrides: Option<&CompilerSettingsOptions>,
) -> Result<Settings> {
  match overrides {
    Some(settings) => {
      let mut merged = settings.clone().overlay(base)?;
      if let Some(selection) = &settings.output_selection {
        merged.output_selection = selection.clone().into();
      }
      sanitize_settings(&merged)
    }
    None => Ok(base.clone()),
  }
}

pub fn sanitize_settings(settings: &Settings) -> Result<Settings> {
  let mut merged = settings.clone();
  if output_selection_is_effectively_empty(&merged.output_selection) {
    merged.output_selection = Settings::default().output_selection;
  }
  Ok(merged)
}

pub fn output_selection_is_effectively_empty(selection: &OutputSelection) -> bool {
  let map = selection.as_ref();
  if map.is_empty() {
    return true;
  }

  map.values().all(|contracts| {
    contracts
      .values()
      .all(|outputs| outputs.iter().all(|output| output.trim().is_empty()))
  })
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BytecodeHash {
  Ipfs,
  None,
  Bzzr1,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RevertStrings {
  Default,
  Strip,
  Debug,
  VerboseDebug,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerEngine {
  Bmc,
  None,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerTarget {
  Assert,
  Require,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerInvariant {
  Contract,
  Reentrancy,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerSolver {
  Chc,
  Eld,
  Bmc,
  AllZ3,
  Cvc4,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerInvariantKind {
  Reentrancy,
  Contract,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvmVersion {
  Byzantium,
  Constantinople,
  Petersburg,
  Istanbul,
  Berlin,
  London,
  Paris,
  Shanghai,
  Cancun,
  Prague,
}

impl TryFrom<&JsCompilerSettingsOptions> for CompilerSettingsOptions {
  type Error = napi::Error;

  fn try_from(options: &JsCompilerSettingsOptions) -> Result<Self> {
    let json = map_napi_error(
      serde_json::to_value(options),
      "Failed to serialise compiler settings",
    )?;
    map_napi_error(
      serde_json::from_value(json),
      "Failed to convert compiler settings",
    )
  }
}

impl TryFrom<JsCompilerSettingsOptions> for CompilerSettingsOptions {
  type Error = napi::Error;

  fn try_from(options: JsCompilerSettingsOptions) -> Result<Self> {
    CompilerSettingsOptions::try_from(&options)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;
  use std::collections::BTreeMap;

  #[test]
  fn sanitize_restores_default_output_selection() {
    let mut base = Settings::default();
    base.output_selection = OutputSelection::default();
    assert!(output_selection_is_effectively_empty(
      &base.output_selection
    ));

    let sanitised = sanitize_settings(&base).expect("sanitize");
    assert!(
      !output_selection_is_effectively_empty(&sanitised.output_selection),
      "sanitised selection should fall back to defaults"
    );
  }

  #[test]
  fn sanitize_preserves_stop_after_and_ast_selection() {
    let mut settings = Settings::default();
    settings.stop_after = Some("parsing".to_string());
    settings.output_selection = OutputSelection::ast_output_selection();

    assert!(
      !output_selection_is_effectively_empty(&settings.output_selection),
      "ast output selection should be considered non-empty"
    );

    let sanitised = sanitize_settings(&settings).expect("sanitize");
    assert_eq!(
      sanitised.stop_after.as_deref(),
      Some("parsing"),
      "stopAfter should remain unchanged"
    );
    assert_eq!(
      sanitised.output_selection, settings.output_selection,
      "non-empty output selection should be preserved"
    );
  }

  #[test]
  fn merge_preserves_base_when_no_overrides() {
    let base = Settings::default();
    let merged = merge_settings(&base, None).expect("merge");
    assert_eq!(
      serde_json::to_value(&base).unwrap(),
      serde_json::to_value(&merged).unwrap()
    );
  }

  #[test]
  fn merge_replaces_output_selection_when_overridden() {
    let base = Settings::default();
    let mut overrides = CompilerSettingsOptions::default();
    let selection = OutputSelection::ast_output_selection();
    overrides.output_selection = Some(selection.as_ref().clone());

    let merged = merge_settings(&base, Some(&overrides)).expect("merge");
    assert_eq!(
      merged.output_selection, selection,
      "merge should replace base output selection with override"
    );
  }

  #[test]
  fn merge_applies_overrides() {
    let base = Settings::default();
    let mut overrides = CompilerSettingsOptions::default();
    overrides.stop_after = Some("parsing".to_string());
    overrides.remappings = Some(vec!["lib/=lib/".to_string()]);
    overrides.via_ir = Some(true);
    overrides.optimizer = Some(OptimizerSettingsOptions {
      enabled: Some(true),
      runs: Some(200),
      details: Some(OptimizerDetailsOptions {
        yul: Some(true),
        ..Default::default()
      }),
    });
    overrides.model_checker = Some(ModelCheckerSettingsOptions {
      engine: Some(ModelCheckerEngine::Bmc),
      timeout: Some(1),
      ..Default::default()
    });
    overrides.metadata = Some(SettingsMetadataOptions {
      use_literal_content: Some(true),
      bytecode_hash: Some(BytecodeHash::None),
      cbor_metadata: Some(false),
    });
    overrides.output_selection = Some(BTreeMap::from([(
      "Example.sol".to_string(),
      BTreeMap::from([("*".to_string(), vec!["abi".to_string()])]),
    )]));
    overrides.evm_version = Some(EvmVersion::Prague);
    overrides.debug = Some(DebuggingSettingsOptions {
      revert_strings: Some(RevertStrings::Debug),
      debug_info: vec!["location".to_string()],
    });
    overrides.libraries = Some(BTreeMap::from([(
      "Example.sol".to_string(),
      BTreeMap::from([(
        "LibExample".to_string(),
        "0x0000000000000000000000000000000000000001".to_string(),
      )]),
    )]));

    let merged = merge_settings(&base, Some(&overrides)).expect("merge");

    let as_json = serde_json::to_value(&merged).expect("serialize settings");

    assert!(merged
      .remappings
      .iter()
      .any(|remapping| remapping.to_string() == "lib/=lib/"));
    assert_eq!(as_json["stopAfter"], json!("parsing"));
    assert_eq!(as_json["viaIR"], json!(true));
    assert_eq!(as_json["optimizer"]["enabled"], json!(true));
    assert_eq!(as_json["optimizer"]["runs"], json!(200));
    assert_eq!(as_json["optimizer"]["details"]["yul"], json!(true));
    assert_eq!(as_json["metadata"]["useLiteralContent"], json!(true));
    assert_eq!(as_json["metadata"]["bytecodeHash"], json!("none"));
    assert_eq!(as_json["evmVersion"], json!("prague"));
    assert_eq!(as_json["debug"]["revertStrings"], json!("debug"));
    assert_eq!(as_json["debug"]["debugInfo"], json!(["location"]));
    assert_eq!(
      as_json["libraries"]["Example.sol"]["LibExample"],
      json!("0x0000000000000000000000000000000000000001")
    );
  }
}
